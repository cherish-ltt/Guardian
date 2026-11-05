mod support;

use async_trait::async_trait;
use axum::http::StatusCode;
use guardian_backend::{
    container::Container,
    create_app::create_app,
    domain::error::AppError,
    infrastructure::{
        repositories::{account::AccountRepository, sql::execute},
        services::{
            auth_store::{
                AuthStore, MemoryAuthStore, SessionMetadata, StoredSessionPage, session_id,
            },
            cache_store::CacheStore,
        },
    },
};
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use support::*;
use tokio::{sync::Notify, time::Duration};

#[derive(Default)]
struct FaultStore {
    inner: MemoryAuthStore,
    reject_auth: AtomicBool,
    reject_save: AtomicBool,
    reject_list: AtomicBool,
    reject_find: AtomicBool,
    reject_delete: AtomicBool,
    reject_cleanup: AtomicBool,
    pause_list: AtomicBool,
    list_entered: Notify,
    resume_list: Notify,
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
    async fn save_session(
        &self,
        token: &str,
        user: &str,
        ttl: u64,
        metadata: &SessionMetadata,
    ) -> Result<(), AppError> {
        if self.reject_save.load(Ordering::SeqCst) {
            return Err(AppError::unavailable("save unavailable"));
        }
        self.inner.save_session(token, user, ttl, metadata).await
    }
    async fn list_sessions(&self, user: &str, cursor: u64) -> Result<StoredSessionPage, AppError> {
        if self.reject_list.load(Ordering::SeqCst) {
            return Err(AppError::unavailable("list unavailable"));
        }
        if self.pause_list.swap(false, Ordering::SeqCst) {
            self.list_entered.notify_one();
            self.resume_list.notified().await;
        }
        self.inner.list_sessions(user, cursor).await
    }
    async fn session_token(&self, user: &str, id: &str) -> Result<Option<String>, AppError> {
        if self.reject_find.load(Ordering::SeqCst) {
            return Err(AppError::unavailable("find unavailable"));
        }
        self.inner.session_token(user, id).await
    }
    async fn revoke_session(&self, user: &str, id: &str) -> Result<bool, AppError> {
        if self.reject_delete.load(Ordering::SeqCst) {
            return Err(AppError::unavailable("delete unavailable"));
        }
        let revoked = self.inner.revoke_session(user, id).await?;
        if self.reject_cleanup.load(Ordering::SeqCst) {
            return Err(AppError::unavailable("cleanup unavailable"));
        }
        Ok(revoked)
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

#[tokio::test]
async fn session_listing_snapshot_does_not_revoke_a_new_security_version() {
    let store = Arc::new(FaultStore::default());
    let container = setup_container_with_store(Vec::new(), store.clone()).await;
    let app = create_app(container.clone());
    let old = admin_token(&app, &store.inner).await;
    store.pause_list.store(true, Ordering::SeqCst);
    let listing = tokio::spawn(send(
        app.clone(),
        get_with_bearer("/api/v1/auth/sessions", &old),
    ));
    tokio::time::timeout(Duration::from_secs(5), store.list_entered.notified())
        .await
        .expect("列表请求已读取身份版本并等待扫描");

    AccountRepository::bump_auth_version(&container.database, 1)
        .await
        .unwrap();
    store.inner.revoke_user_tokens("admin").await.unwrap();
    let new = admin_token(&app, &store.inner).await;
    assert_eq!(
        container
            .jwt_service
            .verify_access_token(&new)
            .unwrap()
            .auth_version,
        1
    );
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/me", &new))
            .await
            .0,
        StatusCode::OK
    );

    store.resume_list.notify_one();
    let (status, body) = listing.await.unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["data"]["items"].as_array().unwrap().is_empty());
    assert_eq!(
        send(app, get_with_bearer("/api/v1/auth/me", &new)).await.0,
        StatusCode::OK,
        "旧版本列表请求不得撤销新版本登录"
    );
}

#[tokio::test]
async fn session_listing_snapshot_does_not_revoke_a_recreated_account() {
    let store = Arc::new(FaultStore::default());
    let container = setup_container_with_store(Vec::new(), store.clone()).await;
    let app = create_app(container.clone());
    let old = login(&app, &store.inner, "linjh", DEMO_PASSWORD).await;
    store.pause_list.store(true, Ordering::SeqCst);
    let listing = tokio::spawn(send(
        app.clone(),
        get_with_bearer("/api/v1/auth/sessions", &old),
    ));
    tokio::time::timeout(Duration::from_secs(5), store.list_entered.notified())
        .await
        .expect("列表请求已读取原账号身份并等待扫描");

    execute(
        &container.database,
        "UPDATE sys_user SET deleted = 1 WHERE id = 2",
        vec![],
    )
    .await
    .unwrap();
    execute(
        &container.database,
        "INSERT INTO sys_user (username, password, nickname, status) \
         SELECT username, password, nickname, status FROM sys_user WHERE id = 2",
        vec![],
    )
    .await
    .unwrap();
    store.inner.revoke_user_tokens("linjh").await.unwrap();
    let new = login(&app, &store.inner, "linjh", DEMO_PASSWORD).await;
    assert_ne!(
        container.jwt_service.verify_access_token(&new).unwrap().sub,
        "2"
    );
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/me", &new))
            .await
            .0,
        StatusCode::OK
    );

    store.resume_list.notify_one();
    let (status, body) = listing.await.unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["data"]["items"].as_array().unwrap().is_empty());
    assert_eq!(
        send(app, get_with_bearer("/api/v1/auth/me", &new)).await.0,
        StatusCode::OK,
        "原账号列表请求不得撤销同名新账号的登录"
    );
}

