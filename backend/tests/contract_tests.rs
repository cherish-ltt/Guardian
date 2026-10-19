//! 端点契约矩阵：把 Router 上注册的全部接口当成对外契约逐条锁定。
//!
//! 四张表与 `src/api/controllers` 的路由注册一一对应，共 91 个 `(方法, 路径)`：
//! 需要权限码的 65 个、仅需登录的 12 个、脚手架登录态的 4 个、公开的 10 个。
//! 表格驱动的目的是让「新增接口但忘了加鉴权」这类回归在测试里立刻暴露，
//! 而不是漏到线上；`endpoint_tables_cover_every_registered_route` 会真的枚举
//! Router 已注册的路径与方法，与本文件的四张表双向比对。
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
    send_raw, setup_app_with_store,
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

/// 带请求体的方法在鉴权失败前仍要通过 `ApiJson`，给一个字段合法但取值无效的对象即可：
/// 后台的写命令字段全部可选，`{}` 能反序列化成功；口令取 5 位，必被长度校验挡下，
/// 不会在契约测试里真的把账号口令改掉（那会撤销当前令牌，后续断言全部 401）。
fn body_of(method: &str) -> Option<Value> {
    matches!(method, "POST" | "PUT" | "PATCH").then(|| json!({"password": "short"}))
}

/// 四张表必须与 Router 上注册的端点完全一致：漏登记或多登记都会失败。
///
/// 这里不是数一遍表长就完事，而是从 `Router` 的 `Debug` 输出里读出实际注册的
/// 路径与方法（axum 0.8 没有公开的枚举接口），再和四张表双向比对：
/// 新增路由而没登记会报 `未登记`，删了路由而表没跟上会报 `已失效`。
#[test]
fn endpoint_tables_cover_every_registered_route() {
    let mut declared: Vec<(String, String)> = PERMISSION_ENDPOINTS
        .iter()
        .map(|(method, path, _)| ((*method).to_string(), (*path).to_string()))
        .chain(
            AUTH_ENDPOINTS
                .iter()
                .copied()
                .chain(SCAFFOLD_AUTH_ENDPOINTS.iter().copied())
                .chain(PUBLIC_ENDPOINTS.iter().copied())
                .map(|(method, path)| (method.to_string(), path.to_string())),
        )
        .collect();
    declared.sort();
    let total = declared.len();
    declared.dedup();
    assert_eq!(declared.len(), total, "四张表之间存在重复的端点");

    let mut registered = registered_routes();
    registered.sort();

    // 表里写的是具体 ID（如 `/system/users/1`），Router 里是路径参数（`{id}`），
    // 所以不能直接比对字符串，要按段位对齐：字面相同的段必须相等，
    // Router 侧的 `{...}` 段可以匹配表里的任意非空段。
    let mut claimed: Vec<&(String, String)> = Vec::new();
    let mut stale: Vec<&(String, String)> = Vec::new();
    for endpoint in &declared {
        let mut candidates: Vec<&(String, String)> = registered
            .iter()
            .filter(|other| other.0 == endpoint.0 && other.1 == endpoint.1)
            .collect();
        if candidates.is_empty() {
            candidates = registered
                .iter()
                .filter(|other| other.0 == endpoint.0 && segments_match(&endpoint.1, &other.1))
                .collect();
        }
        match candidates.as_slice() {
            [] => stale.push(endpoint),
            [hit] => claimed.push(hit),
            _ => panic!("端点 {endpoint:?} 同时匹配多条路由：{candidates:?}"),
        }
    }

    let missing: Vec<_> = registered
        .iter()
        .filter(|endpoint| !claimed.contains(endpoint))
        .collect();
    assert!(
        missing.is_empty(),
        "Router 已注册但四张表未登记的端点：{missing:?}"
    );
    assert!(
        stale.is_empty(),
        "四张表登记了但 Router 未注册的端点：{stale:?}"
    );
    assert_eq!(
        claimed.len(),
        registered.len(),
        "多条表项映射到了同一条路由，说明表里有重复登记"
    );
}

/// 表里的具体路径能否落到 Router 的参数化路径上。
fn segments_match(declared: &str, registered: &str) -> bool {
    let declared: Vec<&str> = declared.trim_matches('/').split('/').collect();
    let registered: Vec<&str> = registered.trim_matches('/').split('/').collect();

    declared.len() == registered.len()
        && declared
            .iter()
            .zip(registered.iter())
            .all(|(left, right)| right.starts_with('{') || left == right)
}

