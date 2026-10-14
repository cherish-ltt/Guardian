use async_trait::async_trait;

use crate::domain::{error::AppError, models::audit_log::OperationLogEntry};

/// 操作留痕用例：由审计中间件在写操作结束后调用。
#[async_trait]
pub trait OperationLogUseCase: Send + Sync {
    async fn record(&self, entry: OperationLogEntry) -> Result<(), AppError>;
}
