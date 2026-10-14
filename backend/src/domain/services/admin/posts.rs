use async_trait::async_trait;

use crate::domain::{
    error::AppError,
    models::pagination::PageResult,
    models::system_admin::{IdResponse, PostRow, PostWriteCommand, StatusQuery},
};

/// 系统管理的岗位用例：岗位列表、导出与维护。
#[async_trait]
pub trait PostAdminUseCase: Send + Sync {
    async fn list_posts(&self, query: &StatusQuery) -> Result<PageResult<PostRow>, AppError>;

    async fn export_posts(
        &self,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<String, AppError>;

    async fn create_post(
        &self,
        command: PostWriteCommand,
        operator: &str,
    ) -> Result<IdResponse, AppError>;

    async fn update_post(
        &self,
        id: i64,
        command: PostWriteCommand,
        operator: &str,
    ) -> Result<(), AppError>;

    async fn delete_post(&self, id: i64, operator: &str) -> Result<(), AppError>;
}
