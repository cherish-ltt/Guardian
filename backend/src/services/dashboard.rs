use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{Duration, Local, NaiveDate, Timelike};
use sea_orm::DatabaseConnection;

use crate::{
    domain::{
        error::AppError,
        models::dashboard::{DailyOps, DashboardStat, DashboardSummary},
        services::dashboard::DashboardUseCase,
    },
    infrastructure::repositories::dashboard::{DashboardRepository, SeaOrmDashboardRepository},
};

/// 24h 档按小时分桶，7d / 30d 档按天分桶。
const HOURS_24: i64 = 24;
const HOURS_7D: i64 = 24 * 7;
const HOURS_30D: i64 = 24 * 30;
/// 近 7 天按日操作量的天数。
const DAILY_OPS_DAYS: i64 = 7;

pub struct DashboardService {
    database: DatabaseConnection,
}

impl DashboardService {
    pub fn new(database: DatabaseConnection) -> Self {
        Self { database }
    }
}

#[async_trait]
impl DashboardUseCase for DashboardService {
    async fn summary(&self, range: Option<&str>) -> Result<DashboardSummary, AppError> {
        let hours = match range {
            Some("7d") => HOURS_7D,
            Some("30d") => HOURS_30D,
            _ => HOURS_24,
        };
        let repository = SeaOrmDashboardRepository;
        let counters = repository.counters(&self.database).await?;

        let stats = vec![
            stat(
                "users",
                counters.users,
                delta_ratio(counters.users, counters.users_new),
            ),
            stat(
                "online",
                counters.online,
                delta_ratio(counters.online, counters.online_2h),
            ),
            stat(
                "roles",
                counters.roles,
                delta_ratio(counters.roles, counters.roles_new),
            ),
            stat(
                "api",
                counters.api_calls,
                delta_compare(counters.api_calls, counters.api_prev),
            ),
            stat("menus", counters.menus, delta_ratio(1, 0)),
            stat("depts", counters.depts, delta_ratio(1, 0)),
        ];
        let (trend_total, success_calls) =
            repository.api_calls_in_hours(&self.database, hours).await?;

        Ok(DashboardSummary {
            stats,
            trend: self.trend(hours).await?,
            trend_prev: repository.previous_logins(&self.database, hours).await?,
            daily_ops: self.daily_ops().await?,
            activities: repository.activities(&self.database).await?,
            role_dist: repository.role_distribution(&self.database).await?,
            trend_total,
            success_rate: if trend_total > 0 {
                (success_calls as f64 * 1000.0 / trend_total as f64).round() / 10.0
            } else {
                100.0
            },
        })
    }
}

impl DashboardService {
    /// 趋势：24h 档按「当前小时往前 24 格」对齐，更长档按天补齐。
    async fn trend(&self, hours: i64) -> Result<Vec<i64>, AppError> {
        let repository = SeaOrmDashboardRepository;

        if hours <= HOURS_24 {
            let by_hour = repository
                .login_trend_hours(&self.database)
                .await?
                .into_iter()
                .collect::<HashMap<_, _>>();
            let now = Local::now().naive_local();

            return Ok((0..HOURS_24)
                .rev()
                .map(|offset| {
                    let hour = i64::from((now - Duration::hours(offset)).hour());

                    by_hour.get(&hour).copied().unwrap_or(0)
                })
                .collect());
        }

        let days = hours / 24;
        let by_day = repository
            .login_trend_days(&self.database, days)
            .await?
            .into_iter()
            .collect::<HashMap<_, _>>();

        Ok((0..days)
            .map(|index| by_day.get(&index).copied().unwrap_or(0))
            .collect())
    }

    /// 近 7 天按日操作量：数据库只回传有数据的日期，这里补零成固定 7 天。
    async fn daily_ops(&self) -> Result<Vec<DailyOps>, AppError> {
        let repository = SeaOrmDashboardRepository;
        let by_day = repository
            .daily_ops(&self.database)
            .await?
            .into_iter()
            .map(|(index, total, failed)| (index, (total, failed)))
            .collect::<HashMap<_, _>>();
        let today = repository
            .today(&self.database)
            .await?
            .parse::<NaiveDate>()
            .map_err(|_| AppError::internal("数据库日期格式异常"))?;

        Ok((0..DAILY_OPS_DAYS)
            .map(|index| {
                let (total, failed) = by_day.get(&index).copied().unwrap_or((0, 0));

                DailyOps {
                    date: (today - Duration::days(DAILY_OPS_DAYS - 1 - index))
                        .format("%Y-%m-%d")
                        .to_string(),
                    total,
                    failed,
                }
            })
            .collect())
    }
}

