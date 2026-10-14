use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use async_trait::async_trait;

use crate::error::AppError;

/// 验证码在缓存里的键前缀，与 Spring 版 `CaptchaService` 一致。
pub const CAPTCHA_KEY_PREFIX: &str = "captcha:";
/// 登录令牌在缓存里的键前缀，与 Spring 版 `TokenStore` 一致。
pub const TOKEN_KEY_PREFIX: &str = "auth:token:";
/// 验证码有效期（秒），与 Spring 版一致。
pub const CAPTCHA_TTL_SECONDS: u64 = 300;

/// 验证码与登录会话的存储端口。
///
/// 生产环境由 [`RedisAuthStore`](super::redis_store::RedisAuthStore) 实现；
/// 测试用 [`MemoryAuthStore`]，避免集成测试依赖外部 Redis。
#[async_trait]
pub trait AuthStore: Send + Sync {
    async fn save_captcha(&self, uuid: &str, code: &str) -> Result<(), AppError>;

    /// 读取并删除验证码：与 Spring 版一致，验证码只能使用一次。
    async fn take_captcha(&self, uuid: &str) -> Result<Option<String>, AppError>;

    async fn save_token(
        &self,
        token: &str,
        username: &str,
        ttl_seconds: u64,
    ) -> Result<(), AppError>;

    /// 令牌对应的账号；令牌不存在或已过期时返回 `None`。
    async fn token_owner(&self, token: &str) -> Result<Option<String>, AppError>;

    /// 撤销令牌，退出登录后旧令牌立刻失效。
    async fn revoke_token(&self, token: &str) -> Result<(), AppError>;
}

pub fn captcha_key(uuid: &str) -> String {
    format!("{CAPTCHA_KEY_PREFIX}{uuid}")
}

pub fn token_key(token: &str) -> String {
    format!("{TOKEN_KEY_PREFIX}{token}")
}

/// 进程内实现：只在测试与本地无 Redis 场景使用，重启即失效。
#[derive(Debug, Default)]
pub struct MemoryAuthStore {
    entries: Mutex<HashMap<String, (String, Option<Instant>)>>,
}

impl MemoryAuthStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn save(&self, key: String, value: String, ttl: Option<Duration>) {
        let mut entries = self.lock();
        entries.insert(key, (value, ttl.map(|ttl| Instant::now() + ttl)));
    }

    fn take(&self, key: &str) -> Option<String> {
        let mut entries = self.lock();
        let (value, expires_at) = entries.remove(key)?;
        if expires_at.is_some_and(|deadline| deadline <= Instant::now()) {
            return None;
        }
        Some(value)
    }

    fn get(&self, key: &str) -> Option<String> {
        let entries = self.lock();
        let (value, expires_at) = entries.get(key)?;
        if expires_at.is_some_and(|deadline| deadline <= Instant::now()) {
            return None;
        }
        Some(value.clone())
    }

    fn remove(&self, key: &str) {
        self.lock().remove(key);
    }

    /// 互斥锁中毒说明另一线程 panic 时仍持有数据，这里直接恢复内部数据：
    /// 存储本身只是键值表，不涉及需要回滚的不变量。
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, (String, Option<Instant>)>> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[async_trait]
impl AuthStore for MemoryAuthStore {
    async fn save_captcha(&self, uuid: &str, code: &str) -> Result<(), AppError> {
        self.save(
            captcha_key(uuid),
            code.to_string(),
            Some(Duration::from_secs(CAPTCHA_TTL_SECONDS)),
        );
        Ok(())
    }

    async fn take_captcha(&self, uuid: &str) -> Result<Option<String>, AppError> {
        Ok(self.take(&captcha_key(uuid)))
    }

    async fn save_token(
        &self,
        token: &str,
        username: &str,
        ttl_seconds: u64,
    ) -> Result<(), AppError> {
        self.save(
            token_key(token),
            username.to_string(),
            Some(Duration::from_secs(ttl_seconds)),
        );
        Ok(())
    }

    async fn token_owner(&self, token: &str) -> Result<Option<String>, AppError> {
        Ok(self.get(&token_key(token)))
    }

    async fn revoke_token(&self, token: &str) -> Result<(), AppError> {
        self.remove(&token_key(token));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{AuthStore, MemoryAuthStore, captcha_key, token_key};

    #[tokio::test]
    async fn captcha_can_only_be_read_once() {
        let store = MemoryAuthStore::new();
        store
            .save_captcha("uuid-1", "ABCD")
            .await
            .expect("写入验证码");

        assert_eq!(
            store.take_captcha("uuid-1").await.expect("读取验证码"),
            Some("ABCD".to_string())
        );
        assert_eq!(store.take_captcha("uuid-1").await.expect("再次读取"), None);
    }

    #[tokio::test]
    async fn token_is_visible_until_revoked() {
        let store = MemoryAuthStore::new();
        store
            .save_token("token-1", "admin", 60)
            .await
            .expect("写入令牌");

        assert_eq!(
            store.token_owner("token-1").await.expect("查询令牌"),
            Some("admin".to_string())
        );
        store.revoke_token("token-1").await.expect("撤销令牌");
        assert_eq!(store.token_owner("token-1").await.expect("再次查询"), None);
    }

    #[test]
    fn key_prefixes_match_spring_layout() {
        assert_eq!(captcha_key("u"), "captcha:u");
        assert_eq!(token_key("t"), "auth:token:t");
    }
}
