use async_trait::async_trait;

use crate::domain::{
    error::AppError,
    models::monitor::{LoginHeatmap, LoginStats, OperStats, OperTimeline},
};

/// 监控日志的只读统计用例：概览、趋势、当日泳道与登录热力。
#[async_trait]
pub trait MonitorUseCase: Send + Sync {
    /// `range` 取 `24h`（缺省）/ `7d` / `30d`。
    async fn oper_stats(&self, range: Option<&str>) -> Result<OperStats, AppError>;

    async fn login_stats(&self, range: Option<&str>) -> Result<LoginStats, AppError>;

    /// `date` 为 `YYYY-MM-DD`，缺省取数据库当天。
    async fn oper_timeline(&self, date: Option<&str>) -> Result<OperTimeline, AppError>;

    /// `days` 取 7 或 30（缺省 7）。
    async fn login_heatmap(&self, days: Option<i64>) -> Result<LoginHeatmap, AppError>;
}
