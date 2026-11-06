mod support;

use axum::http::StatusCode;
use guardian_backend::infrastructure::repositories::sql::{execute, query_count};
use serde_json::json;
use support::*;

async fn import(app: &axum::Router, token: &str, table: &str) -> i64 {
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/tool/generator/import",
            token,
            json!({"tableNames": [table]}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["data"]["ids"][0].as_i64().unwrap()
}

#[tokio::test]
async fn removal_preserves_business_data_and_allows_fresh_import() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, &store).await;
    execute(
        &container.database,
        "CREATE TABLE remove_fixture (record_id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        vec![],
    )
    .await
    .unwrap();
    execute(
        &container.database,
        "INSERT INTO remove_fixture VALUES (7, 'keep')",
        vec![],
    )
    .await
    .unwrap();
    let table_id = import(&app, &token, "remove_fixture").await;
    let other_id = import(&app, &token, "sys_post").await;
    let (status, body) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/tool/generator/tables/{table_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        query_count(
            &container.database,
            "SELECT COUNT(*) AS value FROM gen_table WHERE id = ?",
            vec![table_id.into()]
        )
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        query_count(
            &container.database,
            "SELECT COUNT(*) AS value FROM gen_table_column WHERE table_id = ?",
            vec![table_id.into()]
        )
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        query_count(
            &container.database,
            "SELECT COUNT(*) AS value FROM remove_fixture WHERE record_id = 7 AND name = 'keep'",
            vec![]
        )
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        query_count(
            &container.database,
            "SELECT COUNT(*) AS value FROM gen_table WHERE id = ?",
            vec![other_id.into()]
        )
        .await
        .unwrap(),
        1
    );
    let replacement = import(&app, &token, "remove_fixture").await;
    assert_ne!(replacement, table_id);
    let (status, body) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/tool/generator/tables/{table_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["message"], "代码生成表不存在");
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/tool/generator/tables/{table_id}"),
            &token,
            json!({"tableComment": "late update"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    for suffix in ["preview", "download"] {
        let (status, _, bytes) = send_raw(
            app.clone(),
            get_with_bearer(
                &format!("/api/v1/tool/generator/tables/{replacement}/{suffix}"),
                &token,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(!bytes.is_empty());
    }
    assert_eq!(
        query_count(
            &container.database,
            "SELECT COUNT(*) AS value FROM gen_table_column WHERE table_id = ?",
            vec![replacement.into()]
        )
        .await
        .unwrap(),
        2
    );
}

#[tokio::test]
async fn missing_business_table_does_not_prevent_removal() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, &store).await;
    execute(
        &container.database,
        "CREATE TABLE removed_business (id INTEGER PRIMARY KEY)",
        vec![],
    )
    .await
    .unwrap();
    let table_id = import(&app, &token, "removed_business").await;
    execute(&container.database, "DROP TABLE removed_business", vec![])
        .await
        .unwrap();
    let (status, body) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/tool/generator/tables/{table_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/tool/generator/import",
            &token,
            json!({"tableNames": ["removed_business"]}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(
        query_count(
            &container.database,
            "SELECT COUNT(*) AS value FROM gen_table_column WHERE table_id = ?",
            vec![table_id.into()]
        )
        .await
        .unwrap(),
        0
    );
}

#[tokio::test]
async fn parent_delete_failure_rolls_back_deleted_columns() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, &store).await;
    let table_id = import(&app, &token, "sys_post").await;
    execute(&container.database, "CREATE TRIGGER deny_remove BEFORE DELETE ON gen_table BEGIN SELECT RAISE(ABORT, 'injected'); END", vec![]).await.unwrap();
    let (status, body) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/tool/generator/tables/{table_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert_eq!(body["message"], "系统内部错误");
    assert_eq!(
        query_count(
            &container.database,
            "SELECT COUNT(*) AS value FROM gen_table WHERE id = ?",
            vec![table_id.into()]
        )
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        query_count(
            &container.database,
            "SELECT COUNT(*) AS value FROM gen_table_column WHERE table_id = ?",
            vec![table_id.into()]
        )
        .await
        .unwrap(),
        11
    );
    execute(&container.database, "DROP TRIGGER deny_remove", vec![])
        .await
        .unwrap();
    let (status, body) = send(
        app,
        delete_with_bearer(&format!("/api/v1/tool/generator/tables/{table_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
async fn removal_permissions_and_audit_follow_existing_contract() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, &store).await;
    let table_id = import(&app, &token, "sys_post").await;
    let outsider = login(&app, &store, "linjh", DEMO_PASSWORD).await;
    let url = format!("/api/v1/tool/generator/tables/{table_id}");
    for (bearer, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some("expired"), StatusCode::UNAUTHORIZED),
        (Some(outsider.as_str()), StatusCode::FORBIDDEN),
    ] {
        let (status, body) = send(app.clone(), request("DELETE", &url, None, bearer)).await;
        assert_eq!(status, expected, "{body}");
    }
    assert_eq!(
        query_count(
            &container.database,
            "SELECT COUNT(*) AS value FROM gen_table WHERE id = ?",
            vec![table_id.into()]
        )
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        query_count(
            &container.database,
            "SELECT COUNT(*) AS value FROM sys_oper_log WHERE action = '移除配置'",
            vec![]
        )
        .await
        .unwrap(),
        0
    );
    for expected in [StatusCode::OK, StatusCode::BAD_REQUEST] {
        let (status, body) = send(app.clone(), delete_with_bearer(&url, &token)).await;
        assert_eq!(status, expected, "{body}");
    }
    let (_, logs) = send(
        app,
        get_with_bearer("/api/v1/monitor/oper-logs?keyword=移除配置", &token),
    )
    .await;
    assert_eq!(logs["data"]["total"], 2);
    let rows = logs["data"]["rows"].as_array().unwrap();
    for row in rows {
        assert_eq!(row["module"], "代码生成");
        assert_eq!(row["action"], "移除配置");
        assert_eq!(row["method"], "DELETE");
        assert_eq!(row["url"], format!("/tool/generator/tables/{table_id}"));
        assert_eq!(row["operName"], "admin");
    }
    assert!(rows.iter().any(|row| row["result"] == 1));
    assert!(
        rows.iter()
            .any(|row| row["result"] == 0 && row["errorMsg"] == "代码生成表不存在")
    );
}

#[tokio::test]
async fn audit_failure_does_not_reverse_successful_removal() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, &store).await;
    let table_id = import(&app, &token, "sys_post").await;
    execute(&container.database, "CREATE TRIGGER deny_audit BEFORE INSERT ON sys_oper_log BEGIN SELECT RAISE(ABORT, 'injected'); END", vec![]).await.unwrap();
    let (status, body) = send(
        app,
        delete_with_bearer(&format!("/api/v1/tool/generator/tables/{table_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        query_count(
            &container.database,
            "SELECT COUNT(*) AS value FROM gen_table WHERE id = ?",
            vec![table_id.into()]
        )
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        query_count(
            &container.database,
            "SELECT COUNT(*) AS value FROM gen_table_column WHERE table_id = ?",
            vec![table_id.into()]
        )
        .await
        .unwrap(),
        0
    );
}

#[tokio::test]
async fn concurrent_removal_import_and_update_leave_consistent_metadata() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, &store).await;
    let table_id = import(&app, &token, "sys_post").await;
    let url = format!("/api/v1/tool/generator/tables/{table_id}");
    let (first, second) = tokio::join!(
        send(app.clone(), delete_with_bearer(&url, &token)),
        send(app.clone(), delete_with_bearer(&url, &token)),
    );
    let mut statuses = [first.0, second.0];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::BAD_REQUEST]);
    let replacement = import(&app, &token, "sys_post").await;
    let replacement_url = format!("/api/v1/tool/generator/tables/{replacement}");
    let (removed, imported, updated) = tokio::join!(
        send(app.clone(), delete_with_bearer(&replacement_url, &token)),
        send(
            app.clone(),
            post_json_with_bearer(
                "/api/v1/tool/generator/import",
                &token,
                json!({"tableNames": ["sys_post"]})
            )
        ),
        send(
            app.clone(),
            put_json_with_bearer(
                &replacement_url,
                &token,
                json!({"tableComment": "concurrent"})
            )
        ),
    );
    assert_eq!(removed.0, StatusCode::OK, "{removed:?}");
    assert_eq!(imported.0, StatusCode::OK, "{imported:?}");
    assert!(
        matches!(updated.0, StatusCode::OK | StatusCode::BAD_REQUEST),
        "{updated:?}"
    );
    let final_id = import(&app, &token, "sys_post").await;
    assert_ne!(final_id, replacement);
    assert_eq!(
        query_count(
            &container.database,
            "SELECT COUNT(*) AS value FROM gen_table WHERE table_name = 'sys_post'",
            vec![]
        )
        .await
        .unwrap(),
        1
    );
    assert_eq!(query_count(&container.database, "SELECT COUNT(*) AS value FROM gen_table_column WHERE table_id NOT IN (SELECT id FROM gen_table)", vec![]).await.unwrap(), 0);
    assert_eq!(
        query_count(
            &container.database,
            "SELECT COUNT(*) AS value FROM gen_table_column WHERE table_id = ?",
            vec![final_id.into()]
        )
        .await
        .unwrap(),
        11
    );
}
