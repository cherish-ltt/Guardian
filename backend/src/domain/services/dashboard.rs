use async_trait::async_trait;

use crate::domain::{error::AppError, models::dashboard::DashboardSummary};

/// 工作台汇总用例。
#[async_trait]
pub trait DashboardUseCase: Send + Sync {
    /// `range` 取 `24h`（缺省）/ `7d` / `30d`。
    async fn summary(&self, range: Option<&str>) -> Result<DashboardSummary, AppError>;
}
