use serde::Serialize;

use crate::domain::models::system::{HealthReport, WelcomeInfo};

#[cfg(feature = "docs")]
use utoipa::ToSchema;

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Serialize)]
pub struct WelcomeResponse {
    #[cfg_attr(feature = "docs", schema(example = "guardian-backend"))]
    pub service_name: String,
    #[cfg_attr(feature = "docs", schema(example = "development"))]
    pub environment: String,
    // utoipa 的 example 不接受 env!/const 路径，只能写字面量：发版时需与 Cargo.toml version 同步。
    #[cfg_attr(feature = "docs", schema(example = "0.5.0"))]
    pub version: String,
    #[cfg_attr(feature = "docs", schema(example = true))]
    pub docs_enabled: bool,
}

impl From<WelcomeInfo> for WelcomeResponse {
    fn from(value: WelcomeInfo) -> Self {
        Self {
            service_name: value.service_name,
            environment: value.environment,
            version: value.version,
            docs_enabled: value.docs_enabled,
        }
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    #[cfg_attr(feature = "docs", schema(example = "guardian-backend"))]
    pub service_name: String,
    #[cfg_attr(feature = "docs", schema(example = "development"))]
    pub environment: String,
    // utoipa 的 example 不接受 env!/const 路径，只能写字面量：发版时需与 Cargo.toml version 同步。
    #[cfg_attr(feature = "docs", schema(example = "0.5.0"))]
    pub version: String,
    #[cfg_attr(feature = "docs", schema(example = "ok"))]
    pub status: String,
    #[cfg_attr(feature = "docs", schema(example = "not_checked"))]
    pub database_status: String,
    #[cfg_attr(feature = "docs", schema(example = 1713179523000i64))]
    pub timestamp: i64,
}

impl From<HealthReport> for HealthResponse {
    fn from(value: HealthReport) -> Self {
        Self {
            service_name: value.service_name,
            environment: value.environment,
            version: value.version,
            status: value.status,
            database_status: value.database_status,
            timestamp: value.timestamp,
        }
    }
}
