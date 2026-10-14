use async_trait::async_trait;

use crate::domain::{
    error::AppError,
    models::audit_log::{LoginLogQuery, LoginLogRow, OperLogQuery, OperLogRow},
    models::pagination::PageResult,
};

/// 系统管理的日志用例：操作日志与登录日志的查询、导出与清理。
#[async_trait]
pub trait LogAdminUseCase: Send + Sync {
    async fn list_oper_logs(
        &self,
        query: &OperLogQuery,
    ) -> Result<PageResult<OperLogRow>, AppError>;

    async fn export_oper_logs(
        &self,
        keyword: Option<&str>,
        result: Option<i64>,
    ) -> Result<String, AppError>;

    async fn delete_oper_log(&self, id: i64) -> Result<(), AppError>;

    async fn clear_oper_logs(&self) -> Result<(), AppError>;

    async fn list_login_logs(
        &self,
        query: &LoginLogQuery,
    ) -> Result<PageResult<LoginLogRow>, AppError>;

    async fn export_login_logs(
        &self,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<String, AppError>;

    async fn delete_login_log(&self, id: i64) -> Result<(), AppError>;

    async fn clear_login_logs(&self) -> Result<(), AppError>;
}
