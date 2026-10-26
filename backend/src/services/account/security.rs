use super::{
    AccountService, AttemptStage, active_user, check_password, check_password_lock, consume_factor,
    record_attempt,
};
use crate::{
    domain::{
        error::AppError,
        models::account::{Enrollment, RecoveryCodes, SecurityProof},
    },
    infrastructure::{
        repositories::{
            account::AccountRepository,
            admin_auth::{AdminAuthRepository, SeaOrmAdminAuthRepository},
        },
        services::two_factor::{authenticator, new_recovery_codes, random_bytes, recovery_hash},
    },
};

pub(super) enum SecurityAction {
    Enable,
    Disable,
    Recovery,
}

impl AccountService {
    pub(super) async fn start_enrollment(
        &self,
        username: &str,
        proof: SecurityProof,
    ) -> Result<Enrollment, AppError> {
        let username = username.to_string();
        let crypto = self.crypto.clone();
        crate::services::support::in_immediate_transaction(&self.database, move |tx| {
            Box::pin(async move {
                let user = active_user(tx, &username).await?;
                let mut state = AccountRepository::security(tx, user.id).await?;
                let result = (|| {
                    check_password_lock(&state)?;
                    check_password(&user, &proof)?;
                    if state.secret.is_some() {
                        return Err(AppError::business("2FA已启用，请先关闭"));
                    }
                    let secret = random_bytes::<20>()?.to_vec();
                    let totp = authenticator(secret.clone(), &username)?;
                    state.pending_secret = Some(crypto.encrypt(user.id, &secret)?);
                    state.pending_expires = chrono::Utc::now().timestamp() + 300;
                    Ok(Enrollment {
                        secret: totp.secret().to_base32(),
                        otpauth_url: totp
                            .to_url()
                            .map_err(|_| AppError::internal("TOTP URL generation failed"))?,
                        expires_in: 300,
                    })
                })();
                record_attempt(tx, &mut state, &result, AttemptStage::Password).await?;
                Ok(result)
            })
        })
        .await?
    }

    pub(super) async fn change_security(
        &self,
        username: &str,
        proof: SecurityProof,
        action: SecurityAction,
    ) -> Result<RecoveryCodes, AppError> {
        let owner = username.to_string();
        let crypto = self.crypto.clone();
        let result =
            crate::services::support::in_immediate_transaction(&self.database, move |tx| {
                Box::pin(async move {
                    let user = active_user(tx, &owner).await?;
                    let mut state = AccountRepository::security(tx, user.id).await?;
                    let result = (|| {
                        check_password_lock(&state)?;
                        check_password(&user, &proof)?;
                        match action {
                            SecurityAction::Enable => {
                                if state.secret.is_some() {
                                    return Err(AppError::business("2FA已启用"));
                                }
                                if state.pending_expires <= chrono::Utc::now().timestamp() {
                                    return Err(AppError::business("绑定已过期，请重新开始"));
                                }
                                let encrypted = state
                                    .pending_secret
                                    .clone()
                                    .ok_or_else(|| AppError::business("请先开始绑定"))?;
                                let mut pending = state.clone();
                                pending.secret = Some(encrypted.clone());
                                pending.last_step = -1;
                                consume_factor(
                                    &crypto,
                                    &mut pending,
                                    proof.second_factor.as_deref(),
                                )?;
                                state.secret = Some(encrypted);
                                state.last_step = pending.last_step;
                            },
                            _ => {
                                if state.secret.is_none() {
                                    return Err(AppError::business("尚未启用2FA"));
                                }
                                consume_factor(
                                    &crypto,
                                    &mut state,
                                    proof.second_factor.as_deref(),
                                )?;
                            },
                        }
                        let codes = if matches!(action, SecurityAction::Disable) {
                            state.secret = None;
                            state.last_step = -1;
                            Vec::new()
                        } else {
                            new_recovery_codes()?
                        };
                        state.recovery_hashes = serde_json::to_string(
                            &codes
                                .iter()
                                .map(|code| recovery_hash(code))
                                .collect::<Vec<_>>(),
                        )
                        .map_err(|_| AppError::internal("recovery serialization failed"))?;
                        state.pending_secret = None;
                        state.pending_expires = 0;
                        state.auth_version += 1;
                        state.login_challenge_hash = None;
                        state.login_challenge_expires = 0;
                        state.login_password_hash = None;
                        Ok(RecoveryCodes {
                            recovery_codes: codes,
                        })
                    })();
                    record_attempt(tx, &mut state, &result, AttemptStage::Password).await?;
                    Ok(result)
                })
            })
            .await??;
        self.cleanup_sessions(username).await;
        Ok(result)
    }

    pub(super) async fn reset_security(
        &self,
        operator: &str,
        target: i64,
        proof: SecurityProof,
    ) -> Result<(), AppError> {
        let owner = operator.to_string();
        let target_username =
            crate::services::support::in_immediate_transaction(&self.database, move |tx| {
                Box::pin(async move {
                    let user = active_user(tx, &owner).await?;
                    if user.id == target {
                        return Err(AppError::business("请在个人资料中关闭本人的2FA"));
                    }
                    let roles = SeaOrmAdminAuthRepository.role_codes(tx, user.id).await?;
                    let is_super = roles.iter().any(|r| r == "super_admin");
                    let permissions = SeaOrmAdminAuthRepository.permissions(tx, user.id).await?;
                    if !is_super && !permissions.iter().any(|p| p == "system:user:reset2fa") {
                        return Err(AppError::forbidden("没有访问权限"));
                    }
                    let target_roles = SeaOrmAdminAuthRepository.role_codes(tx, target).await?;
                    if !is_super && target_roles.iter().any(|r| r == "super_admin") {
                        return Err(AppError::forbidden("不能重置超级管理员的2FA"));
                    }
                    let target_username = AccountRepository::username(tx, target).await?;
                    let mut operator_state = AccountRepository::security(tx, user.id).await?;
                    let result = check_password_lock(&operator_state)
                        .and_then(|()| check_password(&user, &proof));
                    record_attempt(tx, &mut operator_state, &result, AttemptStage::Password)
                        .await?;
                    if let Err(error) = result {
                        return Ok(Err(error));
                    }
                    let mut state = AccountRepository::security(tx, target).await?;
                    state.secret = None;
                    state.pending_secret = None;
                    state.pending_expires = 0;
                    state.last_step = -1;
                    state.recovery_hashes = "[]".into();
                    state.auth_version += 1;
                    state.login_challenge_hash = None;
                    state.login_challenge_expires = 0;
                    state.login_password_hash = None;
                    state.failures = 0;
                    state.blocked_until = 0;
                    AccountRepository::save_security(tx, state).await?;
                    Ok(Ok(target_username))
                })
            })
            .await??;
        self.cleanup_sessions(&target_username).await;
        Ok(())
    }

    async fn cleanup_sessions(&self, username: &str) {
        // 数据库版本已使旧令牌失效；Redis 清理失败不能丢弃已提交的一次性恢复码。
        if let Err(error) = self.store.revoke_user_tokens(username).await {
            tracing::warn!(%error, "security version committed; Redis session cleanup deferred to TTL");
        }
    }
}
