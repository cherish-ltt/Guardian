use async_trait::async_trait;

use crate::domain::{
    error::AppError,
    models::pagination::PageResult,
    models::system_admin::{
        DictDataRow, DictDataWriteCommand, DictOption, DictTypeRow, DictTypeWriteCommand,
        IdResponse, StatusQuery,
    },
};

/// 系统管理的字典用例：字典类型、字典数据与业务下拉。
#[async_trait]
pub trait DictAdminUseCase: Send + Sync {
    async fn list_dict_types(
        &self,
        query: &StatusQuery,
    ) -> Result<PageResult<DictTypeRow>, AppError>;

    async fn export_dict_types(
        &self,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<String, AppError>;

    async fn create_dict_type(
        &self,
        command: DictTypeWriteCommand,
        operator: &str,
    ) -> Result<IdResponse, AppError>;

    async fn update_dict_type(
        &self,
        id: i64,
        command: DictTypeWriteCommand,
        operator: &str,
    ) -> Result<(), AppError>;

    async fn delete_dict_type(&self, id: i64) -> Result<(), AppError>;

    async fn list_dict_data(&self, dict_type: &str) -> Result<Vec<DictDataRow>, AppError>;

    /// 业务下拉：只要求已登录，不要求字典管理权限。
    async fn dict_options(&self, dict_type: &str) -> Result<Vec<DictOption>, AppError>;

    async fn create_dict_data(
        &self,
        command: DictDataWriteCommand,
        operator: &str,
    ) -> Result<IdResponse, AppError>;

    async fn update_dict_data(
        &self,
        id: i64,
        command: DictDataWriteCommand,
        operator: &str,
    ) -> Result<(), AppError>;

    async fn delete_dict_data(&self, id: i64) -> Result<(), AppError>;
}
