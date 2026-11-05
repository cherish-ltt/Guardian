use std::{
    collections::{BTreeMap, HashMap},
    sync::Mutex,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

use crate::error::AppError;

/// 验证码在缓存里的键前缀，与 Spring 版 `CaptchaService` 一致。
pub const CAPTCHA_KEY_PREFIX: &str = "captcha:";
/// 登录令牌在缓存里的键前缀，与 Spring 版 `TokenStore` 一致。
pub const TOKEN_KEY_PREFIX: &str = "auth:token:";
/// 账号持有令牌的索引前缀，用于按账号整体撤销会话。
pub const USER_KEY_PREFIX: &str = "auth:user:";
/// 验证码有效期（秒），与 Spring 版一致。
pub const CAPTCHA_TTL_SECONDS: u64 = 300;
pub const SESSION_PAGE_SIZE: usize = 50;

#[derive(Clone, Default, PartialEq, Eq)]
pub struct SessionMetadata {
    pub browser: String,
    pub os: String,
}

pub struct StoredSession {
    pub token: String,
    pub metadata: SessionMetadata,
}

pub struct StoredSessionPage {
    pub sessions: Vec<StoredSession>,
    pub next_cursor: u64,
}

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

    async fn save_session(
        &self,
        token: &str,
        username: &str,
        ttl_seconds: u64,
        metadata: &SessionMetadata,
    ) -> Result<(), AppError>;

    async fn list_sessions(
        &self,
        username: &str,
        cursor: u64,
    ) -> Result<StoredSessionPage, AppError>;

    /// 已撤销会话的映射保留至原到期时间，只用于本人重试，不能用于认证。
    async fn session_token(
        &self,
        username: &str,
        session_id: &str,
    ) -> Result<Option<String>, AppError>;

    async fn revoke_session(&self, username: &str, session_id: &str) -> Result<bool, AppError>;

    /// 令牌对应的账号；令牌不存在或已过期时返回 `None`。
    async fn token_owner(&self, token: &str) -> Result<Option<String>, AppError>;

    /// 撤销令牌，退出登录后旧令牌立刻失效。
    async fn revoke_token(&self, token: &str) -> Result<(), AppError>;

    /// 撤销该账号的全部会话，改密码后已签发的令牌全部立刻失效。
    async fn revoke_user_tokens(&self, username: &str) -> Result<(), AppError>;
}

pub fn captcha_key(uuid: &str) -> String {
    format!("{CAPTCHA_KEY_PREFIX}{uuid}")
}

pub fn token_key(token: &str) -> String {
    format!("{TOKEN_KEY_PREFIX}{token}")
}

pub fn user_key(username: &str) -> String {
    format!("{USER_KEY_PREFIX}{username}")
}

pub fn session_id(token: &str) -> String {
    URL_SAFE_NO_PAD
        .encode(aws_lc_rs::digest::digest(&aws_lc_rs::digest::SHA256, token.as_bytes()).as_ref())
}

pub(super) fn session_key(username: &str, id: &str) -> String {
    format!("auth:session:{username}:{id}")
}

struct MemorySession {
    token: String,
    username: String,
    metadata: SessionMetadata,
    expires_at: Instant,
    revoked: bool,
}

#[derive(Default)]
struct MemoryState {
    captchas: HashMap<String, (String, Instant)>,
    sessions: HashMap<String, MemorySession>,
    users: HashMap<String, BTreeMap<u64, String>>,
    sequence: u64,
}

/// 进程内实现：只在测试与本地无 Redis 场景使用，重启即失效。
#[derive(Default)]
pub struct MemoryAuthStore {
    state: Mutex<MemoryState>,
}

impl MemoryAuthStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 读取尚未消费的验证码明文，不移除条目。
    ///
    /// 仅供测试：`/auth/captcha` 不再回显明文，集成测试需要从服务端侧拿到它
    /// 才能走完登录流程；一次性语义仍由登录时的 `take_captcha` 保证。
    pub fn peek_captcha(&self, uuid: &str) -> Option<String> {
        self.lock()
            .captchas
            .get(uuid)
            .filter(|(_, expires_at)| *expires_at > Instant::now())
            .map(|(value, _)| value.clone())
    }

    /// 互斥锁中毒说明另一线程 panic 时仍持有数据，这里直接恢复内部数据：
    /// 存储本身只是键值表，不涉及需要回滚的不变量。
    fn lock(&self) -> std::sync::MutexGuard<'_, MemoryState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[async_trait]
