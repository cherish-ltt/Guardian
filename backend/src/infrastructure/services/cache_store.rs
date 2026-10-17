use crate::{domain::error::AppError, infrastructure::config::RedisConfig};
use async_trait::async_trait;
use redis::{AsyncCommands, aio::ConnectionManager};
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

#[async_trait]
pub trait CacheStore: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>, AppError>;
    async fn put(&self, key: &str, value: &str, ttl: u64) -> Result<(), AppError>;
}

pub struct RedisCacheStore {
    connection: ConnectionManager,
}
impl RedisCacheStore {
    pub async fn connect(config: &RedisConfig) -> Result<Self, AppError> {
        let client = redis::Client::open(config.connection_url()?)
            .map_err(|e| AppError::Config(e.to_string()))?;
        let connection = client
            .get_connection_manager()
            .await
            .map_err(|e| AppError::unavailable(e.to_string()))?;
        Ok(Self { connection })
    }
}
#[async_trait]
impl CacheStore for RedisCacheStore {
    async fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        self.connection
            .clone()
            .get(key)
            .await
            .map_err(|e| AppError::unavailable(e.to_string()))
    }
    async fn put(&self, key: &str, value: &str, ttl: u64) -> Result<(), AppError> {
        self.connection
            .clone()
            .set_ex(key, value, ttl)
            .await
            .map_err(|e| AppError::unavailable(e.to_string()))
    }
}

#[derive(Default)]
pub struct MemoryCacheStore {
    entries: Mutex<HashMap<String, (String, Instant)>>,
}
#[async_trait]
impl CacheStore for MemoryCacheStore {
    async fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| AppError::internal("cache mutex poisoned"))?;
        entries.retain(|_, (_, until)| *until > Instant::now());
        Ok(entries.get(key).map(|(value, _)| value.clone()))
    }
    async fn put(&self, key: &str, value: &str, ttl: u64) -> Result<(), AppError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| AppError::internal("cache mutex poisoned"))?;
        entries.retain(|_, (_, until)| *until > Instant::now());
        entries.insert(
            key.into(),
            (value.into(), Instant::now() + Duration::from_secs(ttl)),
        );
        Ok(())
    }
}
