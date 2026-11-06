use async_trait::async_trait;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;

use crate::{
    error::AppError,
    infrastructure::{
        config::RedisConfig,
        services::auth_store::{
            AuthStore, CAPTCHA_TTL_SECONDS, SESSION_PAGE_SIZE, SessionMetadata, StoredSession,
            StoredSessionPage, captcha_key, session_id, session_key, token_key, user_key,
        },
    },
};

/// 基于 Redis 的实现：验证码与登录会话写在 `redis://` 里，
/// 服务重启后仍能撤销旧令牌，多实例部署共用同一份会话。
#[derive(Clone)]
pub struct RedisAuthStore {
    connection: ConnectionManager,
}

impl RedisAuthStore {
    pub async fn connect(config: &RedisConfig) -> Result<Self, AppError> {
        let client = config
            .client()
            .map_err(|error| AppError::Config(error.to_string()))?;
        let connection = client
            .get_connection_manager()
            .await
            .map_err(|error| AppError::Config(format!("redis connection failed: {error}")))?;

        Ok(Self { connection })
    }

    async fn connection(&self) -> ConnectionManager {
        self.connection.clone()
    }

    async fn indexed_session(
        &self,
        username: &str,
        token: String,
    ) -> Result<Option<StoredSession>, AppError> {
        let mut connection = self.connection().await;
        let metadata: Option<(String, String)> = redis::cmd("EVAL")
            .arg(READ_SESSION)
            .arg(3)
            .arg(token_key(&token))
            .arg(user_key(username))
            .arg(session_key(username, &session_id(&token)))
            .arg(username)
            .arg(&token)
            .query_async(&mut connection)
            .await
            .map_err(|_| AppError::unavailable("redis read session failed"))?;
        Ok(metadata.map(|(browser, os)| StoredSession {
            token,
            metadata: SessionMetadata { browser, os },
        }))
    }

    async fn revoke_owned_token(&self, username: &str, token: &str) -> Result<bool, AppError> {
        let mut connection = self.connection().await;
        redis::cmd("EVAL")
            .arg(REVOKE_SESSION)
            .arg(3)
            .arg(token_key(token))
            .arg(user_key(username))
            .arg(session_key(username, &session_id(token)))
            .arg(username)
            .arg(token)
            .query_async(&mut connection)
            .await
            .map_err(|_| AppError::unavailable("redis revoke session failed; retry to confirm"))
    }
}

// 校验类型后再写，令牌最后生效；Redis 脚本出错不会自动回滚前面的写入。
const SAVE_SESSION: &str = r#"
local expected = {'string', 'set', 'hash'}
for i = 1, 3 do
    local kind = redis.call('TYPE', KEYS[i]).ok
    if kind ~= 'none' and kind ~= expected[i] then
        return redis.error_reply('invalid session key type')
    end
end
local ttl = tonumber(ARGV[3])
if not ttl or ttl <= 0 or ttl ~= math.floor(ttl) then
    return redis.error_reply('invalid session TTL')
end
local owner = redis.call('GET', KEYS[1])
if owner and owner ~= ARGV[1] then
    return redis.error_reply('session owner mismatch')
end
if owner then
    local remaining = redis.call('PTTL', KEYS[1])
    if remaining <= 0 then return redis.error_reply('invalid session TTL') end
    ttl = math.min(ttl, remaining)
end
local index_ttl = redis.call('PTTL', KEYS[2])
redis.call('HSET', KEYS[3], 'owner', ARGV[1], 'token', ARGV[2], 'browser', ARGV[4], 'os', ARGV[5])
redis.call('PEXPIRE', KEYS[3], ttl)
redis.call('SADD', KEYS[2], ARGV[2])
if index_ttl == -2 or (index_ttl >= 0 and index_ttl < ttl) then
    redis.call('PEXPIRE', KEYS[2], ttl)
end
redis.call('SET', KEYS[1], ARGV[1], 'PX', ttl)
return 1
"#;

// 老索引仍保存 Bearer；只为仍有效且归属匹配的令牌补映射，沿用剩余 TTL。
const READ_SESSION: &str = r#"
local owner = redis.call('GET', KEYS[1])
local ttl = redis.call('PTTL', KEYS[1])
if owner ~= ARGV[1] or ttl <= 0 then
    redis.call('SREM', KEYS[2], ARGV[2])
    return false
