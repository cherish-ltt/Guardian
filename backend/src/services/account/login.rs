use super::{AccountService, active_user, check_lock, consume_factor, record_attempt};
use crate::{
    domain::{
        error::AppError,
        models::identity::{AuthUserRow, VerifyLoginCommand},
    },
    infrastructure::{
        repositories::account::AccountRepository,
        services::two_factor::{random_bytes, recovery_hash},
    },
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

pub enum PasswordStep {
    Complete(i64),
    Challenge(String),
}

impl AccountService {
    pub async fn begin_login(&self, user: &AuthUserRow) -> Result<PasswordStep, AppError> {
        let user = user.clone();
        crate::services::support::in_immediate_transaction(&self.database, move |tx| {
            Box::pin(async move {
                let current = active_user(tx, &user.username).await?;
                if current.password != user.password {
                    return Err(AppError::business("账号信息已变更，请重新登录"));
                }
                let mut state = AccountRepository::security(tx, user.id).await?;
                check_lock(&state)?;
                if state.secret.is_none() {
                    return Ok(PasswordStep::Complete(state.auth_version));
                }
                let challenge = URL_SAFE_NO_PAD.encode(random_bytes::<32>()?);
                state.login_challenge_hash = Some(recovery_hash(&challenge));
                state.login_challenge_expires = chrono::Utc::now().timestamp() + 300;
                state.login_password_hash = Some(user.password);
                AccountRepository::save_security(tx, state).await?;
                Ok(PasswordStep::Challenge(challenge))
            })
        })
        .await
    }

    pub async fn complete_login(
        &self,
        command: VerifyLoginCommand,
    ) -> Result<(AuthUserRow, i64), AppError> {
        let crypto = self.crypto.clone();
        crate::services::support::in_immediate_transaction(&self.database, move |tx| {
            Box::pin(async move {
                let hash = recovery_hash(&command.challenge);
                let username = AccountRepository::challenge_owner(tx, &hash)
                    .await?
                    .ok_or_else(|| AppError::business("验证已失效，请返回登录页重新登录"))?;
                let user = active_user(tx, &username).await?;
                let mut state = AccountRepository::security(tx, user.id).await?;
                if state.login_challenge_expires <= chrono::Utc::now().timestamp()
                    || state.login_password_hash.as_deref() != Some(user.password.as_str())
                    || state.secret.is_none()
                {
                    return Err(AppError::business("验证已失效，请返回登录页重新登录"));
                }
                let result = check_lock(&state).and_then(|()| {
                    consume_factor(&crypto, &mut state, Some(&command.second_factor))?;
                    state.login_challenge_hash = None;
                    state.login_challenge_expires = 0;
                    state.login_password_hash = None;
                    Ok((user, state.auth_version))
                });
                record_attempt(tx, &mut state, &result).await?;
                Ok(result)
            })
        })
        .await?
    }
}
