mod support;
use axum::http::StatusCode;
use guardian_backend::{
    create_app::create_app,
    infrastructure::{repositories::sql::execute, services::auth_store::MemoryAuthStore},
};
use serde_json::{Value, json};
use std::sync::Arc;
use support::*;

async fn challenge(app: &axum::Router, store: &MemoryAuthStore) -> Value {
    let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    let code = take_captcha(store, captcha["data"]["uuid"].as_str().unwrap());
    let (status, result) = send(app.clone(), post_json("/api/v1/auth/login", json!({"username":"admin","password":DEMO_PASSWORD,"captcha":code,"captchaUuid":captcha["data"]["uuid"]}))).await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["data"]["twoFactorRequired"], true);
    assert!(result["data"].get("token").is_none());
    result["data"]["challenge"].clone()
}
async fn verify(app: &axum::Router, challenge: &Value, code: &Value) -> (StatusCode, Value) {
    send(
        app.clone(),
        post_json(
            "/api/v1/auth/login/2fa",
            json!({"challenge":challenge,"secondFactor":code}),
        ),
    )
    .await
}

#[tokio::test]
async fn login_challenge_is_not_a_session_expires_and_is_consumed_once() {
    let store = Arc::new(MemoryAuthStore::new());
    let container = setup_container_with_store(Vec::new(), store.clone()).await;
    let app = create_app(container.clone());
    let token = admin_token(&app, store.as_ref()).await;
    let (_, setup) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/setup",
            &token,
            json!({"password":DEMO_PASSWORD}),
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
            &token,
            json!({"password":DEMO_PASSWORD,"secondFactor":totp.generate_current().to_string()}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let codes = &enabled["data"]["recoveryCodes"];
    let first = challenge(&app, store.as_ref()).await;
    assert_eq!(
        send(
            app.clone(),
            get_with_bearer("/api/v1/auth/me", first.as_str().unwrap())
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let current = challenge(&app, store.as_ref()).await;
    assert_eq!(
        verify(&app, &first, &codes[0]).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        verify(&app, &current, &json!("wrong")).await.0,
        StatusCode::BAD_REQUEST
    );
    let (a, b) = tokio::join!(
        verify(&app, &current, &codes[0]),
        verify(&app, &current, &codes[1])
    );
    assert_eq!(
        [a.0, b.0].iter().filter(|s| **s == StatusCode::OK).count(),
        1
    );
    assert_eq!(
        verify(&app, &current, &codes[2]).await.0,
        StatusCode::BAD_REQUEST
    );
    let expired = challenge(&app, store.as_ref()).await;
    execute(
        &container.database,
        "UPDATE sys_user_security SET login_challenge_expires = 1 WHERE user_id = 1",
        vec![],
    )
    .await
    .unwrap();
    assert_eq!(
        verify(&app, &expired, &codes[2]).await.0,
        StatusCode::BAD_REQUEST
    );
    let changed_password = challenge(&app, store.as_ref()).await;
    execute(
        &container.database,
        "UPDATE sys_user_security SET login_password_hash = 'changed' WHERE user_id = 1",
        vec![],
    )
    .await
    .unwrap();
    assert_eq!(
        verify(&app, &changed_password, &codes[2]).await.0,
        StatusCode::BAD_REQUEST
    );
    let limited = challenge(&app, store.as_ref()).await;
    for _ in 0..5 {
        assert_eq!(
            verify(&app, &limited, &json!("wrong")).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    let denied = verify(&app, &limited, &codes[2]).await;
    assert!(denied.1["message"].as_str().unwrap().contains("次数过多"));
}

/// 2FA 失败计数必须独立于口令阶段：口令校验成功不能清零，否则 2FA 重试次数无限。
#[tokio::test]
async fn password_step_success_does_not_reset_factor_failures() {
    let store = Arc::new(MemoryAuthStore::new());
    let container = setup_container_with_store(Vec::new(), store.clone()).await;
    let app = create_app(container.clone());
    let token = admin_token(&app, store.as_ref()).await;
    let (_, setup) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/setup",
            &token,
            json!({"password":DEMO_PASSWORD}),
        ),
    )
    .await;
    let totp = totp_rs::Builder::new()
        .with_secret(
            totp_rs::Secret::try_from_base32(setup["data"]["secret"].as_str().unwrap()).unwrap(),
        )
        .build()
        .unwrap();
    let (status, _) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/enable",
            &token,
            json!({"password":DEMO_PASSWORD,"secondFactor":totp.generate_current().to_string()}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 4 次 2FA 失败后，再用正确口令拿一个新 challenge（口令阶段成功）。
    let mut challenge_value = challenge(&app, store.as_ref()).await;
    for _ in 0..4 {
        let (status, _) = verify(&app, &challenge_value, &json!("wrong")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        challenge_value = challenge(&app, store.as_ref()).await;
    }

    // 第 5 次失败必须直接锁住：若计数被口令阶段清零，这里仍然是「第四次」。
    let (status, _) = verify(&app, &challenge_value, &json!("wrong")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let locked = challenge(&app, store.as_ref()).await;
    let (_, body) = verify(&app, &locked, &json!("wrong")).await;
    assert!(
        body["message"].as_str().unwrap().contains("次数过多"),
        "{body}"
    );
    assert_eq!(
        verify(&app, &locked, &json!("123456")).await.0,
        StatusCode::BAD_REQUEST
    );
}

/// 2FA 锁定不影响必须验口令的入口：锁定期间仍能开启 / 关闭 2FA，
/// 否则用户只能干等锁定期结束。
#[tokio::test]
async fn factor_lock_does_not_block_password_step() {
    let store = Arc::new(MemoryAuthStore::new());
    let container = setup_container_with_store(Vec::new(), store.clone()).await;
    let app = create_app(container.clone());
    let token = admin_token(&app, store.as_ref()).await;

    // 直接构造「2FA 阶段已锁定」的状态，不必真的打满重试。
    let now = chrono::Utc::now().timestamp();
    execute(
        &container.database,
        &format!(
            "UPDATE sys_user_security SET factor_failures = 5, factor_blocked_until = {} WHERE user_id = 1",
            now + 300
        ),
        vec![],
    )
    .await
    .expect("写入 2FA 锁定");

    // 口令阶段照常通过：能拿到 2FA 绑定密钥，而不是被 2FA 计数挡住。
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/setup",
            &token,
            json!({"password":DEMO_PASSWORD}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body["data"]["secret"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );

    // 口令错误时仍然按口令计数报错，而不是报 2FA 的次数。
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/setup",
            &token,
            json!({"password":"wrong-password"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "密码不正确", "{body}");
}
