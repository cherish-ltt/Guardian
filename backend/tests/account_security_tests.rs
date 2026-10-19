mod support;
use axum::http::StatusCode;
use guardian_backend::{
    create_app::create_app,
    infrastructure::{
        repositories::sql::{execute, query_count, query_opt},
        services::auth_store::MemoryAuthStore,
    },
};
use serde_json::json;
use std::sync::Arc;
use support::*;

#[tokio::test]
async fn enrollment_expiry_failed_confirmation_and_encrypted_storage() {
    let store = Arc::new(MemoryAuthStore::new());
    let container = setup_container_with_store(Vec::new(), store.clone()).await;
    let app = create_app(container.clone());
    let token = admin_token(&app, store.as_ref()).await;
    let (status, headers, bytes) = send_raw(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/setup",
            &token,
            json!({"password":DEMO_PASSWORD}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["cache-control"], "no-store");
    let setup: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let secret = setup["data"]["secret"].as_str().unwrap();
    let row = query_opt(
        &container.database,
        "SELECT * FROM sys_user_security WHERE user_id = 1",
        vec![],
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        row.try_get::<Option<String>>("", "secret")
            .unwrap()
            .is_none()
    );
    let encrypted = row.try_get::<String>("", "pending_secret").unwrap();
    assert!(!encrypted.contains(secret));
    let invalid = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/enable",
            &token,
            json!({"password":DEMO_PASSWORD,"secondFactor":"invalid"}),
        ),
    )
    .await;
    assert_eq!(invalid.0, StatusCode::BAD_REQUEST);
    assert_eq!(
        query_count(
            &container.database,
            "SELECT COUNT(*) AS value FROM sys_user_security WHERE secret IS NOT NULL",
            vec![]
        )
        .await
        .unwrap(),
        0
    );
    execute(
        &container.database,
        "UPDATE sys_user_security SET pending_expires = 1 WHERE user_id = 1",
        vec![],
    )
    .await
    .unwrap();
    let totp = totp_rs::Builder::new()
        .with_secret(totp_rs::Secret::try_from_base32(secret).unwrap())
        .build()
        .unwrap();
    let expired = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/auth/2fa/enable",
            &token,
            json!({"password":DEMO_PASSWORD,"secondFactor":totp.generate_current().to_string()}),
        ),
    )
    .await;
    assert!(expired.1["message"].as_str().unwrap().contains("过期"));
}

#[tokio::test]
async fn upgrade_grants_reset2fa_once_without_restoring_revoked_grants() {
    let store = Arc::new(MemoryAuthStore::new());
    let container = setup_container_with_store(Vec::new(), store.clone()).await;
    let db = &container.database;
    execute(db, "DELETE FROM sys_role_menu WHERE menu_id IN (SELECT id FROM sys_menu WHERE permission = 'system:user:reset2fa')", vec![]).await.unwrap();
    execute(
        db,
        "DELETE FROM sys_menu WHERE permission = 'system:user:reset2fa'",
        vec![],
    )
    .await
    .unwrap();
    execute(db, "DROP TABLE sys_user_security", vec![])
        .await
        .unwrap();
    execute(
        db,
        "DELETE FROM _sqlx_migrations WHERE version >= 3",
        vec![],
    )
    .await
    .unwrap();
    execute(
        db,
        "INSERT OR IGNORE INTO sys_role_menu (role_id,menu_id) VALUES (2,1014)",
        vec![],
    )
    .await
    .unwrap();
    guardian_backend::infrastructure::databases::run_migrations(
        db,
        &container.config.database.url,
        &container.config.seed,
    )
    .await
    .unwrap();
    let sql = "SELECT COUNT(*) AS value FROM sys_role_menu rm JOIN sys_menu m ON rm.menu_id = m.id WHERE rm.role_id = 2 AND m.permission = 'system:user:reset2fa'";
    assert_eq!(query_count(db, sql, vec![]).await.unwrap(), 1);
    execute(db, "DELETE FROM sys_role_menu WHERE role_id = 2 AND menu_id IN (SELECT id FROM sys_menu WHERE permission = 'system:user:reset2fa')", vec![]).await.unwrap();
    guardian_backend::infrastructure::databases::run_migrations(
        db,
        &container.config.database.url,
        &container.config.seed,
    )
    .await
    .unwrap();
    assert_eq!(query_count(db, sql, vec![]).await.unwrap(), 0);
    let app = create_app(container.clone());
    let user = login(&app, store.as_ref(), "linjh", DEMO_PASSWORD).await;
    let path = "/api/v1/system/users/3/2fa/reset";
    assert_eq!(
        send(
            app.clone(),
            post_json_with_bearer(path, &user, json!({"password":DEMO_PASSWORD}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    execute(db, "INSERT INTO sys_role_menu (role_id,menu_id) SELECT 2,id FROM sys_menu WHERE permission = 'system:user:reset2fa'", vec![]).await.unwrap();
    assert_eq!(
        send(
            app.clone(),
            post_json_with_bearer(path, &user, json!({"password":DEMO_PASSWORD}))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        send(
            app.clone(),
            post_json_with_bearer(
                "/api/v1/system/users/1/2fa/reset",
                &user,
                json!({"password":DEMO_PASSWORD})
            )
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
}
