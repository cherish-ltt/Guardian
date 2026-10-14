//! 代码生成接口的契约测试：表导入、字段元数据、预览、生成与 ZIP 下载。

mod support;

use std::io::{Cursor, Read};

use axum::http::StatusCode;
use serde_json::json;
use zip::ZipArchive;

use support::{
    DEMO_PASSWORD, admin_token, delete_with_bearer, get_with_bearer, login, post_json_with_bearer,
    put_json_with_bearer, send, send_raw, setup_app,
};

async fn import_file_table(app: &axum::Router, token: &str) -> i64 {
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/tool/generator/import",
            token,
            json!({ "tableNames": ["sys_file"] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "导入失败: {body}");
    assert_eq!(body["data"]["imported"], 1);

    body["data"]["ids"][0].as_i64().expect("生成配置 ID")
}

#[tokio::test]
async fn importable_tables_mark_imported_rows_and_validate_requests() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

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
    assert!(names.contains(&"sys_file".to_string()));
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
            "/api/v1/tool/generator/importable-tables?keyword=sys_file",
            &token,
        ),
    )
    .await;
    assert_eq!(filtered["data"].as_array().expect("候选表").len(), 1);
    assert_eq!(filtered["data"][0]["tableName"], "sys_file");

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
            json!({ "tableNames": ["sys_file"] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(again["data"]["ids"], json!([table_id]));

    let (_, candidates) = send(
        app.clone(),
        get_with_bearer(
            "/api/v1/tool/generator/importable-tables?keyword=sys_file",
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
    assert_eq!(row["tableName"], "sys_file");
    assert_eq!(row["className"], "SysFile");
    assert_eq!(row["module"], "system");
    assert_eq!(row["businessName"], "file");
    assert_eq!(row["synced"], true, "与 Spring 版一致：导入即标记为已生成");
    assert_eq!(
        row["tableComment"],
        serde_json::Value::Null,
        "SQLite 没有表注释"
    );

    let (_, searched) = send(
        app.clone(),
        get_with_bearer("/api/v1/tool/generator/tables?keyword=SysFile", &token),
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
    let outsider = login(&app, "linjh", DEMO_PASSWORD).await;
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
    let app = setup_app().await;
    let token = admin_token(&app).await;
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
    assert_eq!(columns.len(), 10);

    // 主键不进表单与列表。
    assert_eq!(columns[0]["col"], "id");
    assert_eq!(columns[0]["javaField"], "id");
    assert_eq!(columns[0]["javaType"], "Integer");
    assert_eq!(columns[0]["formType"], "数字框");
    assert_eq!(columns[0]["insert"], false);
    assert_eq!(columns[0]["edit"], false);
    assert_eq!(columns[0]["list"], false);
    assert_eq!(columns[0]["required"], false);

    // 含 name 的列默认作为模糊查询条件，NOT NULL 视为必填。
    let original_name = &columns[1];
    assert_eq!(original_name["col"], "original_name");
    assert_eq!(original_name["javaField"], "originalName");
    assert_eq!(original_name["javaType"], "String");
    assert_eq!(
        original_name["formType"], "文本域",
        "SQLite 的 TEXT 对应 MySQL 的 text"
    );
    assert_eq!(original_name["query"], true);
    assert_eq!(original_name["queryType"], "LIKE");
    assert_eq!(original_name["required"], true);
    assert_eq!(
        original_name["comment"],
        serde_json::Value::Null,
        "SQLite 无列注释"
    );

    let size = columns
        .iter()
        .find(|column| column["col"] == "size")
        .expect("size 列");
    assert_eq!(size["javaType"], "Integer");
    assert_eq!(size["formType"], "数字框");
    assert_eq!(size["query"], false);

    let url = columns
        .iter()
        .find(|column| column["col"] == "url")
        .expect("url 列");
    assert_eq!(url["required"], true);

    let create_time = columns
        .iter()
        .find(|column| column["col"] == "create_time")
        .expect("create_time 列");
    assert_eq!(create_time["javaType"], "String", "SQLite 只有 TEXT 类型");
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
async fn preview_lists_seven_files_with_controller_active() {
    let app = setup_app().await;
    let token = admin_token(&app).await;
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
    assert_eq!(files.len(), 7);
    let names = files
        .iter()
        .map(|file| file["name"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "SysFile.java".to_string(),
            "SysFileMapper.java".to_string(),
            "SysFileService.java".to_string(),
            "SysFileController.java".to_string(),
            "sysFile.ts".to_string(),
            "index.vue".to_string(),
            "sys_file_menu.sql".to_string(),
        ]
    );
    assert_eq!(body["data"]["activeFile"], "SysFileController.java");
    let code = body["data"]["code"].as_str().expect("默认展开的代码");
    assert_eq!(
        code,
        files[3]["code"].as_str().expect("Controller 代码"),
        "code 与 activeFile 指向同一份内容"
    );
    // 模板文本与 Spring 版的文本块逐字一致。
    assert_eq!(
        code,
        "package com.scaffold.system.controller;\n\n\
         import org.springframework.web.bind.annotation.RequestMapping;\n\
         import org.springframework.web.bind.annotation.RestController;\n\n\
         @RestController\n@RequestMapping(\"/api/system/sysFile\")\n\
         public class SysFileController {\n}\n"
    );
    assert_eq!(
        files[1]["code"].as_str().expect("Mapper 代码"),
        "package com.scaffold.system.mapper;\n\nimport org.apache.ibatis.annotations.Mapper;\n\n\
         @Mapper\npublic interface SysFileMapper {\n}\n"
    );
    assert_eq!(
        files[2]["code"].as_str().expect("Service 代码"),
        "package com.scaffold.system.service;\n\nimport org.springframework.stereotype.Service;\n\n\
         @Service\npublic class SysFileService {\n}\n"
    );
    assert_eq!(
        files[4]["code"].as_str().expect("前端 api 代码"),
        "import { httpClient } from '@/api/http'\n\nexport const sysFileApi = {\n  \
         list: (query?: Record<string, unknown>) => httpClient.get('/system/sysFile', query as any),\n}\n"
    );
    assert_eq!(
        files[6]["code"].as_str().expect("菜单 SQL"),
        "-- sys_file 菜单权限\n\
         INSERT INTO sys_menu (id, parent_id, menu_name, menu_type, path, component, permission, sort, visible, status, create_by, update_by)\n\
         SELECT COALESCE(MAX(id), 0) + 1, 0, 'SysFile管理', 'menu', '/system/sysFile', 'system/sysFile/index', 'system:sysFile:list', 1, 1, 1, 'system', 'system'\n\
         FROM sys_menu;\n"
    );
    assert_eq!(files[6]["path"], "sql/sys_file_menu.sql");

    // 实体类逐字段生成注释与类型；SQLite 全是 TEXT，因此不引入 LocalDateTime。
    let entity = files[0]["code"].as_str().expect("实体代码");
    assert!(
        entity.starts_with("package com.scaffold.system.domain;\n\n\npublic class SysFile {\n")
    );
    assert!(entity.contains("    /**  */\n    private Integer id;\n\n"));
    assert!(entity.contains("    /**  */\n    private String originalName;\n\n"));
    assert!(entity.ends_with("    private String updateTime;\n\n}\n"));
    assert!(!entity.contains("import java.time.LocalDateTime;"));

    // 前端页面取第一个参与列表展示的字段作为主字段。
    let view = files[5]["code"].as_str().expect("页面代码");
    assert!(view.contains("const tableName = 'sys_file'"));
    assert!(view.contains("const className = 'SysFile'"));
    assert!(view.contains("const primaryField = 'originalName'"));

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
    let app = setup_app().await;
    let token = admin_token(&app).await;
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
    assert_eq!(row["className"], "SysFile", "类名由表名推导，不接受覆盖");
    assert_eq!(row["module"], "system");
    assert_eq!(row["businessName"], "file");

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
    let app = setup_app().await;
    let token = admin_token(&app).await;
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
    assert_eq!(body["data"]["tableName"], "sys_file");
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
        "attachment; filename=\"SysFile.zip\""
    );

    let mut archive = ZipArchive::new(Cursor::new(bytes.clone())).expect("ZIP 内容");
    assert_eq!(archive.len(), 7, "ZIP 里是完整路径的 7 个文件");
    let mut controller = String::new();
    archive
        .by_name("backend/src/main/java/com/scaffold/system/controller/SysFileController.java")
        .expect("Controller 条目")
        .read_to_string(&mut controller)
        .expect("读取条目");
    assert!(controller.starts_with("package com.scaffold.system.controller;\n"));
    assert!(controller.contains("public class SysFileController {\n}\n"));
    // 条目顺序与 Spring 版的 LinkedHashMap 一致。
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
    assert_eq!(
        names[0],
        "backend/src/main/java/com/scaffold/system/domain/SysFile.java"
    );
    assert_eq!(names[6], "sql/sys_file_menu.sql");

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