end
local kind = redis.call('TYPE', KEYS[3]).ok
if kind ~= 'none' and kind ~= 'hash' then
    return redis.error_reply('invalid session metadata type')
end
local index_kind = redis.call('TYPE', KEYS[2]).ok
if index_kind ~= 'none' and index_kind ~= 'set' then
    return redis.error_reply('invalid session index type')
end
if kind == 'none' then
    redis.call('HSET', KEYS[3], 'owner', ARGV[1], 'token', ARGV[2], 'browser', '', 'os', '')
    redis.call('PEXPIRE', KEYS[3], ttl)
elseif redis.call('HGET', KEYS[3], 'owner') ~= ARGV[1] or redis.call('HGET', KEYS[3], 'token') ~= ARGV[2] then
    return redis.error_reply('session metadata mismatch')
end
local index_ttl = redis.call('PTTL', KEYS[2])
redis.call('SADD', KEYS[2], ARGV[2])
if index_ttl == -2 or (index_ttl >= 0 and index_ttl < ttl) then
    redis.call('PEXPIRE', KEYS[2], ttl)
end
return {redis.call('HGET', KEYS[3], 'browser') or '', redis.call('HGET', KEYS[3], 'os') or ''}
"#;

// 先使凭据失效；索引清理失败显式报错，映射保留至原 TTL，允许重试确认。
const REVOKE_SESSION: &str = r#"
local owner = redis.call('GET', KEYS[1])
if owner and owner ~= ARGV[1] then return 0 end
local removed = redis.call('DEL', KEYS[1])
redis.call('SREM', KEYS[2], ARGV[2])
redis.call('HDEL', KEYS[3], 'browser', 'os')
return removed
"#;

#[async_trait]
impl AuthStore for RedisAuthStore {
    async fn save_captcha(&self, uuid: &str, code: &str) -> Result<(), AppError> {
        let mut connection = self.connection().await;
        connection
            .set_ex::<_, _, ()>(captcha_key(uuid), code, CAPTCHA_TTL_SECONDS)
            .await
            .map_err(|error| AppError::internal(format!("redis set captcha failed: {error}")))
    }

    async fn take_captcha(&self, uuid: &str) -> Result<Option<String>, AppError> {
        let mut connection = self.connection().await;
        // GETDEL 是单条命令：GET + DEL 两步之间同一验证码可能被并发请求各取一次。
        redis::cmd("GETDEL")
            .arg(captcha_key(uuid))
            .query_async(&mut connection)
            .await
            .map_err(|error| AppError::internal(format!("redis getdel captcha failed: {error}")))
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
        let ttl_millis = ttl_seconds
            .checked_mul(1000)
            .filter(|ttl| *ttl <= i64::MAX as u64 && *ttl > 0)
            .ok_or_else(|| AppError::internal("invalid session TTL"))?;
        let mut connection = self.connection().await;
        redis::cmd("EVAL")
            .arg(SAVE_SESSION)
            .arg(3)
            .arg(token_key(token))
            .arg(user_key(username))
            .arg(session_key(username, &session_id(token)))
            .arg(username)
            .arg(token)
            .arg(ttl_millis)
            .arg(&metadata.browser)
            .arg(&metadata.os)
            .query_async::<()>(&mut connection)
            .await
            .map_err(|_| AppError::unavailable("redis save session failed"))
    }

    async fn list_sessions(
        &self,
        username: &str,
        cursor: u64,
    ) -> Result<StoredSessionPage, AppError> {
        let mut connection = self.connection().await;
        let (next_cursor, tokens): (u64, Vec<String>) = redis::cmd("SSCAN")
            .arg(user_key(username))
            .arg(cursor)
            .arg("COUNT")
            .arg(SESSION_PAGE_SIZE)
            .query_async(&mut connection)
            .await
            .map_err(|_| AppError::unavailable("redis list sessions failed"))?;
        let mut sessions = Vec::new();
        for token in tokens {
            if let Some(session) = self.indexed_session(username, token).await? {
                sessions.push(session);
            }
        }
        Ok(StoredSessionPage {
            sessions,
            next_cursor,
        })
    }

    async fn session_token(&self, username: &str, id: &str) -> Result<Option<String>, AppError> {
        let mut connection = self.connection().await;
        let (owner, token): (Option<String>, Option<String>) = redis::cmd("HMGET")
            .arg(session_key(username, id))
            .arg("owner")
            .arg("token")
            .query_async(&mut connection)
            .await
            .map_err(|_| AppError::unavailable("redis find session failed"))?;
        Ok(token.filter(|token| owner.as_deref() == Some(username) && session_id(token) == id))
    }