impl AuthStore for MemoryAuthStore {
    async fn save_captcha(&self, uuid: &str, code: &str) -> Result<(), AppError> {
        self.lock().captchas.insert(
            uuid.to_string(),
            (
                code.to_string(),
                Instant::now() + Duration::from_secs(CAPTCHA_TTL_SECONDS),
            ),
        );
        Ok(())
    }

    async fn take_captcha(&self, uuid: &str) -> Result<Option<String>, AppError> {
        Ok(self
            .lock()
            .captchas
            .remove(uuid)
            .filter(|(_, expires_at)| *expires_at > Instant::now())
            .map(|(value, _)| value))
    }

    async fn save_token(
        &self,
        token: &str,
        username: &str,
        ttl_seconds: u64,
    ) -> Result<(), AppError> {
        self.save_session(token, username, ttl_seconds, &SessionMetadata::default())
            .await
    }

    async fn save_session(
        &self,
        token: &str,
        username: &str,
        ttl_seconds: u64,
        metadata: &SessionMetadata,
    ) -> Result<(), AppError> {
        let mut state = self.lock();
        let id = session_id(token);
        let expires_at = Instant::now() + Duration::from_secs(ttl_seconds);
        if let Some(session) = state.sessions.get_mut(&id) {
            if session.username != username {
                return Err(AppError::internal("session owner mismatch"));
            }
            session.expires_at = session.expires_at.min(expires_at);
            session.metadata = metadata.clone();
            return Ok(());
        }
        state.sequence += 1;
        let sequence = state.sequence;
        state
            .users
            .entry(username.to_string())
            .or_default()
            .insert(sequence, id.clone());
        state.sessions.insert(
            id,
            MemorySession {
                token: token.to_string(),
                username: username.to_string(),
                metadata: metadata.clone(),
                expires_at,
                revoked: false,
            },
        );
        Ok(())
    }

    async fn list_sessions(
        &self,
        username: &str,
        cursor: u64,
    ) -> Result<StoredSessionPage, AppError> {
        let mut state = self.lock();
        let entries: Vec<_> = state
            .users
            .get(username)
            .into_iter()
            .flat_map(|index| {
                index.range((
                    std::ops::Bound::Excluded(cursor),
                    std::ops::Bound::Unbounded,
                ))
            })
            .take(SESSION_PAGE_SIZE + 1)
            .map(|(sequence, id)| (*sequence, id.clone()))
            .collect();
        let next_cursor = if entries.len() > SESSION_PAGE_SIZE {
            entries[SESSION_PAGE_SIZE - 1].0
        } else {
            0
        };
        let mut sessions = Vec::new();
        for (sequence, id) in entries.into_iter().take(SESSION_PAGE_SIZE) {
            let Some(session) = state.sessions.get(&id) else {
                continue;
            };
            if session.expires_at <= Instant::now() {
                state.sessions.remove(&id);
                if let Some(index) = state.users.get_mut(username) {
                    index.remove(&sequence);
                }
            } else if session.revoked || session.username != username {
                if let Some(index) = state.users.get_mut(username) {
                    index.remove(&sequence);
                }
            } else {
                sessions.push(StoredSession {
                    token: session.token.clone(),
                    metadata: session.metadata.clone(),
                });
            }
        }
        Ok(StoredSessionPage {
            sessions,
            next_cursor,
        })
    }

    async fn session_token(&self, username: &str, id: &str) -> Result<Option<String>, AppError> {
        Ok(self
            .lock()
            .sessions
            .get(id)
            .filter(|session| session.username == username && session.expires_at > Instant::now())
            .map(|session| session.token.clone()))
    }

    async fn revoke_session(&self, username: &str, id: &str) -> Result<bool, AppError> {
        let mut state = self.lock();
        let Some(session) = state
            .sessions
            .get_mut(id)
            .filter(|session| session.username == username && session.expires_at > Instant::now())
        else {
            return Ok(false);
        };
        Ok(!std::mem::replace(&mut session.revoked, true))
    }

    async fn token_owner(&self, token: &str) -> Result<Option<String>, AppError> {
        Ok(self
            .lock()
            .sessions
            .get(&session_id(token))
            .filter(|session| !session.revoked && session.expires_at > Instant::now())
            .map(|session| session.username.clone()))
    }

    async fn revoke_token(&self, token: &str) -> Result<(), AppError> {
        if let Some(session) = self.lock().sessions.get_mut(&session_id(token)) {
            session.revoked = true;
        }
        Ok(())
    }

