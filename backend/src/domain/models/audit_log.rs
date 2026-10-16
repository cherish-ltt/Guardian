use serde::Serialize;

use crate::domain::models::{identity::ClientSource, pagination::PageQuery};

/// 操作日志列表查询。
#[derive(Debug, Clone, Default)]
pub struct OperLogQuery {
    pub page: PageQuery,
    pub keyword: Option<String>,
    pub result: Option<i64>,
}

/// 登录日志列表查询。
#[derive(Debug, Clone, Default)]
pub struct LoginLogQuery {
    pub page: PageQuery,
    pub keyword: Option<String>,
    pub status: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperLogRow {
    pub id: i64,
    pub module: String,
    pub action: String,
    pub method: Option<String>,
    pub url: Option<String>,
    pub params: Option<String>,
    pub oper_name: Option<String>,
    pub dept: Option<String>,
    pub ip: Option<String>,
    pub location: Option<String>,
    pub cost: Option<i64>,
    pub result: i64,
    pub error_msg: Option<String>,
    pub oper_time: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginLogRow {
    pub id: i64,
    pub username: Option<String>,
    pub ip: Option<String>,
    pub location: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub status: i64,
    pub msg: Option<String>,
    pub login_time: Option<String>,
}

/// 一次登录 / 退出的留痕，字段与 `sys_login_log` 对齐。
#[derive(Debug, Clone)]
pub struct LoginLogEntry {
    pub username: String,
    pub status: i64,
    pub message: String,
    pub source: ClientSource,
}

/// 一次写操作留痕，字段与 Spring 版 `OperationLogEntry` 对齐。
#[derive(Debug, Clone)]
pub struct OperationLogEntry {
    pub module: String,
    pub action: String,
    pub request_method: Option<String>,
    pub request_url: Option<String>,
    pub request_params: Option<String>,
    pub operator: Option<String>,
    pub ip: Option<String>,
    pub cost_ms: i64,
    pub result: i64,
    pub error_message: Option<String>,
}

impl OperationLogEntry {
    /// 请求参数与异常信息各截断到 4000 字符，与 Spring 版一致。
    pub const MAX_TEXT_LENGTH: usize = 4_000;

    pub fn truncate(value: Option<String>) -> Option<String> {
        value.map(|value| {
            if value.chars().count() <= Self::MAX_TEXT_LENGTH {
                value
            } else {
                value.chars().take(Self::MAX_TEXT_LENGTH).collect()
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::OperationLogEntry;

    #[test]
    fn truncate_keeps_short_text_and_caps_long_text() {
        let short = "订单创建失败".to_string();
        assert_eq!(
            OperationLogEntry::truncate(Some(short.clone())),
            Some(short.clone())
        );

        let long = "错".repeat(OperationLogEntry::MAX_TEXT_LENGTH + 5);
        let truncated = OperationLogEntry::truncate(Some(long)).expect("有值时返回 Some");
        assert_eq!(
            truncated.chars().count(),
            OperationLogEntry::MAX_TEXT_LENGTH,
            "超长文本截断到上限"
        );
        assert!(
            truncated.chars().all(|value| value == '错'),
            "不混入其他字符"
        );

        assert_eq!(OperationLogEntry::truncate(None), None);
    }

    /// 截断按字符计数：多字节字符数以字符为准，不能按字节切。
    #[test]
    fn truncate_counts_characters_not_bytes() {
        let long = "中文".repeat(OperationLogEntry::MAX_TEXT_LENGTH);
        let truncated = OperationLogEntry::truncate(Some(long)).expect("有值时返回 Some");

        assert_eq!(
            truncated.chars().count(),
            OperationLogEntry::MAX_TEXT_LENGTH
        );
        assert!(
            truncated
                .chars()
                .all(|value| value == '中' || value == '文')
        );
    }
}
