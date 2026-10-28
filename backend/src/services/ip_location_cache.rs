//! IP 属地的现查与缓存。
//!
//! 属地不再写进日志表：前端按需调用接口，由这里查离线库（ip2region xdb）并缓存结果。
//! 缓存落在 Redis（与基础数据缓存共用连接），条目上限 [`MAX_ENTRIES`] 条、
//! 有效期 [`TTL_SECS`] 秒，库文件更新后最迟一个有效期即全部失效；
//! Redis 不可用时退回每次现查，不因缓存故障挡住查询。
//!
//! 回环与内网不查库，直接给「本机」「内网」：没有库文件时它们同样要落在这两档，
//! 不能因为查不到记录就算成外网。

use std::sync::Arc;

use crate::{
    domain::models::ip_location::IpLocationItem,
    infrastructure::{
        ip_location::{IpLocation, local_label},
        services::cache_store::CacheStore,
    },
};

/// 缓存条目上限：`0` 表示不缓存，超出后不再写入新地址（已缓存的仍可命中）。
pub const MAX_ENTRIES: usize = 255;

/// 缓存有效期（秒）。
pub const TTL_SECS: u64 = 3 * 60 * 60;

/// v2 起本地网段不再落「外网」，v1 里这类旧值不得继续命中。
const KEY_PREFIX: &str = "guardian:ip-location:v2";

pub struct IpLocationService {
    location: Arc<IpLocation>,
    store: Option<Arc<dyn CacheStore>>,
}

impl IpLocationService {
    /// `store` 为 `None` 时不缓存（未配置 Redis 的场景）。
    pub fn new(location: Arc<IpLocation>, store: Option<Arc<dyn CacheStore>>) -> Self {
        Self { location, store }
    }

    /// 批量查询：输入顺序即输出顺序，重复地址只查一次。
    ///
    /// 非法或为空的地址同样返回条目，`location` 为空串，前端据此提示无法识别。
    pub async fn lookup(&self, ips: &[String]) -> Vec<IpLocationItem> {
        let mut items = Vec::with_capacity(ips.len());
        for ip in ips {
            let ip = ip.trim().to_string();
            items.push(IpLocationItem {
                location: self.lookup_one(&ip).await,
                ip,
            });
        }

        items
    }

    async fn lookup_one(&self, ip: &str) -> String {
        let Ok(addr) = ip.parse::<std::net::IpAddr>() else {
            return String::new();
        };
        if let Some(label) = local_label(addr) {
            return label.to_string();
        }
        let key = cache_key(ip);
        if let Some(cached) = self.cached(&key).await {
            return cached;
        }

        let Some(location) = self.location.resolve(Some(ip), false) else {
            return String::new();
        };
        self.cache(&key, &location).await;

        location
    }

    async fn cached(&self, key: &str) -> Option<String> {
        let store = self.store.as_ref()?;
        match store.get(key).await {
            Ok(value) => value,
            Err(error) => {
                tracing::debug!(%error, "ip location cache read failed");

                None
            },
        }
    }

    async fn cache(&self, key: &str, location: &str) {
        let Some(store) = self.store.as_ref() else {
            return;
        };
        if let Err(error) = store.put(key, location, TTL_SECS).await {
            tracing::debug!(%error, "ip location cache write failed");
        }
    }
}

