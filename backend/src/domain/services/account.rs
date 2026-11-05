use crate::domain::{
    error::AppError,
    models::account::{
        Enrollment, Profile, RecoveryCodes, SecurityProof, SessionPage, SessionRevocation,
        UpdateProfile,
    },
};
use async_trait::async_trait;

#[async_trait]
pub trait AccountUseCase: Send + Sync {
    async fn sessions(
        &self,
        username: &str,
        token: &str,
        cursor: u64,
    ) -> Result<SessionPage, AppError>;
    async fn revoke_session(
        &self,
        username: &str,
        token: &str,
        id: &str,
        proof: SecurityProof,
    ) -> Result<SessionRevocation, AppError>;
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
