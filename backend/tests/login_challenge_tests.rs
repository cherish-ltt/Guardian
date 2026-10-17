mod support;
use axum::http::StatusCode;
use guardian_backend::{create_app::create_app, infrastructure::repositories::sql::execute};
use serde_json::{Value, json};
use support::*;

async fn challenge(app: &axum::Router) -> Value {
    let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    let (status, result) = send(app.clone(), post_json("/api/v1/auth/login", json!({"username":"admin","password":DEMO_PASSWORD,"captcha":captcha["data"]["code"],"captchaUuid":captcha["data"]["uuid"]}))).await;
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
    let container = setup_container(Vec::new()).await;
    let app = create_app(container.clone());
    let token = admin_token(&app).await;
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
    let first = challenge(&app).await;
    assert_eq!(
        send(
            app.clone(),
            get_with_bearer("/api/v1/auth/me", first.as_str().unwrap())
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let current = challenge(&app).await;
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
    let expired = challenge(&app).await;
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
    let changed_password = challenge(&app).await;
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
    let limited = challenge(&app).await;
    for _ in 0..5 {
        assert_eq!(
            verify(&app, &limited, &json!("wrong")).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    let denied = verify(&app, &limited, &codes[2]).await;
    assert!(denied.1["message"].as_str().unwrap().contains("次数过多"));
}
