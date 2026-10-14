use async_trait::async_trait;

use crate::domain::{
    error::AppError,
    models::auth::{AccessToken, CurrentUser, DevLoginCommand},
};

#[async_trait]
pub trait AuthUseCase: Send + Sync {
    async fn issue_dev_token(&self, command: DevLoginCommand) -> Result<AccessToken, AppError>;

    async fn current_user_from_authorization(
        &self,
        authorization: &str,
    ) -> Result<CurrentUser, AppError>;
}
