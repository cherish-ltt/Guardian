mod support;

use async_trait::async_trait;
use axum::http::StatusCode;
use guardian_backend::{
    container::Container,
    create_app::create_app,
    domain::error::AppError,
    infrastructure::services::{
        auth_store::{AuthStore, MemoryAuthStore},
        cache_store::CacheStore,
    },
};
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use support::*;

#[derive(Default)]
struct FaultStore {
    inner: MemoryAuthStore,
    reject_auth: AtomicBool,
}
#[async_trait]
impl AuthStore for FaultStore {
    async fn save_captcha(&self, id: &str, code: &str) -> Result<(), AppError> {
        self.inner.save_captcha(id, code).await
    }
    async fn take_captcha(&self, id: &str) -> Result<Option<String>, AppError> {
        self.inner.take_captcha(id).await
    }
    async fn save_token(&self, token: &str, user: &str, ttl: u64) -> Result<(), AppError> {
        self.inner.save_token(token, user, ttl).await
    }
    async fn token_owner(&self, token: &str) -> Result<Option<String>, AppError> {
        if self.reject_auth.load(Ordering::SeqCst) {
            return Err(AppError::unavailable("auth unavailable"));
        }
        self.inner.token_owner(token).await
    }
    async fn revoke_token(&self, token: &str) -> Result<(), AppError> {
        self.inner.revoke_token(token).await
    }
    async fn revoke_user_tokens(&self, _: &str) -> Result<(), AppError> {
        Err(AppError::unavailable("cleanup unavailable"))
    }
}
struct FailedCache;
#[async_trait]
impl CacheStore for FailedCache {
    async fn get(&self, _: &str) -> Result<Option<String>, AppError> {
        Err(AppError::unavailable("cache unavailable"))
    }
    async fn put(&self, _: &str, _: &str, _: u64) -> Result<(), AppError> {
        Err(AppError::unavailable("cache unavailable"))
    }
}

#[tokio::test]
async fn committed_security_returns_codes_when_redis_cleanup_fails_and_old_tokens_are_invalid() {
    let initial = setup_container(Vec::new()).await;
    let store = Arc::new(FaultStore::default());
    let container = Arc::new(
        Container::bootstrap_with_auth_store((*initial.config).clone(), store.clone())
            .await
            .unwrap(),
    );
    let app = create_app(container);
    let token = admin_token(&app, &store.inner).await;
    let (_, setup) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/setup",
            &token,
            json!({"password":DEMO_PASSWORD}),
        ),
    )
    .await;
    let secret = setup["data"]["secret"].as_str().unwrap();
    let totp = totp_rs::Builder::new()
        .with_secret(totp_rs::Secret::try_from_base32(secret).unwrap())
        .build()
        .unwrap();
    let (status, enabled) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/enable",
            &token,
            json!({"password":DEMO_PASSWORD,"secondFactor":totp.generate_current().to_string()}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{enabled}");
    assert_eq!(
        enabled["data"]["recoveryCodes"].as_array().unwrap().len(),
        10
    );
    assert!(store.inner.token_owner(&token).await.unwrap().is_some());
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/me", &token))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    let code = take_captcha(&store.inner, captcha["data"]["uuid"].as_str().unwrap());
    let (_, step) = send(app.clone(), post_json("/api/v1/auth/login", json!({"username":"admin","password":DEMO_PASSWORD,"captcha":code,"captchaUuid":captcha["data"]["uuid"]}))).await;
    let (status, signed) = send(app.clone(), post_json("/api/v1/auth/login/2fa", json!({"challenge":step["data"]["challenge"],"secondFactor":enabled["data"]["recoveryCodes"][0]}))).await;
    assert_eq!(status, StatusCode::OK, "{signed}");
    assert_eq!(
        send(
            app.clone(),
            get_with_bearer("/api/v1/auth/me", signed["data"]["token"].as_str().unwrap())
        )
        .await
        .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn cache_failure_allows_authenticated_reads_but_never_bypasses_auth_store() {
    let initial = setup_container(Vec::new()).await;
    let store = Arc::new(FaultStore::default());
    let container = Arc::new(
        Container::bootstrap_with_stores(
            (*initial.config).clone(),
            store.clone(),
            Arc::new(FailedCache),
        )
        .await
        .unwrap(),
    );
    let app = create_app(container);
    let token = admin_token(&app, &store.inner).await;
    assert_eq!(
        send(
            app.clone(),
            get_with_bearer("/api/v1/system/depts/tree", &token)
        )
        .await
        .0,
        StatusCode::OK
    );
    store.reject_auth.store(true, Ordering::SeqCst);
    assert_eq!(
        send(
            app.clone(),
            get_with_bearer("/api/v1/system/depts/tree", &token)
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
}
