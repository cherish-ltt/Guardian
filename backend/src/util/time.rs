//! 数据库时间的唯一表示：`YYYY-MM-DD HH:MM:SS` 的本地时间文本。
//!
//! DATETIME 列在 SQLite 里存 TEXT，MySQL 版的 `NOW()` / `DATE_FORMAT()` 语义
//! 因此变成「可直接比较、可直接截断的字符串」，服务端与数据库时钟不再有偏移。

use chrono::{Local, NaiveDate, NaiveDateTime, Timelike};

pub const DB_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// 当前时间（本地时区），与 MySQL `NOW()` 的用法一致。
pub fn now() -> String {
    format_datetime(Local::now().naive_local())
}

/// 当前时间的「当天秒数」，泳道用它定位当前时刻。
pub fn seconds_since_midnight() -> i64 {
    let time = Local::now().time();

    i64::from(time.hour()) * 3600 + i64::from(time.minute()) * 60 + i64::from(time.second())
}

pub fn format_datetime(value: NaiveDateTime) -> String {
    value.format(DB_FORMAT).to_string()
}

pub fn parse_datetime(value: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value, DB_FORMAT).ok()
}

/// 解析 `yyyy-MM-dd`（泳道接口的 `date` 参数）。
pub fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

pub fn date_at_midnight(date: NaiveDate) -> NaiveDateTime {
    date.and_hms_opt(0, 0, 0).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{now, parse_date, parse_datetime};

    #[test]
    fn now_is_parseable_and_second_precise() {
        let value = now();
        assert_eq!(value.len(), 19);
        assert!(parse_datetime(&value).is_some());
    }

    #[test]
    fn invalid_values_are_rejected() {
        assert!(parse_datetime("2026/01/01 00:00:00").is_none());
        assert!(parse_date("2026-13-40").is_none());
        assert!(parse_date("2026-01-01").is_some());
    }

    /// 时间文本按字典序比较等价于按时间先后比较，这是统计区间能直接用字符串边界的前提。
    #[test]
    fn text_comparison_matches_time_order() {
        assert!("2026-01-01 09:00:00" < "2026-01-01 10:00:00");
        assert!("2026-01-02 10:00:00" > "2026-01-01 23:59:59");
    }
}
