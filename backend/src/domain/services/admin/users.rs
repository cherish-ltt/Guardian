use async_trait::async_trait;

use crate::domain::{
    error::AppError,
    models::identity::DataScope,
    models::pagination::PageResult,
    models::system_admin::{IdResponse, UserOptions, UserQuery, UserRow, UserWriteCommand},
};

/// 系统管理的用户用例：用户列表、导出与账户维护。
#[async_trait]
pub trait UserAdminUseCase: Send + Sync {
    async fn user_options(
        &self,
        can_edit: bool,
        can_set_password: bool,
    ) -> Result<UserOptions, AppError>;

    async fn list_users(
        &self,
        query: &UserQuery,
        scope: &DataScope,
    ) -> Result<PageResult<UserRow>, AppError>;

    async fn export_users(&self, query: &UserQuery, scope: &DataScope) -> Result<String, AppError>;

    async fn create_user(
        &self,
        command: UserWriteCommand,
        operator: &str,
        operator_is_super: bool,
    ) -> Result<IdResponse, AppError>;

    async fn update_user(
        &self,
        id: i64,
        command: UserWriteCommand,
        operator: &str,
        operator_is_super: bool,
    ) -> Result<(), AppError>;

    async fn update_user_status(
        &self,
        id: i64,
        status: Option<i64>,
        operator: &str,
    ) -> Result<(), AppError>;

    async fn reset_user_password(
        &self,
        id: i64,
        password: Option<String>,
        operator: &str,
    ) -> Result<(), AppError>;

    async fn delete_user(&self, id: i64, operator: &str) -> Result<(), AppError>;
}
