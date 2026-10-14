use async_trait::async_trait;

use crate::domain::{
    error::AppError,
    models::pagination::PageResult,
    models::system_admin::{IdResponse, RoleRow, RoleWriteCommand, StatusQuery},
};

/// 系统管理的角色用例：角色维护与权限关系图。
#[async_trait]
pub trait RoleAdminUseCase: Send + Sync {
    async fn list_roles(&self, query: &StatusQuery) -> Result<PageResult<RoleRow>, AppError>;

    async fn export_roles(
        &self,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<String, AppError>;

    async fn role_menus(&self, role_id: i64) -> Result<Vec<i64>, AppError>;

    async fn create_role(
        &self,
        command: RoleWriteCommand,
        operator: &str,
    ) -> Result<IdResponse, AppError>;

    async fn update_role(
        &self,
        id: i64,
        command: RoleWriteCommand,
        operator: &str,
    ) -> Result<(), AppError>;

    async fn delete_role(&self, id: i64, operator: &str) -> Result<(), AppError>;
}
