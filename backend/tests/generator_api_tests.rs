//! 代码生成接口的契约测试：表导入、字段元数据、预览、生成与 ZIP 下载。

mod support;

use std::io::{Cursor, Read};

use axum::http::StatusCode;
use serde_json::json;
use zip::ZipArchive;

use support::{
    DEMO_PASSWORD, admin_token, delete_with_bearer, get_with_bearer, login, post_json_with_bearer,
    put_json_with_bearer, send, send_raw, setup_app_with_store,
};

async fn import_file_table(app: &axum::Router, token: &str) -> i64 {
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/tool/generator/import",
            token,
            json!({ "tableNames": ["sys_post"] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "导入失败: {body}");
    assert_eq!(body["data"]["imported"], 1);

    body["data"]["ids"][0].as_i64().expect("生成配置 ID")
}

#[tokio::test]
async fn importable_tables_mark_imported_rows_and_validate_requests() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, empty) = send(
        app.clone(),
        get_with_bearer("/api/v1/tool/generator/tables", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(empty["data"]["total"], 0, "种子数据不带生成配置");
    assert_eq!(empty["data"]["pageNum"], 1);
    assert_eq!(empty["data"]["pageSize"], 10);

    let (status, candidates) = send(
        app.clone(),
        get_with_bearer("/api/v1/tool/generator/importable-tables", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let names = candidates["data"]
        .as_array()
        .expect("候选表")
        .iter()
        .map(|table| table["tableName"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert!(names.contains(&"sys_user".to_string()));
    assert!(names.contains(&"sys_post".to_string()));
    assert!(names.contains(&"gen_table".to_string()));
    assert!(
        !names.iter().any(|name| name.starts_with("sqlite_")),
        "SQLite 内部表与迁移表不出现在候选里: {names:?}"
    );
    assert!(!names.contains(&"_sqlx_migrations".to_string()));
    assert!(candidates["data"][0]["imported"] == false);

    let (_, filtered) = send(
        app.clone(),
        get_with_bearer(
            "/api/v1/tool/generator/importable-tables?keyword=sys_post",
            &token,
        ),
    )
    .await;
    assert_eq!(filtered["data"].as_array().expect("候选表").len(), 1);
    assert_eq!(filtered["data"][0]["tableName"], "sys_post");

    let (status, body) = send(
        app.clone(),
        post_json_with_bearer("/api/v1/tool/generator/import", &token, json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "请选择要导入的数据库表");

    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/tool/generator/import",
            &token,
            json!({ "tableNames": ["not_exists_table"] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "数据库表不存在：not_exists_table");

    let table_id = import_file_table(&app, &token).await;

    // 重复导入复用已存在的配置，不新增行。
    let (status, again) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/tool/generator/import",
            &token,
            json!({ "tableNames": ["sys_post"] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(again["data"]["ids"], json!([table_id]));

    let (_, candidates) = send(
        app.clone(),
        get_with_bearer(
            "/api/v1/tool/generator/importable-tables?keyword=sys_post",
            &token,
        ),
    )
    .await;
    assert_eq!(candidates["data"][0]["imported"], true);

    let (status, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/tool/generator/tables", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["data"]["total"], 1);
    let row = &page["data"]["rows"][0];
    assert_eq!(row["tableName"], "sys_post");
    assert_eq!(row["className"], "SysPost");
    assert_eq!(row["module"], "system");
    assert_eq!(row["businessName"], "post");
    assert_eq!(row["synced"], true, "与 Spring 版一致：导入即标记为已生成");
    assert_eq!(
        row["tableComment"],
        serde_json::Value::Null,
        "SQLite 没有表注释"
    );

    let (_, searched) = send(
        app.clone(),
        get_with_bearer("/api/v1/tool/generator/tables?keyword=SysPost", &token),
    )
    .await;
    assert_eq!(searched["data"]["total"], 1);
    let (_, missing) = send(
        app.clone(),
        get_with_bearer("/api/v1/tool/generator/tables?keyword=zzz", &token),
    )
    .await;
    assert_eq!(missing["data"]["total"], 0);

    // 没有 `tool:gen:list` 权限的账号被拒绝。
    let outsider = login(&app, store.as_ref(), "linjh", DEMO_PASSWORD).await;
    let (status, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/tool/generator/tables", &outsider),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], "没有访问权限");
}

#[tokio::test]
async fn columns_describe_sqlite_metadata() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;
    let table_id = import_file_table(&app, &token).await;

    let (status, body) = send(
        app.clone(),
        get_with_bearer(
            &format!("/api/v1/tool/generator/tables/{table_id}/columns"),
            &token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let columns = body["data"].as_array().expect("字段列表");
    assert_eq!(columns.len(), 11);

    // 主键不进表单与列表。
    assert_eq!(columns[0]["col"], "id");
    assert_eq!(columns[0]["rustField"], "id");
    assert_eq!(columns[0]["rustType"], "i64");
    assert_eq!(columns[0]["formType"], "数字框");
    assert_eq!(columns[0]["insert"], false);
    assert_eq!(columns[0]["edit"], false);
    assert_eq!(columns[0]["list"], false);
    assert_eq!(columns[0]["required"], false);

    // 含 name / code 的列默认作为模糊查询条件，NOT NULL 视为必填。
    let post_code = &columns[1];
    assert_eq!(post_code["col"], "post_code");
    assert_eq!(post_code["rustField"], "post_code");
    assert_eq!(post_code["rustType"], "String");
    assert_eq!(
        post_code["formType"], "文本域",
        "SQLite 的 TEXT 对应 MySQL 的 text"
    );
    assert_eq!(post_code["query"], true);
    assert_eq!(post_code["queryType"], "LIKE");
    assert_eq!(post_code["required"], true);
    assert_eq!(
        post_code["comment"],
        serde_json::Value::Null,
        "SQLite 无列注释"
    );

    let sort = columns
        .iter()
        .find(|column| column["col"] == "sort")
        .expect("sort 列");
    assert_eq!(sort["rustType"], "i64");
    assert_eq!(sort["formType"], "数字框");
    assert_eq!(sort["query"], false);

    let status = columns
        .iter()
        .find(|column| column["col"] == "status")
        .expect("status 列");
    assert_eq!(status["query"], true);
    assert_eq!(status["required"], true);

    let create_time = columns
        .iter()
        .find(|column| column["col"] == "create_time")
        .expect("create_time 列");
    assert_eq!(create_time["rustType"], "String", "SQLite 只有 TEXT 类型");
    assert_eq!(create_time["query"], false);
    assert_eq!(create_time["required"], false);

    let (status, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/tool/generator/tables/999999/columns", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "代码生成表不存在");
}

#[tokio::test]
async fn preview_lists_ten_files_with_controller_active() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;
    let table_id = import_file_table(&app, &token).await;

    let (status, body) = send(
        app.clone(),
        get_with_bearer(
            &format!("/api/v1/tool/generator/tables/{table_id}/preview"),
            &token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let files = body["data"]["files"].as_array().expect("预览文件");
    assert_eq!(files.len(), 10);
    let names = files
        .iter()
        .map(|file| file["name"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "sys_post.rs".to_string(),
            "sys_post.rs".to_string(),
            "sys_post.rs".to_string(),
            "sys_post.rs".to_string(),
            "sys_post.rs".to_string(),
            "sys_post.rs".to_string(),
            "post.ts".to_string(),
            "SysPost.vue".to_string(),
            "sys_post_menu.sql".to_string(),
            "sys_post.md".to_string(),
        ]
    );
    let paths = files
        .iter()
        .map(|file| file["path"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        vec![
            "backend/src/entities/sys_post.rs".to_string(),
            "backend/src/domain/models/sys_post.rs".to_string(),
            "backend/src/domain/services/sys_post.rs".to_string(),
            "backend/src/infrastructure/repositories/sys_post.rs".to_string(),
            "backend/src/services/sys_post.rs".to_string(),
            "backend/src/api/controllers/sys_post.rs".to_string(),
            "frontend/src/api/post.ts".to_string(),
            "frontend/src/views/system/SysPost.vue".to_string(),
            "sql/sys_post_menu.sql".to_string(),
            "docs/generated/sys_post.md".to_string(),
        ]
    );
    assert_eq!(
        body["data"]["activeFile"], "backend/src/api/controllers/sys_post.rs",
        "默认展开控制器，取值与 files[].path 一致"
    );
    let code = body["data"]["code"].as_str().expect("默认展开的代码");
    assert_eq!(
        code,
        files[5]["code"].as_str().expect("Controller 代码"),
        "code 与 activeFile 指向同一份内容"
    );
    // 控制器是按本项目 axum 分层生成的骨架：路由、权限码与提取器与后台管理接口一致。
    assert!(code.contains("pub fn router() -> Router<Arc<Container>> {"));
    assert!(code.contains(".route(\"/system/post\", get(list).post(create))"));
    assert!(code.contains(".route(\"/system/post/{id}\", put(update).delete(remove))"));
    assert!(code.contains("user.require(\"system:post:list\")?;"));
    assert!(code.contains("let query = QueryString::parse(raw.as_deref());"));
    assert!(code.contains("ApiJson(command): ApiJson<SysPostWriteCommand>,"));
    assert!(code.contains("Ok(done())"));
    assert!(!code.contains("package com.scaffold"), "不再输出 Java 包名");
    assert!(!code.contains("@RestController"));

    // 实体是 sea-orm 的 DeriveEntityModel，主键列带 primary_key 标注。
    let entity = files[0]["code"].as_str().expect("实体代码");
    assert!(entity.starts_with("use sea_orm::entity::prelude::*;\n"));
    assert!(entity.contains("#[sea_orm(table_name = \"sys_post\")]"));
    assert!(entity.contains("    #[sea_orm(primary_key)]\n    pub id: i64,"));
    assert!(entity.contains("    pub post_code: String,"));
    assert!(entity.ends_with("impl ActiveModelBehavior for ActiveModel {}\n"));
    assert!(!entity.contains("import java"));

    // 领域模型给出列表行与写入命令，写入命令不含主键。
    let model = files[1]["code"].as_str().expect("领域模型代码");
    assert!(model.contains("pub struct SysPostRow {"));
    assert!(model.contains("pub struct SysPostWriteCommand {"));
    assert!(model.contains("    pub post_code: Option<String>,"));

    // 用例接口与仓储接口按同项目的命名与签名生成。
    let use_case = files[2]["code"].as_str().expect("用例接口代码");
    assert!(use_case.contains("pub trait SysPostUseCase: Send + Sync {"));
    assert!(use_case.contains("async fn list_post(&self, query: &KeywordQuery)"));
    let repository = files[3]["code"].as_str().expect("仓储代码");
    assert!(repository.contains("pub trait SysPostRepository: Send + Sync {"));
    assert!(repository.contains("pub struct SeaOrmSysPostRepository;"));

    // 服务层装配数据库并实现用例，未实现的写方法显式 todo。
    let service = files[4]["code"].as_str().expect("服务代码");
    assert!(service.contains("impl SysPostUseCase for SysPostService {"));
    assert!(service.contains("pub fn new(database: DatabaseConnection) -> Self {"));
    assert!(service.contains("todo!(\"按 repositories/sys_post.rs 的 SQL 实现新增\")"));

    // 前端 api 产物按本项目的 createXxxApi 工厂写法。
    let api = files[6]["code"].as_str().expect("前端 api 代码");
    assert!(api.contains("import type { HttpClient, QueryParams } from './http'"));
    assert!(api.contains("import type { IdResponse, PageResult } from './system'"));
    assert!(api.contains("export function createPostApi(client: HttpClient) {"));
    assert!(api.contains("client.get<PageResult<Record<string, unknown>>>('/system/post', query)"));
    assert!(api.contains("client.post<IdResponse>('/system/post', body)"));

    // 菜单 SQL 的权限码与控制器里的 require 一致。
    assert_eq!(
        files[8]["code"].as_str().expect("菜单 SQL"),
        "-- SysPost 菜单权限\n\
         INSERT INTO sys_menu (id, parent_id, menu_name, menu_type, path, component, permission, sort, visible, status, create_by, update_by)\n\
         SELECT COALESCE(MAX(id), 0) + 1, 0, 'SysPost管理', 'menu', '/system/post', 'system/SysPost', 'system:post:list', 1, 1, 1, 'system', 'system'\n\
         FROM sys_menu;\n"
    );

    // 接入说明列出要注册的模块与装配点。
    let readme = files[9]["code"].as_str().expect("接入说明");
    assert!(readme.contains("# SysPost 接入步骤"));
    assert!(readme.contains("pub mod sys_post;"));
    assert!(readme.contains(".merge(controllers::sys_post::router())"));

    // 前端页面取第一个参与列表展示的字段作为主字段。
    let view = files[7]["code"].as_str().expect("页面代码");
    assert!(view.contains("const tableName = 'sys_post'"));
    assert!(view.contains("const className = 'SysPost'"));
    assert!(view.contains("const primaryField = 'post_code'"));

    let (status, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/tool/generator/tables/999999/preview", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "代码生成表不存在");
}

#[tokio::test]
async fn update_table_config_only_accepts_comment_and_synced() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;
    let table_id = import_file_table(&app, &token).await;

    // 与 Spring 版一致：生成配置只能改表说明与确认状态。
    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/tool/generator/tables/{table_id}"),
            &token,
            json!({ "tableComment": "附件表", "synced": false, "className": "Attachment" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/tool/generator/tables", &token),
    )
    .await;
    let row = &page["data"]["rows"][0];
    assert_eq!(row["tableComment"], "附件表");
    assert_eq!(row["synced"], false);
    assert_eq!(row["className"], "SysPost", "类名由表名推导，不接受覆盖");
    assert_eq!(row["module"], "system");
    assert_eq!(row["businessName"], "post");

    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/tool/generator/tables/999999",
            &token,
            json!({ "tableComment": "不存在" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "代码生成表不存在");
}

#[tokio::test]
async fn generate_and_download_returns_zip_with_every_file() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;
    let table_id = import_file_table(&app, &token).await;

    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            &format!("/api/v1/tool/generator/tables/{table_id}/generate"),
            &token,
            json!({}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["tableName"], "sys_post");
    assert_eq!(body["data"]["message"], "代码压缩包已准备就绪");

    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/tool/generator/tables/999999/generate",
            &token,
            json!({}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "代码生成表不存在");

    let (status, headers, bytes) = send_raw(
        app.clone(),
        get_with_bearer(
            &format!("/api/v1/tool/generator/tables/{table_id}/download"),
            &token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers
            .get(axum::http::header::CONTENT_TYPE)
            .expect("content-type"),
        "application/zip"
    );
    assert_eq!(
        headers
            .get(axum::http::header::CONTENT_DISPOSITION)
            .expect("disposition"),
        "attachment; filename=\"SysPost.zip\""
    );

    let mut archive = ZipArchive::new(Cursor::new(bytes.clone())).expect("ZIP 内容");
    assert_eq!(archive.len(), 10, "ZIP 里是完整路径的 10 个文件");
    let mut controller = String::new();
    archive
        .by_name("backend/src/api/controllers/sys_post.rs")
        .expect("Controller 条目")
        .read_to_string(&mut controller)
        .expect("读取条目");
    assert!(controller.contains("pub fn router() -> Router<Arc<Container>> {"));
    assert!(controller.contains("user.require(\"system:post:list\")?;"));
    // 条目顺序与模板顺序一致。
    let names = (0..archive.len())
        .map(|index| {
            archive
                .by_index(index)
                .expect("条目")
                .name()
                .expect("条目名")
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(names[0], "backend/src/entities/sys_post.rs");
    assert_eq!(names[9], "docs/generated/sys_post.md");

    let (status, _, _) = send_raw(
        app.clone(),
        get_with_bearer("/api/v1/tool/generator/tables/999999/download", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // 导入 / 修改配置 / 生成 / 下载都要留痕（Spring 侧同样标注了这些动作）。
    let (_, logs) = send(
        app.clone(),
        get_with_bearer(
            "/api/v1/monitor/oper-logs?keyword=代码生成&pageSize=20",
            &token,
        ),
    )
    .await;
    let rows = logs["data"]["rows"].as_array().expect("日志");
    let mut actions = rows
        .iter()
        .map(|row| row["action"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    actions.sort();
    assert_eq!(
        actions,
        vec![
            "下载代码".to_string(),
            "下载代码".to_string(),
            "导入表".to_string(),
            "生成代码".to_string(),
            "生成代码".to_string(),
        ],
        "失败与成功请求都留痕"
    );
    let failed = rows
        .iter()
        .filter(|row| row["action"] == "下载代码" && row["result"] == 0)
        .count();
    assert_eq!(failed, 1);
    let failed_row = rows
        .iter()
        .find(|row| row["action"] == "下载代码" && row["result"] == 0)
        .expect("失败的下载请求");
    assert_eq!(failed_row["errorMsg"], "代码生成表不存在");
    assert_eq!(failed_row["url"], "/tool/generator/tables/999999/download");

    // 生成配置只提供新增 / 修改，没有删除接口。
    let (status, body) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/tool/generator/tables/{table_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(body["message"], "不支持的请求方法: DELETE");
}
