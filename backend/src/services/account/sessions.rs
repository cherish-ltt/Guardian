use super::{
    AccountService, AttemptStage, active_user, check_factor_lock, check_password,
    check_password_lock, consume_factor, record_attempt,
};
use crate::{
    domain::{
        error::AppError,
        models::{
            account::{SecurityProof, Session, SessionPage, SessionRevocation},
            auth::AccessClaims,
        },
    },
    infrastructure::{repositories::account::AccountRepository, services::auth_store::session_id},
};

impl AccountService {
    async fn session_identity(&self, username: &str, token: &str) -> Result<(i64, i64), AppError> {
        let claims = self
            .jwt
            .verify_access_token(token)
            .map_err(|_| AppError::session_expired("未认证或登录已过期"))?;
        let user = active_user(&self.database, username).await?;
        let state = AccountRepository::security(&self.database, user.id).await?;
        if !belongs_to(&claims, username, user.id, state.auth_version)
            || self.store.token_owner(token).await?.as_deref() != Some(username)
        {
            return Err(AppError::session_expired("未认证或登录已过期"));
        }
        Ok((user.id, state.auth_version))
    }

    pub(super) async fn list_sessions(
        &self,
        username: &str,
        token: &str,
        cursor: u64,
    ) -> Result<SessionPage, AppError> {
        let (user_id, version) = self.session_identity(username, token).await?;
        let page = self.store.list_sessions(username, cursor).await?;
        let mut items = Vec::new();
        for session in page.sessions {
            let id = session_id(&session.token);
            let Ok(claims) = self.jwt.verify_access_token(&session.token) else {
                self.store.revoke_session(username, &id).await?;
                continue;
            };
            let same_identity = claims.username == username && claims.sub == user_id.to_string();
            if claims.exp <= chrono::Utc::now().timestamp() as usize
                || (same_identity && claims.auth_version < version)
            {
                self.store.revoke_session(username, &id).await?;
                continue;
            }
            // 身份快照可能早于并发登录，不能据此撤销较新版本或同名新账号的会话。
            if !same_identity || claims.auth_version != version {
                continue;
            }
            items.push(Session {
                id,
                current: session.token == token,
                issued_at: claims.iat,
                expires_at: claims.exp,
                browser: session.metadata.browser,
                os: session.metadata.os,
            });
        }
        Ok(SessionPage {
            items,
            next_cursor: (page.next_cursor != 0).then(|| page.next_cursor.to_string()),
        })
    }

    pub(super) async fn revoke_other_session(
        &self,
        username: &str,
        token: &str,
        id: &str,
        proof: SecurityProof,
    ) -> Result<SessionRevocation, AppError> {
        if id.len() != 43
            || !id
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
        {
            return Err(AppError::not_found("会话不存在或已过期"));
        }
        if id == session_id(token) {
            return Err(AppError::business("当前会话请使用退出登录"));
        }
        let target = self
            .store
            .session_token(username, id)
            .await?
            .ok_or_else(|| AppError::not_found("会话不存在或已过期"))?;
        let target_claims = self
            .jwt
            .verify_access_token(&target)
            .map_err(|_| AppError::not_found("会话不存在或已过期"))?;
        let current = self
            .jwt
            .verify_access_token(token)
            .map_err(|_| AppError::session_expired("未认证或登录已过期"))?;
        let owner = username.to_string();
        let token = token.to_string();
        let store = self.store.clone();
        let crypto = self.crypto.clone();
        let was_valid =
            crate::services::support::in_immediate_transaction(&self.database, move |tx| {
                Box::pin(async move {
                    let user = active_user(tx, &owner).await?;
                    let mut state = AccountRepository::security(tx, user.id).await?;
                    if !belongs_to(&current, &owner, user.id, state.auth_version)
                        || store.token_owner(&token).await?.as_deref() != Some(&owner)
                    {
                        return Err(AppError::session_expired("未认证或登录已过期"));
                    }
                    if target_claims.username != owner || target_claims.sub != user.id.to_string() {
                        return Err(AppError::not_found("会话不存在或已过期"));
                    }
                    let was_valid = belongs_to(&target_claims, &owner, user.id, state.auth_version);
                    let result =
                        check_password_lock(&state).and_then(|()| check_password(&user, &proof));
                    record_attempt(tx, &mut state, &result, AttemptStage::Password).await?;
                    if let Err(error) = result {
                        return Ok(Err(error));
                    }
                    if state.secret.is_some() {
                        let result = check_factor_lock(&state).and_then(|()| {
                            consume_factor(&crypto, &mut state, proof.second_factor.as_deref())
                        });
                        record_attempt(tx, &mut state, &result, AttemptStage::Factor).await?;
                        return Ok(result.map(|()| was_valid));
                    }
                    Ok(Ok(was_valid))
                })
            })
            .await??;
        // 一次性证明先提交；Redis 超时或清理失败不能让已消费的第二因素恢复可用。
        let revoked = self.store.revoke_session(username, id).await? && was_valid;
        Ok(SessionRevocation { revoked })
    }
}

fn belongs_to(claims: &AccessClaims, username: &str, user_id: i64, version: i64) -> bool {
    claims.username == username
        && claims.sub == user_id.to_string()
        && claims.auth_version == version
        && claims.exp > chrono::Utc::now().timestamp() as usize
}
