use async_trait::async_trait;
use guardian_backend::{
    domain::error::AppError,
    infrastructure::{
        config::CacheConfig,
        services::cache_store::{CacheStore, MemoryCacheStore, RedisCacheStore},
    },
    services::hot_cache::HotCache,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[tokio::test]
async fn concurrent_misses_load_once_and_writes_prevent_stale_refill() {
    let cache = Arc::new(HotCache::new(
        Arc::new(MemoryCacheStore::default()),
        CacheConfig::default(),
    ));
    let loads = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();
    for _ in 0..20 {
        let (cache, loads) = (cache.clone(), loads.clone());
        tasks.push(tokio::spawn(async move {
            cache
                .get_or_load("same", || async {
                    loads.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    Ok(1)
                })
                .await
                .unwrap()
        }));
    }
    for task in tasks {
        assert_eq!(task.await.unwrap(), 1);
    }
    assert_eq!(loads.load(Ordering::SeqCst), 1);
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (finish_tx, finish_rx) = tokio::sync::oneshot::channel();
    let reading = cache.clone();
    let task = tokio::spawn(async move {
        reading
            .get_or_load("race", || async {
                started_tx.send(()).unwrap();
                finish_rx.await.unwrap();
                Ok(1)
            })
            .await
            .unwrap()
    });
    started_rx.await.unwrap();
    let writing = cache.clone();
    let writer = tokio::spawn(async move {
        let _guard = writing.begin_write().await;
    });
    tokio::task::yield_now().await;
    assert!(!writer.is_finished());
    finish_tx.send(()).unwrap();
    assert_eq!(task.await.unwrap(), 1);
    writer.await.unwrap();
    assert_eq!(
        cache.get_or_load("race", || async { Ok(2) }).await.unwrap(),
        2
    );
}

struct FailedStore;
#[async_trait]
impl CacheStore for FailedStore {
    async fn get(&self, _: &str) -> Result<Option<String>, AppError> {
        Err(AppError::unavailable("offline"))
    }
    async fn put(&self, _: &str, _: &str, _: u64) -> Result<(), AppError> {
        Err(AppError::unavailable("offline"))
    }
}

#[tokio::test]
async fn unavailable_cache_falls_back_and_excess_loads_are_rejected() {
    let cache = Arc::new(HotCache::new(
        Arc::new(FailedStore),
        CacheConfig {
            max_loads: 1,
            ..Default::default()
        },
    ));
    assert_eq!(
        cache
            .get_or_load("first", || async { Ok(4) })
            .await
            .unwrap(),
        4
    );
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let working = cache.clone();
    let task = tokio::spawn(async move {
        working
            .get_or_load("held", || async {
                started_tx.send(()).unwrap();
                std::future::pending::<()>().await;
                Ok(1)
            })
            .await
    });
    started_rx.await.unwrap();
    let mut rejected = false;
    for index in 0..64 {
        let result = tokio::time::timeout(
            Duration::from_millis(10),
            cache.get_or_load(&format!("other-{index}"), || async { Ok(2) }),
        )
        .await;
        if let Ok(Err(error)) = result {
            assert_eq!(error.http_code(), 503);
            rejected = true;
            break;
        }
    }
    assert!(rejected);
    task.abort();
    let _ = task.await;
    assert_eq!(
        cache.get_or_load("held", || async { Ok(3) }).await.unwrap(),
        3
    );
}

#[tokio::test]
async fn empty_results_are_cached_errors_are_not_and_admission_is_bounded() {
    let cache = HotCache::new(
        Arc::new(MemoryCacheStore::default()),
        CacheConfig {
            max_entries: 1,
            ..Default::default()
        },
    );
    let empty: Vec<i32> = cache
        .get_or_load("missing", || async { Ok(Vec::new()) })
        .await
        .unwrap();
    assert!(empty.is_empty());
    let empty: Vec<i32> = cache
        .get_or_load("missing", || async {
            panic!("empty result should be cached");
            #[allow(unreachable_code)]
            Ok(vec![1])
        })
        .await
        .unwrap();
    assert!(empty.is_empty());
    assert_eq!(
        cache
            .get_or_load("overflow", || async { Ok(1) })
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        cache
            .get_or_load("overflow", || async { Ok(2) })
            .await
            .unwrap(),
        2
    );
    assert!(
        cache
            .get_or_load::<i32, _, _>("error", || async { Err(AppError::business("failed")) })
            .await
            .is_err()
    );
    assert_eq!(
        cache
            .get_or_load("error", || async { Ok(3) })
            .await
            .unwrap(),
        3
    );
}

#[tokio::test]
#[ignore = "requires TEST_REDIS_URL pointing to a disposable Redis"]
async fn redis_store_roundtrip_ttl_and_cache_invalidation() {
    let url = std::env::var("TEST_REDIS_URL").expect("TEST_REDIS_URL is required");
    let store = Arc::new(
        RedisCacheStore::connect(&guardian_backend::infrastructure::config::RedisConfig {
            url,
            password: None,
        })
        .await
        .unwrap(),
    );
    let key = format!("guardian:test:{}", uuid::Uuid::now_v7());
    store.put(&key, "test", 1).await.unwrap();
    assert_eq!(store.get(&key).await.unwrap().as_deref(), Some("test"));
    tokio::time::sleep(Duration::from_millis(1100)).await;
    assert!(store.get(&key).await.unwrap().is_none());
    let cache = HotCache::new(store, CacheConfig::default());
    assert_eq!(
        cache
            .get_or_load("redis", || async { Ok(1) })
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        cache
            .get_or_load("redis", || async { Ok(2) })
            .await
            .unwrap(),
        1
    );
    {
        let _guard = cache.begin_write().await;
    }
    assert_eq!(
        cache
            .get_or_load("redis", || async { Ok(2) })
            .await
            .unwrap(),
        2
    );
}

/// 验证码取用必须原子：并发请求同一个 uuid 只能有一个拿到明文。
#[tokio::test]
#[ignore = "requires TEST_REDIS_URL pointing to a disposable Redis"]
async fn redis_captcha_take_is_atomic() {
    use guardian_backend::infrastructure::services::{
        auth_store::AuthStore, redis_store::RedisAuthStore,
    };

    let url = std::env::var("TEST_REDIS_URL").expect("TEST_REDIS_URL is required");
    let store = Arc::new(
        RedisAuthStore::connect(&guardian_backend::infrastructure::config::RedisConfig {
            url,
            password: None,
        })
        .await
        .unwrap(),
    );
    let uuid = uuid::Uuid::now_v7().to_string();
    store.save_captcha(&uuid, "AB12").await.unwrap();

    let first = {
        let store = Arc::clone(&store);
        let uuid = uuid.clone();
        tokio::spawn(async move { store.take_captcha(&uuid).await.unwrap() })
    };
    let second = {
        let store = Arc::clone(&store);
        let uuid = uuid.clone();
        tokio::spawn(async move { store.take_captcha(&uuid).await.unwrap() })
    };
    let (first, second) = tokio::join!(first, second);
    let winners = [first.unwrap(), second.unwrap()]
        .into_iter()
        .filter(|value| value.as_deref() == Some("AB12"))
        .count();

    assert_eq!(winners, 1, "同一个验证码只能被取用一次");
    assert!(store.take_captcha(&uuid).await.unwrap().is_none());
}
