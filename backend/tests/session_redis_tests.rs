mod support;

use axum::{Router, http::StatusCode};
use guardian_backend::{
    create_app::create_app,
    infrastructure::{
        config::RedisConfig,
        repositories::sql::execute,
        services::{
            auth_store::{AuthStore, captcha_key, session_id, token_key, user_key},
            redis_store::RedisAuthStore,
        },
    },
};
use jsonwebtoken::{EncodingKey, Header, encode};
use redis::{AsyncCommands, aio::MultiplexedConnection};
use serde_json::{Value, json};
use std::sync::Arc;
use support::*;

async fn redis_login(
    app: &Router,
    connection: &mut MultiplexedConnection,
    username: &str,
) -> (StatusCode, Value) {
    let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    let uuid = captcha["data"]["uuid"].as_str().unwrap();
    let code: String = connection.get(captcha_key(uuid)).await.unwrap();
    send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({
                "username": username, "password": DEMO_PASSWORD,
                "captcha": code, "captchaUuid": uuid,
            }),
        ),
    )
    .await
}

async fn revoke(app: &Router, current: &str, target: &str) -> (StatusCode, Value) {
    send(
        app.clone(),
        post_json_with_bearer(
            &format!("/api/v1/auth/sessions/{}/revoke", session_id(target)),
            current,
            json!({"password": DEMO_PASSWORD}),
        ),
    )
    .await
}

