//! 后台管理接口的契约测试：登录会话、权限校验与 `/api/v1/system/*` 全量 CRUD。
//!
//! 覆盖 Spring 版的可观察行为：权限码、错误文案、分页形状、数据范围与导出内容。

mod support;

use std::sync::{Arc, Mutex};

use axum::http::StatusCode;
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};

use guardian_backend::infrastructure::repositories::account::AccountRepository;
use guardian_backend::infrastructure::services::auth_store::MemoryAuthStore;
use support::{
    DEMO_PASSWORD, admin_token, delete_with_bearer, get, get_with_bearer, login, post_json,
    post_json_with_bearer, put_json_with_bearer, send, setup_app_with_container,
    setup_app_with_store, setup_app_with_store_with_trusted_proxies, take_captcha,
};

#[tokio::test]
async fn captcha_login_and_session_lifecycle() {
    let (app, store) = setup_app_with_store().await;

    let (status, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(captcha["code"], 200);
    assert_eq!(captcha["message"], "操作成功");
    let uuid = captcha["data"]["uuid"]
        .as_str()
        .expect("验证码标识")
        .to_string();
    assert!(
        captcha["data"]["image"]
            .as_str()
            .expect("验证码图片")
            .starts_with("data:image/svg+xml;base64,"),
        "验证码以图片形式下发"
    );
    assert!(captcha["data"].get("code").is_none(), "验证码明文不得回显");
    assert_eq!(uuid.len(), 32);
    let code = take_captcha(store.as_ref(), &uuid);
    // 追踪 ID 由中间件注入响应体，与响应头同源。
    let trace_id = captcha["traceId"].as_str().expect("traceId");
    assert_eq!(trace_id.len(), 32);

    let (status, signed_in) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({ "username": "admin", "password": DEMO_PASSWORD, "captcha": code, "captchaUuid": uuid }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(signed_in["data"]["expiresIn"], 7200);
    let token = signed_in["data"]["token"]
        .as_str()
        .expect("登录令牌")
        .to_string();

    // 验证码只能用一次：同一个 uuid 重放必然失败。
    let reuse = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({ "username": "admin", "password": DEMO_PASSWORD, "captcha": code, "captchaUuid": uuid }),
        ),
    )
    .await;
    assert_eq!(reuse.0, StatusCode::BAD_REQUEST);
    assert_eq!(reuse.1["message"], "验证码错误或已过期");

    let (status, body) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({ "username": "admin", "password": "wrong-password", "captcha": "ABCD", "captchaUuid": "unknown" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "验证码错误或已过期");

    let (status, me) = send(app.clone(), get_with_bearer("/api/v1/auth/me", &token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["data"]["userId"], 1);
    assert_eq!(me["data"]["username"], "admin");
    assert_eq!(me["data"]["deptId"], 5);
    assert_eq!(me["data"]["deptName"], "平台架构部");
    assert_eq!(me["data"]["roles"], json!(["super_admin"]));
    let permissions = me["data"]["permissions"].as_array().expect("权限码数组");
    assert!(permissions.iter().any(|code| code == "system:user:list"));
    assert!(permissions.iter().any(|code| code == "tool:gen:list"));

    let (status, routes) = send(app.clone(), get_with_bearer("/api/v1/auth/routes", &token)).await;
    assert_eq!(status, StatusCode::OK);
    let routes = routes["data"]["routes"].as_array().expect("路由数组");
    assert_eq!(routes.len(), 4);
    // 第一层按 parentId / sort 排序：工作台、系统管理、系统监控、系统工具。
    assert_eq!(routes[0]["name"], "工作台");
    assert_eq!(routes[1]["name"], "系统管理");
    let system = &routes[1];
    assert_eq!(system["path"], "/system");
    let children = system["children"].as_array().expect("子菜单");
    assert!(children.iter().any(|node| node["name"] == "用户管理"));

    // 登出后令牌立刻失效（JWT 仍合法，但缓存里的会话已撤销）。
    let (status, _) = send(
        app.clone(),
        post_json_with_bearer("/api/v1/auth/logout", &token, json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = send(app.clone(), get_with_bearer("/api/v1/auth/me", &token)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "未认证或登录已过期");
}

/// 验证码开关默认开启：`enabled` 为 true，登录必须带验证码。
#[tokio::test]
async fn captcha_is_enabled_by_default() {
    let (app, _store) = setup_app_with_store().await;

    let (status, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(captcha["data"]["enabled"], true);
    assert!(
        captcha["data"]["image"]
            .as_str()
            .expect("验证码图片")
            .starts_with("data:image/svg+xml;base64,")
    );
    assert!(captcha["data"].get("code").is_none(), "验证码明文不得回显");

    let (status, body) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({ "username": "admin", "password": DEMO_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "验证码错误或已过期");
}

/// 后台把参数改成 false 后：`/auth/captcha` 不再生成验证码，登录也不校验。
#[tokio::test]
async fn disabled_captcha_skips_verification() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/configs?keyword=captchaEnabled", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["data"]["total"], 1, "验证码开关应作为可改参数存在");
    let row = &page["data"]["rows"][0];
    assert_eq!(row["key"], "sys.account.captchaEnabled");
    assert_eq!(row["builtin"], false, "内置参数不可改，与开关语义冲突");
    let id = row["id"].as_i64().expect("参数 ID");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/configs/{id}"),
            &token,
            json!({ "value": "false" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(captcha["data"]["enabled"], false);
    assert_eq!(captcha["data"]["uuid"], "");
    assert_eq!(captcha["data"]["image"], "");

    let (status, body) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({ "username": "admin", "password": DEMO_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "关闭后不带验证码应能登录: {body}");
    assert!(body["data"]["token"].is_string());

    // 密码仍要校验：开关只放宽验证码，不放宽口令。
    let (status, body) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({ "username": "admin", "password": "wrong-password" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "账号或密码错误");
}

#[tokio::test]
async fn missing_invalid_and_revoked_credentials_are_rejected() {
    let (app, _store) = setup_app_with_store().await;

    let (status, body) = send(app.clone(), get("/api/v1/auth/me")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "未认证或登录已过期");

    let (status, body) = send(app.clone(), get_with_bearer("/api/v1/auth/me", "not-a-jwt")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "未认证或登录已过期");

    let (status, body) = send(app.clone(), get("/api/v1/system/users")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "未认证或登录已过期");
}

#[tokio::test]
async fn accounts_without_permission_get_403() {
    let (app, store) = setup_app_with_store().await;

    // 种子里的 `sys_admin` 未分配任何菜单，因此没有任何权限码。
    let token = login(&app, store.as_ref(), "linjh", DEMO_PASSWORD).await;
    let (status, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?pageNum=1&pageSize=10", &token),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], "没有访问权限");

    let (status, body) = send(
        app.clone(),
        post_json_with_bearer("/api/v1/system/users", &token, json!({ "username": "x" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], "没有访问权限");
}

#[tokio::test]
async fn unknown_route_and_unsupported_method_use_spring_wording() {
    let (app, _store) = setup_app_with_store().await;

    let (status, body) = send(app.clone(), get("/api/v1/system/unknown")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "资源不存在");

    let (status, body) = send(app.clone(), get("/api/v1/auth/login")).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(body["message"], "不支持的请求方法: GET");
}

/// 未匹配路由的日志只记路径：未匹配请求同样可能带令牌、验证码等 query 参数。
#[tokio::test]
async fn unknown_route_log_omits_query_string() {
    let (app, _store) = setup_app_with_store().await;
    let captured = Arc::new(Mutex::new(String::new()));

    #[derive(Clone)]
    struct Writer(Arc<Mutex<String>>);

    impl std::io::Write for Writer {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .expect("日志缓冲可写")
                .push_str(&String::from_utf8_lossy(buf));
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let writer = Writer(Arc::clone(&captured));
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::WARN)
        .finish();

    let guard = tracing::subscriber::set_default(subscriber);
    let (status, _) = send(app, get("/api/v1/system/unknown?token=do-not-log")).await;
    drop(guard);
    assert_eq!(status, StatusCode::NOT_FOUND);

    let logs = captured.lock().expect("日志缓冲可读").clone();
    assert!(logs.contains("no route matched"), "{logs}");
    assert!(logs.contains("/api/v1/system/unknown"), "{logs}");
    assert!(
        !logs.contains("do-not-log"),
        "日志不得包含 query 值: {logs}"
    );
}

#[tokio::test]
async fn users_page_supports_paging_filters_and_options() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    // 超过上限的 pageSize 必须显式拒绝，不能静默截断后返回半页数据。
    let (status, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?pageNum=1&pageSize=101", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "每页条数不能超过100");

    let (status, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?pageNum=1&pageSize=2", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["data"]["total"], 3);
    assert_eq!(page["data"]["pageNum"], 1);
    assert_eq!(page["data"]["pageSize"], 2);
    assert_eq!(page["data"]["rows"].as_array().expect("用户行").len(), 2);
    let first = &page["data"]["rows"][0];
    assert_eq!(first["username"], "admin");
    assert_eq!(first["deptName"], "平台架构部");
    assert_eq!(first["roleName"], "超级管理员");
    assert_eq!(first["postName"], "技术总监");

    let (_, filtered) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?keyword=heyj", &token),
    )
    .await;
    assert_eq!(filtered["data"]["total"], 1);
    assert_eq!(filtered["data"]["rows"][0]["nickname"], "何雅静");

    let (_, by_dept) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?deptId=6", &token),
    )
    .await;
    assert_eq!(by_dept["data"]["total"], 1);

    let (_, by_post) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?postId=6", &token),
    )
    .await;
    assert_eq!(by_post["data"]["total"], 1);

    let (status, options) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users/options", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(options["data"]["initPassword"], "Yy@123456");
    assert_eq!(
        options["data"]["roles"].as_array().expect("角色选项").len(),
        5
    );
    assert_eq!(
        options["data"]["posts"].as_array().expect("岗位选项").len(),
        9
    );
    let depts = options["data"]["depts"].as_array().expect("部门树");
    assert_eq!(depts.len(), 1, "种子部门只有一个根节点");
    assert_eq!(depts[0]["name"], "Guardian 科技集团");
    assert_eq!(
        depts[0]["children"].as_array().expect("子部门").len(),
        3,
        "根节点下有研发 / 产品 / 职能中心"
    );

    // 参数非法时明确报错，不静默忽略过滤条件。
    let (status, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?status=abc", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "参数 status 必须是整数");
}

#[tokio::test]
async fn reset_password_bumps_auth_version_so_old_tokens_die() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (_, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/users",
            &token,
            json!({ "username": "rotate", "nickname": "改密对象", "deptId": 6, "roleIds": [3] }),
        ),
    )
    .await;
    let id = created["data"]["id"].as_i64().expect("新用户 ID");

    let before = AccountRepository::security(&container.database, id)
        .await
        .expect("读取安全状态")
        .auth_version;

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/users/{id}/password"),
            &token,
            json!({ "password": "Zz@654321" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let after = AccountRepository::security(&container.database, id)
        .await
        .expect("读取安全状态")
        .auth_version;
    // 光删 Redis 令牌不够：索引缺失或清理失败时旧 JWT 仍能验签通过，
    // 版本号是兜底的第二道闸。
    assert_eq!(after, before + 1, "重置口令必须递增 auth_version");
}

#[tokio::test]
async fn user_write_flow_covers_create_update_status_password_and_delete() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/users",
            &token,
            json!({
                "username": "newbie",
                "nickname": "新同学",
                "email": "newbie@example.com",
                "deptId": 6,
                "roleIds": [3],
                "postIds": [6],
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = created["data"]["id"].as_i64().expect("新用户 ID");

    // 新建用户未传口令时用系统参数 `sys.user.initPassword`，可直接登录。
    let newbie_token = login(&app, store.as_ref(), "newbie", "Yy@123456").await;
    let (_, me) = send(
        app.clone(),
        get_with_bearer("/api/v1/auth/me", &newbie_token),
    )
    .await;
    assert_eq!(me["data"]["username"], "newbie");
    assert_eq!(me["data"]["nickname"], "新同学");
    assert_eq!(me["data"]["roles"], json!(["biz_admin"]));

    // 登录账号重复。
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/users",
            &token,
            json!({ "username": "newbie", "nickname": "重复" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "登录账号已存在");

    // 必填校验。
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer("/api/v1/system/users", &token, json!({ "username": "   " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "登录账号不能为空");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/users/{id}"),
            &token,
            json!({ "nickname": "改名了", "deptId": 7, "roleIds": [4], "postIds": [] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?keyword=newbie", &token),
    )
    .await;
    let row = &page["data"]["rows"][0];
    assert_eq!(row["nickname"], "改名了");
    assert_eq!(row["deptName"], "测试质量部");
    assert_eq!(row["roleName"], "研发工程师");
    assert_eq!(row["postName"], "", "岗位被清空后按空串返回");

    // 停用后无法登录，且状态值必须合法。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/users/{id}/status"),
            &token,
            json!({ "status": 2 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "用户状态只能为正常或停用");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/users/{id}/status"),
            &token,
            json!({ "status": 0 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({ "username": "newbie", "password": DEMO_PASSWORD, "captcha": "AB12", "captchaUuid": "nope" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "验证码错误或已过期");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/users/{id}/status"),
            &token,
            json!({ "status": 1 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 重置密码后新口令生效。
    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/users/{id}/password"),
            &token,
            json!({ "password": "Zz@654321" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rotated = login(&app, store.as_ref(), "newbie", "Zz@654321").await;
    assert!(!rotated.is_empty());

    // 重置口令必须显式给出新口令，没有默认弱口令兜底。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/users/{id}/password"),
            &token,
            json!({}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "新密码不能为空");

    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/users/{id}/password"),
            &token,
            json!({ "password": "short" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "密码长度必须在6到64位之间");

    // 内置管理员受保护。
    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/system/users/1", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "内置管理员不允许删除");

    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/users/1/status",
            &token,
            json!({ "status": 0 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "内置管理员不允许停用");

    // 编辑接口同样能改 status，保护必须一致。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/users/1",
            &token,
            json!({ "nickname": "若依", "status": 0 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "内置管理员不允许停用");

    // 编辑接口的 status 取值与 /status 端点同口径：非 0/1 一律拒绝。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/users/{id}"),
            &token,
            json!({ "nickname": "测试账号", "status": 2 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "用户状态只能为正常或停用");

    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/users/{id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/users/{id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "用户不存在或已删除");
}

/// 软删除的账号、角色、岗位不再占用唯一约束：删除后可以立刻重建同名记录。
#[tokio::test]
async fn soft_deleted_records_do_not_block_recreating_the_same_name() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    // 用户：删除后重建同名账号。
    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/users",
            &token,
            json!({
                "username": "recycle",
                "nickname": "回收站",
                "password": "Rr@123456",
                "deptId": 5,
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let user_id = created["data"]["id"].as_i64().expect("用户 ID");
    assert_eq!(
        send(
            app.clone(),
            delete_with_bearer(&format!("/api/v1/system/users/{user_id}"), &token)
        )
        .await
        .0,
        StatusCode::OK
    );
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/users",
            &token,
            json!({
                "username": "recycle",
                "nickname": "回收站二",
                "password": "Rr@123456",
                "deptId": 5,
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "删除后应能重建同名账号: {body}");

    // 角色：删除后重建同编码角色。
    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/roles",
            &token,
            json!({ "name": "回收角色", "code": "recycled_role", "dataScope": "SELF" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let role_id = created["data"]["id"].as_i64().expect("角色 ID");
    assert_eq!(
        send(
            app.clone(),
            delete_with_bearer(&format!("/api/v1/system/roles/{role_id}"), &token)
        )
        .await
        .0,
        StatusCode::OK
    );
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/roles",
            &token,
            json!({ "name": "回收角色二", "code": "recycled_role", "dataScope": "SELF" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "删除后应能重建同编码角色: {body}");

    // 岗位：删除后重建同编码岗位。
    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/posts",
            &token,
            json!({ "postCode": "recycled_post", "postName": "回收岗位" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let post_id = created["data"]["id"].as_i64().expect("岗位 ID");
    assert_eq!(
        send(
            app.clone(),
            delete_with_bearer(&format!("/api/v1/system/posts/{post_id}"), &token)
        )
        .await
        .0,
        StatusCode::OK
    );
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/posts",
            &token,
            json!({ "postCode": "recycled_post", "postName": "回收岗位二" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "删除后应能重建同编码岗位: {body}");
}

#[tokio::test]
async fn user_export_returns_spring_csv() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, headers, bytes) = support::send_raw(
        app.clone(),
        get_with_bearer("/api/v1/system/users/export", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers
            .get(axum::http::header::CONTENT_TYPE)
            .expect("content-type"),
        "text/csv; charset=UTF-8"
    );
    assert_eq!(
        headers
            .get(axum::http::header::CONTENT_DISPOSITION)
            .expect("disposition"),
        "attachment; filename=\"users.csv\""
    );
    let csv = String::from_utf8(bytes).expect("CSV 为 UTF-8");

    assert!(
        csv.starts_with("登录账号,用户昵称,手机号,邮箱,部门,岗位,角色,状态,创建时间,最近登录\n")
    );
    assert!(csv.contains("admin,陈立诚,,chenlc@example.com,平台架构部,技术总监,超级管理员,正常"));
}

#[tokio::test]
async fn role_flow_covers_menus_graph_and_guards() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/roles?pageNum=1&pageSize=20", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["data"]["total"], 6);
    let seeded = &page["data"]["rows"][0];
    assert_eq!(seeded["code"], "super_admin");
    assert_eq!(seeded["dataScope"], "全部数据权限");
    assert_eq!(seeded["builtin"], true);
    assert_eq!(seeded["userCount"], 1);
    assert!(
        seeded["menuIds"].as_array().expect("菜单 ID").len() > 30,
        "超级管理员拥有全部菜单"
    );

    let (_, menus) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/roles/1/menus", &token),
    )
    .await;
    assert_eq!(
        menus["data"]["menuIds"].as_array().expect("菜单 ID").len(),
        39
    );

    // 不存在的菜单不能写进授权，否则 sys_role_menu 会留下悬空关联。
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/roles",
            &token,
            json!({ "name": "越界角色", "code": "beyond", "menuIds": [100, 999999] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "菜单不存在或已删除");

    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/roles",
            &token,
            json!({ "name": "受限角色", "code": "limited", "dataScope": "DEPT", "menuIds": [100, 101] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let role_id = created["data"]["id"].as_i64().expect("角色 ID");

    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/roles/{role_id}"),
            &token,
            json!({ "menuIds": [100, 999999] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "菜单不存在或已删除");

    let (_, duplicate) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/roles",
            &token,
            json!({ "name": "重复编码", "code": "limited" }),
        ),
    )
    .await;
    assert_eq!(duplicate["message"], "角色编码已存在");

    let (_, roles) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/roles?keyword=受限", &token),
    )
    .await;
    assert_eq!(roles["data"]["rows"][0]["menuIds"], json!([100, 101]));
    assert_eq!(roles["data"]["rows"][0]["dataScope"], "本部门数据权限");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/roles/{role_id}"),
            &token,
            json!({ "name": "受限角色改", "dataScope": "SELF", "menuIds": [101] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, updated) = send(
        app.clone(),
        get_with_bearer(&format!("/api/v1/system/roles/{role_id}/menus"), &token),
    )
    .await;
    assert_eq!(updated["data"]["menuIds"], json!([101]));

    // CUSTOM 必须带部门，且写入后可读回；切回其他范围时清空。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/roles/{role_id}"),
            &token,
            json!({ "dataScope": "CUSTOM" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "自定义数据权限必须选择部门");

    // 不存在的部门不能被写进 sys_role_dept，否则会留下悬空关联。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/roles/{role_id}"),
            &token,
            json!({ "dataScope": "CUSTOM", "deptIds": [99999] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "部门不存在或已删除");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/roles/{role_id}"),
            &token,
            json!({ "dataScope": "CUSTOM", "deptIds": [5, 6] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, custom) = send(
        app.clone(),
        get_with_bearer(&format!("/api/v1/system/roles/{role_id}/menus"), &token),
    )
    .await;
    assert_eq!(custom["data"]["deptIds"], json!([5, 6]));

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/roles/{role_id}"),
            &token,
            json!({ "dataScope": "SELF" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, cleared) = send(
        app.clone(),
        get_with_bearer(&format!("/api/v1/system/roles/{role_id}/menus"), &token),
    )
    .await;
    assert_eq!(cleared["data"]["deptIds"], json!([]));

    // 内置角色不可修改 / 删除。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/roles/1",
            &token,
            json!({ "name": "改超管" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "角色不存在、已删除或为内置角色");

    // 已分配用户的角色不能删除。
    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/system/roles/2", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "角色已分配用户，不能删除");

    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/roles/{role_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, graph) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/permissions/graph", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(graph["data"]["usersVisible"], true);
    assert_eq!(
        graph["data"]["users"].as_array().expect("用户节点").len(),
        3
    );
    assert_eq!(
        graph["data"]["menus"].as_array().expect("菜单节点").len(),
        39
    );
    assert_eq!(graph["data"]["userRoles"], json!([[1, 1], [2, 2], [3, 3]]));
}

#[tokio::test]
async fn menu_tree_export_and_crud() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, tree) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/menus/tree", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let roots = tree["data"].as_array().expect("菜单树");
    assert_eq!(roots.len(), 4);
    assert_eq!(roots[0]["name"], "工作台");
    assert_eq!(roots[0]["type"], "menu");

    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/menus",
            &token,
            json!({ "parentId": 105, "name": "导入岗位", "type": "btn", "permission": "system:post:import", "sort": 4 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = created["data"]["id"].as_i64().expect("菜单 ID");
    assert!(id > 1073, "新菜单 ID 由当前最大值 +1 得到");

    let (status, body) = send(
        app.clone(),
        post_json_with_bearer("/api/v1/system/menus", &token, json!({ "name": "  " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "菜单名称不能为空");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/menus/{id}"),
            &token,
            json!({ "name": "导入岗位改", "sort": 9 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, tree) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/menus/tree?keyword=导入岗位", &token),
    )
    .await;
    // 命中过滤的菜单没有带上父节点，因此自己成为根。
    assert_eq!(tree["data"].as_array().expect("过滤后的菜单").len(), 1);
    assert_eq!(tree["data"][0]["name"], "导入岗位改");
    assert_eq!(tree["data"][0]["parentId"], 105);
    assert_eq!(tree["data"][0]["permission"], "system:post:import");

    // 有子菜单不能删；删除按钮本身可以删。
    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/system/menus/105", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "存在子菜单，不能删除");

    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/system/menus/1", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "菜单已分配角色，不能删除");

    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/menus/{id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, _, bytes) = support::send_raw(
        app.clone(),
        get_with_bearer("/api/v1/system/menus/export", &token),
    )
    .await;
    let csv = String::from_utf8(bytes).expect("CSV");
    assert!(csv.starts_with("菜单名称,类型,图标,路由地址,组件路径,权限标识,排序,状态\n"));
    assert!(
        csv.contains("用户管理,menu,users,/system/user,system/user/index,system:user:list,1,正常")
    );
}

#[tokio::test]
async fn dept_tree_and_crud_keep_ancestors_consistent() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, tree) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/depts/tree", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let roots = tree["data"].as_array().expect("部门树");
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0]["memberCount"], 0, "集团本部没有直属成员");
    let rd = &roots[0]["children"][0];
    assert_eq!(rd["name"], "研发中心");
    assert_eq!(
        rd["children"][0]["memberCount"], 2,
        "平台架构部有 admin 与 linjh"
    );

    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/depts",
            &token,
            json!({ "parentId": 5, "name": "架构一组", "leader": "李维" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = created["data"]["id"].as_i64().expect("部门 ID");

    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/depts",
            &token,
            json!({ "parentId": 9999, "name": "悬空部门" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "上级部门不存在或已删除");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/depts/{id}"),
            &token,
            json!({ "name": "架构一组改", "leader": "李维", "sort": 2 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/system/depts/5", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "存在下级部门，不能删除");

    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/system/depts/6", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "部门已关联用户，不能删除");

    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/depts/{id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

/// 移动部门子树：后代祖先链一起重算，且不允许挂到自己的下级下。
#[tokio::test]
async fn moving_dept_subtree_rebuilds_descendant_ancestors() {
    use guardian_backend::infrastructure::repositories::dept::{
        DeptRepository, SeaOrmDeptRepository,
    };
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, store.as_ref()).await;

    // 把「研发中心」(2) 挂到「职能中心」(4) 下：其子部门 5/6/7 的祖先链必须一起变。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer("/api/v1/system/depts/2", &token, json!({ "parentId": 4 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    for dept_id in [2, 5, 6, 7] {
        let ancestors = SeaOrmDeptRepository
            .dept_ancestors(&container.database, dept_id)
            .await
            .expect("读取祖先链")
            .expect("部门存在");
        let expected = match dept_id {
            2 => "0,1,4".to_string(),
            5..=7 => "0,1,4,2".to_string(),
            _ => unreachable!(),
        };
        assert_eq!(ancestors, expected, "部门 {dept_id} 的祖先链");
    }

    // 挂到自己的下级下成环：拒绝，并且原结构保持不变。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer("/api/v1/system/depts/2", &token, json!({ "parentId": 6 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "上级部门不能是部门自身的下级");
    assert_eq!(
        SeaOrmDeptRepository
            .dept_ancestors(&container.database, 2)
            .await
            .expect("读取祖先链")
            .as_deref(),
        Some("0,1,4")
    );

    // 自己是自己的上级同样拒绝。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer("/api/v1/system/depts/2", &token, json!({ "parentId": 2 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "上级部门不能是部门自身");
}

#[tokio::test]
async fn post_flow_and_export() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/posts?pageNum=1&pageSize=20", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["data"]["total"], 9);
    assert_eq!(page["data"]["rows"][0]["postCode"], "ceo");
    assert_eq!(page["data"]["rows"][0]["userCount"], 0);

    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/posts",
            &token,
            json!({ "postCode": "intern", "postName": "实习生", "remark": "校园招聘" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = created["data"]["id"].as_i64().expect("岗位 ID");

    let (_, duplicate) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/posts",
            &token,
            json!({ "postCode": "intern", "postName": "重复" }),
        ),
    )
    .await;
    assert_eq!(duplicate["message"], "岗位编码已存在");

    let (_, missing) = send(
        app.clone(),
        post_json_with_bearer("/api/v1/system/posts", &token, json!({ "postCode": "x" })),
    )
    .await;
    assert_eq!(missing["message"], "岗位名称不能为空");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/posts/{id}"),
            &token,
            json!({ "postName": "实习生改", "status": 0 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/posts/{id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, _, bytes) = support::send_raw(
        app.clone(),
        get_with_bearer("/api/v1/system/posts/export", &token),
    )
    .await;
    let csv = String::from_utf8(bytes).expect("CSV");
    assert!(csv.starts_with("岗位编码,岗位名称,排序,状态,创建时间,备注\n"));
    assert!(csv.contains("ceo,董事长,1,正常"));
}

#[tokio::test]
async fn dict_flow_covers_types_data_and_business_options() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/dict/types?pageNum=1&pageSize=20", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["data"]["total"], 3);

    // 管理页的 `def` 是布尔值，业务下拉里是 0/1。
    let (_, data) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/dict/data/sys_normal_disable", &token),
    )
    .await;
    let rows = data["data"].as_array().expect("字典数据");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["label"], "正常");
    assert_eq!(rows[0]["def"], true);
    assert_eq!(rows[1]["def"], false);

    let (status, options) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/dict/options/sys_normal_disable", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let options = options["data"].as_array().expect("业务下拉");
    assert_eq!(options[0]["def"], 1);
    assert_eq!(options[0]["tone"], "ok");

    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/dict/types",
            &token,
            json!({ "dictName": "任务状态", "dictType": "task_status" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let type_id = created["data"]["id"].as_i64().expect("字典类型 ID");

    let (_, duplicate) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/dict/types",
            &token,
            json!({ "dictName": "重复", "dictType": "task_status" }),
        ),
    )
    .await;
    assert_eq!(duplicate["message"], "字典类型已存在");

    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/dict/data",
            &token,
            json!({ "dictType": "task_status", "label": "进行中", "value": "1", "tone": "warn", "def": true }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let data_id = created["data"]["id"].as_i64().expect("字典数据 ID");

    // 有字典数据时类型不能删除。
    let (status, body) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/dict/types/{type_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "字典类型存在字典数据，不能删除");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/dict/data/{data_id}"),
            &token,
            json!({ "label": "已完成", "value": "2" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/dict/data/{data_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/dict/types/{type_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 不允许删除内置字典类型（数据仍在）。
    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/system/dict/types/2", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "字典类型存在字典数据，不能删除");
}

#[tokio::test]
async fn config_flow_and_export() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/configs?pageNum=1&pageSize=20", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["data"]["total"], 3);
    let rows = page["data"]["rows"].as_array().expect("参数行");
    assert!(rows.iter().any(|row| row["key"] == "sys.user.initPassword"
        && row["value"] == "Yy@123456"
        && row["builtin"] == true
        && row["type"] == "Y"));
    assert!(
        rows.iter()
            .any(|row| row["key"] == "sys.account.captchaEnabled"
                && row["value"] == "true"
                && row["builtin"] == false)
    );

    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/configs",
            &token,
            json!({ "name": "任务并发数", "key": "task.concurrency", "value": "4", "type": "N" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = created["data"]["id"].as_i64().expect("参数 ID");

    let (_, duplicate) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/configs",
            &token,
            json!({ "name": "重复", "key": "task.concurrency", "value": "1" }),
        ),
    )
    .await;
    assert_eq!(duplicate["message"], "参数键名已存在");

    let (_, missing) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/configs",
            &token,
            json!({ "name": "缺键值", "key": "task.missing" }),
        ),
    )
    .await;
    assert_eq!(missing["message"], "参数键值不能为空");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/configs/{id}"),
            &token,
            json!({ "value": "8" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 内置参数不可修改 / 删除。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/configs/1",
            &token,
            json!({ "value": "hacked" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "参数配置不存在或为内置参数");

    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/configs/{id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, _, bytes) = support::send_raw(
        app.clone(),
        get_with_bearer("/api/v1/system/configs/export", &token),
    )
    .await;
    let csv = String::from_utf8(bytes).expect("CSV");
    assert!(csv.starts_with("参数名称,参数键名,参数键值,系统内置,创建时间,备注\n"));
    assert!(csv.contains("用户管理-账号初始密码,sys.user.initPassword,Yy@123456,Y"));
}

#[tokio::test]
async fn data_scope_limits_user_list_for_dept_scoped_role() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    // 给「系统管理员」授予用户管理与角色管理菜单：它拥有 DEPT_AND_CHILD 范围（部门 5）。
    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/roles/2",
            &token,
            json!({ "menuIds": [101, 102] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let scoped = login(&app, store.as_ref(), "linjh", DEMO_PASSWORD).await;
    let (status, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?pageNum=1&pageSize=20", &scoped),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["data"]["total"], 2, "只能看到平台架构部的两个账号");
    let usernames = page["data"]["rows"]
        .as_array()
        .expect("用户行")
        .iter()
        .map(|row| row["username"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert_eq!(usernames, vec!["admin".to_string(), "linjh".to_string()]);

    // 导出同样遵守数据范围。
    let (_, _, bytes) = support::send_raw(
        app.clone(),
        get_with_bearer("/api/v1/system/users/export", &scoped),
    )
    .await;
    let csv = String::from_utf8(bytes).expect("CSV");
    assert!(csv.contains("admin"));
    assert!(csv.contains("linjh"));
    assert!(!csv.contains("heyj"), "部门外的账号不能出现在导出里");

    // 只读权限不会带回可编辑选项与初始密码。
    let (_, options) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users/options", &scoped),
    )
    .await;
    assert_eq!(options["data"]["roles"], json!([]));
    assert_eq!(options["data"]["posts"], json!([]));
    assert_eq!(options["data"]["initPassword"], "");

    let (status, graph) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/permissions/graph", &scoped),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(graph["data"]["usersVisible"], true);
    assert_eq!(
        graph["data"]["users"].as_array().expect("用户节点").len(),
        2,
        "关系图里的用户节点同样受数据范围限制"
    );
    assert_eq!(
        graph["data"]["userRoles"],
        json!([[1, 1], [2, 2]]),
        "被数据范围挡掉的用户不出现授权连线"
    );

    // 缺少权限码的只读账号访问关系图仍然被拒。
    let outsider = login(&app, store.as_ref(), "heyj", DEMO_PASSWORD).await;
    let (status, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/permissions/graph", &outsider),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], "没有访问权限");
}

#[tokio::test]
async fn password_change_requires_old_password_and_rotates_credentials() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/auth/password",
            &token,
            json!({ "oldPassword": "wrong", "newPassword": "Nn@123456" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "旧密码不正确");

    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/auth/password",
            &token,
            json!({ "oldPassword": DEMO_PASSWORD, "newPassword": DEMO_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "新密码不能与旧密码一致");

    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/auth/password",
            &token,
            json!({ "oldPassword": DEMO_PASSWORD, "newPassword": "123" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "newPassword 长度必须在6到64位之间");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/auth/password",
            &token,
            json!({ "oldPassword": DEMO_PASSWORD, "newPassword": "Nn@123456" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let rotated = login(&app, store.as_ref(), "admin", "Nn@123456").await;
    assert!(!rotated.is_empty());

    // 复原口令，避免影响同一测试文件里的其他用例（各自独立数据库，这里只是保持语义清晰）。
    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/auth/password",
            &rotated,
            json!({ "oldPassword": "Nn@123456", "newPassword": DEMO_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

/// 改密码后，该账号已签发的会话（含当前与其它设备）必须全部失效。
#[tokio::test]
async fn password_change_revokes_every_existing_session() {
    let (app, store, container) = setup_app_with_container().await;
    let first = admin_token(&app, store.as_ref()).await;
    let admin_id = 1;
    let before = AccountRepository::security(&container.database, admin_id)
        .await
        .expect("读取安全状态")
        .auth_version;
    // JWT 的 iat / exp 是秒粒度，同一秒内重复登录会签发逐字相同的令牌，
    // 先错开一秒，才能确定拿到两个不同会话。
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let second = login(&app, store.as_ref(), "admin", DEMO_PASSWORD).await;
    assert_ne!(first, second, "两个会话必须是不同令牌");
    let other = login(&app, store.as_ref(), "linjh", DEMO_PASSWORD).await;

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/auth/password",
            &second,
            json!({ "oldPassword": DEMO_PASSWORD, "newPassword": "Nn@123456" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    for (label, token) in [("改密前的会话", &first), ("改密用的会话", &second)] {
        let (status, body) = send(app.clone(), get_with_bearer("/api/v1/auth/me", token)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{label} 应失效");
        assert_eq!(body["message"], "未认证或登录已过期");
    }

    // 光撤销 Redis 会话不够：auth_version 必须递增，让验签阶段就拒绝旧令牌。
    let after = AccountRepository::security(&container.database, admin_id)
        .await
        .expect("读取安全状态")
        .auth_version;
    assert_eq!(after, before + 1, "改密必须递增会话版本");

    // 只清该账号的会话，其他账号不受影响。
    let (status, _) = send(app.clone(), get_with_bearer("/api/v1/auth/me", &other)).await;
    assert_eq!(status, StatusCode::OK, "其他账号的会话不应被误撤");

    let rotated = login(&app, store.as_ref(), "admin", "Nn@123456").await;
    assert!(!rotated.is_empty());
}

#[tokio::test]
async fn login_log_records_success_and_failure() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, failure) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({ "username": "admin", "password": "wrong", "captcha": "AB12", "captchaUuid": "unknown" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(failure["message"], "验证码错误或已过期");

    let (_, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/login-logs?pageNum=1&pageSize=10", &token),
    )
    .await;
    let rows = logs["data"]["rows"].as_array().expect("登录日志");
    assert!(!rows.is_empty());
    assert_eq!(logs["data"]["total"], 1, "验证码失败不写登录日志");
    assert_eq!(rows[0]["username"], "admin");
    assert_eq!(rows[0]["status"], 1);
    assert_eq!(rows[0]["msg"], "登录成功");
    assert_eq!(rows[0]["ip"], "127.0.0.1");
    assert_eq!(rows[0]["location"], "本机");
    assert_eq!(
        rows[0]["browser"],
        Value::Null,
        "装置不带 User-Agent 时留空"
    );
    assert_eq!(rows[0]["os"], Value::Null);
}

/// 登录与退出都采集客户端来源：可信代理透传的真实 IP 覆盖对端地址，
/// `User-Agent` 解析出浏览器与操作系统；伪造的转发头不再被采信。
#[tokio::test]
async fn login_log_records_trusted_client_source() {
    let (app, store) =
        setup_app_with_store_with_trusted_proxies(vec!["127.0.0.1/32".parse().expect("构造网段")])
            .await;
    let token = login_with_source(&app, store.as_ref(), FORWARDED_FOR).await;

    let (status, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/login-logs?pageNum=1&pageSize=10", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "查询登录日志失败: {logs}");
    let row = &logs["data"]["rows"][0];
    assert_eq!(row["username"], "admin");
    assert_eq!(row["msg"], "登录成功");
    assert_eq!(row["ip"], "203.0.113.7", "可信代理透传的真实来源");
    assert_eq!(row["location"], "外网");
    assert_eq!(row["browser"], "Chrome");
    assert_eq!(row["os"], "Windows 10");

    // 退出登录同样带客户端来源；令牌会被撤销，用新会话重新查询。
    let (status, _) = send(
        app.clone(),
        with_source(
            post_json_with_bearer("/api/v1/auth/logout", &token, json!({})),
            FORWARDED_FOR,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let fresh = login_with_source(&app, store.as_ref(), FORWARDED_FOR).await;
    let (_, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/login-logs?pageNum=1&pageSize=10", &fresh),
    )
    .await;
    let rows = logs["data"]["rows"].as_array().expect("登录日志");
    let logout = rows
        .iter()
        .find(|row| row["msg"] == "退出登录")
        .expect("退出登录日志");
    assert_eq!(logout["ip"], "203.0.113.7");
    assert_eq!(logout["browser"], "Chrome");
    assert_eq!(logout["os"], "Windows 10");
}

/// 可信代理透传的来源地址，测试里由 `X-Forwarded-For` 表示。
const FORWARDED_FOR: &str = "203.0.113.7";
const CHROME_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
     Chrome/141.0.0.0 Safari/537.36";

/// 补上可信代理透传的来源与浏览器标识。
fn with_source(
    mut request: axum::http::Request<axum::body::Body>,
    forwarded_for: &str,
) -> axum::http::Request<axum::body::Body> {
    request.headers_mut().insert(
        "x-forwarded-for",
        forwarded_for.parse().expect("构造请求头"),
    );
    request.headers_mut().insert(
        axum::http::header::USER_AGENT,
        CHROME_USER_AGENT.parse().expect("构造请求头"),
    );

    request
}

/// 走正确验证码 + 可信代理头的登录：用于断言日志里的客户端来源。
async fn login_with_source(
    app: &axum::Router,
    store: &MemoryAuthStore,
    forwarded_for: &str,
) -> String {
    let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    let code = take_captcha(store, captcha["data"]["uuid"].as_str().expect("验证码标识"));
    let request = with_source(
        post_json(
            "/api/v1/auth/login",
            json!({
                "username": "admin",
                "password": DEMO_PASSWORD,
                "captcha": code,
                "captchaUuid": captcha["data"]["uuid"],
            }),
        ),
        forwarded_for,
    );
    let (status, body) = send(app.clone(), request).await;
    assert_eq!(status, StatusCode::OK, "登录失败: {body}");

    body["data"]["token"]
        .as_str()
        .expect("登录令牌")
        .to_string()
}

/// 三个此前没有用例的导出端点：表头、附件名与过滤参数都要与 Spring 版对齐。
#[tokio::test]
async fn report_exports_return_spring_csv_with_attachment_name() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let cases = [
        (
            "/api/v1/system/depts/export?keyword=研发&status=1",
            "depts.csv",
            "部门名称,负责人,联系电话,邮箱,排序,状态\n",
            "研发中心",
        ),
        (
            "/api/v1/system/dict/types/export?keyword=sys_&status=1",
            "dict-types.csv",
            "字典名称,字典类型,状态,创建时间,备注\n",
            "sys_normal_disable",
        ),
        (
            "/api/v1/system/roles/export?keyword=管理员&status=1",
            "roles.csv",
            "角色名称,权限字符,数据权限,状态,创建时间,备注\n",
            "super_admin",
        ),
    ];

    for (path, file_name, header, expected_row) in cases {
        let (status, headers, bytes) =
            support::send_raw(app.clone(), get_with_bearer(path, &token)).await;

        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(
            headers
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("text/csv; charset=UTF-8"),
            "{path}"
        );
        assert_eq!(
            headers
                .get("content-disposition")
                .and_then(|value| value.to_str().ok()),
            Some(format!("attachment; filename=\"{file_name}\"").as_str()),
            "{path}"
        );
        let csv = String::from_utf8(bytes).expect("CSV 为合法 UTF-8");
        assert!(csv.starts_with(header), "{path} 表头不符: {csv}");
        assert!(
            csv.contains(expected_row),
            "{path} 缺少 {expected_row}: {csv}"
        );
    }
}

/// 字典类型编辑：部分字段更新，目标不存在时报业务错而不是静默成功。
#[tokio::test]
async fn dict_type_update_applies_partial_changes_and_reports_missing_target() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (_, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/dict/types",
            &token,
            json!({ "dictName": "订单状态", "dictType": "order_status", "remark": "旧备注" }),
        ),
    )
    .await;
    let type_id = created["data"]["id"].as_i64().expect("字典类型 ID");

    // 只改字典名：dict_type 由 COALESCE 保留，过滤条件仍然命中。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/dict/types/{type_id}"),
            &token,
            json!({ "dictName": "订单状态改" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "操作成功");

    let (_, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/dict/types?keyword=order_status", &token),
    )
    .await;
    assert_eq!(page["data"]["total"], 1);
    assert_eq!(page["data"]["rows"][0]["dictName"], "订单状态改");

    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/dict/types/999999",
            &token,
            json!({ "dictName": "不存在" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "字典类型不存在");
}

/// 列表接口的 keyword / status 过滤确实下推到 SQL，而不是取回全量后在内存里筛。
#[tokio::test]
async fn list_filters_narrow_the_result_set() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let total_of = |body: &Value| body["data"]["total"].as_i64().unwrap_or_default();

    // 用户：keyword 命中昵称（linjh → 林嘉豪），status 过滤启用状态。
    let (status, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?keyword=林嘉豪&status=1", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(total_of(&body), 1);
    assert_eq!(body["data"]["rows"][0]["username"], "linjh");

    let (_, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?status=0", &token),
    )
    .await;
    assert_eq!(total_of(&body), 0, "演示账号全部启用");

    // 岗位：keyword 命中岗位名，status 过滤。
    let (_, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/posts?keyword=研发&status=1", &token),
    )
    .await;
    assert_eq!(total_of(&body), 1);
    assert_eq!(body["data"]["rows"][0]["postCode"], "dev");

    let (_, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/posts?status=0", &token),
    )
    .await;
    assert_eq!(total_of(&body), 1, "演示数据里只有「财务专员」是停用岗位");
    assert_eq!(body["data"]["rows"][0]["postCode"], "finance");

    // 角色：keyword 命中角色名，status 过滤。
    let (_, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/roles?keyword=系统管理员&status=1", &token),
    )
    .await;
    assert_eq!(total_of(&body), 1);
    assert_eq!(body["data"]["rows"][0]["code"], "sys_admin");

    let (_, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/roles?status=0", &token),
    )
    .await;
    assert_eq!(total_of(&body), 1, "演示数据里只有「只读访客」是停用角色");
    assert_eq!(body["data"]["rows"][0]["code"], "guest");

    // 字典类型：keyword 命中类型编码，status 过滤。
    let (_, body) = send(
        app.clone(),
        get_with_bearer(
            "/api/v1/system/dict/types?keyword=sys_normal&status=1",
            &token,
        ),
    )
    .await;
    assert_eq!(total_of(&body), 1);
    assert_eq!(body["data"]["rows"][0]["dictType"], "sys_normal_disable");

    let (_, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/dict/types?status=0", &token),
    )
    .await;
    assert_eq!(total_of(&body), 0);

    // 参数配置：只有 keyword 过滤。
    let (_, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/configs?keyword=初始密码", &token),
    )
    .await;
    assert_eq!(total_of(&body), 1);
    assert_eq!(body["data"]["rows"][0]["key"], "sys.user.initPassword");

    // 部门树与菜单树：status 过滤后仍是树形结构。
    let (_, tree) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/depts/tree?status=1", &token),
    )
    .await;
    assert_eq!(tree["data"].as_array().expect("部门树").len(), 1);

    let (_, tree) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/menus/tree?status=1", &token),
    )
    .await;
    assert_eq!(tree["data"].as_array().expect("菜单树").len(), 4);

    // 登录日志：status 区分成功与失败，keyword 命中账号。
    let (_, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/login-logs?status=1&keyword=admin", &token),
    )
    .await;
    assert_eq!(total_of(&logs), 1);
    let (_, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/login-logs?status=0", &token),
    )
    .await;
    assert_eq!(total_of(&logs), 0);
}

/// 非超管的动态路由走按角色授权查询，只返回被授予的菜单；超管走另一条分支。
#[tokio::test]
async fn non_super_admin_routes_only_expose_granted_menus() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    // linjh 的角色（系统管理员）只授「系统管理 → 用户管理」。
    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/roles/2",
            &token,
            json!({ "menuIds": [100, 101] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let linjh = login(&app, store.as_ref(), "linjh", DEMO_PASSWORD).await;
    let (status, routes) = send(app.clone(), get_with_bearer("/api/v1/auth/routes", &linjh)).await;
    assert_eq!(status, StatusCode::OK);
    let roots = routes["data"]["routes"].as_array().expect("路由数组");
    assert_eq!(roots.len(), 1, "只授权了系统管理这一个目录");
    assert_eq!(roots[0]["name"], "系统管理");
    let children = roots[0]["children"].as_array().expect("子菜单");
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["name"], "用户管理");
    assert_eq!(children[0]["path"], "/system/user");

    // 超管：不受角色授权限制，拿到全部顶层目录。
    let (_, admin_routes) = send(app.clone(), get_with_bearer("/api/v1/auth/routes", &token)).await;
    assert_eq!(
        admin_routes["data"]["routes"]
            .as_array()
            .expect("路由数组")
            .len(),
        4
    );
}

/// 停用账号：已签发的会话立刻失效，重新登录被拒并留下失败日志。
#[tokio::test]
async fn disabling_an_account_invalidates_its_existing_session() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let linjh = login(&app, store.as_ref(), "linjh", DEMO_PASSWORD).await;
    let (status, _) = send(app.clone(), get_with_bearer("/api/v1/auth/me", &linjh)).await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/users/2/status",
            &token,
            json!({ "status": 0 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // JWT 本身仍然有效，是账号快照判定为停用。
    let (status, body) = send(app.clone(), get_with_bearer("/api/v1/auth/me", &linjh)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "未认证或登录已过期");

    let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    let code = take_captcha(
        store.as_ref(),
        captcha["data"]["uuid"].as_str().expect("验证码标识"),
    );
    let (status, body) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({
                "username": "linjh",
                "password": DEMO_PASSWORD,
                "captcha": code,
                "captchaUuid": captcha["data"]["uuid"],
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "账号已停用");

    let (_, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/login-logs?keyword=linjh", &token),
    )
    .await;
    assert_eq!(logs["data"]["rows"][0]["msg"], "账号已停用");
    assert_eq!(logs["data"]["rows"][0]["status"], 0);
}

/// 验证码缺失或未命中缓存一律 400，且不写登录日志（避免被刷成失败记录）。
#[tokio::test]
async fn login_requires_a_matching_captcha() {
    let (app, store) = setup_app_with_store().await;

    let cases = [
        json!({ "username": "admin", "password": DEMO_PASSWORD }),
        json!({ "username": "admin", "password": DEMO_PASSWORD, "captcha": "AB12" }),
        json!({ "username": "admin", "password": DEMO_PASSWORD, "captchaUuid": "unknown" }),
    ];

    for body in cases {
        let (status, response) = send(app.clone(), post_json("/api/v1/auth/login", body)).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(response["message"], "验证码错误或已过期");
    }

    let token = admin_token(&app, store.as_ref()).await;
    let (_, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/login-logs?pageNum=1&pageSize=10", &token),
    )
    .await;
    assert_eq!(logs["data"]["total"], 1, "只有成功登录留下了记录");
}

/// 角色 / 岗位关联必须指向真实存在的记录，不能留下悬空关联行。
#[tokio::test]
async fn user_relations_must_point_to_existing_records() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/users",
            &token,
            json!({
                "username": "dangling",
                "nickname": "悬空关联",
                "password": "Dd@123456",
                "deptId": 5,
                "roleIds": [9999],
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "角色不存在或已停用");

    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/users",
            &token,
            json!({
                "username": "dangling",
                "nickname": "悬空关联",
                "password": "Dd@123456",
                "deptId": 5,
                "postIds": [9999],
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "岗位不存在或已删除");

    // 编辑同样校验，且失败时整笔回滚（昵称不会被改掉）。
    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/users/2",
            &token,
            json!({ "nickname": "不该生效", "roleIds": [9999] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (_, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?keyword=linjh", &token),
    )
    .await;
    assert_ne!(page["data"]["rows"][0]["nickname"], "不该生效");
}

/// 授予内置超管角色属于提权：只有超管本人可以，普通「用户编辑」账号不行。
#[tokio::test]
async fn granting_super_admin_role_requires_super_admin() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    // 系统管理员（id=2）拿到用户管理与角色菜单，仍不是超管。
    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/roles/2",
            &token,
            json!({ "menuIds": [101, 102, 1011, 1012, 1021, 1022] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let scoped = login(&app, store.as_ref(), "linjh", DEMO_PASSWORD).await;

    // 1) 新建账号时直接挂超管角色：拒绝，且不留下半截数据。
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/users",
            &scoped,
            json!({
                "username": "escalate",
                "nickname": "提权",
                "password": "Ee@123456",
                "deptId": 5,
                "roleIds": [1],
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], "只有超级管理员可以授予超级管理员角色");
    let (_, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?keyword=escalate", &token),
    )
    .await;
    assert_eq!(page["data"]["total"], 0, "拒绝后不得写入用户");

    // 2) 编辑自己、把超管角色挂上去：拒绝。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/users/2",
            &scoped,
            json!({ "roleIds": [1, 2] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], "只有超级管理员可以授予超级管理员角色");

    // 3) 超管本人同样的操作照旧可以。
    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/users/3",
            &token,
            json!({ "roleIds": [1, 3] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

/// 密码阶段连续失败 5 次后锁定 5 分钟：第 6 次连正确口令也不放行。
#[tokio::test]
async fn password_failures_lock_the_account_even_with_the_right_password() {
    let (app, store) = setup_app_with_store().await;

    for _ in 0..5 {
        let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
        let code = take_captcha(
            store.as_ref(),
            captcha["data"]["uuid"].as_str().expect("验证码标识"),
        );
        let (status, body) = send(
            app.clone(),
            post_json(
                "/api/v1/auth/login",
                json!({
                    "username": "admin",
                    "password": "wrong-password",
                    "captcha": code,
                    "captchaUuid": captcha["data"]["uuid"],
                }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["message"], "账号或密码错误");
    }

    let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    let code = take_captcha(
        store.as_ref(),
        captcha["data"]["uuid"].as_str().expect("验证码标识"),
    );
    let (status, body) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({
                "username": "admin",
                "password": DEMO_PASSWORD,
                "captcha": code,
                "captchaUuid": captcha["data"]["uuid"],
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "密码错误次数过多，请稍后重试");
}

/// 登录成功后失败计数清零，不会因为历史失败把正常用户挡在门外。
#[tokio::test]
async fn successful_login_clears_the_failure_counter() {
    let (app, store) = setup_app_with_store().await;

    for _ in 0..3 {
        let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
        let code = take_captcha(
            store.as_ref(),
            captcha["data"]["uuid"].as_str().expect("验证码标识"),
        );
        send(
            app.clone(),
            post_json(
                "/api/v1/auth/login",
                json!({
                    "username": "admin",
                    "password": "wrong-password",
                    "captcha": code,
                    "captchaUuid": captcha["data"]["uuid"],
                }),
            ),
        )
        .await;
    }

    let token = admin_token(&app, store.as_ref()).await;
    assert!(!token.is_empty());

    // 清零后再错 4 次仍然只是「账号或密码错误」，没有触发锁定。
    for _ in 0..4 {
        let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
        let code = take_captcha(
            store.as_ref(),
            captcha["data"]["uuid"].as_str().expect("验证码标识"),
        );
        let (status, body) = send(
            app.clone(),
            post_json(
                "/api/v1/auth/login",
                json!({
                    "username": "admin",
                    "password": "wrong-password",
                    "captcha": code,
                    "captchaUuid": captcha["data"]["uuid"],
                }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["message"], "账号或密码错误");
    }
}

/// 顶层部门的 `ancestors` 为空串，挂在它下面的部门只追加自己的父级。
#[tokio::test]
async fn dept_at_root_level_keeps_an_empty_ancestor_chain() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, root) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/depts",
            &token,
            json!({ "parentId": 0, "name": "新事业群" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let root_id = root["data"]["id"].as_i64().expect("部门 ID");

    let (status, child) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/depts",
            &token,
            json!({ "parentId": root_id, "name": "事业群一组" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let child_id = child["data"]["id"].as_i64().expect("部门 ID");

    let (_, tree) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/depts/tree", &token),
    )
    .await;
    let roots = tree["data"].as_array().expect("部门树");
    assert_eq!(roots.len(), 2, "集团本部之外多了一个顶层部门");
    let created = roots
        .iter()
        .find(|node| node["id"] == root_id)
        .expect("新建的顶层部门");
    let children = created["children"].as_array().expect("子部门");
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["id"], child_id);
}

/// 写操作指向不存在的目标时给出明确业务文案，而不是静默成功。
#[tokio::test]
async fn writes_to_missing_targets_report_business_errors() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/users/999999",
            &token,
            json!({ "nickname": "查无此人" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "用户不存在或已删除");

    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/system/roles/999999", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "角色不存在、已删除或为内置角色");

    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/system/dict/types/999999", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "字典类型不存在");
}

/// 可选字段缺省时回落到稳定默认值，避免前端漏传导致写入 NULL。
#[tokio::test]
async fn omitted_write_fields_fall_back_to_documented_defaults() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, _) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/roles",
            &token,
            json!({ "name": "默认范围角色", "code": "default_scope" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, roles) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/roles?keyword=默认范围角色", &token),
    )
    .await;
    assert_eq!(roles["data"]["rows"][0]["dataScope"], "仅本人数据权限");

    let (status, _) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/menus",
            &token,
            json!({ "parentId": 0, "name": "默认类型菜单" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, tree) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/menus/tree?keyword=默认类型菜单", &token),
    )
    .await;
    assert_eq!(tree["data"][0]["type"], "menu");

    let (status, _) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/configs",
            &token,
            json!({ "name": "默认类型参数", "key": "probe.noType", "value": "1" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, configs) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/configs?keyword=probe.noType", &token),
    )
    .await;
    assert_eq!(configs["data"]["rows"][0]["type"], "N");
}

/// 没有命中任何角色时不查授权菜单，返回空分页而不是报错。
#[tokio::test]
async fn role_list_without_matches_returns_an_empty_page() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/roles?keyword=不存在的角色", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 0);
    assert_eq!(body["data"]["rows"], json!([]));
}

/// 查询串里 `%` 后面直接跟多字节字符时曾按字符切片而 panic，现在按字节解码后照常给出业务响应。
#[tokio::test]
async fn raw_multibyte_query_values_are_decoded_without_panicking() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    // 非法转义保留原字符：筛选参数把解码结果当整数校验，按既有文案被拒绝，而不是让请求崩掉。
    let (status, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?status=%中", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "参数 status 必须是整数");

    // 分页参数与筛选参数同语义：非法值一律 400，不再静默回落默认值。
    let (status, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?pageNum=%中", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "参数 pageNum 必须是整数");

    // 合法转义与字面多字节混排（含 emoji）都能解码，其余参数照常工作。
    let (status, body) = send(
        app.clone(),
        get_with_bearer(
            "/api/v1/system/users?keyword=%E7%94%A8%E6%88%B7%中%F0%9F%99%82",
            &token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["code"], 200);
    assert_eq!(body["data"]["total"], 0);
}

/// 新增用户未传口令时取系统参数 `sys.user.initPassword`（种子值为 `Yy@123456`），
/// 而不是 `.env` 的 `SEED_ADMIN_PASSWORD`。这条与 `docs/ADMIN.md` 的说明、
/// 前端「初始密码取自系统参数」的提示一致。
#[tokio::test]
async fn created_user_password_follows_the_initial_password_config() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, _) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/users",
            &token,
            json!({ "username": "viaconfig", "nickname": "参数口令", "deptId": 6, "roleIds": [3] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    login(&app, store.as_ref(), "viaconfig", "Yy@123456").await;

    // 用 .env 里的播种口令登不进去，证明回退顺序不是「先 .env」。
    let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    let uuid = captcha["data"]["uuid"].as_str().expect("验证码标识");
    let (status, _) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({
                "username": "viaconfig",
                "password": DEMO_PASSWORD,
                "captcha": take_captcha(store.as_ref(), uuid),
                "captchaUuid": uuid,
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// 同一字典类型只允许一个默认项：后设置的默认会顶掉前一个，旧项降级为普通项。
#[tokio::test]
async fn dict_default_is_unique_per_type() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (_, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/dict/types",
            &token,
            json!({ "dictName": "工单状态", "dictType": "ticket_status" }),
        ),
    )
    .await;
    let _type_id = created["data"]["id"].as_i64().expect("字典类型 ID");

    let create_data = |label: &str, value: &str, def: bool| {
        let body = json!({
            "dictType": "ticket_status", "label": label, "value": value, "def": def,
        });
        let token = token.clone();
        let app = app.clone();
        async move {
            let (_, created) = send(
                app,
                post_json_with_bearer("/api/v1/system/dict/data", &token, body),
            )
            .await;
            created["data"]["id"].as_i64().expect("字典数据 ID")
        }
    };

    let pending = create_data("待处理", "0", true).await;
    let _doing = create_data("处理中", "1", true).await;
    let done = create_data("已完成", "2", false).await;

    // 新建默认项时清掉同类型旧默认项。
    let (_, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/dict/data/ticket_status", &token),
    )
    .await;
    let rows = page["data"].as_array().expect("字典数据");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["def"], false);
    assert_eq!(rows[1]["def"], true);
    assert_eq!(rows[2]["def"], false);

    // 编辑时把另一项设为默认，同样只留一个。
    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/dict/data/{done}"),
            &token,
            json!({ "def": true }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, options) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/dict/options/ticket_status", &token),
    )
    .await;
    let options = options["data"].as_array().expect("业务下拉");
    assert_eq!(options.iter().filter(|row| row["def"] == 1).count(), 1);
    assert_eq!(options[2]["def"], 1);
    assert_eq!(options[1]["def"], 0);

    // 数据库层也挡住第二个默认项：绕过接口直写会被部分唯一索引拒绝。
    let conflict = container
        .database
        .execute_unprepared(&format!(
            "UPDATE sys_dict_data SET is_default = 1 WHERE id = {pending}"
        ))
        .await;
    assert!(conflict.is_err(), "同类型第二个默认项应被唯一索引拒绝");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/dict/data/{pending}"),
            &token,
            json!({ "def": true }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, options) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/dict/options/ticket_status", &token),
    )
    .await;
    let options = options["data"].as_array().expect("业务下拉");
    assert_eq!(options[0]["def"], 1);
    assert_eq!(options.iter().filter(|row| row["def"] == 1).count(), 1);

    // 把默认项整条搬到另一个已有默认项的字典类型：按最终状态收敛，不撞唯一索引。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/dict/data/{pending}"),
            &token,
            json!({ "dictType": "sys_yes_no" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (_, options) = send(
        app,
        get_with_bearer("/api/v1/system/dict/options/sys_yes_no", &token),
    )
    .await;
    let options = options["data"].as_array().expect("业务下拉");
    assert_eq!(options.len(), 3);
    assert_eq!(options.iter().filter(|row| row["def"] == 1).count(), 1);
    assert_eq!(options[0]["value"], "0");
}