    async fn revoke_session(&self, username: &str, id: &str) -> Result<bool, AppError> {
        let Some(token) = self.session_token(username, id).await? else {
            return Ok(false);
        };
        self.revoke_owned_token(username, &token).await
    }

    async fn token_owner(&self, token: &str) -> Result<Option<String>, AppError> {
        let mut connection = self.connection().await;
        connection
            .get(token_key(token))
            .await
            .map_err(|_| AppError::unavailable("redis get token failed"))
    }

    async fn revoke_token(&self, token: &str) -> Result<(), AppError> {
        let Some(owner) = self.token_owner(token).await? else {
            return Ok(());
        };
        self.revoke_owned_token(&owner, token).await?;
        Ok(())
    }

    async fn revoke_user_tokens(&self, username: &str) -> Result<(), AppError> {
        let mut connection = self.connection().await;
        let tokens: Vec<String> = connection
            .smembers(user_key(username))
            .await
            .map_err(|_| AppError::unavailable("redis get token index failed"))?;

        for token in tokens {
            self.revoke_owned_token(username, &token).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires TEST_REDIS_URL pointing to a disposable Redis"]
    async fn redis_sessions_preserve_legacy_ttl_ownership_and_retry_after_cleanup_failure() {
        let store = RedisAuthStore::connect(&RedisConfig {
            url: std::env::var("TEST_REDIS_URL").expect("TEST_REDIS_URL is required"),
            password: None,
        })
        .await
        .unwrap();
        let mut connection = store.connection().await;
        let username = format!("session-test-{}", uuid::Uuid::now_v7());
        let other_user = format!("{username}-other");
        let legacy = format!("{username}-legacy-token");
        let other = format!("{username}-other-token");
        let current = format!("{username}-current-token");
        let stale = format!("{username}-stale-token");
        connection
            .set_ex::<_, _, ()>(token_key(&legacy), &username, 120)
            .await
            .unwrap();
        connection
            .sadd::<_, _, ()>(user_key(&username), &[&legacy, &stale])
            .await
            .unwrap();
        connection
            .expire::<_, ()>(user_key(&username), 120)
            .await
            .unwrap();
        store.save_token(&other, &other_user, 60).await.unwrap();
        connection
            .sadd::<_, _, ()>(user_key(&username), &other)
            .await
            .unwrap();
        let before: i64 = connection.pttl(token_key(&legacy)).await.unwrap();
        let page = store.list_sessions(&username, 0).await.unwrap();
        assert_eq!(page.sessions.len(), 1);
        assert_eq!(page.sessions[0].token, legacy);
        assert!(page.sessions[0].metadata.browser.is_empty());
        let token_ttl: i64 = connection.pttl(token_key(&legacy)).await.unwrap();
        let metadata_ttl: i64 = connection
            .pttl(session_key(&username, &session_id(&legacy)))
            .await
            .unwrap();
        assert!(token_ttl <= before && metadata_ttl <= token_ttl);
        store.save_token(&legacy, &username, 600).await.unwrap();
        let saved_ttl: i64 = connection.pttl(token_key(&legacy)).await.unwrap();
        assert!(saved_ttl <= token_ttl);
        assert!(
            !store
                .revoke_session(&username, &session_id(&other))
                .await
                .unwrap()
        );
        assert_eq!(
            store.token_owner(&other).await.unwrap().as_deref(),
            Some(other_user.as_str())
        );
        let metadata = SessionMetadata {
            browser: "Firefox".into(),
            os: "Linux".into(),
        };
        store
            .save_session(&current, &username, 30, &metadata)
            .await
            .unwrap();
        let index_ttl: i64 = connection.pttl(user_key(&username)).await.unwrap();
        assert!(
            index_ttl > 100_000,
            "shorter new sessions must not shorten the user index TTL"
        );
        assert!(
            store
                .list_sessions(&username, 0)
                .await
                .unwrap()
                .sessions
                .iter()
                .any(|session| session.token == current && session.metadata == metadata)
        );

        let broken_metadata = format!("{username}-broken-metadata-token");
        let broken_key = session_key(&username, &session_id(&broken_metadata));
        connection
            .set_ex::<_, _, ()>(&broken_key, "wrong-type", 30)
            .await
            .unwrap();
        assert!(
            store
                .save_session(&broken_metadata, &username, 30, &metadata)
                .await
                .is_err()
        );
        assert!(store.token_owner(&broken_metadata).await.unwrap().is_none());
        connection.del::<_, ()>(broken_key).await.unwrap();

        connection.del::<_, ()>(user_key(&username)).await.unwrap();
        connection
            .set_ex::<_, _, ()>(user_key(&username), "wrong-type", 120)
            .await
            .unwrap();
        let failed = format!("{username}-failed-token");
        assert!(
            store
                .save_session(&failed, &username, 30, &metadata)
                .await
                .is_err()
        );
        assert!(store.token_owner(&failed).await.unwrap().is_none());
        let error = store
            .revoke_session(&username, &session_id(&legacy))
            .await
            .expect_err("cleanup failure must be visible");
        assert!(!format!("{error}").contains(&legacy));
        assert!(store.token_owner(&legacy).await.unwrap().is_none());
        assert_eq!(
            store
                .session_token(&username, &session_id(&legacy))
                .await
                .unwrap()
                .as_deref(),
            Some(legacy.as_str())
        );
        connection.del::<_, ()>(user_key(&username)).await.unwrap();
        connection
            .sadd::<_, _, ()>(user_key(&username), &legacy)
            .await
            .unwrap();
        connection
            .expire::<_, ()>(user_key(&username), 120)
            .await
            .unwrap();
        assert!(
            !store
                .revoke_session(&username, &session_id(&legacy))
                .await
                .unwrap()
        );
        assert!(
            store
                .revoke_session(&username, &session_id(&current))
                .await
                .unwrap()
        );
        assert!(
            store
                .revoke_session(&other_user, &session_id(&other))
                .await
                .unwrap()
        );
        for (owner, token) in [
            (&username, &legacy),
            (&username, &current),
            (&other_user, &other),
        ] {
            connection
                .del::<_, ()>(session_key(owner, &session_id(token)))
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    #[ignore = "requires TEST_REDIS_URL pointing to a disposable Redis"]
    async fn redis_sessions_page_only_the_owner_and_expire_with_the_token() {
        let store = RedisAuthStore::connect(&RedisConfig {
            url: std::env::var("TEST_REDIS_URL").expect("TEST_REDIS_URL is required"),
            password: None,
        })
        .await
        .unwrap();
        let mut connection = store.connection().await;
        let username = format!("session-page-test-{}", uuid::Uuid::now_v7());
        let other_user = format!("{username}-other");
        let other = format!("{username}-other-token");
        let expiring = format!("{username}-expiring-token");
        store.save_token(&other, &other_user, 60).await.unwrap();
        store.save_token(&expiring, &username, 1).await.unwrap();
        let mut expected = std::collections::HashSet::new();
        for index in 0..(SESSION_PAGE_SIZE * 3 + 1) {
            let token = format!("{username}-token-{index}");
            store.save_token(&token, &username, 60).await.unwrap();
            expected.insert(token);
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        assert!(store.token_owner(&expiring).await.unwrap().is_none());
        assert!(
            store
                .session_token(&username, &session_id(&expiring))
                .await
                .unwrap()
                .is_none()
        );
        let mut cursor = 0;
        let mut pages = 0;
        let mut listed = std::collections::HashSet::new();
        loop {
            let page = store.list_sessions(&username, cursor).await.unwrap();
            listed.extend(page.sessions.into_iter().map(|session| session.token));
            cursor = page.next_cursor;
            pages += 1;
            if cursor == 0 {
                break;
            }
            assert!(pages <= expected.len(), "session scan must make progress");
        }
        assert!(pages > 1);
        assert_eq!(listed, expected);
        store.revoke_user_tokens(&username).await.unwrap();
        assert!(
            store
                .list_sessions(&username, 0)
                .await
                .unwrap()
                .sessions
                .is_empty()
        );
        assert_eq!(
            store.token_owner(&other).await.unwrap().as_deref(),
            Some(other_user.as_str())
        );
        store.revoke_token(&other).await.unwrap();
        for token in expected {
            connection
                .del::<_, ()>(session_key(&username, &session_id(&token)))
                .await
                .unwrap();
        }
        connection
            .del::<_, ()>(session_key(&other_user, &session_id(&other)))
            .await
            .unwrap();
    }
}