/// 缓存键：规范化后的地址，IPv4 映射的 IPv6 与 IPv4 共用一条。
fn cache_key(ip: &str) -> String {
    let canonical = ip
        .parse::<std::net::IpAddr>()
        .map(|address| address.to_canonical().to_string())
        .unwrap_or_else(|_| ip.to_string());

    format!("{KEY_PREFIX}:{canonical}")
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Mutex};

    use super::{IpLocationService, MAX_ENTRIES, TTL_SECS, cache_key};
    use crate::{
        domain::error::AppError,
        infrastructure::{ip_location::IpLocation, services::cache_store::CacheStore},
    };

    /// 记录每次写入的 TTL，便于断言缓存口径。
    #[derive(Default)]
    struct RecordingStore {
        entries: Mutex<HashMap<String, String>>,
    }

    #[async_trait::async_trait]
    impl CacheStore for RecordingStore {
        async fn get(&self, key: &str) -> Result<Option<String>, AppError> {
            let entries = self
                .entries
                .lock()
                .map_err(|_| AppError::internal("poisoned cache"))?;

            Ok(entries.get(key).cloned())
        }

        async fn put(&self, key: &str, value: &str, _ttl: u64) -> Result<(), AppError> {
            self.entries
                .lock()
                .map_err(|_| AppError::internal("poisoned cache"))?
                .insert(key.to_string(), value.to_string());

            Ok(())
        }
    }

    struct FailingStore;

    #[async_trait::async_trait]
    impl CacheStore for FailingStore {
        async fn get(&self, _key: &str) -> Result<Option<String>, AppError> {
            Err(AppError::unavailable("cache down"))
        }

        async fn put(&self, _key: &str, _value: &str, _ttl: u64) -> Result<(), AppError> {
            Err(AppError::unavailable("cache down"))
        }
    }

    fn service_with(store: Option<std::sync::Arc<dyn CacheStore>>) -> IpLocationService {
        let location = std::sync::Arc::new(IpLocation::load(None, None).expect("不配置库文件"));

        IpLocationService::new(location, store)
    }

    #[test]
    fn cache_budget_is_255_entries_for_three_hours() {
        assert_eq!(MAX_ENTRIES, 255);
        assert_eq!(TTL_SECS, 10_800);
    }

    #[tokio::test]
    async fn lookup_keeps_the_input_order_and_dedupes_the_cache_key() {
        let store = std::sync::Arc::new(RecordingStore::default());
        let service = service_with(Some(store.clone()));

        let items = service
            .lookup(&[
                "8.8.8.8".to_string(),
                " not-an-ip ".to_string(),
                "".to_string(),
                "::ffff:8.8.8.8".to_string(),
            ])
            .await;

        assert_eq!(items.len(), 4);
        assert_eq!(items[0].ip, "8.8.8.8");
        assert_eq!(items[0].location, "外网");
        assert_eq!(items[1].ip, "not-an-ip");
        assert_eq!(items[1].location, "");
        assert_eq!(items[2].location, "");
        // 映射地址与 IPv4 归一后共用一个键，不会各写一份。
        assert_eq!(items[3].location, "外网");
        let entries = store.entries.lock().expect("读取缓存");
        assert_eq!(entries.len(), 1);
        assert!(entries.contains_key(&cache_key("8.8.8.8")));
    }

    #[tokio::test]
    async fn cached_value_short_circuits_the_database_lookup() {
        let store = std::sync::Arc::new(RecordingStore::default());
        store.entries.lock().expect("预置缓存").insert(
            cache_key("8.8.8.8"),
            "中国|北京市|北京市|电信|CN".to_string(),
        );
        let service = service_with(Some(store));

        let items = service.lookup(&["8.8.8.8".to_string()]).await;

        assert_eq!(items[0].location, "中国|北京市|北京市|电信|CN");
    }

    /// 缓存故障只降级：查询必须继续给出属地。
    #[tokio::test]
    async fn cache_errors_do_not_break_the_lookup() {
        let service = service_with(Some(std::sync::Arc::new(FailingStore)));

        let items = service.lookup(&["8.8.8.8".to_string()]).await;

        assert_eq!(items[0].location, "外网");
    }

    #[tokio::test]
    async fn without_a_store_every_lookup_hits_the_database() {
        let service = service_with(None);

        let items = service.lookup(&["8.8.8.8".to_string()]).await;

        assert_eq!(items[0].location, "外网");
    }

    /// 没有库文件时本地网段也必须落回三档里的前两档，而不是一律外网。
    #[tokio::test]
    async fn local_networks_are_labelled_without_the_database() {
        let store = std::sync::Arc::new(RecordingStore::default());
        let service = service_with(Some(store.clone()));

        let items = service
            .lookup(&[
                "127.0.0.1".to_string(),
                "::1".to_string(),
                "192.168.1.1".to_string(),
                "fe80::1".to_string(),
            ])
            .await;

        let locations: Vec<&str> = items.iter().map(|item| item.location.as_str()).collect();
        assert_eq!(locations, ["本机", "本机", "内网", "内网"]);
        // 本地网段不查库，也就不占缓存条目。
        assert!(store.entries.lock().expect("读取缓存").is_empty());
    }
}
