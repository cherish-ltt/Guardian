use crate::domain::{
    error::AppError,
    models::account::{Enrollment, Profile, RecoveryCodes, SecurityProof, UpdateProfile},
};
use async_trait::async_trait;

#[async_trait]
pub trait AccountUseCase: Send + Sync {
    async fn profile(&self, username: &str) -> Result<Profile, AppError>;
    async fn update_profile(&self, username: &str, command: UpdateProfile) -> Result<(), AppError>;
    async fn setup(&self, username: &str, proof: SecurityProof) -> Result<Enrollment, AppError>;
    async fn enable(&self, username: &str, proof: SecurityProof)
    -> Result<RecoveryCodes, AppError>;
    async fn disable(&self, username: &str, proof: SecurityProof) -> Result<(), AppError>;
    async fn recovery_codes(
        &self,
        username: &str,
        proof: SecurityProof,
    ) -> Result<RecoveryCodes, AppError>;
    async fn reset(
        &self,
        operator: &str,
        target: i64,
        proof: SecurityProof,
    ) -> Result<(), AppError>;
}
