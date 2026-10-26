use std::sync::OnceLock;

use crate::{domain::error::AppError, infrastructure::repositories::account::AccountRepository};

/// 密码阶段允许的连续失败次数与锁定时长。
///
/// 密码是登录的第一道关卡，必须在验完口令后立刻计数：只靠 2FA 阶段的计数
/// 会让「猜密码」这一最容易被自动化的环节完全没有成本。
const MAX_FAILURES: i64 = 5;
const LOCK_SECONDS: i64 = 300;

/// 账号是否处于锁定窗口内。锁定期内连口令都不再校验，避免锁定只是限速而非阻断。
pub(crate) async fn check_lock(
    database: &sea_orm::DatabaseConnection,
    user_id: i64,
) -> Result<(), AppError> {
    let state = AccountRepository::security(database, user_id).await?;
    if state.blocked_until > chrono::Utc::now().timestamp() {
        return Err(AppError::business("密码错误次数过多，请稍后重试"));
    }

    Ok(())
}

/// 记录一次口令校验结果：失败累加，连续 5 次后锁定 5 分钟；成功清零。
pub(crate) async fn record(
    database: &sea_orm::DatabaseConnection,
    user_id: i64,
    succeeded: bool,
) -> Result<(), AppError> {
    crate::services::support::in_immediate_transaction(database, move |tx| {
        Box::pin(async move {
            let mut state = AccountRepository::security(tx, user_id).await?;
            let now = chrono::Utc::now().timestamp();
            if succeeded {
                state.failures = 0;
                state.blocked_until = 0;
            } else {
                // 锁定窗口已过：旧计数作废重新起算，否则累计到 5 次后每次失败都会立刻再锁一遍。
                if state.blocked_until != 0 && state.blocked_until <= now {
                    state.failures = 0;
                    state.blocked_until = 0;
                }
                state.failures += 1;
                if state.failures >= MAX_FAILURES {
                    state.blocked_until = now + LOCK_SECONDS;
                }
            }
            AccountRepository::save_security(tx, state).await
        })
    })
    .await
}

/// 账号不存在时也付出一次等价的 Argon2 校验成本，避免响应时间泄露账号是否存在。
pub(crate) fn spend_unknown_account_cost() -> Result<(), AppError> {
    static DUMMY: OnceLock<String> = OnceLock::new();
    let hash = DUMMY.get_or_init(|| {
        crate::util::password::hash_password("guardian-unknown-account-placeholder")
            .unwrap_or_default()
    });
    if hash.is_empty() {
        return Ok(());
    }

    crate::util::password::verify_password("not-the-real-password", hash)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{LOCK_SECONDS, MAX_FAILURES};

    #[test]
    fn lock_policy_is_five_attempts_for_five_minutes() {
        assert_eq!(MAX_FAILURES, 5);
        assert_eq!(LOCK_SECONDS, 300);
    }
}