#[tokio::test]
async fn session_store_failures_return_503_without_success_or_unrelated_revocation() {
    let store = Arc::new(FaultStore::default());
    let container = setup_container_with_store(Vec::new(), store.clone()).await;
    let app = create_app(container);
    let current = admin_token(&app, &store.inner).await;
    let target = admin_token(&app, &store.inner).await;
    let other = login(&app, &store.inner, "linjh", DEMO_PASSWORD).await;
    let path = format!("/api/v1/auth/sessions/{}/revoke", session_id(&target));
    store.reject_save.store(true, Ordering::SeqCst);
    let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    let uuid = captcha["data"]["uuid"].as_str().unwrap();
    let (status, body) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({
                "username": "admin", "password": DEMO_PASSWORD,
                "captcha": take_captcha(&store.inner, uuid), "captchaUuid": uuid,
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert!(body["data"].get("token").is_none());
    assert_eq!(
        store
            .inner
            .list_sessions("admin", 0)
            .await
            .unwrap()
            .sessions
            .len(),
        2
    );
    store.reject_save.store(false, Ordering::SeqCst);
    store.reject_list.store(true, Ordering::SeqCst);
    assert_eq!(
        send(
            app.clone(),
            get_with_bearer("/api/v1/auth/sessions", &current)
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    store.reject_list.store(false, Ordering::SeqCst);
    for flag in [&store.reject_find, &store.reject_delete] {
        flag.store(true, Ordering::SeqCst);
        let (status, body) = send(
            app.clone(),
            post_json_with_bearer(&path, &current, json!({"password": DEMO_PASSWORD})),
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
        assert!(body["data"].get("revoked").is_none());
        flag.store(false, Ordering::SeqCst);
        assert_eq!(
            send(app.clone(), get_with_bearer("/api/v1/auth/me", &target))
                .await
                .0,
            StatusCode::OK
        );
    }
    store.reject_cleanup.store(true, Ordering::SeqCst);
    assert_eq!(
        send(
            app.clone(),
            post_json_with_bearer(&path, &current, json!({"password": DEMO_PASSWORD}))
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/me", &target))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    store.reject_cleanup.store(false, Ordering::SeqCst);
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(&path, &current, json!({"password": DEMO_PASSWORD})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["revoked"], false);
    for token in [&current, &other] {
        assert_eq!(
            send(app.clone(), get_with_bearer("/api/v1/auth/me", token))
                .await
                .0,
            StatusCode::OK
        );
    }
}

#[tokio::test]
async fn redis_delete_failure_never_restores_consumed_second_factor() {
    let store = Arc::new(FaultStore::default());
    let container = setup_container_with_store(Vec::new(), store.clone()).await;
    let app = create_app(container.clone());
    let initial = admin_token(&app, &store.inner).await;
    let (_, setup) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/setup",
            &initial,
            json!({"password": DEMO_PASSWORD}),
        ),
    )
    .await;
    let totp = totp_rs::Builder::new()
        .with_secret(
            totp_rs::Secret::try_from_base32(setup["data"]["secret"].as_str().unwrap()).unwrap(),
        )
        .build()
        .unwrap();
    let (status, enabled) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/enable",
            &initial,
            json!({"password": DEMO_PASSWORD, "secondFactor": totp.generate_current().to_string()}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{enabled}");
    let codes = enabled["data"]["recoveryCodes"].as_array().unwrap();
    let mut tokens = Vec::new();
    for code in &codes[..3] {
        let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
        let uuid = captcha["data"]["uuid"].as_str().unwrap();
        let (_, step) = send(app.clone(), post_json("/api/v1/auth/login", json!({"username": "admin", "password": DEMO_PASSWORD, "captcha": take_captcha(&store.inner, uuid), "captchaUuid": uuid}))).await;
        let (status, signed) = send(
            app.clone(),
            post_json(
                "/api/v1/auth/login/2fa",
                json!({"challenge": step["data"]["challenge"], "secondFactor": code}),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{signed}");
        tokens.push(signed["data"]["token"].as_str().unwrap().to_string());
    }
    for (index, flag) in [&store.reject_delete, &store.reject_cleanup]
        .into_iter()
        .enumerate()
    {
        let path = format!(
            "/api/v1/auth/sessions/{}/revoke",
            session_id(&tokens[index + 1])
        );
        let proof = json!({"password": DEMO_PASSWORD, "secondFactor": codes[3 + index * 2]});
        flag.store(true, Ordering::SeqCst);
        let (status, body) = send(
            app.clone(),
            post_json_with_bearer(&path, &tokens[0], proof.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
        flag.store(false, Ordering::SeqCst);
        assert_eq!(
            send(app.clone(), post_json_with_bearer(&path, &tokens[0], proof))
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
        let (status, body) = send(
            app.clone(),
            post_json_with_bearer(
                &path,
                &tokens[0],
                json!({"password": DEMO_PASSWORD, "secondFactor": codes[4 + index * 2]}),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["data"]["revoked"], index == 0);
        assert_eq!(
            send(
                app.clone(),
                get_with_bearer("/api/v1/auth/me", &tokens[index + 1])
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        send(app, get_with_bearer("/api/v1/auth/me", &tokens[0]))
            .await
            .0,
        StatusCode::OK
    );
    let state = AccountRepository::security(&container.database, 1)
        .await
        .unwrap();
    assert_eq!(state.auth_version, 1);
    assert_eq!(
        serde_json::from_str::<Vec<String>>(&state.recovery_hashes)
            .unwrap()
            .len(),
        3
    );
}