    async fn revoke_user_tokens(&self, username: &str) -> Result<(), AppError> {
        let mut state = self.lock();
        if let Some(index) = state.users.get(username) {
            let ids: Vec<_> = index.values().cloned().collect();
            for id in ids {
                if let Some(session) = state.sessions.get_mut(&id)
                    && session.username == username
                {
                    session.revoked = true;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AuthStore, MemoryAuthStore, SESSION_PAGE_SIZE, SessionMetadata, captcha_key, session_id,
        token_key, user_key,
    };

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

    #[tokio::test]
    async fn revoking_user_tokens_drops_only_that_account() {
        let store = MemoryAuthStore::new();
        for (token, owner) in [
            ("token-a", "admin"),
            ("token-b", "admin"),
            ("token-c", "linjh"),
        ] {
            store.save_token(token, owner, 60).await.expect("写入令牌");
        }
        store
            .save_captcha("uuid-1", "ABCD")
            .await
            .expect("写入验证码");

        store
            .revoke_user_tokens("admin")
            .await
            .expect("按账号撤销会话");

        assert_eq!(store.token_owner("token-a").await.expect("查询令牌"), None);
        assert_eq!(store.token_owner("token-b").await.expect("查询令牌"), None);
        assert_eq!(
            store.token_owner("token-c").await.expect("查询令牌"),
            Some("linjh".to_string())
        );
        assert_eq!(
            store.take_captcha("uuid-1").await.expect("读取验证码"),
            Some("ABCD".to_string()),
            "撤销会话不得连带清掉验证码"
        );
    }

    #[test]
    fn key_prefixes_match_spring_layout() {
        assert_eq!(captcha_key("u"), "captcha:u");
        assert_eq!(token_key("t"), "auth:token:t");
        assert_eq!(user_key("admin"), "auth:user:admin");
    }

    #[tokio::test]
    async fn session_listing_is_scoped_paged_and_preserves_retry_mapping() {
        let store = MemoryAuthStore::new();
        let metadata = SessionMetadata {
            browser: "Firefox".into(),
            os: "Linux".into(),
        };
        store.save_token("other", "bob", 60).await.unwrap();
        store.save_token("expired", "alice", 0).await.unwrap();
        for index in 0..=SESSION_PAGE_SIZE {
            store
                .save_session(&format!("token-{index}"), "alice", 60, &metadata)
                .await
                .unwrap();
        }
        let first = store.list_sessions("alice", 0).await.unwrap();
        assert_eq!(first.sessions.len(), SESSION_PAGE_SIZE - 1);
        assert_ne!(first.next_cursor, 0);
        let second = store
            .list_sessions("alice", first.next_cursor)
            .await
            .unwrap();
        assert_eq!(second.sessions.len(), 2);
        assert_eq!(second.next_cursor, 0);
        assert!(
            first
                .sessions
                .iter()
                .all(|session| session.metadata == metadata)
        );
        let target = &first.sessions[0].token;
        let id = session_id(target);
        assert_ne!(id, *target);
        assert_eq!(id.len(), 43);
        assert!(store.session_token("bob", &id).await.unwrap().is_none());
        assert!(!store.revoke_session("bob", &id).await.unwrap());
        assert!(store.revoke_session("alice", &id).await.unwrap());
        assert!(store.token_owner(target).await.unwrap().is_none());
        assert_eq!(
            store.session_token("alice", &id).await.unwrap().as_ref(),
            Some(target)
        );
        assert!(!store.revoke_session("alice", &id).await.unwrap());
        assert_eq!(
            store.token_owner("other").await.unwrap().as_deref(),
            Some("bob")
        );
        assert!(
            store
                .session_token("alice", &session_id("expired"))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn saving_metadata_does_not_extend_an_existing_session() {
        let store = MemoryAuthStore::new();
        store.save_token("legacy", "alice", 60).await.unwrap();
        let deadline = store.lock().sessions[&session_id("legacy")].expires_at;
        store
            .save_session("legacy", "alice", 600, &SessionMetadata::default())
            .await
            .unwrap();
        assert_eq!(
            store.lock().sessions[&session_id("legacy")].expires_at,
            deadline
        );
        assert_eq!(
            store
                .list_sessions("alice", 0)
                .await
                .unwrap()
                .sessions
                .len(),
            1
        );
        assert!(store.save_token("legacy", "bob", 600).await.is_err());
    }
}