/// 环比读数：占比、是否上升、上一周期为空时是否算「新增」。
type Delta = (f64, bool, bool);

fn stat(key: &str, value: i64, delta: Delta) -> DashboardStat {
    DashboardStat {
        key: key.to_string(),
        label: stat_label(key).to_string(),
        value,
        icon: stat_icon(key).to_string(),
        tone: stat_tone(key).to_string(),
        delta: delta.0,
        up: delta.1,
        fresh: delta.2,
        sub: sub_text(key).to_string(),
    }
}

/// 固定文案按统计键取，避免六张卡把同一组常量抄在调用点上。
fn stat_label(key: &str) -> &'static str {
    match key {
        "users" => "用户总数",
        "online" => "在线用户",
        "roles" => "角色数量",
        "api" => "24h 接口调用",
        "menus" => "菜单项",
        "depts" => "部门数",
        _ => "",
    }
}

fn stat_icon(key: &str) -> &'static str {
    match key {
        "users" => "users",
        "online" => "monitor",
        "roles" => "shield",
        "api" => "command",
        "menus" => "menu",
        "depts" => "dept",
        _ => "",
    }
}

fn stat_tone(key: &str) -> &'static str {
    match key {
        "users" | "depts" => "ok",
        "online" | "menus" => "info",
        "roles" => "warn",
        "api" => "purple",
        _ => "",
    }
}

/// 新增占比：`recent` 占 `total` 的比例。
fn delta_ratio(total: i64, recent: i64) -> Delta {
    if total == 0 {
        return (0.0, false, false);
    }

    let percent = (recent as f64 * 1000.0 / total.max(1) as f64).round() / 10.0;

    (percent, true, false)
}

/// 环比：上一周期为 0 时无法计算，标记为「新增」，避免出现误导的 +100%。
fn delta_compare(current: i64, previous: i64) -> Delta {
    if previous == 0 {
        return (0.0, current > 0, current > 0);
    }

    let percent = ((current - previous) as f64 * 1000.0 / previous as f64).round() / 10.0;

    (percent.abs(), percent >= 0.0, false)
}

fn sub_text(key: &str) -> &'static str {
    match key {
        "users" => "当前启用账号",
        "online" => "最近 1 小时登录",
        "roles" => "已启用角色",
        "api" => "最近 24 小时操作",
        "menus" => "目录 / 菜单 / 按钮",
        "depts" => "未删除部门",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::{delta_compare, delta_ratio, sub_text};

    #[test]
    fn delta_ratio_marks_up_when_total_is_known() {
        assert_eq!(delta_ratio(0, 5), (0.0, false, false));
        assert_eq!(delta_ratio(1, 0), (0.0, true, false));
        assert_eq!(delta_ratio(4, 1), (25.0, true, false));
    }

    #[test]
    fn delta_compare_flags_fresh_when_previous_is_empty() {
        assert_eq!(delta_compare(0, 0), (0.0, false, false));
        assert_eq!(delta_compare(3, 0), (0.0, true, true));
        assert_eq!(delta_compare(30, 15), (100.0, true, false));
        assert_eq!(delta_compare(15, 30), (50.0, false, false));
    }

    #[test]
    fn sub_text_matches_spring_wording() {
        assert_eq!(sub_text("users"), "当前启用账号");
        assert_eq!(sub_text("api"), "最近 24 小时操作");
        assert_eq!(sub_text("unknown"), "");
    }

    /// 未知统计键取空文案而不是 panic：新增卡片时忘配文案也不会把工作台带崩。
    #[test]
    fn unknown_stat_key_falls_back_to_empty_wording() {
        assert_eq!(super::stat_label("unknown"), "");
        assert_eq!(super::stat_icon("unknown"), "");
        assert_eq!(super::stat_tone("unknown"), "");
    }
}
