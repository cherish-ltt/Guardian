use serde::Serialize;

/// 时间档：24h 按小时分桶，7d / 30d 按天分桶。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsRange {
    Last24Hours,
    Last7Days,
    Last30Days,
}

impl StatsRange {
    /// 未知取值按 `24h` 处理，与 Spring 版一致。
    pub fn parse(raw: Option<&str>) -> Self {
        match raw {
            Some("7d") => Self::Last7Days,
            Some("30d") => Self::Last30Days,
            _ => Self::Last24Hours,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Last24Hours => "24h",
            Self::Last7Days => "7d",
            Self::Last30Days => "30d",
        }
    }

    pub fn hourly(&self) -> bool {
        matches!(self, Self::Last24Hours)
    }

    pub fn buckets(&self) -> usize {
        match self {
            Self::Last24Hours => 24,
            Self::Last7Days => 7,
            Self::Last30Days => 30,
        }
    }

    /// 统计窗口的起始时刻：24h 档按当前时刻回退，其余两档从当天零点算起。
    pub fn window_start(&self, now: chrono::NaiveDateTime) -> chrono::NaiveDateTime {
        let date = match self {
            Self::Last24Hours => return now - chrono::Duration::hours(24),
            Self::Last7Days => now.date() - chrono::Duration::days(6),
            Self::Last30Days => now.date() - chrono::Duration::days(29),
        };

        date.and_hms_opt(0, 0, 0).unwrap_or(now)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsBucket {
    pub bucket: String,
    pub total: i64,
    pub failed: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperStats {
    pub range: String,
    pub total: i64,
    pub failed: i64,
    pub success_rate: f64,
    pub avg_cost: i64,
    pub p95_cost: i64,
    pub active_users: i64,
    pub series: Vec<StatsBucket>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginStats {
    pub range: String,
    pub total: i64,
    pub failed: i64,
    pub success_rate: f64,
    pub users: i64,
    pub ips: i64,
    pub series: Vec<StatsBucket>,
}

/// 当日泳道时间块：起止用当日秒数表示，前端落在 00:00–24:00 横轴上。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineBlock {
    pub start_sec: i64,
    pub end_sec: i64,
    pub count: i64,
    pub failed: i64,
    pub max_cost: i64,
    pub ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineLane {
    pub module: String,
    pub count: i64,
    pub blocks: Vec<TimelineBlock>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperTimeline {
    pub date: String,
    pub now_sec: Option<i64>,
    pub lanes: Vec<TimelineLane>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HeatmapCell {
    pub dow: i64,
    pub hour: i64,
    pub total: i64,
    pub failed: i64,
}

/// 失败聚合告警：`kind` 为 `ip` 时 `key` 是来源 IP，为 `user` 时是账号。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeatmapAnomaly {
    pub kind: String,
    pub key: String,
    pub failed: i64,
    pub total: i64,
    pub spread: i64,
    pub first_time: Option<String>,
    pub last_time: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginHeatmap {
    pub days: i64,
    pub since: String,
    pub threshold: i64,
    pub cells: Vec<HeatmapCell>,
    pub anomalies: Vec<HeatmapAnomaly>,
}

/// 一次操作的原始点：`sec` 为当日秒数。
#[derive(Debug, Clone)]
pub struct OperPoint {
    pub id: i64,
    pub module: Option<String>,
    pub sec: i64,
    pub result: i64,
    pub cost: i64,
}

#[cfg(test)]
mod tests {
    use super::StatsRange;

    #[test]
    fn range_parsing_falls_back_to_24h() {
        assert_eq!(StatsRange::parse(Some("7d")), StatsRange::Last7Days);
        assert_eq!(StatsRange::parse(Some("30d")), StatsRange::Last30Days);
        assert_eq!(StatsRange::parse(Some("1y")), StatsRange::Last24Hours);
        assert_eq!(StatsRange::parse(None), StatsRange::Last24Hours);
        assert_eq!(StatsRange::Last7Days.as_str(), "7d");
        assert_eq!(StatsRange::Last30Days.buckets(), 30);
        assert!(StatsRange::Last24Hours.hourly());
    }
}
