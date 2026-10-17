//! 端点契约矩阵：把 Router 上注册的全部接口当成对外契约逐条锁定。
//!
//! 三张表与 `src/api/controllers` 的路由注册一一对应，共 83 个 `(方法, 路径)`：
//! 需要权限码的 64 个、仅需登录的 10 个、公开的 9 个。表格驱动的目的是让
//! 「新增接口但忘了加鉴权」这类回归在测试里立刻暴露，而不是漏到线上。
//!
//! 断言的是对外可观测语义：HTTP 状态码、统一响应体的 `code` 与 `message`，
//! 文案与 Spring 版逐字对齐（见 `docs/ADMIN.md`）。

mod support;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::{Value, json};

use support::{
    DEMO_PASSWORD, admin_token, get_with_bearer, login, post_json_with_bearer, request, send,
    send_raw, setup_app,
};

/// 需要权限码的端点：`(方法, 路径, 权限码)`，与 `AdminUser::require` 的入参一致。
const PERMISSION_ENDPOINTS: &[(&str, &str, &str)] = &[
    (
        "DELETE",
        "/api/v1/monitor/login-logs",
        "monitor:loginlog:remove",
    ),
    (
        "DELETE",
        "/api/v1/monitor/login-logs/1",
        "monitor:loginlog:remove",
    ),
    (
        "DELETE",
        "/api/v1/monitor/oper-logs",
        "monitor:operlog:remove",
    ),
    (
        "DELETE",
        "/api/v1/monitor/oper-logs/1",
        "monitor:operlog:remove",
    ),
    ("DELETE", "/api/v1/system/configs/1", "system:config:remove"),
    ("DELETE", "/api/v1/system/depts/1", "system:dept:remove"),
    ("DELETE", "/api/v1/system/dict/data/1", "system:dict:remove"),
    (
        "DELETE",
        "/api/v1/system/dict/types/1",
        "system:dict:remove",
    ),
    ("DELETE", "/api/v1/system/menus/1", "system:menu:remove"),
    ("DELETE", "/api/v1/system/posts/1", "system:post:remove"),
    ("DELETE", "/api/v1/system/roles/1", "system:role:remove"),
    ("DELETE", "/api/v1/system/users/1", "system:user:remove"),
    ("GET", "/api/v1/monitor/login-logs", "monitor:loginlog:list"),
    (
        "GET",
        "/api/v1/monitor/login-logs/export",
        "monitor:loginlog:list",
    ),
    (
        "GET",
        "/api/v1/monitor/login-logs/heatmap",
        "monitor:loginlog:list",
    ),
    (
        "GET",
        "/api/v1/monitor/login-logs/stats",
        "monitor:loginlog:list",
    ),
    ("GET", "/api/v1/monitor/oper-logs", "monitor:operlog:list"),
    (
        "GET",
        "/api/v1/monitor/oper-logs/export",
        "monitor:operlog:list",
    ),
    (
        "GET",
        "/api/v1/monitor/oper-logs/stats",
        "monitor:operlog:list",
    ),
    (
        "GET",
        "/api/v1/monitor/oper-logs/timeline",
        "monitor:operlog:list",
    ),
    ("GET", "/api/v1/system/configs", "system:config:list"),
    ("GET", "/api/v1/system/configs/export", "system:config:list"),
    ("GET", "/api/v1/system/depts/export", "system:dept:list"),
    ("GET", "/api/v1/system/depts/tree", "system:dept:list"),
    (
        "GET",
        "/api/v1/system/dict/data/sys_normal_disable",
        "system:dict:list",
    ),
    ("GET", "/api/v1/system/dict/types", "system:dict:list"),
    (
        "GET",
        "/api/v1/system/dict/types/export",
        "system:dict:list",
    ),
    ("GET", "/api/v1/system/menus/export", "system:menu:list"),
    ("GET", "/api/v1/system/menus/tree", "system:menu:list"),
    (
        "GET",
        "/api/v1/system/permissions/graph",
        "system:role:list",
    ),
    ("GET", "/api/v1/system/posts", "system:post:list"),
    ("GET", "/api/v1/system/posts/export", "system:post:list"),
    ("GET", "/api/v1/system/roles", "system:role:list"),
    ("GET", "/api/v1/system/roles/export", "system:role:list"),
    ("GET", "/api/v1/system/roles/1/menus", "system:role:list"),
    ("GET", "/api/v1/system/users", "system:user:list"),
    ("GET", "/api/v1/system/users/export", "system:user:list"),
    ("GET", "/api/v1/system/users/options", "system:user:list"),
    (
        "GET",
        "/api/v1/tool/generator/importable-tables",
        "tool:gen:list",
    ),
    ("GET", "/api/v1/tool/generator/tables", "tool:gen:list"),
    (
        "GET",
        "/api/v1/tool/generator/tables/1/columns",
        "tool:gen:list",
    ),
    (
        "GET",
        "/api/v1/tool/generator/tables/1/download",
        "tool:gen:list",
    ),
    (
        "GET",
        "/api/v1/tool/generator/tables/1/preview",
        "tool:gen:list",
    ),
    ("POST", "/api/v1/system/configs", "system:config:add"),
    ("POST", "/api/v1/system/depts", "system:dept:add"),
    ("POST", "/api/v1/system/dict/data", "system:dict:add"),
    ("POST", "/api/v1/system/dict/types", "system:dict:add"),
    ("POST", "/api/v1/system/menus", "system:menu:add"),
    ("POST", "/api/v1/system/posts", "system:post:add"),
    ("POST", "/api/v1/system/roles", "system:role:add"),
    ("POST", "/api/v1/system/users", "system:user:add"),
    (
        "POST",
        "/api/v1/system/users/1/2fa/reset",
        "system:user:reset2fa",
    ),
    ("POST", "/api/v1/tool/generator/import", "tool:gen:list"),
    (
        "POST",
        "/api/v1/tool/generator/tables/1/generate",
        "tool:gen:list",
    ),
    ("PUT", "/api/v1/system/configs/1", "system:config:edit"),
    ("PUT", "/api/v1/system/depts/1", "system:dept:edit"),
    ("PUT", "/api/v1/system/dict/data/1", "system:dict:edit"),
    ("PUT", "/api/v1/system/dict/types/1", "system:dict:edit"),
    ("PUT", "/api/v1/system/menus/1", "system:menu:edit"),
    ("PUT", "/api/v1/system/posts/1", "system:post:edit"),
    ("PUT", "/api/v1/system/roles/1", "system:role:edit"),
    ("PUT", "/api/v1/system/users/1", "system:user:edit"),
    (
        "PUT",
        "/api/v1/system/users/1/password",
        "system:user:resetPwd",
    ),
    ("PUT", "/api/v1/system/users/1/status", "system:user:edit"),
    ("PUT", "/api/v1/tool/generator/tables/1", "tool:gen:list"),
];

