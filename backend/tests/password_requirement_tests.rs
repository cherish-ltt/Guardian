mod support;

use axum::http::StatusCode;
use guardian_backend::{
    domain::{error::AppError, models::identity::ChangePasswordCommand},
    infrastructure::{
        repositories::{
            account::AccountRepository,
            admin_auth::{AdminAuthRepository, SeaOrmAdminAuthRepository},
            sql::{execute, query_all, query_count},
        },
        services::auth_store::MemoryAuthStore,
    },
    util::password,
};
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde_json::{Value, json};
use support::*;

async fn sign_in(
    app: &axum::Router,
    store: &MemoryAuthStore,
    username: &str,
    secret: &str,
) -> (StatusCode, Value) {
    let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    let uuid = captcha["data"]["uuid"].as_str().unwrap();
    send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({
                "username": username, "password": secret,
                "captcha": take_captcha(store, uuid), "captchaUuid": uuid,
            }),
        ),
    )
    .await
}

#[tokio::test]
async fn newly_created_account_must_change_password_before_business_access() {
    let (app, store, container) = setup_app_with_container().await;
    let next_secret = uuid::Uuid::now_v7().to_string();
    let admin = admin_token(&app, &store).await;
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/users",
            &admin,
            json!({
                "username": "handoff", "nickname": "交付账号",
                "password": DEMO_PASSWORD, "passwordChangeRequired": true,
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let id = body["data"]["id"].as_i64().unwrap();
    let (status, signed) = sign_in(&app, &store, "handoff", DEMO_PASSWORD).await;
    assert_eq!(status, StatusCode::OK, "{signed}");
    assert_eq!(signed["data"]["passwordChangeRequired"], true);
    let token = signed["data"]["token"].as_str().unwrap();
    let (status, body) = send(app.clone(), get_with_bearer("/api/v1/auth/me", token)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["passwordChangeRequired"], true);
    for path in [
        "/api/v1/auth/routes",
        "/api/v1/auth/profile",
        "/api/v1/auth/sessions",
        "/api/v1/system/users",
        "/api/v1/system/users/export",
        "/api/v1/tool/generator/tables",
        "/api/v1/transactions",
        "/api/v1/auth/session",
    ] {
        let (status, body) = send(app.clone(), get_with_bearer(path, token)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}: {body}");
        assert_eq!(body["code"], 403);
        assert_eq!(body["data"]["passwordChangeRequired"], true);
    }
    for (path, body) in [
        ("/api/v1/auth/2fa/setup", json!({"password": DEMO_PASSWORD})),
        (
            "/api/v1/system/users",
            json!({"username":"bypass","nickname":"x"}),
        ),
    ] {
        let (status, body) = send(app.clone(), post_json_with_bearer(path, token, body)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}: {body}");
        assert_eq!(body["data"]["passwordChangeRequired"], true);
    }
    for body in [
        json!({"oldPassword": &next_secret, "newPassword": &next_secret}),
        json!({"oldPassword":DEMO_PASSWORD, "newPassword": &next_secret[..5]}),
        json!({"oldPassword":DEMO_PASSWORD, "newPassword":DEMO_PASSWORD}),
    ] {
        let (status, body) = send(
            app.clone(),
            put_json_with_bearer("/api/v1/auth/password", token, body),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        let (_, me) = send(app.clone(), get_with_bearer("/api/v1/auth/me", token)).await;
        assert_eq!(me["data"]["passwordChangeRequired"], true);
        assert_eq!(
            AccountRepository::security(&container.database, id)
                .await
                .unwrap()
                .auth_version,
            0
        );
    }
    let second = login(&app, &store, "handoff", DEMO_PASSWORD).await;
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/auth/password",
            token,
            json!({"oldPassword":DEMO_PASSWORD,"newPassword": &next_secret}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        AccountRepository::security(&container.database, id)
            .await
            .unwrap()
            .auth_version,
        1
    );
    for old in [token, second.as_str()] {
        for path in ["/api/v1/auth/me", "/api/v1/transactions"] {
            assert_eq!(
                send(app.clone(), get_with_bearer(path, old)).await.0,
                StatusCode::UNAUTHORIZED
            );
        }
    }
    assert_eq!(
        sign_in(&app, &store, "handoff", DEMO_PASSWORD).await.0,
        StatusCode::BAD_REQUEST
    );
    let (status, signed) = sign_in(&app, &store, "handoff", &next_secret).await;
    assert_eq!(status, StatusCode::OK, "{signed}");
    assert_eq!(signed["data"]["passwordChangeRequired"], false);
    let fresh = signed["data"]["token"].as_str().unwrap();
    let (status, routes) = send(app.clone(), get_with_bearer("/api/v1/auth/routes", fresh)).await;
    assert_eq!(status, StatusCode::OK, "{routes}");
    assert_eq!(routes["data"]["routes"], json!([]));
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/system/users", fresh))
            .await
            .0,
        StatusCode::FORBIDDEN
    );

    let logs = query_all(
        &container.database,
        "SELECT request_params, error_msg FROM sys_oper_log",
        vec![],
    )
    .await
    .unwrap();
    for log in logs {
        let params: Option<String> = log.try_get("", "request_params").unwrap();
        let error: Option<String> = log.try_get("", "error_msg").unwrap();
        let serialized = format!("{params:?}{error:?}");
        assert!(!serialized.contains(&next_secret));
        assert!(!serialized.contains(DEMO_PASSWORD));
        assert!(!serialized.contains(token));
    }
}

#[tokio::test]
async fn creating_required_account_rolls_back_when_a_relation_write_fails() {
    let (app, store, container) = setup_app_with_container().await;
    let admin = admin_token(&app, &store).await;
    let db = &container.database;
    let before_roles = query_count(db, "SELECT COUNT(*) AS value FROM sys_user_role", vec![])
        .await
        .unwrap();
    let before_posts = query_count(db, "SELECT COUNT(*) AS value FROM sys_user_post", vec![])
        .await
        .unwrap();
    db.execute_unprepared("CREATE TRIGGER reject_required_account_post BEFORE INSERT ON sys_user_post WHEN EXISTS (SELECT 1 FROM sys_user WHERE id = NEW.user_id AND username = 'failed-handoff') BEGIN SELECT RAISE(ABORT, 'forced relation failure'); END").await.unwrap();
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/users",
            &admin,
            json!({"username":"failed-handoff", "nickname":"回滚账号", "password":DEMO_PASSWORD, "passwordChangeRequired":true, "roleIds":[3], "postIds":[6]}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert!(
        SeaOrmAdminAuthRepository
            .find_user_by_username(db, "failed-handoff")
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        query_count(db, "SELECT COUNT(*) AS value FROM sys_user_role", vec![])
            .await
            .unwrap(),
        before_roles
    );
    assert_eq!(
        query_count(db, "SELECT COUNT(*) AS value FROM sys_user_post", vec![])
            .await
            .unwrap(),
        before_posts
    );
    db.execute_unprepared("DROP TRIGGER reject_required_account_post")
        .await
        .unwrap();
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/users",
            &admin,
            json!({"username":"failed-handoff", "nickname":"重试账号", "password":DEMO_PASSWORD}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        !SeaOrmAdminAuthRepository
            .find_user_by_username(db, "failed-handoff")
            .await
            .unwrap()
            .unwrap()
            .password_change_required
    );
}

#[tokio::test]
async fn migration_upgrades_existing_accounts_without_forcing_a_password_change() {
    let (_, _, container) = setup_app_with_container().await;
    let before = SeaOrmAdminAuthRepository
        .find_user_by_username(&container.database, "admin")
        .await
        .unwrap()
        .unwrap();
    container
        .database
        .execute_unprepared("ALTER TABLE sys_user DROP COLUMN password_change_required")
        .await
        .unwrap();
    execute(
        &container.database,
        "DELETE FROM _sqlx_migrations WHERE version = 12",
        vec![],
    )
    .await
    .unwrap();
    guardian_backend::infrastructure::databases::run_migrations(
        &container.database,
        &container.config.database.url,
        &container.config.seed,
    )
    .await
    .unwrap();
    let after = SeaOrmAdminAuthRepository
        .find_user_by_username(&container.database, "admin")
        .await
        .unwrap()
        .unwrap();
    assert!(!after.password_change_required);
    assert_eq!(before.password, after.password);
    assert_eq!(before.auth_version, after.auth_version);
}

#[tokio::test]
async fn default_and_legacy_accounts_are_unrestricted_and_super_admin_cannot_bypass() {
    let (app, store, container) = setup_app_with_container().await;
    let admin = admin_token(&app, &store).await;
    let (_, me) = send(app.clone(), get_with_bearer("/api/v1/auth/me", &admin)).await;
    assert_eq!(me["data"]["passwordChangeRequired"], false);
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/routes", &admin))
            .await
            .0,
        StatusCode::OK
    );
    for required in [None, Some(false)] {
        let username = format!("default-{}", required.is_some());
        let mut payload =
            json!({"username":username,"nickname":"默认账号","password":DEMO_PASSWORD});
        if let Some(required) = required {
            payload["passwordChangeRequired"] = json!(required);
        }
        let (status, created) = send(
            app.clone(),
            post_json_with_bearer("/api/v1/system/users", &admin, payload),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{created}");
        let token = login(&app, &store, &username, DEMO_PASSWORD).await;
        let (_, me) = send(app.clone(), get_with_bearer("/api/v1/auth/me", &token)).await;
        assert_eq!(me["data"]["passwordChangeRequired"], false);
    }
    execute(
        &container.database,
        "UPDATE sys_user SET password_change_required = 1 WHERE id = 1",
        vec![],
    )
    .await
    .unwrap();
    for path in [
        "/api/v1/auth/routes",
        "/api/v1/system/users",
        "/api/v1/transactions",
    ] {
        let (status, body) = send(app.clone(), get_with_bearer(path, &admin)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}: {body}");
        assert_eq!(body["data"]["passwordChangeRequired"], true);
    }
    let (_, me) = send(app.clone(), get_with_bearer("/api/v1/auth/me", &admin)).await;
    assert_eq!(me["data"]["passwordChangeRequired"], true);
    assert_eq!(
        send(
            app.clone(),
            post_json_with_bearer("/api/v1/auth/logout", &admin, json!({}))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        send(app, get_with_bearer("/api/v1/auth/me", &admin))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn admin_reset_atomically_sets_requirement_and_blocks_stale_validated_updates() {
    let (app, store, container) = setup_app_with_container().await;
    let admin = admin_token(&app, &store).await;
    let old = login(&app, &store, "linjh", DEMO_PASSWORD).await;
    let identity = container
        .admin_auth_service
        .authenticate(&old)
        .await
        .unwrap();
    let snapshot = SeaOrmAdminAuthRepository
        .find_user_by_username(&container.database, "linjh")
        .await
        .unwrap()
        .unwrap();
    assert!(password::verify_password(DEMO_PASSWORD, &snapshot.password).unwrap());
    let path = format!("/api/v1/system/users/{}/password", snapshot.id);
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            &path,
            &admin,
            json!({"password":DEMO_PASSWORD,"passwordChangeRequired":true}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/me", &old))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );

    let stale_hash = password::hash_password("stale-request-value").unwrap();
    let tx = container.database.begin().await.unwrap();
    assert_eq!(
        SeaOrmAdminAuthRepository
            .update_password(&tx, &snapshot, &stale_hash)
            .await
            .unwrap(),
        0,
        "先校验旧密码、管理员重置后再写入的请求必须被拒绝"
    );
    tx.rollback().await.unwrap();
    let error = container
        .admin_auth_service
        .change_password(
            &identity,
            ChangePasswordCommand {
                old_password: DEMO_PASSWORD.into(),
                new_password: "stale-request-value".into(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::SessionExpired(_)));
    let current = SeaOrmAdminAuthRepository
        .find_user_by_username(&container.database, "linjh")
        .await
        .unwrap()
        .unwrap();
    assert!(current.password_change_required);
    assert_eq!(current.auth_version, snapshot.auth_version + 1);
    assert!(password::verify_password(DEMO_PASSWORD, &current.password).unwrap());

    // 即使口令散列未变化，认证版本变化仍应挡住已校验的旧请求。
    AccountRepository::bump_auth_version(&container.database, current.id)
        .await
        .unwrap();
    assert_eq!(
        SeaOrmAdminAuthRepository
            .update_password(&container.database, &current, &stale_hash)
            .await
            .unwrap(),
        0
    );
    let token = login(&app, &store, "linjh", DEMO_PASSWORD).await;
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/users/{}", current.id),
            &admin,
            json!({"passwordChangeRequired":false}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/routes", &token))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(&path, &admin, json!({"password":DEMO_PASSWORD})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (_, signed) = sign_in(&app, &store, "linjh", DEMO_PASSWORD).await;
    assert_eq!(signed["data"]["passwordChangeRequired"], false);
}

#[tokio::test]
async fn failed_transactions_preserve_password_requirement_and_version() {
    let (app, store, container) = setup_app_with_container().await;
    let rejected_secret = uuid::Uuid::now_v7().to_string();
    let admin = admin_token(&app, &store).await;
    execute(
        &container.database,
        "UPDATE sys_user SET password_change_required = 1 WHERE username = 'linjh'",
        vec![],
    )
    .await
    .unwrap();
    let token = login(&app, &store, "linjh", DEMO_PASSWORD).await;
    let before = SeaOrmAdminAuthRepository
        .find_user_by_username(&container.database, "linjh")
        .await
        .unwrap()
        .unwrap();
    container.database.execute_unprepared("CREATE TRIGGER reject_auth_version BEFORE UPDATE OF auth_version ON sys_user_security BEGIN SELECT RAISE(ABORT, 'forced transaction failure'); END").await.unwrap();
    for (path, bearer, payload) in [
        (
            "/api/v1/auth/password".to_string(),
            token.as_str(),
            json!({"oldPassword":DEMO_PASSWORD,"newPassword": &rejected_secret}),
        ),
        (
            format!("/api/v1/system/users/{}/password", before.id),
            admin.as_str(),
            json!({"password": &rejected_secret,"passwordChangeRequired":false}),
        ),
    ] {
        let (status, body) = send(app.clone(), put_json_with_bearer(&path, bearer, payload)).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
        let after = SeaOrmAdminAuthRepository
            .find_user_by_username(&container.database, "linjh")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.password, before.password);
        assert_eq!(
            after.password_change_required,
            before.password_change_required
        );
        assert_eq!(after.auth_version, before.auth_version);
    }
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/me", &token))
            .await
            .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn configured_second_factor_is_completed_before_restricted_session_is_issued() {
    let (app, store, container) = setup_app_with_container().await;
    let next_secret = uuid::Uuid::now_v7().to_string();
    let token = login(&app, &store, "linjh", DEMO_PASSWORD).await;
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
    assert_eq!(status, StatusCode::OK, "{enabled}");
    let user = SeaOrmAdminAuthRepository
        .find_user_by_username(&container.database, "linjh")
        .await
        .unwrap()
        .unwrap();
    let admin = admin_token(&app, &store).await;
    assert_eq!(
        send(
            app.clone(),
            put_json_with_bearer(
                &format!("/api/v1/system/users/{}/password", user.id),
                &admin,
                json!({"password":DEMO_PASSWORD,"passwordChangeRequired":true})
            )
        )
        .await
        .0,
        StatusCode::OK
    );
    let (status, step) = sign_in(&app, &store, "linjh", DEMO_PASSWORD).await;
    assert_eq!(status, StatusCode::OK, "{step}");
    assert_eq!(step["data"]["twoFactorRequired"], true);
    assert!(step["data"].get("token").is_none());
    assert_eq!(
        send(
            app.clone(),
            post_json(
                "/api/v1/auth/login/2fa",
                json!({"challenge":step["data"]["challenge"],"secondFactor":"incorrect"})
            )
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let (status, signed) = send(app.clone(), post_json("/api/v1/auth/login/2fa",
        json!({"challenge":step["data"]["challenge"],"secondFactor":enabled["data"]["recoveryCodes"][0]}))).await;
    assert_eq!(status, StatusCode::OK, "{signed}");
    assert_eq!(signed["data"]["passwordChangeRequired"], true);
    let token = signed["data"]["token"].as_str().unwrap();
    assert_eq!(
        send(app.clone(), get_with_bearer("/api/v1/auth/routes", token))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(
            app.clone(),
            put_json_with_bearer(
                "/api/v1/auth/password",
                token,
                json!({"oldPassword":DEMO_PASSWORD,"newPassword": &next_secret})
            )
        )
        .await
        .0,
        StatusCode::OK
    );
    let (_, step) = sign_in(&app, &store, "linjh", &next_secret).await;
    assert_eq!(step["data"]["twoFactorRequired"], true);
    let (status, signed) = send(app.clone(), post_json("/api/v1/auth/login/2fa",
        json!({"challenge":step["data"]["challenge"],"secondFactor":enabled["data"]["recoveryCodes"][1]}))).await;
    assert_eq!(status, StatusCode::OK, "{signed}");
    assert_eq!(signed["data"]["passwordChangeRequired"], false);
}
