use crate::{
    domain::{
        error::AppError,
        models::{account::SecurityProof, identity::AuthUserRow},
    },
    entities::sys_user_security::Model,
    infrastructure::{
        repositories::{
            account::AccountRepository,
            admin_auth::{AdminAuthRepository, SeaOrmAdminAuthRepository},
        },
        services::{
            auth_store::AuthStore,
            two_factor::{TwoFactorCrypto, authenticator, recovery_hash},
        },
    },
    util::password,
};
use sea_orm::{DatabaseConnection, DatabaseTransaction};
use std::sync::Arc;

pub(crate) mod login;
mod profile;
mod security;

pub struct AccountService {
    database: DatabaseConnection,
    crypto: Arc<TwoFactorCrypto>,
    store: Arc<dyn AuthStore>,
}

impl AccountService {
    pub fn new(
        database: DatabaseConnection,
        crypto: Arc<TwoFactorCrypto>,
        store: Arc<dyn AuthStore>,
    ) -> Self {
        Self {
            database,
            crypto,
            store,
        }
    }
}

async fn active_user(tx: &DatabaseTransaction, username: &str) -> Result<AuthUserRow, AppError> {
    let user = SeaOrmAdminAuthRepository
        .find_user_by_username(tx, username)
        .await?
        .ok_or_else(|| AppError::session_expired("未认证或登录已过期"))?;
    if user.status != 1 {
        return Err(AppError::session_expired("未认证或登录已过期"));
    }
    Ok(user)
}

/// 2FA 阶段的锁定判定：只认 2FA 自己的计数。
fn check_factor_lock(state: &Model) -> Result<(), AppError> {
    if state.factor_blocked_until > chrono::Utc::now().timestamp() {
        return Err(AppError::business("安全验证失败次数过多，请稍后重试"));
    }
    Ok(())
}

/// 口令阶段的锁定判定：只认口令自己的计数（与 admin_auth 的 password_guard 同源）。
fn check_password_lock(state: &Model) -> Result<(), AppError> {
    if state.blocked_until > chrono::Utc::now().timestamp() {
        return Err(AppError::business("密码错误次数过多，请稍后重试"));
    }
    Ok(())
}

fn check_password(user: &AuthUserRow, proof: &SecurityProof) -> Result<(), AppError> {
    if !password::verify_password(&proof.password, &user.password)? {
        return Err(AppError::business("密码不正确"));
    }
    Ok(())
}

fn consume_factor(
    crypto: &TwoFactorCrypto,
    state: &mut Model,
    factor: Option<&str>,
) -> Result<(), AppError> {
    let factor = factor
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::business("请输入认证器动态码或恢复码"))?;
    if factor.len() == 6 && factor.bytes().all(|ch| ch.is_ascii_digit()) {
        let encrypted = state
            .secret
            .as_deref()
            .ok_or_else(|| AppError::business("尚未启用2FA"))?;
        let totp = authenticator(crypto.decrypt(state.user_id, encrypted)?, "account")?;
        if let Some(step) = totp
            .check(factor, chrono::Utc::now().timestamp() as u64)
            .filter(|step| *step as i64 > state.last_step)
        {
            state.last_step = step as i64;
            return Ok(());
        }
    } else {
        let mut hashes: Vec<String> = serde_json::from_str(&state.recovery_hashes)
            .map_err(|_| AppError::internal("invalid recovery code state"))?;
        let hash = recovery_hash(factor);
        if let Some(index) = hashes.iter().position(|stored| stored == &hash) {
            hashes.remove(index);
            state.recovery_hashes = serde_json::to_string(&hashes)
                .map_err(|_| AppError::internal("recovery code serialization failed"))?;
            return Ok(());
        }
    }
    Err(AppError::business("动态码或恢复码无效、已使用或已过期"))
}

/// 一次校验失败 / 成功记到哪个计数器上。
/// 口令阶段（开启、启用、重置 2FA 都要验当前口令）记 `failures`；
/// 2FA 阶段（两步登录提交动态码）记 `factor_failures`。两段共用一份计数会让
/// 「口令正确」变成「2FA 重试次数无限」，反之 2FA 锁定会把人挡在验口令之前。
#[derive(Clone, Copy)]
pub(crate) enum AttemptStage {
    Password,
    Factor,
}

async fn record_attempt<T>(
    tx: &DatabaseTransaction,
    state: &mut Model,
    result: &Result<T, AppError>,
    stage: AttemptStage,
) -> Result<(), AppError> {
    if let Err(error) = result
        && !matches!(error, AppError::Business(_))
    {
        return Err(AppError::internal(
            "security operation failed before commit",
        ));
    }
    let now = chrono::Utc::now().timestamp();
    let (failures, blocked_until) = match stage {
        AttemptStage::Password => (&mut state.failures, &mut state.blocked_until),
        AttemptStage::Factor => (&mut state.factor_failures, &mut state.factor_blocked_until),
    };
    if *blocked_until > now {
        return Ok(());
    }
    if result.is_ok() {
        *failures = 0;
        *blocked_until = 0;
    } else if matches!(result, Err(AppError::Business(_))) {
        // 锁定窗口已过：旧计数作废重新起算，否则每次失败都会立刻再锁一遍。
        if *blocked_until != 0 {
            *failures = 0;
            *blocked_until = 0;
        }
        *failures += 1;
        if *failures >= 5 {
            *blocked_until = now + 300;
        }
    }
    AccountRepository::save_security(tx, state.clone()).await
}
