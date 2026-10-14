use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use crate::{
    domain::{
        error::AppError, models::audit_log::OperationLogEntry, services::audit::OperationLogUseCase,
    },
    infrastructure::repositories::audit_log::{AuditLogRepository, SeaOrmAuditLogRepository},
};

pub struct AuditService {
    database: DatabaseConnection,
}

impl AuditService {
    pub fn new(database: DatabaseConnection) -> Self {
        Self { database }
    }
}

#[async_trait]
impl OperationLogUseCase for AuditService {
    async fn record(&self, entry: OperationLogEntry) -> Result<(), AppError> {
        SeaOrmAuditLogRepository
            .insert_oper_log(&self.database, &entry)
            .await
    }
}
