mod support;

use axum::{
    Router,
    http::{StatusCode, header},
};
use guardian_backend::{
    container::Container,
    infrastructure::{
        repositories::{account::AccountRepository, sql::execute},
        services::auth_store::{AuthStore, MemoryAuthStore, session_id},
    },
};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde_json::{Value, json};
use support::*;

async fn sessions(app: &Router, token: &str) -> Value {
    let (status, body) = send(app.clone(), get_with_bearer("/api/v1/auth/sessions", token)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["data"].clone()
}

async fn revoke(app: &Router, token: &str, id: &str, proof: Value) -> (StatusCode, Value) {
    send(
        app.clone(),
        post_json_with_bearer(&format!("/api/v1/auth/sessions/{id}/revoke"), token, proof),
    )
    .await
}

async fn enroll(app: &Router, token: &str) -> (Vec<String>, String) {
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
    let totp = totp_rs::Builder::new()
        .with_secret(
            totp_rs::Secret::try_from_base32(setup["data"]["secret"].as_str().unwrap()).unwrap(),
        )
        .build()
        .unwrap();
    let used = totp.generate_current().to_string();
    let (status, enabled) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/enable",
            token,
            json!({"password": DEMO_PASSWORD, "secondFactor": used}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{enabled}");
    (
        serde_json::from_value(enabled["data"]["recoveryCodes"].clone()).unwrap(),
        used,
    )
}

async fn login_factor(app: &Router, store: &MemoryAuthStore, factor: &str) -> String {
    let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    let uuid = captcha["data"]["uuid"].as_str().unwrap();
    let (_, challenge) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({
                "username": "admin", "password": DEMO_PASSWORD,
                "captcha": take_captcha(store, uuid), "captchaUuid": uuid,
            }),
        ),
    )
    .await;
    let (status, result) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login/2fa",
            json!({
                "challenge": challenge["data"]["challenge"], "secondFactor": factor,
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    result["data"]["token"].as_str().unwrap().to_string()
}

fn sign(container: &Container, claims: &Value) -> String {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(container.config.jwt.secret.as_bytes()),
    )
    .unwrap()
}

#[tokio::test]
async fn authenticated_user_can_list_own_sessions() {
    let (app, store, container) = setup_app_with_container().await;
    execute(
        &container.database,
        "DELETE FROM sys_user_role WHERE user_id = 2",
        vec![],
    )
    .await
    .unwrap();
    let first = login(&app, &store, "linjh", DEMO_PASSWORD).await;
    let second = login(&app, &store, "linjh", DEMO_PASSWORD).await;
    let third = login(&app, &store, "linjh", DEMO_PASSWORD).await;
    let other = admin_token(&app, &store).await;
    let listed = sessions(&app, &first).await;
    let items = listed["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(
        items.iter().filter(|item| item["current"] == true).count(),
        1
    );
    assert_eq!(
        items.iter().find(|item| item["current"] == true).unwrap()["id"],
        session_id(&first)
    );
    assert!(listed["nextCursor"].is_null());
    for item in items {
        assert!(item["issuedAt"].as_i64().unwrap() > 0);
        assert!(item["expiresAt"].as_i64().unwrap() > item["issuedAt"].as_i64().unwrap());
        assert!(item.get("ip").is_none());
        assert!(item.get("token").is_none());
    }
    for secret in [&first, &second, &third, &other] {
        assert!(!listed.to_string().contains(secret));
    }
    let before = AccountRepository::security(&container.database, 2)
        .await
        .unwrap();
    let (status, body) = revoke(
        &app,
        &first,
        &session_id(&second),
        json!({"password": DEMO_PASSWORD}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["revoked"], true);
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/me", &second))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    for token in [&first, &third, &other] {
        assert_eq!(
            send(app.clone(), get_with_bearer("/api/v1/auth/me", token))
                .await
                .0,
            StatusCode::OK
        );
    }
    assert_eq!(
        sessions(&app, &first).await["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let (status, repeated) = revoke(
        &app,
        &first,
        &session_id(&second),
        json!({"password": DEMO_PASSWORD}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{repeated}");
    assert_eq!(repeated["data"]["revoked"], false);
    let after = AccountRepository::security(&container.database, 2)
        .await
        .unwrap();
    assert_eq!(before.auth_version, after.auth_version);
    assert_eq!(before.secret, after.secret);
    assert_eq!(before.recovery_hashes, after.recovery_hashes);
}

#[tokio::test]
async fn identity_and_input_boundaries_cannot_revoke_current_or_foreign_sessions() {
    let (app, store) = setup_app_with_store().await;
    let admin = admin_token(&app, &store).await;
    let user = login(&app, &store, "linjh", DEMO_PASSWORD).await;
    assert_eq!(
        send(app.clone(), get("/api/v1/auth/sessions")).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(
            app.clone(),
            post_json(
                &format!("/api/v1/auth/sessions/{}/revoke", session_id(&user)),
                json!({"password": DEMO_PASSWORD})
            )
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    for cursor in ["", "-1", "+1", "1.0", "abc", "18446744073709551616"] {
        let (status, body) = send(
            app.clone(),
            get_with_bearer(&format!("/api/v1/auth/sessions?cursor={cursor}"), &admin),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "cursor={cursor}: {body}");
    }
    for (token, id, expected) in [
        (&admin, session_id(&admin), StatusCode::BAD_REQUEST),
        (&admin, session_id(&user), StatusCode::NOT_FOUND),
        (&user, session_id(&admin), StatusCode::NOT_FOUND),
        (&admin, session_id("unknown"), StatusCode::NOT_FOUND),
    ] {
        let (status, body) = revoke(&app, token, &id, json!({"password": DEMO_PASSWORD})).await;
        assert_eq!(status, expected, "{body}");
    }
    for token in [&admin, &user] {
        assert_eq!(
            send(app.clone(), get_with_bearer("/api/v1/auth/me", token))
                .await
                .0,
            StatusCode::OK
        );
    }
}

#[tokio::test]
async fn failed_password_proofs_lock_revoke_without_revoking_target() {
    let (app, store, container) = setup_app_with_container().await;
    let current = admin_token(&app, &store).await;
    let target = admin_token(&app, &store).await;
    for _ in 0..5 {
        let (status, body) = revoke(
            &app,
            &current,
            &session_id(&target),
            json!({"password": format!("wrong-{DEMO_PASSWORD}")}),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    }
    let (status, body) = revoke(
        &app,
        &current,
        &session_id(&target),
        json!({"password": DEMO_PASSWORD}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body["message"].as_str().unwrap().contains("次数过多"));
    let state = AccountRepository::security(&container.database, 1)
        .await
        .unwrap();
    assert_eq!(state.failures, 5);
    assert!(state.blocked_until > chrono::Utc::now().timestamp());
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/me", &target))
            .await
            .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn second_factor_is_required_and_recovery_code_is_consumed_once() {
    let (app, store, container) = setup_app_with_container().await;
    let initial = admin_token(&app, &store).await;
    let (codes, used_totp) = enroll(&app, &initial).await;
    let current = login_factor(&app, &store, &codes[0]).await;
    let target = login_factor(&app, &store, &codes[1]).await;
    let third = login_factor(&app, &store, &codes[2]).await;
    for proof in [
        json!({"password": DEMO_PASSWORD}),
        json!({"password": DEMO_PASSWORD, "secondFactor": used_totp}),
    ] {
        let (status, body) = revoke(&app, &current, &session_id(&target), proof).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    }
    let proof = json!({"password": DEMO_PASSWORD, "secondFactor": codes[3]});
    let target_id = session_id(&target);
    let third_id = session_id(&third);
    let (a, b) = tokio::join!(
        revoke(&app, &current, &target_id, proof.clone()),
        revoke(&app, &current, &third_id, proof.clone()),
    );
    assert_eq!(
        [a.0, b.0].iter().filter(|s| **s == StatusCode::OK).count(),
        1,
        "{a:?} {b:?}"
    );
    assert_eq!(
        [a.0, b.0]
            .iter()
            .filter(|s| **s == StatusCode::BAD_REQUEST)
            .count(),
        1
    );
    let live_target = if a.0 == StatusCode::OK {
        &third
    } else {
        &target
    };
    assert_eq!(
        revoke(&app, &current, &session_id(live_target), proof)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/me", live_target))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/me", &current))
            .await
            .0,
        StatusCode::OK
    );
    let state = AccountRepository::security(&container.database, 1)
        .await
        .unwrap();
    assert_eq!(state.auth_version, 1);
    assert!(state.secret.is_some());
    assert_eq!(
        serde_json::from_str::<Vec<String>>(&state.recovery_hashes)
            .unwrap()
            .len(),
        6
    );
    let (_, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/oper-logs", &current),
    )
    .await;
    let text = logs.to_string();
    assert!(text.contains("撤销会话"));
    assert!(!text.contains(DEMO_PASSWORD));
    assert!(!text.contains(&codes[3]));
    assert!(!text.contains(&current));
    assert!(!text.contains(&target));
}

#[tokio::test]
async fn successful_password_proof_does_not_reset_factor_lock() {
    let (app, store, container) = setup_app_with_container().await;
    let initial = admin_token(&app, &store).await;
    let (codes, _) = enroll(&app, &initial).await;
    let current = login_factor(&app, &store, &codes[0]).await;
    let target = login_factor(&app, &store, &codes[1]).await;
    for _ in 0..5 {
        assert_eq!(
            revoke(
                &app,
                &current,
                &session_id(&target),
                json!({"password": DEMO_PASSWORD, "secondFactor": "invalid"})
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    let (status, body) = revoke(
        &app,
        &current,
        &session_id(&target),
        json!({"password": DEMO_PASSWORD, "secondFactor": codes[2]}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body["message"].as_str().unwrap().contains("次数过多"));
    let state = AccountRepository::security(&container.database, 1)
        .await
        .unwrap();
    assert_eq!(state.failures, 0);
    assert_eq!(state.factor_failures, 5);
    assert!(state.factor_blocked_until > chrono::Utc::now().timestamp());
    assert_eq!(
        serde_json::from_str::<Vec<String>>(&state.recovery_hashes)
            .unwrap()
            .len(),
        8
    );
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/me", &target))
            .await
            .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn legacy_sessions_are_manageable_and_invalid_candidates_are_filtered() {
    let (app, store, container) = setup_app_with_container().await;
    let current = admin_token(&app, &store).await;
    let claims =
        serde_json::to_value(container.jwt_service.verify_access_token(&current).unwrap()).unwrap();
    let mut legacy = claims.clone();
    legacy.as_object_mut().unwrap().remove("jti");
    legacy.as_object_mut().unwrap().remove("auth_version");
    let old_token = sign(&container, &legacy);
    store.save_token(&old_token, "admin", 3600).await.unwrap();
    for (field, value) in [
        ("exp", json!(chrono::Utc::now().timestamp() - 1)),
        ("exp", json!(chrono::Utc::now().timestamp() - 600)),
        ("sub", json!("2")),
        ("username", json!("linjh")),
        ("auth_version", json!(99)),
    ] {
        let mut invalid = claims.clone();
        invalid[field] = value;
        store
            .save_token(&sign(&container, &invalid), "admin", 3600)
            .await
            .unwrap();
    }
    let mut expired_in_store = claims.clone();
    expired_in_store["jti"] = json!("expired-in-store");
    store
        .save_token(&sign(&container, &expired_in_store), "admin", 0)
        .await
        .unwrap();
    store.save_token("not-a-jwt", "admin", 3600).await.unwrap();
    let listed = sessions(&app, &current).await;
    let items = listed["items"].as_array().unwrap();
    assert_eq!(items.len(), 2, "{listed}");
    let old = items
        .iter()
        .find(|item| item["id"] == session_id(&old_token))
        .unwrap();
    assert_eq!(old["browser"], "");
    assert_eq!(old["os"], "");
    let old_list = sessions(&app, &old_token).await;
    assert_eq!(
        old_list["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["current"] == true)
            .unwrap()["id"],
        session_id(&old_token)
    );
    assert_eq!(
        revoke(
            &app,
            &current,
            &session_id(&old_token),
            json!({"password": DEMO_PASSWORD})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/me", &old_token))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    AccountRepository::bump_auth_version(&container.database, 1)
        .await
        .unwrap();
    let fresh = admin_token(&app, &store).await;
    let listed = sessions(&app, &fresh).await;
    assert_eq!(listed["items"].as_array().unwrap().len(), 1, "{listed}");
    assert_eq!(listed["items"][0]["id"], session_id(&fresh));
    assert_eq!(
        send(
            app.clone(),
            get_with_bearer("/api/v1/auth/sessions", &current)
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    execute(
        &container.database,
        "UPDATE sys_user SET status = 0 WHERE id = 1",
        vec![],
    )
    .await
    .unwrap();
    assert_eq!(
        send(app, get_with_bearer("/api/v1/auth/sessions", &fresh))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn cursor_pages_preserve_remaining_sessions_when_a_target_is_revoked() {
    let (app, store, container) = setup_app_with_container().await;
    let current = admin_token(&app, &store).await;
    for _ in 0..55 {
        let token = container
            .jwt_service
            .issue_access_token("1", "admin", &[])
            .unwrap();
        store
            .save_token(&token.access_token, "admin", 3600)
            .await
            .unwrap();
    }
    let first = sessions(&app, &current).await;
    let items = first["items"].as_array().unwrap();
    assert_eq!(items.len(), 50);
    let cursor = first["nextCursor"].as_str().unwrap();
    let target = items.iter().find(|item| item["current"] == false).unwrap()["id"]
        .as_str()
        .unwrap();
    assert_eq!(
        revoke(&app, &current, target, json!({"password": DEMO_PASSWORD}))
            .await
            .0,
        StatusCode::OK
    );
    let (status, page) = send(
        app.clone(),
        get_with_bearer(&format!("/api/v1/auth/sessions?cursor={cursor}"), &current),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(page["data"]["items"].as_array().unwrap().len(), 6);
    assert!(page["data"]["nextCursor"].is_null());
    for item in page["data"]["items"].as_array().unwrap() {
        assert!(!items.iter().any(|previous| previous["id"] == item["id"]));
    }
}

#[tokio::test]
async fn already_invalid_target_returns_false_after_proof_and_cleans_storage() {
    let (app, store, container) = setup_app_with_container().await;
    let current = admin_token(&app, &store).await;
    let claims =
        serde_json::to_value(container.jwt_service.verify_access_token(&current).unwrap()).unwrap();
    for (field, value) in [
        ("exp", json!(chrono::Utc::now().timestamp() - 1)),
        ("auth_version", json!(99)),
    ] {
        let mut invalid = claims.clone();
        invalid[field] = value;
        let target = sign(&container, &invalid);
        store.save_token(&target, "admin", 3600).await.unwrap();
        let (status, body) = revoke(
            &app,
            &current,
            &session_id(&target),
            json!({"password": format!("wrong-{DEMO_PASSWORD}")}),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert!(store.token_owner(&target).await.unwrap().is_some());
        let (status, body) = revoke(
            &app,
            &current,
            &session_id(&target),
            json!({"password": DEMO_PASSWORD}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["data"]["revoked"], false);
        assert!(store.token_owner(&target).await.unwrap().is_none());
    }
    assert_eq!(
        send(app, get_with_bearer("/api/v1/auth/me", &current))
            .await
            .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn session_metadata_uses_parsed_login_client_without_ip_or_raw_agent() {
    let (app, store) = setup_app_with_store().await;
    let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    let uuid = captcha["data"]["uuid"].as_str().unwrap();
    let mut request = post_json(
        "/api/v1/auth/login",
        json!({
            "username": "admin", "password": DEMO_PASSWORD,
            "captcha": take_captcha(&store, uuid), "captchaUuid": uuid,
        }),
    );
    let agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
    request
        .headers_mut()
        .insert(header::USER_AGENT, agent.parse().unwrap());
    let (status, login) = send(app.clone(), request).await;
    assert_eq!(status, StatusCode::OK, "{login}");
    let listed = sessions(&app, login["data"]["token"].as_str().unwrap()).await;
    assert_eq!(listed["items"][0]["browser"], "Chrome");
    assert_eq!(listed["items"][0]["os"], "Windows 10");
    assert!(listed["items"][0].get("ip").is_none());
    assert!(!listed.to_string().contains(agent));
    assert!(!listed.to_string().contains("141.0.0.0"));
}
