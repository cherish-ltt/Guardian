use async_trait::async_trait;

use crate::domain::{
    error::AppError,
    models::system::{HealthReport, WelcomeInfo},
};

#[async_trait]
pub trait SystemUseCase: Send + Sync {
    async fn welcome(&self) -> Result<WelcomeInfo, AppError>;

    async fn health(&self) -> Result<HealthReport, AppError>;

    async fn ready(&self) -> Result<(), AppError>;
}