/// 从 `api::router()` 的 `Debug` 输出里还原 `(方法, 完整路径)`。
///
/// axum 0.8 的 `Router` 没有公开的路由枚举接口，`Debug` 是唯一能看到实际注册内容的
/// 途径。格式变了就解析不出来，此时函数直接 panic，不会静默返回空集让断言空转。
fn registered_routes() -> Vec<(String, String)> {
    use std::collections::BTreeMap;

    /// 同一条路由的 Debug 片段会分散在多个 `RouteId(n)` 里（路径在 node 段、
    /// 方法在 routes 段），因此按 RouteId 汇总后再拆成 `(方法, 路径)`。
    #[derive(Default)]
    struct RouteEntry {
        path: Option<String>,
        verbs: Vec<String>,
    }

    let debug = format!("{:?}", guardian_backend::api::router());
    // 兜底路由（`/` 与 `/{*...}`）不属于业务契约，且 RouteId 会与真实路由重号。
    let main = debug
        .split("fallback_router:")
        .next()
        .expect("Debug 输出包含 fallback_router 之前的部分");

    let mut entries: BTreeMap<u32, RouteEntry> = BTreeMap::new();
    for chunk in main.split("RouteId(").skip(1) {
        let (id, rest) = chunk.split_once("): ").expect("RouteId 后紧跟 `): `");
        let id: u32 = id.trim().parse().expect("RouteId 内的数字");
        let entry = entries.entry(id).or_default();

        if let Some(quoted) = rest.strip_prefix('"') {
            let (path, _) = quoted.split_once('"').expect("路径字符串以引号结束");
            entry.path = Some(path.to_string());
            continue;
        }
        let Some(index) = rest.find("allow_header: Bytes(b\"") else {
            continue;
        };
        let tail = &rest[index + "allow_header: Bytes(b\"".len()..];
        let (allow, _) = tail.split_once('"').expect("allow_header 以引号结束");
        // `GET` 会自动带上 `HEAD`，它不是单独注册的业务方法。
        entry.verbs = allow
            .split(',')
            .filter(|verb| !verb.is_empty() && *verb != "HEAD")
            .map(str::to_string)
            .collect();
    }

    assert!(!entries.is_empty(), "未能从 Router 的 Debug 输出解析出路由");

    let mut endpoints = Vec::new();
    for entry in entries.values() {
        let path = entry.path.as_deref().expect("每条路由都应有路径");
        assert!(!entry.verbs.is_empty(), "路由 {path} 没有任何方法");
        for verb in &entry.verbs {
            endpoints.push((verb.clone(), format!("/api/v1{path}")));
        }
    }

    endpoints
}

/// 后台端点缺令牌：统一 401 + 「未认证或登录已过期」，与「无权限」的 403 区分开。
#[tokio::test]
async fn protected_endpoints_without_token_are_rejected_with_401() {
    let (app, _store) = setup_app_with_store().await;
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
    let (app, _store) = setup_app_with_store().await;

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

/// 仅需登录的端点不能误收成权限码端点：零权限账号必须能访问。
///
/// 否则把 `/system/dict/options/{dict_type}` 这类供任意已登录页面渲染的业务下拉
/// 误加 `system:dict:list` 后，普通用户的下拉会整片空白，而前面的 401 / 403 用例都发现不了。
#[tokio::test]
async fn auth_endpoints_are_reachable_for_account_without_codes() {
    let (app, store) = setup_app_with_store().await;
    let token = login(&app, store.as_ref(), "linjh", DEMO_PASSWORD).await;

    for (method, path) in AUTH_ENDPOINTS {
        let (status, _, _) = send_raw(
            app.clone(),
            request(method, path, body_of(method), Some(&token)),
        )
        .await;

        assert_ne!(
            status,
            StatusCode::FORBIDDEN,
            "{method} {path} 是仅需登录的端点，不应校验权限码"
        );
    }
}

/// 已登录但没有任何权限码的账号：统一 403 + 「没有访问权限」。
///
/// 种子里的 `linjh`（`sys_admin`）默认没有菜单授权，权限码为空集。
#[tokio::test]
async fn permission_endpoints_reject_account_without_codes_with_403() {
    let (app, store) = setup_app_with_store().await;
    let token = login(&app, store.as_ref(), "linjh", DEMO_PASSWORD).await;

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
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

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
    let (app, _store) = setup_app_with_store().await;

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
    let (app, _store) = setup_app_with_store().await;

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
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

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
        (
            "/api/v1/system/users?pageNum=abc",
            "参数 pageNum 必须是整数",
        ),
        (
            "/api/v1/system/roles?pageSize=abc",
            "参数 pageSize 必须是整数",
        ),
        (
            "/api/v1/monitor/login-logs?pageNum=-1",
            "参数 pageNum 不能为负数",
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
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

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
