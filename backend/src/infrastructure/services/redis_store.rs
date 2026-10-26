use async_trait::async_trait;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;

use crate::{
    error::AppError,
    infrastructure::{
        config::RedisConfig,
        services::auth_store::{AuthStore, CAPTCHA_TTL_SECONDS, captcha_key, token_key, user_key},
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
        let client = redis::Client::open(config.connection_url().map_err(AppError::from)?)
            .map_err(|error| AppError::Config(format!("invalid redis url: {error}")))?;
        let connection = client
            .get_connection_manager()
            .await
            .map_err(|error| AppError::Config(format!("redis connection failed: {error}")))?;

        Ok(Self { connection })
    }

    async fn connection(&self) -> ConnectionManager {
        self.connection.clone()
    }
}

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
        let mut connection = self.connection().await;
        connection
            .set_ex::<_, _, ()>(token_key(token), username, ttl_seconds)
            .await
            .map_err(|error| AppError::internal(format!("redis set token failed: {error}")))?;
        // 同一个账号的令牌记入索引，改密码时按索引整批撤销。
        connection
            .sadd::<_, _, ()>(user_key(username), token)
            .await
            .map_err(|error| AppError::internal(format!("redis index token failed: {error}")))?;
        connection
            .expire::<_, ()>(user_key(username), ttl_seconds as i64)
            .await
            .map_err(|error| {
                AppError::internal(format!("redis expire token index failed: {error}"))
            })
    }

    async fn token_owner(&self, token: &str) -> Result<Option<String>, AppError> {
        let mut connection = self.connection().await;
        connection
            .get(token_key(token))
            .await
            .map_err(|error| AppError::internal(format!("redis get token failed: {error}")))
    }

    async fn revoke_token(&self, token: &str) -> Result<(), AppError> {
        let mut connection = self.connection().await;
        let owner: Option<String> = connection
            .get(token_key(token))
            .await
            .map_err(|error| AppError::internal(format!("redis get token failed: {error}")))?;
        connection
            .del::<_, ()>(token_key(token))
            .await
            .map_err(|error| AppError::internal(format!("redis del token failed: {error}")))?;

        let Some(owner) = owner else {
            return Ok(());
        };
        connection
            .srem::<_, _, ()>(user_key(&owner), token)
            .await
            .map_err(|error| AppError::internal(format!("redis unindex token failed: {error}")))
    }

    async fn revoke_user_tokens(&self, username: &str) -> Result<(), AppError> {
        let mut connection = self.connection().await;
        let tokens: Vec<String> =
            connection
                .smembers(user_key(username))
                .await
                .map_err(|error| {
                    AppError::internal(format!("redis get token index failed: {error}"))
                })?;

        for token in tokens {
            connection
                .del::<_, ()>(token_key(&token))
                .await
                .map_err(|error| AppError::internal(format!("redis del token failed: {error}")))?;
        }
        connection
            .del::<_, ()>(user_key(username))
            .await
            .map_err(|error| AppError::internal(format!("redis del token index failed: {error}")))
    }
}
