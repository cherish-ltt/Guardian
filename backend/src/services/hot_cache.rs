use crate::{
    domain::error::AppError,
    infrastructure::{config::CacheConfig, services::cache_store::CacheStore},
};
use base64::Engine;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::{Mutex as AsyncMutex, RwLock, RwLockWriteGuard, Semaphore};

pub struct HotCache {
    store: Arc<dyn CacheStore>,
    config: CacheConfig,
    namespace: String,
    generation: RwLock<u64>,
    admitted: Mutex<HashMap<String, Instant>>,
    flights: Vec<AsyncMutex<()>>,
    loads: Semaphore,
    retry_at: Mutex<Option<Instant>>,
}

impl HotCache {
    pub fn new(store: Arc<dyn CacheStore>, config: CacheConfig) -> Self {
        Self {
            store,
            loads: Semaphore::new(config.max_loads),
            config,
            namespace: uuid::Uuid::now_v7().to_string(),
            generation: RwLock::new(0),
            admitted: Mutex::new(HashMap::new()),
            flights: (0..64).map(|_| AsyncMutex::new(())).collect(),
            retry_at: Mutex::new(None),
        }
    }

    // 先换代再写库，读锁覆盖回源和填充；失败、取消也不会重新使用旧缓存。
    pub async fn begin_write(&self) -> RwLockWriteGuard<'_, u64> {
        let mut guard = self.generation.write().await;
        *guard += 1;
        guard
    }

    pub async fn get_or_load<T, F, Fut>(&self, key: &str, load: F) -> Result<T, AppError>
    where
        T: Serialize + DeserializeOwned + Send,
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T, AppError>> + Send,
    {
        let generation = self.generation.read().await;
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        let digest = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(aws_lc_rs::digest::digest(&aws_lc_rs::digest::SHA256, key.as_bytes()).as_ref());
        let key = format!(
            "guardian:cache:v1:{}:{}:{digest}",
            self.namespace, *generation
        );
        let admitted = self.admit(&key)?;
        if admitted && let Some(value) = self.cached(&key).await {
            return Ok(value);
        }
        let _flight = self.flights[hash as usize % self.flights.len()]
            .lock()
            .await;
        if admitted && let Some(value) = self.cached(&key).await {
            return Ok(value);
        }
        let _permit = self
            .loads
            .try_acquire()
            .map_err(|_| AppError::unavailable("cache database fallback capacity exhausted"))?;
        let value = load().await?;
        if admitted && self.cache_available() {
            let encoded =
                serde_json::to_string(&value).map_err(|e| AppError::internal(e.to_string()))?;
            if encoded.len() <= self.config.max_value_bytes {
                let json =
                    serde_json::to_value(&value).map_err(|e| AppError::internal(e.to_string()))?;
                let empty = json.is_null()
                    || json.as_array().is_some_and(Vec::is_empty)
                    || json
                        .get("rows")
                        .and_then(|v| v.as_array())
                        .is_some_and(Vec::is_empty);
                let ttl = if empty {
                    self.config.negative_ttl_secs
                } else {
                    self.config.ttl_secs + hash % (self.config.ttl_secs / 5 + 1)
                };
                if !matches!(
                    tokio::time::timeout(
                        Duration::from_millis(self.config.timeout_ms),
                        self.store.put(&key, &encoded, ttl)
                    )
                    .await,
                    Ok(Ok(()))
                ) {
                    self.cache_failed();
                }
            }
        }
        Ok(value)
    }

    fn admit(&self, key: &str) -> Result<bool, AppError> {
        let mut entries = self
            .admitted
            .lock()
            .map_err(|_| AppError::internal("cache admission mutex poisoned"))?;
        let now = Instant::now();
        entries.retain(|_, until| *until > now);
        if !entries.contains_key(key) && entries.len() >= self.config.max_entries {
            return Ok(false);
        }
        entries.insert(
            key.to_string(),
            now + Duration::from_secs(self.config.ttl_secs + self.config.ttl_secs / 5 + 1),
        );
        Ok(true)
    }

    fn cache_available(&self) -> bool {
        self.retry_at
            .lock()
            .expect("cache circuit mutex poisoned")
            .is_none_or(|until| until <= Instant::now())
    }
    fn cache_failed(&self) {
        let mut retry = self.retry_at.lock().expect("cache circuit mutex poisoned");
        if retry.is_none_or(|until| until <= Instant::now()) {
            tracing::warn!("business cache unavailable; using bounded database fallback");
        }
        *retry = Some(Instant::now() + Duration::from_secs(1));
    }
    async fn cached<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        if !self.cache_available() {
            return None;
        }
        match tokio::time::timeout(
            Duration::from_millis(self.config.timeout_ms),
            self.store.get(key),
        )
        .await
        {
            Ok(Ok(Some(value))) => match serde_json::from_str(&value) {
                Ok(value) => Some(value),
                Err(_) => {
                    self.cache_failed();
                    None
                },
            },
            Ok(Ok(None)) => None,
            _ => {
                self.cache_failed();
                None
            },
        }
    }
}
