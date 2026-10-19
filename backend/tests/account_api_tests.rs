mod support;
use axum::http::StatusCode;
use guardian_backend::infrastructure::services::auth_store::MemoryAuthStore;
use serde_json::{Value, json};
use support::*;

async fn enroll(app: &axum::Router, token: &str) -> (String, Vec<String>, String) {
    let (status, setup) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/setup",
            token,
            json!({"password": DEMO_PASSWORD}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{setup}");
    let secret = setup["data"]["secret"].as_str().unwrap().to_string();
    assert!(
        setup["data"]["otpauthUrl"]
            .as_str()
            .unwrap()
            .starts_with("otpauth://totp/")
    );
    let totp = totp_rs::Builder::new()
        .with_secret(totp_rs::Secret::try_from_base32(&secret).unwrap())
        .build()
        .unwrap();
    let used_code = totp.generate_current().to_string();
    let (status, enabled) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/enable",
            token,
            json!({"password": DEMO_PASSWORD, "secondFactor": used_code}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{enabled}");
    (
        secret,
        serde_json::from_value(enabled["data"]["recoveryCodes"].clone()).unwrap(),
        used_code,
    )
}
async fn login_factor(
    app: &axum::Router,
    store: &MemoryAuthStore,
    username: &str,
    factor: Option<&str>,
) -> (StatusCode, Value) {
    let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    let code = take_captcha(store, captcha["data"]["uuid"].as_str().unwrap());
    let result = send(app.clone(), post_json("/api/v1/auth/login", json!({"username": username, "password": DEMO_PASSWORD, "captcha": code, "captchaUuid": captcha["data"]["uuid"]}))).await;
    if result.1["data"]["twoFactorRequired"] != true {
        return result;
    }
    assert!(result.1["data"].get("token").is_none());
    send(app.clone(), post_json("/api/v1/auth/login/2fa", json!({"challenge":result.1["data"]["challenge"], "secondFactor":factor.unwrap_or_default()}))).await
}

#[tokio::test]
async fn profile_updates_only_own_allowed_fields() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;
    let (_, before) = send(app.clone(), get_with_bearer("/api/v1/auth/profile", &token)).await;
    assert_eq!(before["data"]["twoFactorEnabled"], false);
    let body = json!({"nickname": "新昵称", "phone": "+86 13800000000", "email": "me@example.com"});
    assert_eq!(
        send(
            app.clone(),
            put_json_with_bearer("/api/v1/auth/profile", &token, body)
        )
        .await
        .0,
        StatusCode::OK
    );
    let (_, after) = send(app.clone(), get_with_bearer("/api/v1/auth/profile", &token)).await;
    assert_eq!(after["data"]["nickname"], "新昵称");
    assert_eq!(after["data"]["phone"], "+86 13800000000");
    for body in [
        json!({"nickname":"", "email":null}),
        json!({"nickname":"x", "roles":["super_admin"]}),
        json!({"nickname":"x","email":"invalid"}),
    ] {
        assert_eq!(
            send(
                app.clone(),
                put_json_with_bearer("/api/v1/auth/profile", &token, body)
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn two_factor_lifecycle_replay_recovery_and_revocation() {
    let (app, store) = setup_app_with_store().await;
    let first = admin_token(&app, store.as_ref()).await;
    let second = admin_token(&app, store.as_ref()).await;
    assert_ne!(first, second);
    let (secret, codes, replay) = enroll(&app, &first).await;
    assert_eq!(codes.len(), 10);
    for token in [&first, &second] {
        assert_eq!(
            send(app.clone(), get_with_bearer("/api/v1/auth/me", token))
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        login_factor(&app, store.as_ref(), "admin", None).await.0,
        StatusCode::BAD_REQUEST
    );

    assert_eq!(
        login_factor(&app, store.as_ref(), "admin", Some(&replay))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let (a, b) = tokio::join!(
        login_factor(&app, store.as_ref(), "admin", Some(&codes[0])),
        login_factor(&app, store.as_ref(), "admin", Some(&codes[0]))
    );
    assert_eq!(
        [a.0, b.0].iter().filter(|s| **s == StatusCode::OK).count(),
        1
    );
    let token = if a.0 == StatusCode::OK { &a.1 } else { &b.1 }["data"]["token"]
        .as_str()
        .unwrap();
    let (_, profile) = send(app.clone(), get_with_bearer("/api/v1/auth/profile", token)).await;
    assert_eq!(profile["data"]["recoveryCodesRemaining"], 9);
    let (status, rotated) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/recovery-codes",
            token,
            json!({"password":DEMO_PASSWORD,"secondFactor":codes[1]}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{rotated}");
    assert_eq!(
        login_factor(&app, store.as_ref(), "admin", Some(&codes[2]))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let new_codes = rotated["data"]["recoveryCodes"].as_array().unwrap();
    let (_, signed) = login_factor(&app, store.as_ref(), "admin", new_codes[0].as_str()).await;
    let token = signed["data"]["token"].as_str().unwrap();
    assert_eq!(
        send(
            app.clone(),
            post_json_with_bearer(
                "/api/v1/auth/2fa/disable",
                token,
                json!({"password":DEMO_PASSWORD,"secondFactor":new_codes[1]})
            )
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        login_factor(&app, store.as_ref(), "admin", None).await.0,
        StatusCode::OK
    );
    let token = admin_token(&app, store.as_ref()).await;
    let (_, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/oper-logs", &token),
    )
    .await;
    let logs = logs.to_string();
    assert!(logs.contains("启用2FA"));
    assert!(!logs.contains(&secret));
    assert!(!logs.contains(&codes[1]));
}

#[tokio::test]
async fn reset_requires_separate_permission_password_and_invalidates_target_session() {
    let (app, store) = setup_app_with_store().await;
    let admin = admin_token(&app, store.as_ref()).await;
    let user = login(&app, store.as_ref(), "linjh", DEMO_PASSWORD).await;
    let (_, codes, _) = enroll(&app, &user).await;
    let (_, signed) = login_factor(&app, store.as_ref(), "linjh", Some(&codes[0])).await;
    let user = signed["data"]["token"].as_str().unwrap();
    let path = "/api/v1/system/users/2/2fa/reset";
    assert_eq!(
        send(
            app.clone(),
            post_json_with_bearer(path, user, json!({"password":DEMO_PASSWORD}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(
            app.clone(),
            post_json_with_bearer(path, &admin, json!({"password":"bad"}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(
            app.clone(),
            post_json_with_bearer(path, &admin, json!({"password":DEMO_PASSWORD}))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/me", user))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        login_factor(&app, store.as_ref(), "linjh", None).await.0,
        StatusCode::OK
    );
    assert_eq!(
        send(
            app.clone(),
            post_json_with_bearer(
                "/api/v1/system/users/1/2fa/reset",
                &admin,
                json!({"password":DEMO_PASSWORD})
            )
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn enrollment_requires_confirmation_and_limits_failed_proofs() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;
    for _ in 0..5 {
        assert_eq!(
            send(
                app.clone(),
                post_json_with_bearer(
                    "/api/v1/auth/2fa/setup",
                    &token,
                    json!({"password":"wrong"})
                )
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/setup",
            &token,
            json!({"password":DEMO_PASSWORD}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["message"].as_str().unwrap().contains("次数过多"));
    let (_, profile) = send(app.clone(), get_with_bearer("/api/v1/auth/profile", &token)).await;
    assert_eq!(profile["data"]["twoFactorEnabled"], false);
}
