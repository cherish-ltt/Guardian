use async_trait::async_trait;

use crate::domain::{
    error::AppError,
    models::{
        auth::CurrentUser,
        example::{CreateExampleCommand, ExampleDetail, ExampleEcho, ExampleFilters, ExampleList},
    },
};

#[async_trait]
pub trait ExampleUseCase: Send + Sync {
    async fn create_echo(&self, command: CreateExampleCommand) -> Result<ExampleEcho, AppError>;

    async fn list_examples(&self, filters: ExampleFilters) -> Result<ExampleList, AppError>;

    async fn get_example_detail(
        &self,
        id: String,
        current_user: CurrentUser,
    ) -> Result<ExampleDetail, AppError>;
}