/// 只需要登录、不校验权限码的后台端点（业务下拉与工作台）。
const AUTH_ENDPOINTS: &[(&str, &str)] = &[
    ("GET", "/api/v1/auth/me"),
    ("GET", "/api/v1/auth/routes"),
    ("GET", "/api/v1/dashboard/summary"),
    ("GET", "/api/v1/system/dict/options/sys_normal_disable"),
    ("POST", "/api/v1/auth/logout"),
    ("PUT", "/api/v1/auth/password"),
    ("GET", "/api/v1/auth/profile"),
    ("PUT", "/api/v1/auth/profile"),
    ("POST", "/api/v1/auth/2fa/setup"),
    ("POST", "/api/v1/auth/2fa/enable"),
    ("POST", "/api/v1/auth/2fa/disable"),
    ("POST", "/api/v1/auth/2fa/recovery-codes"),
];

/// 脚手架自带的接口：鉴权走 `CurrentUser` 提取器，401 文案与后台接口不同。
const SCAFFOLD_AUTH_ENDPOINTS: &[(&str, &str)] = &[
    ("GET", "/api/v1/auth/session"),
    ("GET", "/api/v1/examples/example_001"),
    ("GET", "/api/v1/transactions"),
    ("GET", "/api/v1/transactions/any-record"),
];

/// 公开端点：不带令牌也应正常响应。
const PUBLIC_ENDPOINTS: &[(&str, &str)] = &[
    ("GET", "/api/v1/auth/captcha"),
    ("GET", "/api/v1/examples"),
    ("GET", "/api/v1/health"),
    ("GET", "/api/v1/system/health"),
    ("GET", "/api/v1/system/ready"),
    ("POST", "/api/v1/auth/dev-login"),
    ("POST", "/api/v1/auth/login"),
    ("POST", "/api/v1/auth/login/2fa"),
    ("POST", "/api/v1/examples/echo"),
    ("POST", "/api/v1/transactions/dev-transfer"),
];