#[tokio::test]
#[ignore = "requires TEST_REDIS_URL pointing to a disposable Redis"]
async fn redis_router_sessions_enforce_ownership_legacy_compatibility_and_failure_semantics() {
    let url = std::env::var("TEST_REDIS_URL").expect("TEST_REDIS_URL is required");
    let mut connection = redis::Client::open(url.as_str())
        .unwrap()
        .get_multiplexed_async_connection()
        .await
        .unwrap();
    let store = Arc::new(
        RedisAuthStore::connect(&RedisConfig {
            url,
            password: None,
        })
        .await
        .unwrap(),
    );
    let container = setup_container_with_store(Vec::new(), store.clone()).await;
    let app = create_app(container.clone());
    let owner = format!("redis-{}", uuid::Uuid::now_v7());
    let other_owner = format!("other-{}", uuid::Uuid::now_v7());
    for (id, username) in [(1, &owner), (2, &other_owner)] {
        execute(
            &container.database,
            "UPDATE sys_user SET username = ? WHERE id = ?",
            vec![username.clone().into(), id.into()],
        )
        .await
        .unwrap();
    }
    let mut tokens = Vec::new();
    for username in [&owner, &owner, &owner, &other_owner] {
        let (status, result) = redis_login(&app, &mut connection, username).await;
        assert_eq!(status, StatusCode::OK, "{result}");
        tokens.push(result["data"]["token"].as_str().unwrap().to_string());
    }
    let current = &tokens[0];
    let target = &tokens[1];
    let trusted = &tokens[2];
    let other = &tokens[3];
    let (status, listed) = send(
        app.clone(),
        get_with_bearer("/api/v1/auth/sessions", current),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert_eq!(listed["data"]["items"].as_array().unwrap().len(), 3);
    assert_eq!(
        listed["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["current"] == true)
            .count(),
        1
    );
    for token in &tokens {
        assert!(!listed.to_string().contains(token));
    }
    assert_eq!(revoke(&app, current, other).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        revoke(&app, current, current).await.0,
        StatusCode::BAD_REQUEST
    );

    let mut legacy_claims =
        serde_json::to_value(container.jwt_service.verify_access_token(current).unwrap()).unwrap();
    legacy_claims.as_object_mut().unwrap().remove("jti");
    legacy_claims
        .as_object_mut()
        .unwrap()
        .remove("auth_version");
    let legacy = encode(
        &Header::default(),
        &legacy_claims,
        &EncodingKey::from_secret(container.config.jwt.secret.as_bytes()),
    )
    .unwrap();
    connection
        .set_ex::<_, _, ()>(token_key(&legacy), &owner, 120)
        .await
        .unwrap();
    connection
        .sadd::<_, _, ()>(user_key(&owner), &legacy)
        .await
        .unwrap();
    connection
        .sadd::<_, _, ()>(user_key(&owner), "expired-residual-token")
        .await
        .unwrap();
    let legacy_mapping = format!("auth:session:{owner}:{}", session_id(&legacy));
    assert!(!connection.exists::<_, bool>(&legacy_mapping).await.unwrap());
    let before: i64 = connection.pttl(token_key(&legacy)).await.unwrap();
    let (status, listed) = send(
        app.clone(),
        get_with_bearer("/api/v1/auth/sessions", current),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    let old = listed["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == session_id(&legacy))
        .unwrap();
    assert_eq!(old["browser"], "");
    assert_eq!(old["os"], "");
    let after: i64 = connection.pttl(token_key(&legacy)).await.unwrap();
    let mapping_ttl: i64 = connection.pttl(&legacy_mapping).await.unwrap();
    assert!(after <= before && mapping_ttl <= after);
    assert!(
        !connection
            .sismember::<_, _, bool>(user_key(&owner), "expired-residual-token")
            .await
            .unwrap()
    );

    let expiring = container
        .jwt_service
        .issue_access_token("1", &owner, &[])
        .unwrap()
        .access_token;
    store.save_token(&expiring, &owner, 1).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let (status, listed) = send(
        app.clone(),
        get_with_bearer("/api/v1/auth/sessions", current),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert!(
        !listed["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == session_id(&expiring))
    );
    assert!(store.token_owner(&expiring).await.unwrap().is_none());
    assert!(
        store
            .session_token(&owner, &session_id(&expiring))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        !connection
            .sismember::<_, _, bool>(user_key(&owner), &expiring)
            .await
            .unwrap()
    );

    let target_mapping = format!("auth:session:{owner}:{}", session_id(target));
    connection.del::<_, ()>(&target_mapping).await.unwrap();
    connection
        .set_ex::<_, _, ()>(&target_mapping, "wrong-type", 120)
        .await
        .unwrap();
    assert_eq!(
        revoke(&app, current, target).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/me", target))
            .await
            .0,
        StatusCode::OK
    );
    connection.del::<_, ()>(&target_mapping).await.unwrap();
    assert_eq!(
        send(
            app.clone(),
            get_with_bearer("/api/v1/auth/sessions", current)
        )
        .await
        .0,
        StatusCode::OK
    );

    connection.del::<_, ()>(user_key(&owner)).await.unwrap();
    connection
        .set_ex::<_, _, ()>(user_key(&owner), "wrong-type", 120)
        .await
        .unwrap();
    assert_eq!(
        send(
            app.clone(),
            get_with_bearer("/api/v1/auth/sessions", current)
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    let (status, failed_login) = redis_login(&app, &mut connection, &owner).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{failed_login}");
    assert!(failed_login["data"].get("token").is_none());
    let failed_token = format!("failed-{owner}");
    assert!(store.save_token(&failed_token, &owner, 120).await.is_err());
    assert!(store.token_owner(&failed_token).await.unwrap().is_none());
    let (status, failed_revoke) = revoke(&app, current, target).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{failed_revoke}");
    assert!(failed_revoke["data"].get("revoked").is_none());
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/me", target))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    for token in [current, trusted, other] {
        assert_eq!(
            send(app.clone(), get_with_bearer("/api/v1/auth/me", token))
                .await
                .0,
            StatusCode::OK
        );
    }
    connection.del::<_, ()>(user_key(&owner)).await.unwrap();
    connection
        .sadd::<_, _, ()>(user_key(&owner), &[current, target, trusted, &legacy])
        .await
        .unwrap();
    connection
        .expire::<_, ()>(user_key(&owner), 7200)
        .await
        .unwrap();
    let (status, confirmed) = revoke(&app, current, target).await;
    assert_eq!(status, StatusCode::OK, "{confirmed}");
    assert_eq!(confirmed["data"]["revoked"], false);
    let (status, revoked) = revoke(&app, current, &legacy).await;
    assert_eq!(status, StatusCode::OK, "{revoked}");
    assert_eq!(revoked["data"]["revoked"], true);
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/me", &legacy))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );

    for (username, token) in [
        (&owner, current),
        (&owner, target),
        (&owner, trusted),
        (&other_owner, other),
        (&owner, &legacy),
    ] {
        connection
            .del::<_, ()>(&[
                token_key(token),
                format!("auth:session:{username}:{}", session_id(token)),
            ])
            .await
            .unwrap();
    }
    connection
        .del::<_, ()>(&[user_key(&owner), user_key(&other_owner)])
        .await
        .unwrap();
}
