use super::{AccountService, security::SecurityAction};
use crate::{
    domain::{
        error::AppError,
        models::account::{Enrollment, Profile, RecoveryCodes, SecurityProof, UpdateProfile},
        services::account::AccountUseCase,
    },
    infrastructure::repositories::account::AccountRepository,
};
use async_trait::async_trait;

#[async_trait]
impl AccountUseCase for AccountService {
    async fn profile(&self, username: &str) -> Result<Profile, AppError> {
        let mut profile = AccountRepository::profile(&self.database, username).await?;
        use crate::infrastructure::repositories::admin_auth::{
            AdminAuthRepository, SeaOrmAdminAuthRepository,
        };
        let user = SeaOrmAdminAuthRepository
            .find_user_by_username(&self.database, username)
            .await?
            .ok_or_else(|| AppError::session_expired("未认证或登录已过期"))?;
        profile.roles = SeaOrmAdminAuthRepository
            .role_codes(&self.database, user.id)
            .await?;
        let state = AccountRepository::security(&self.database, user.id).await?;
        profile.two_factor_enabled = state.secret.is_some();
        profile.recovery_codes_remaining =
            serde_json::from_str::<Vec<String>>(&state.recovery_hashes)
                .map_err(|_| AppError::internal("invalid recovery code state"))?
                .len();
        Ok(profile)
    }

    async fn update_profile(
        &self,
        username: &str,
        mut command: UpdateProfile,
    ) -> Result<(), AppError> {
        command.nickname = command.nickname.trim().to_string();
        if command.nickname.is_empty() || command.nickname.chars().count() > 64 {
            return Err(AppError::business("昵称长度必须在1到64位之间"));
        }
        command.phone = command
            .phone
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        command.email = command
            .email
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        if command.phone.as_ref().is_some_and(|v| {
            v.len() > 32 || !v.chars().all(|c| c.is_ascii_digit() || "+- ()".contains(c))
        }) {
            return Err(AppError::business("手机号码格式不正确"));
        }
        if command.email.as_ref().is_some_and(|v| {
            v.len() > 254
                || v.contains(char::is_whitespace)
                || !v
                    .split_once('@')
                    .is_some_and(|(a, b)| !a.is_empty() && !b.is_empty() && !b.contains('@'))
        }) {
            return Err(AppError::business("邮箱格式不正确"));
        }
        AccountRepository::update_profile(&self.database, username, command).await
    }

    async fn setup(&self, username: &str, proof: SecurityProof) -> Result<Enrollment, AppError> {
        self.start_enrollment(username, proof).await
    }
    async fn enable(
        &self,
        username: &str,
        proof: SecurityProof,
    ) -> Result<RecoveryCodes, AppError> {
        self.change_security(username, proof, SecurityAction::Enable)
            .await
    }
    async fn disable(&self, username: &str, proof: SecurityProof) -> Result<(), AppError> {
        self.change_security(username, proof, SecurityAction::Disable)
            .await?;
        Ok(())
    }
    async fn recovery_codes(
        &self,
        username: &str,
        proof: SecurityProof,
    ) -> Result<RecoveryCodes, AppError> {
        self.change_security(username, proof, SecurityAction::Recovery)
            .await
    }
    async fn reset(
        &self,
        operator: &str,
        target: i64,
        proof: SecurityProof,
    ) -> Result<(), AppError> {
        self.reset_security(operator, target, proof).await
    }
}
