//! 监控页的 IP 属地：地址按需现查，不再随日志落库。

use serde::{Deserialize, Serialize};
#[cfg(feature = "docs")]
use utoipa::ToSchema;

/// 一次归属地查询的结果，字段名与前端一致。
#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpLocationItem {
    /// 请求里的原始地址（去首尾空白）。
    pub ip: String,
    /// 离线库给出的地名原文；地址非法或未收录时为空串。
    pub location: String,
}

/// 单条查询：`GET /monitor/ip-locations?ip=...`。
#[derive(Debug, Clone, Default)]
pub struct IpLocationQuery {
    pub ip: Option<String>,
}

/// 批量查询（整页）：`POST /monitor/ip-locations`。
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "docs", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IpLocationBatch {
    /// 整页的地址列表，返回顺序与之一致；最多 100 条（与分页上限一致）。
    pub ips: Vec<String>,
}