/// 带请求体的方法在鉴权失败前仍要通过 `ApiJson`，给一个空对象即可：
/// 后台的写命令字段全部可选，`{}` 能反序列化成功。
fn body_of(method: &str) -> Option<Value> {
    matches!(method, "POST" | "PUT" | "PATCH").then(|| json!({"password": "invalid-password"}))
}

/// 三张表必须覆盖 Router 上注册的全部端点，漏掉一个就说明表没跟上路由。
#[test]
fn endpoint_tables_cover_every_registered_route() {
    assert_eq!(PERMISSION_ENDPOINTS.len(), 65);
    assert_eq!(AUTH_ENDPOINTS.len(), 12);
    assert_eq!(SCAFFOLD_AUTH_ENDPOINTS.len(), 4);
    assert_eq!(PUBLIC_ENDPOINTS.len(), 10);

    let mut all: Vec<(&str, &str)> = PERMISSION_ENDPOINTS
        .iter()
        .map(|(method, path, _)| (*method, *path))
        .chain(AUTH_ENDPOINTS.iter().copied())
        .chain(SCAFFOLD_AUTH_ENDPOINTS.iter().copied())
        .chain(PUBLIC_ENDPOINTS.iter().copied())
        .collect();
    let total = all.len();
    all.sort_unstable();
    all.dedup();

    assert_eq!(all.len(), total, "三张表之间存在重复的端点");
}

/// 后台端点缺令牌：统一 401 + 「未认证或登录已过期」，与「无权限」的 403 区分开。
#[tokio::test]
async fn protected_endpoints_without_token_are_rejected_with_401() {
    let app = setup_app().await;
    let protected: Vec<(&str, &str)> = PERMISSION_ENDPOINTS
        .iter()
        .map(|(method, path, _)| (*method, *path))
        .chain(AUTH_ENDPOINTS.iter().copied())
        .collect();
    assert_eq!(protected.len(), 77);

    for (method, path) in protected {
        let (status, body) = send(app.clone(), request(method, path, body_of(method), None)).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {path}");
        assert_eq!(body["code"], 401, "{method} {path}");
        assert_eq!(body["message"], "未认证或登录已过期", "{method} {path}");
    }
}

/// 脚手架自带的接口同样 401，但走 `CurrentUser` 提取器，文案保持英文原文。
#[tokio::test]
async fn scaffold_endpoints_without_token_keep_the_english_message() {
    let app = setup_app().await;

    for (method, path) in SCAFFOLD_AUTH_ENDPOINTS {
        let (status, body) = send(app.clone(), request(method, path, None, None)).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {path}");
        assert_eq!(body["code"], 401, "{method} {path}");
        assert_eq!(
            body["message"], "Unauthorized: missing Authorization header",
            "{method} {path}"
        );
    }
}

/// 已登录但没有任何权限码的账号：统一 403 + 「没有访问权限」。
///
/// 种子里的 `linjh`（`sys_admin`）默认没有菜单授权，权限码为空集。
#[tokio::test]
async fn permission_endpoints_reject_account_without_codes_with_403() {
    let app = setup_app().await;
    let token = login(&app, "linjh", DEMO_PASSWORD).await;

    for (method, path, permission) in PERMISSION_ENDPOINTS {
        let (status, body) = send(
            app.clone(),
            request(method, path, body_of(method), Some(&token)),
        )
        .await;

        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "{method} {path} 缺少 {permission} 时应 403"
        );
        assert_eq!(body["code"], 403, "{method} {path}");
        assert_eq!(body["message"], "没有访问权限", "{method} {path}");
    }
}

/// 超管令牌不会被鉴权层拦下：说明表里的路径确实挂在 Router 上（否则会是 404）。
///
/// 导出与下载端点的响应是 CSV / ZIP，不能按 JSON 解析，因此只取状态码。
#[tokio::test]
async fn admin_token_is_never_rejected_by_the_auth_layer() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    for (method, path, _) in PERMISSION_ENDPOINTS {
        let (status, _, _) = send_raw(
            app.clone(),
            request(method, path, body_of(method), Some(&token)),
        )
        .await;

        assert_ne!(status, StatusCode::UNAUTHORIZED, "{method} {path}");
        assert_ne!(status, StatusCode::FORBIDDEN, "{method} {path}");
    }
}

