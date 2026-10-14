use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{Datelike, NaiveDateTime, Timelike};
use sea_orm::DatabaseConnection;
use tracing::warn;

use crate::{
    domain::{
        error::AppError,
        models::monitor::{
            HeatmapAnomaly, HeatmapCell, LoginHeatmap, LoginStats, OperStats, OperTimeline,
            StatsBucket, StatsRange, TimelineBlock, TimelineLane,
        },
        services::monitor::MonitorUseCase,
    },
    infrastructure::repositories::monitor::{
        AnomalyRow, BucketCount, MonitorRepository, SeaOrmMonitorRepository,
    },
    util::time,
};

/// 同一模块两次操作间隔不超过 15 分钟时合并为一个时间块。
const MERGE_GAP_SECONDS: i64 = 15 * 60;
/// 每个时间块最多回传的日志 ID 数，供前端定位明细。
const BLOCK_ID_LIMIT: usize = 50;
/// 登录失败告警阈值，可由参数 `sys.login.alertThreshold` 覆盖。
const DEFAULT_ALERT_THRESHOLD: i64 = 5;

pub struct MonitorService {
    database: DatabaseConnection,
}

impl MonitorService {
    pub fn new(database: DatabaseConnection) -> Self {
        Self { database }
    }

    /// 数据库时钟：统计窗口与分桶标签都以它为准。
    async fn db_now(&self) -> Result<NaiveDateTime, AppError> {
        let raw = SeaOrmMonitorRepository.db_now(&self.database).await?;

        time::parse_datetime(&raw).ok_or_else(|| AppError::internal("数据库时间不可解析"))
    }
}

#[async_trait]
impl MonitorUseCase for MonitorService {
    async fn oper_stats(&self, range: Option<&str>) -> Result<OperStats, AppError> {
        let range = StatsRange::parse(range);
        let now = self.db_now().await?;
        let since = range.window_start(now);
        let (since_text, now_text) = (time::format_datetime(since), time::format_datetime(now));

        let repository = SeaOrmMonitorRepository;
        let overview = repository
            .oper_overview(&self.database, &since_text)
            .await?;
        let p95_cost = if overview.cost_count == 0 {
            0
        } else {
            let offset = (overview.cost_count as f64 * 0.95).ceil() as i64 - 1;
            repository
                .oper_cost_at(&self.database, &since_text, offset)
                .await?
                .unwrap_or(0)
        };
        let rows = if range.hourly() {
            repository
                .oper_hourly(&self.database, &since_text, &now_text)
                .await?
        } else {
            repository
                .oper_daily(&self.database, &since_text, &now_text)
                .await?
        };

        Ok(OperStats {
            range: range.as_str().to_string(),
            total: overview.total,
            failed: overview.failed,
            success_rate: success_rate(overview.total, overview.failed),
            avg_cost: overview.avg_cost,
            p95_cost,
            active_users: overview.active_users,
            series: series_of(range, now, rows),
        })
    }

    async fn login_stats(&self, range: Option<&str>) -> Result<LoginStats, AppError> {
        let range = StatsRange::parse(range);
        let now = self.db_now().await?;
        let since = range.window_start(now);
        let (since_text, now_text) = (time::format_datetime(since), time::format_datetime(now));

        let repository = SeaOrmMonitorRepository;
        let overview = repository
            .login_overview(&self.database, &since_text)
            .await?;
        let rows = if range.hourly() {
            repository
                .login_hourly(&self.database, &since_text, &now_text)
                .await?
        } else {
            repository
                .login_daily(&self.database, &since_text, &now_text)
                .await?
        };

        Ok(LoginStats {
            range: range.as_str().to_string(),
            total: overview.total,
            failed: overview.failed,
            success_rate: success_rate(overview.total, overview.failed),
            users: overview.users,
            ips: overview.ips,
            series: series_of(range, now, rows),
        })
    }

    async fn oper_timeline(&self, date: Option<&str>) -> Result<OperTimeline, AppError> {
        let now = self.db_now().await?;
        let day = match date {
            Some(raw) => time::parse_date(raw)
                .ok_or_else(|| AppError::business("date 格式应为 yyyy-MM-dd"))?,
            None => now.date(),
        };
        let start = time::format_datetime(time::date_at_midnight(day));

        let points = SeaOrmMonitorRepository
            .oper_points_of_day(&self.database, &start)
            .await?;

        let mut lanes: Vec<(String, Vec<TimelineBlock>)> = Vec::new();
        let mut lane_index: HashMap<String, usize> = HashMap::new();
        for point in points {
            let module = point.module.clone().unwrap_or_default();
            let index = *lane_index.entry(module.clone()).or_insert_with(|| {
                lanes.push((module.clone(), Vec::new()));
                lanes.len() - 1
            });
            let blocks = &mut lanes[index].1;
            let open_new = blocks
                .last()
                .is_none_or(|block| point.sec - block.end_sec > MERGE_GAP_SECONDS);
            if open_new {
                blocks.push(TimelineBlock {
                    start_sec: point.sec,
                    end_sec: point.sec,
                    count: 0,
                    failed: 0,
                    max_cost: 0,
                    ids: Vec::new(),
                });
            }
            if let Some(block) = blocks.last_mut() {
                block.end_sec = point.sec;
                block.count += 1;
                if point.result == 0 {
                    block.failed += 1;
                }
                block.max_cost = block.max_cost.max(point.cost);
                if block.ids.len() < BLOCK_ID_LIMIT {
                    block.ids.push(point.id);
                }
            }
        }

        let mut lane_rows = lanes
            .into_iter()
            .map(|(module, blocks)| TimelineLane {
                count: blocks.iter().map(|block| block.count).sum(),
                module,
                blocks,
            })
            .collect::<Vec<_>>();
        // 稳定排序：操作量相同的模块保持首次出现的顺序，与 Spring 版一致。
        lane_rows.sort_by_key(|lane| std::cmp::Reverse(lane.count));

        Ok(OperTimeline {
            date: day.format("%Y-%m-%d").to_string(),
            now_sec: (day == now.date()).then(time::seconds_since_midnight),
            lanes: lane_rows,
        })
    }

