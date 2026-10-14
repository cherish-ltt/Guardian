use async_trait::async_trait;

use crate::domain::{
    error::AppError,
    models::system_admin::{DeptNode, DeptWriteCommand, IdResponse, StatusQuery},
};

/// 系统管理的部门用例：部门树、导出与维护。
#[async_trait]
pub trait DeptAdminUseCase: Send + Sync {
    async fn list_depts(&self, query: &StatusQuery) -> Result<Vec<DeptNode>, AppError>;

    async fn export_depts(
        &self,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<String, AppError>;

    async fn create_dept(
        &self,
        command: DeptWriteCommand,
        operator: &str,
    ) -> Result<IdResponse, AppError>;

    async fn update_dept(
        &self,
        id: i64,
        command: DeptWriteCommand,
        operator: &str,
    ) -> Result<(), AppError>;

    async fn delete_dept(&self, id: i64, operator: &str) -> Result<(), AppError>;
}
