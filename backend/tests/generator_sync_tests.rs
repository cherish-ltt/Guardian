mod support;

use std::io::{Cursor, Read};

use axum::{Router, http::StatusCode};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde_json::{Value, json};
use zip::ZipArchive;

use support::{
    DEMO_PASSWORD, admin_token, get_with_bearer, login, post_json_with_bearer,
    put_json_with_bearer, send, send_raw, setup_app_with_container,
};

async fn import(app: &Router, token: &str, table: &str) -> i64 {
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

async fn preview(app: &Router, token: &str, id: i64) -> Value {
    let (status, body) = send(
        app.clone(),
        get_with_bearer(
            &format!("/api/v1/tool/generator/tables/{id}/sync-preview"),
            token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["data"].clone()
}

async fn synchronize(app: &Router, token: &str, id: i64, revision: &Value) -> (StatusCode, Value) {
    send(
        app.clone(),
        post_json_with_bearer(
            &format!("/api/v1/tool/generator/tables/{id}/sync"),
            token,
            json!({"revision": revision}),
        ),
    )
    .await
}

async fn columns(app: &Router, token: &str, id: i64) -> Value {
    let (status, body) = send(
        app.clone(),
        get_with_bearer(
            &format!("/api/v1/tool/generator/tables/{id}/columns"),
            token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["data"].clone()
}

async fn sql_json(database: &DatabaseConnection, sql: &str) -> Value {
    let row = database
        .query_one_raw(Statement::from_string(DbBackend::Sqlite, sql))
        .await
        .unwrap()
        .unwrap();
    serde_json::from_str(&row.try_get::<String>("", "value").unwrap()).unwrap()
}

async fn configuration(database: &DatabaseConnection) -> Value {
    sql_json(database,
        "SELECT json_object(\
         'tables', (SELECT json_group_array(json_array(id, table_name, table_comment, class_name, module_name, business_name, synced, create_by, update_by, create_time, update_time, version)) FROM gen_table ORDER BY id),\
         'columns', (SELECT json_group_array(json_array(id, table_id, column_name, rust_field, rust_type, jdbc_type, column_type, column_comment, is_insert, is_edit, is_list, is_query, query_type, form_type, required, nullable, primary_key, sort)) FROM gen_table_column ORDER BY id)) AS value"
    ).await
}

async fn generated(app: &Router, token: &str, id: i64) -> String {
    let (status, body) = send(
        app.clone(),
        get_with_bearer(
            &format!("/api/v1/tool/generator/tables/{id}/preview"),
            token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, _, bytes) = send_raw(
        app.clone(),
        get_with_bearer(
            &format!("/api/v1/tool/generator/tables/{id}/download"),
            token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let files = body["data"]["files"].as_array().unwrap();
    assert_eq!(zip.len(), files.len());
    for file in files {
        let mut code = String::new();
        zip.by_name(file["path"].as_str().unwrap())
            .unwrap()
            .read_to_string(&mut code)
            .unwrap();
        assert_eq!(code, file["code"].as_str().unwrap());
    }
    files[0]["code"].as_str().unwrap().to_string()
}

fn compile_entity(code: &str) {
    let deps = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let library = std::fs::read_dir(&deps)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("libsea_orm-") && name.ends_with(".rlib"))
        })
        .max_by_key(|path| path.metadata().and_then(|meta| meta.modified()).ok())
        .unwrap();
    let dir = std::env::temp_dir().join(format!("guardian-sync-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir(&dir).unwrap();
    let source = dir.join("generated.rs");
    std::fs::write(&source, code).unwrap();
    let result = std::process::Command::new("rustc")
        .args(["--edition=2024", "--crate-type=lib"])
        .arg(source)
        .arg("--out-dir")
        .arg(&dir)
        .arg("-L")
        .arg(format!("dependency={}", deps.display()))
        .arg("--extern")
        .arg(format!("sea_orm={}", library.display()))
        .output()
        .unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[tokio::test]
async fn added_column_preview_is_read_only_and_sync_preserves_identity() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, store.as_ref()).await;
    container
        .database
        .execute_unprepared(
            "CREATE TABLE sync_device (device_id INTEGER PRIMARY KEY, name TEXT NOT NULL); \
         INSERT INTO sync_device VALUES (7, 'business-row-kept')",
        )
        .await
        .unwrap();
    let id = import(&app, &token, "sync_device").await;
    let other = import(&app, &token, "sys_post").await;
    container.database.execute_unprepared(&format!(
        "UPDATE gen_table SET table_comment = '设备表', class_name = 'Device', module_name = 'custom', business_name = 'device' WHERE id = {id};\
         UPDATE gen_table_column SET rust_field = 'custom_name', column_comment = 'custom comment', is_query = 0, query_type = '=', form_type = 'custom form', is_edit = 0, is_list = 0, required = 0 WHERE table_id = {id} AND column_name = 'name';\
         ALTER TABLE sync_device ADD COLUMN note TEXT"
    )).await.unwrap();
    let before = configuration(&container.database).await;
    let old_columns = columns(&app, &token, id).await;
    let other_columns = columns(&app, &token, other).await;
    let diff = preview(&app, &token, id).await;
    assert_eq!(diff["changes"].as_array().unwrap().len(), 1);
    assert_eq!(diff["changes"][0]["columnName"], "note");
    assert_eq!(diff["changes"][0]["kind"], "added");
    assert_eq!(diff["changes"][0]["after"]["rustType"], "String");
    assert_eq!(diff["changes"][0]["after"]["nullable"], true);
    assert_eq!(
        configuration(&container.database).await,
        before,
        "预览不可写配置"
    );
    assert_eq!(import(&app, &token, "sync_device").await, id);
    assert_eq!(
        configuration(&container.database).await,
        before,
        "重复导入不可隐式同步"
    );
    let (status, result) = synchronize(&app, &token, id, &diff["revision"]).await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["data"]["changed"], 1);
    let current_columns = columns(&app, &token, id).await;
    assert_eq!(
        &current_columns.as_array().unwrap()[..2],
        old_columns.as_array().unwrap()
    );
    assert_eq!(current_columns.as_array().unwrap().len(), 3);
    assert_eq!(columns(&app, &token, other).await, other_columns);
    let table = sql_json(&container.database, &format!(
        "SELECT json_array(id, table_name, table_comment, class_name, module_name, business_name, synced) AS value FROM gen_table WHERE id = {id}"
    )).await;
    assert_eq!(
        table,
        json!([id, "sync_device", "设备表", "Device", "custom", "device", 0])
    );
    let data = sql_json(
        &container.database,
        "SELECT json_group_array(json_array(device_id, name, note)) AS value FROM sync_device",
    )
    .await;
    assert_eq!(data, json!([[7, "business-row-kept", null]]));
    let entity = generated(&app, &token, id).await;
    assert!(entity.contains("pub device_id: i64"));
    assert!(entity.contains("pub custom_name: String"));
    assert!(entity.contains("pub note: Option<String>"));
    compile_entity(&entity);
    let diff = preview(&app, &token, id).await;
    assert_eq!(diff["changes"], json!([]));
    let before = configuration(&container.database).await;
    let (status, result) = synchronize(&app, &token, id, &diff["revision"]).await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["data"]["changed"], 0);
    assert_eq!(
        configuration(&container.database).await,
        before,
        "无差异同步不触碰版本或元数据"
    );
}

#[tokio::test]
async fn changed_columns_refresh_only_related_attributes_and_renames_are_explicit() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, store.as_ref()).await;
    container.database.execute_unprepared(
        "CREATE TABLE sync_shape (record_id INTEGER PRIMARY KEY, name VARCHAR(64) NOT NULL, note TEXT, amount INTEGER, old_name TEXT, spare INTEGER NOT NULL)"
    ).await.unwrap();
    let id = import(&app, &token, "sync_shape").await;
    container.database.execute_unprepared(&format!(
        "UPDATE gen_table_column SET column_comment = '保留说明', rust_field = 'custom_amount', query_type = '>=', is_query = 1, is_insert = 0, is_edit = 0, form_type = 'custom', required = 1 WHERE table_id = {id} AND column_name = 'amount';\
         DROP TABLE sync_shape;\
         CREATE TABLE sync_shape (spare INTEGER PRIMARY KEY, record_id INTEGER NOT NULL, amount REAL, name VARCHAR(128), renamed TEXT)"
    )).await.unwrap();
    let before = configuration(&container.database).await;
    let old_columns = columns(&app, &token, id).await;
    let diff = preview(&app, &token, id).await;
    let changes = diff["changes"].as_array().unwrap();
    let change = |name: &str| {
        changes
            .iter()
            .find(|entry| entry["columnName"] == name)
            .unwrap()
    };
    assert_eq!(
        change("spare")["changedAttributes"],
        json!(["primaryKey", "sort"])
    );
    assert_eq!(
        change("record_id")["changedAttributes"],
        json!(["primaryKey", "sort"])
    );
    assert_eq!(
        change("amount")["changedAttributes"],
        json!(["columnType", "sort"])
    );
    assert_eq!(
        change("name")["changedAttributes"],
        json!(["columnType", "nullable", "sort"])
    );
    assert_eq!(change("note")["kind"], "removed");
    assert_eq!(change("old_name")["kind"], "removed");
    assert_eq!(change("renamed")["kind"], "added");
    assert_eq!(configuration(&container.database).await, before);
    let (status, body) = synchronize(&app, &token, id, &diff["revision"]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let current = columns(&app, &token, id).await;
    let current = current.as_array().unwrap();
    assert_eq!(
        current
            .iter()
            .map(|entry| entry["col"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["spare", "record_id", "amount", "name", "renamed"]
    );
    let amount = &current[2];
    let old_amount = old_columns
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["col"] == "amount")
        .unwrap();
    for attribute in [
        "id",
        "rustField",
        "comment",
        "query",
        "queryType",
        "insert",
        "edit",
        "list",
        "required",
    ] {
        assert_eq!(
            amount[attribute], old_amount[attribute],
            "不相关属性 {attribute} 必须保留"
        );
    }
    assert_eq!(amount["rustType"], "f64");
    assert_eq!(amount["formType"], "文本框");
    assert_eq!(current[0]["primaryKey"], true);
    assert_eq!(current[0]["insert"], false);
    assert_eq!(current[1]["primaryKey"], false);
    assert_eq!(current[1]["insert"], true);
    assert_eq!(current[1]["required"], true);
    assert_eq!(current[3]["nullable"], true);
    assert_eq!(current[3]["required"], false);
    let entity = generated(&app, &token, id).await;
    assert!(entity.contains("pub spare: i64"));
    assert!(entity.contains("pub custom_amount: Option<f64>"));
    assert!(entity.contains("pub name: Option<String>"));
    assert!(!entity.contains("old_name"));
    assert!(!entity.contains("pub note"));
    compile_entity(&entity);
}

#[tokio::test]
async fn stale_structure_and_configuration_are_rejected_without_overwriting() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, store.as_ref()).await;
    container
        .database
        .execute_unprepared("CREATE TABLE sync_stale (id INTEGER PRIMARY KEY, name TEXT)")
        .await
        .unwrap();
    let id = import(&app, &token, "sync_stale").await;
    container
        .database
        .execute_unprepared("ALTER TABLE sync_stale ADD COLUMN added TEXT")
        .await
        .unwrap();
    let diff = preview(&app, &token, id).await;
    container
        .database
        .execute_unprepared("ALTER TABLE sync_stale ADD COLUMN later INTEGER")
        .await
        .unwrap();
    let before = configuration(&container.database).await;
    let (status, body) = synchronize(&app, &token, id, &diff["revision"]).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(body["message"].as_str().unwrap().contains("重新预览"));
    assert_eq!(configuration(&container.database).await, before);
    container
        .database
        .execute_unprepared(&format!(
            "UPDATE gen_table SET table_comment = 'initial' WHERE id = {id}"
        ))
        .await
        .unwrap();
    let diff = preview(&app, &token, id).await;
    for comment in ["modified", "initial"] {
        let (status, body) = send(
            app.clone(),
            put_json_with_bearer(
                &format!("/api/v1/tool/generator/tables/{id}"),
                &token,
                json!({"tableComment": comment}),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
    let before = configuration(&container.database).await;
    let (status, body) = synchronize(&app, &token, id, &diff["revision"]).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(configuration(&container.database).await, before);
    let diff = preview(&app, &token, id).await;
    container.database.execute_unprepared(&format!(
        "UPDATE gen_table_column SET column_comment = 'new column setting' WHERE table_id = {id} AND column_name = 'name'"
    )).await.unwrap();
    let before = configuration(&container.database).await;
    let (status, body) = synchronize(&app, &token, id, &diff["revision"]).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(configuration(&container.database).await, before);
    let (status, body) = send(
        app,
        post_json_with_bearer(
            &format!("/api/v1/tool/generator/tables/{id}/sync"),
            &token,
            json!({}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

#[tokio::test]
async fn missing_unsupported_and_deleted_tables_cannot_sync() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, store.as_ref()).await;
    let id = import(&app, &token, "sys_post").await;
    let original = preview(&app, &token, id).await;
    let before = configuration(&container.database).await;
    for schema in [
        "CREATE TABLE sys_post (code TEXT PRIMARY KEY)",
        "CREATE TABLE sys_post (id INTEGER PRIMARY KEY) WITHOUT ROWID",
        "CREATE TABLE sys_post (id INTEGER PRIMARY KEY DESC)",
        "CREATE TABLE sys_post (a INTEGER, b INTEGER, PRIMARY KEY(a, b))",
        "CREATE TABLE sys_post (name TEXT)",
        "",
    ] {
        container
            .database
            .execute_unprepared("DROP TABLE IF EXISTS sys_post")
            .await
            .unwrap();
        if !schema.is_empty() {
            container.database.execute_unprepared(schema).await.unwrap();
        }
        let (status, body) = send(
            app.clone(),
            get_with_bearer(
                &format!("/api/v1/tool/generator/tables/{id}/sync-preview"),
                &token,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        let (status, body) = synchronize(&app, &token, id, &original["revision"]).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(configuration(&container.database).await, before);
    }
    container.database.execute_unprepared(&format!(
        "DELETE FROM gen_table_column WHERE table_id = {id}; DELETE FROM gen_table WHERE id = {id}"
    )).await.unwrap();
    let before = configuration(&container.database).await;
    let (status, body) = synchronize(&app, &token, id, &original["revision"]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(
        configuration(&container.database).await,
        before,
        "确认时不可重新创建孤儿列"
    );
}

#[tokio::test]
async fn failure_after_column_writes_rolls_back_every_configuration_row() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, store.as_ref()).await;
    container
        .database
        .execute_unprepared(
            "CREATE TABLE sync_fault (id INTEGER PRIMARY KEY, name TEXT, removed TEXT)",
        )
        .await
        .unwrap();
    let id = import(&app, &token, "sync_fault").await;
    import(&app, &token, "sys_post").await;
    container
        .database
        .execute_unprepared(
            "ALTER TABLE sync_fault DROP COLUMN removed; \
         ALTER TABLE sync_fault ADD COLUMN added TEXT; \
         INSERT INTO sync_fault VALUES (1, 'unmodified-business-value', NULL); \
         CREATE TRIGGER reject_sync BEFORE UPDATE OF synced ON gen_table \
         BEGIN SELECT RAISE(ABORT, 'injected sync failure'); END",
        )
        .await
        .unwrap();
    let diff = preview(&app, &token, id).await;
    let before = configuration(&container.database).await;
    let (status, body) = synchronize(&app, &token, id, &diff["revision"]).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert_eq!(body["message"], "系统内部错误");
    assert_eq!(configuration(&container.database).await, before);
    let data = sql_json(
        &container.database,
        "SELECT json_group_array(json_array(id, name, added)) AS value FROM sync_fault",
    )
    .await;
    assert_eq!(data, json!([[1, "unmodified-business-value", null]]));
    container
        .database
        .execute_unprepared("DROP TRIGGER reject_sync")
        .await
        .unwrap();
    let (status, body) = synchronize(&app, &token, id, &diff["revision"]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["changed"], 2);
}

#[tokio::test]
async fn simultaneous_confirmations_apply_once_and_keep_columns_unique() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, store.as_ref()).await;
    container
        .database
        .execute_unprepared("CREATE TABLE sync_race (id INTEGER PRIMARY KEY)")
        .await
        .unwrap();
    let id = import(&app, &token, "sync_race").await;
    container
        .database
        .execute_unprepared("ALTER TABLE sync_race ADD COLUMN name TEXT")
        .await
        .unwrap();
    let diff = preview(&app, &token, id).await;
    let revision = &diff["revision"];
    let (first, second) = tokio::join!(
        synchronize(&app, &token, id, revision),
        synchronize(&app, &token, id, revision)
    );
    let mut statuses = [first.0.as_u16(), second.0.as_u16()];
    statuses.sort_unstable();
    assert_eq!(statuses, [200, 409], "{first:?} / {second:?}");
    let current = columns(&app, &token, id).await;
    assert_eq!(current.as_array().unwrap().len(), 2);
    assert_eq!(current[1]["col"], "name");
}

#[tokio::test]
async fn permission_denials_are_not_audited_and_audit_failure_does_not_undo_sync() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, store.as_ref()).await;
    let outsider = login(&app, store.as_ref(), "heyj", DEMO_PASSWORD).await;
    container
        .database
        .execute_unprepared("CREATE TABLE sync_audit (id INTEGER PRIMARY KEY)")
        .await
        .unwrap();
    let id = import(&app, &token, "sync_audit").await;
    container
        .database
        .execute_unprepared("ALTER TABLE sync_audit ADD COLUMN name TEXT")
        .await
        .unwrap();
    let diff = preview(&app, &token, id).await;
    for unauthorized in [None, Some(outsider.as_str())] {
        for (method, endpoint) in [("GET", "sync-preview"), ("POST", "sync")] {
            let (status, body) = send(
                app.clone(),
                support::request(
                    method,
                    &format!("/api/v1/tool/generator/tables/{id}/{endpoint}"),
                    (method == "POST").then(|| json!({"revision": diff["revision"]})),
                    unauthorized,
                ),
            )
            .await;
            assert_eq!(
                status.as_u16(),
                if unauthorized.is_none() { 401 } else { 403 },
                "{body}"
            );
        }
    }
    let logs = sql_json(
        &container.database,
        "SELECT json(count(*)) AS value FROM sys_oper_log WHERE action = '同步结构'",
    )
    .await;
    assert_eq!(logs, 0);
    let (status, _) = synchronize(&app, &token, id, &json!("obsolete")).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, body) = synchronize(&app, &token, id, &diff["revision"]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let logs = sql_json(&container.database,
        "SELECT json_group_array(json_array(result, request_params, error_msg)) AS value FROM sys_oper_log WHERE action = '同步结构' ORDER BY id"
    ).await;
    assert_eq!(logs.as_array().unwrap().len(), 2);
    assert_eq!(logs[0][0], 0);
    assert_eq!(logs[1][0], 1);
    let params: Value = serde_json::from_str(logs[1][1].as_str().unwrap()).unwrap();
    assert_eq!(params, json!({"tableName": "sync_audit", "changed": 1}));
    container.database.execute_unprepared(
        "ALTER TABLE sync_audit ADD COLUMN note TEXT; \
         CREATE TRIGGER reject_audit BEFORE INSERT ON sys_oper_log BEGIN SELECT RAISE(ABORT, 'audit unavailable'); END"
    ).await.unwrap();
    let diff = preview(&app, &token, id).await;
    let (status, body) = synchronize(&app, &token, id, &diff["revision"]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(columns(&app, &token, id).await.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn migration_preserves_legacy_configuration_instead_of_copying_current_schema() {
    let database = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
    database.execute_unprepared(
        "CREATE TABLE gen_table (id INTEGER PRIMARY KEY, table_name TEXT); \
         CREATE TABLE gen_table_column (id INTEGER PRIMARY KEY, table_id INTEGER, column_name TEXT, jdbc_type TEXT); \
         CREATE TABLE legacy (id INTEGER PRIMARY KEY, name REAL); \
         INSERT INTO gen_table VALUES (1, 'legacy'); \
         INSERT INTO gen_table_column VALUES (1, 1, 'name', 'varchar')"
    ).await.unwrap();
    database
        .execute_unprepared(include_str!("../migrations/0013_generator_sync.sql"))
        .await
        .unwrap();
    let row = sql_json(&database, "SELECT json_array(jdbc_type, column_type, version) AS value FROM gen_table_column c JOIN gen_table t ON t.id = c.table_id").await;
    assert_eq!(row, json!(["varchar", "varchar", 0]));
}