    async fn login_heatmap(&self, days: Option<i64>) -> Result<LoginHeatmap, AppError> {
        let days = days.unwrap_or(7);
        let span = if days >= 30 { 30 } else { 7 };
        let now = self.db_now().await?;
        let since = time::date_at_midnight(now.date() - chrono::Duration::days(span - 1));
        let since_text = time::format_datetime(since);

        let repository = SeaOrmMonitorRepository;
        let configured = repository
            .login_alert_threshold(&self.database)
            .await
            .unwrap_or_else(|error| {
                warn!(%error, "读取登录失败告警阈值失败，沿用默认值");
                None
            });
        let threshold = configured
            .as_deref()
            .map(str::trim)
            .and_then(|value| value.parse::<i64>().ok())
            .map_or(DEFAULT_ALERT_THRESHOLD, |value| value.max(1));

        let cells = repository
            .login_heatmap(&self.database, &since_text)
            .await?
            .into_iter()
            .map(|point| HeatmapCell {
                dow: point.dow,
                hour: point.hour,
                total: point.total,
                failed: point.failed,
            })
            .collect::<Vec<_>>();
        let mut anomalies = repository
            .login_ip_anomalies(&self.database, &since_text, threshold)
            .await?
            .into_iter()
            .map(|row| anomaly("ip", row))
            .collect::<Vec<_>>();
        anomalies.extend(
            repository
                .login_user_anomalies(&self.database, &since_text, threshold)
                .await?
                .into_iter()
                .map(|row| anomaly("user", row)),
        );
        anomalies.sort_by_key(|item| std::cmp::Reverse(item.failed));

        Ok(LoginHeatmap {
            days: span,
            since: since_text,
            threshold,
            cells,
            anomalies,
        })
    }
}

fn anomaly(kind: &str, row: AnomalyRow) -> HeatmapAnomaly {
    HeatmapAnomaly {
        kind: kind.to_string(),
        key: row.key,
        failed: row.failed,
        total: row.total,
        spread: row.spread,
        first_time: row.first_time,
        last_time: row.last_time,
    }
}

/// 按窗口长度补零：数据库只回传有数据的分桶，这里补齐成固定长度的序列。
fn series_of(range: StatsRange, now: NaiveDateTime, rows: Vec<BucketCount>) -> Vec<StatsBucket> {
    let buckets = range.buckets() as i64;
    let mut by_ago = rows
        .into_iter()
        .map(|row| (row.ago, row))
        .collect::<HashMap<_, _>>();

    (0..buckets)
        .map(|index| {
            let ago = buckets - 1 - index;
            let bucket = if range.hourly() {
                format!("{:02}:00", (now - chrono::Duration::hours(ago)).hour())
            } else {
                let date = now.date() - chrono::Duration::days(ago);

                format!("{:02}-{:02}", date.month(), date.day())
            };
            let row = by_ago.remove(&ago);

            StatsBucket {
                bucket,
                total: row.as_ref().map_or(0, |row| row.total),
                failed: row.as_ref().map_or(0, |row| row.failed),
            }
        })
        .collect()
}

/// 成功率保留一位小数；没有数据时按 100% 处理。
fn success_rate(total: i64, failed: i64) -> f64 {
    if total > 0 {
        ((total - failed) as f64 * 1000.0 / total as f64).round() / 10.0
    } else {
        100.0
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::{StatsRange, series_of, success_rate};
    use crate::infrastructure::repositories::monitor::BucketCount;

    #[test]
    fn success_rate_keeps_one_decimal() {
        assert_eq!(success_rate(0, 0), 100.0);
        assert_eq!(success_rate(3, 1), 66.7);
        assert_eq!(success_rate(100, 0), 100.0);
        assert_eq!(success_rate(100, 100), 0.0);
    }

    #[test]
    fn series_is_padded_and_labelled_from_oldest_to_newest() {
        let now = NaiveDate::from_ymd_opt(2024, 3, 7)
            .and_then(|date| date.and_hms_opt(9, 30, 0))
            .expect("固定时间");

        let daily = series_of(
            StatsRange::Last7Days,
            now,
            vec![BucketCount {
                ago: 0,
                total: 4,
                failed: 1,
            }],
        );
        assert_eq!(daily.len(), 7);
        assert_eq!(daily[0].bucket, "03-01");
        assert_eq!(daily[6].bucket, "03-07");
        assert_eq!(daily[6].total, 4);
        assert_eq!(daily[6].failed, 1);
        assert_eq!(daily[0].total, 0);

        let hourly = series_of(StatsRange::Last24Hours, now, vec![]);
        assert_eq!(hourly.len(), 24);
        assert_eq!(hourly[23].bucket, "09:00");
        assert_eq!(hourly[0].bucket, "10:00");
    }
}
