use async_trait::async_trait;

use crate::domain::{
    error::AppError,
    models::pagination::PageResult,
    models::system_admin::{ConfigRow, ConfigWriteCommand, IdResponse, KeywordQuery},
};

/// 系统管理的参数用例：参数配置列表、导出与维护。
#[async_trait]
pub trait ConfigAdminUseCase: Send + Sync {
    async fn list_configs(&self, query: &KeywordQuery) -> Result<PageResult<ConfigRow>, AppError>;

    async fn export_configs(&self, keyword: Option<&str>) -> Result<String, AppError>;

    async fn create_config(
        &self,
        command: ConfigWriteCommand,
        operator: &str,
    ) -> Result<IdResponse, AppError>;

    async fn update_config(
        &self,
        id: i64,
        command: ConfigWriteCommand,
        operator: &str,
    ) -> Result<(), AppError>;

    async fn delete_config(&self, id: i64) -> Result<(), AppError>;
}