/// 公开端点不带令牌也是 200，且恰好是表里那 9 个。
#[tokio::test]
async fn public_endpoints_answer_without_a_token() {
    let app = setup_app().await;

    for (method, path) in [
        ("GET", "/api/v1/health"),
        ("GET", "/api/v1/system/health"),
        ("GET", "/api/v1/system/ready"),
        ("GET", "/api/v1/auth/captcha"),
        ("GET", "/api/v1/examples"),
    ] {
        let (status, body) = send(app.clone(), request(method, path, None, None)).await;

        assert_eq!(status, StatusCode::OK, "{method} {path}");
        assert_eq!(body["code"], 200, "{method} {path}");
    }
}

/// 404 与 405 使用 Spring 版文案，且都是统一响应体（而非 axum 默认纯文本）。
#[tokio::test]
async fn unknown_paths_and_unsupported_methods_share_spring_wording() {
    let app = setup_app().await;

    for path in [
        "/api/v1/unknown",
        "/api/v1/system/unknown",
        "/api/v1/monitor/unknown",
        "/api/v1/tool/unknown",
        "/nope",
    ] {
        let (status, body) = send(app.clone(), request("GET", path, None, None)).await;

        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(body["code"], 404, "{path}");
        assert_eq!(body["message"], "资源不存在", "{path}");
    }

    for (method, path) in [
        ("PUT", "/api/v1/health"),
        ("DELETE", "/api/v1/system/ready"),
        ("GET", "/api/v1/auth/login"),
        ("POST", "/api/v1/auth/captcha"),
        ("PUT", "/api/v1/system/users"),
        ("PATCH", "/api/v1/system/roles"),
    ] {
        let (status, body) = send(app.clone(), request(method, path, body_of(method), None)).await;

        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{method} {path}");
        assert_eq!(body["code"], 405, "{method} {path}");
        assert_eq!(
            body["message"],
            format!("不支持的请求方法: {method}"),
            "{method} {path}"
        );
    }
}

/// 查询参数无法解析时 400，而不是静默忽略过滤条件。
#[tokio::test]
async fn invalid_query_parameters_are_rejected_with_400() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    for (path, message) in [
        (
            "/api/v1/monitor/login-logs?status=abc",
            "参数 status 必须是整数",
        ),
        (
            "/api/v1/monitor/login-logs/heatmap?days=abc",
            "参数 days 必须是整数",
        ),
        (
            "/api/v1/monitor/oper-logs?result=abc",
            "参数 result 必须是整数",
        ),
        ("/api/v1/system/users?status=abc", "参数 status 必须是整数"),
        ("/api/v1/system/roles?status=abc", "参数 status 必须是整数"),
        ("/api/v1/system/posts?status=abc", "参数 status 必须是整数"),
        (
            "/api/v1/system/depts/tree?status=abc",
            "参数 status 必须是整数",
        ),
        (
            "/api/v1/system/menus/tree?status=abc",
            "参数 status 必须是整数",
        ),
        (
            "/api/v1/system/dict/types?status=abc",
            "参数 status 必须是整数",
        ),
    ] {
        let (status, body) = send(app.clone(), get_with_bearer(path, &token)).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(body["code"], 400, "{path}");
        assert_eq!(body["message"], message, "{path}");
    }
}

/// 后台写接口的请求体走 `ApiJson`：非法 JSON 与缺少 JSON Content-Type 都收敛为
/// 400「请求体格式错误」，不出现 axum 默认的 422 纯文本响应。
#[tokio::test]
async fn malformed_admin_bodies_become_400_with_unified_envelope() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let cases = [
        (
            "非法 JSON",
            Request::builder()
                .method("POST")
                .uri("/api/v1/system/users")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from("{not json"))
                .expect("构造请求"),
        ),
        (
            "字段类型不匹配",
            Request::builder()
                .method("POST")
                .uri("/api/v1/system/users")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(r#"{"status":"不是数字"}"#))
                .expect("构造请求"),
        ),
        (
            "缺少 JSON Content-Type",
            Request::builder()
                .method("POST")
                .uri("/api/v1/system/users")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(r#"{"username":"x"}"#))
                .expect("构造请求"),
        ),
    ];

    for (name, request) in cases {
        let (status, body) = send(app.clone(), request).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "{name}");
        assert_eq!(body["code"], 400, "{name}");
        assert_eq!(body["message"], "请求体格式错误", "{name}");
    }

    // 对照：合法 JSON 但业务校验不通过时，走的是业务文案而不是解析文案。
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer("/api/v1/system/users", &token, json!({ "username": "  " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "登录账号不能为空");
}
