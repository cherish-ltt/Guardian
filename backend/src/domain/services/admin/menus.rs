use async_trait::async_trait;

use crate::domain::{
    error::AppError,
    models::system_admin::{IdResponse, MenuNode, MenuWriteCommand, StatusQuery},
};

/// 系统管理的菜单用例：菜单树、导出与维护。
#[async_trait]
pub trait MenuAdminUseCase: Send + Sync {
    async fn list_menus(&self, query: &StatusQuery) -> Result<Vec<MenuNode>, AppError>;

    async fn export_menus(
        &self,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<String, AppError>;

    async fn create_menu(
        &self,
        command: MenuWriteCommand,
        operator: &str,
    ) -> Result<IdResponse, AppError>;

    async fn update_menu(
        &self,
        id: i64,
        command: MenuWriteCommand,
        operator: &str,
    ) -> Result<(), AppError>;

    async fn delete_menu(&self, id: i64, operator: &str) -> Result<(), AppError>;
}
