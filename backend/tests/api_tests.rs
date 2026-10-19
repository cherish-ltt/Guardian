use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode, header};
use axum::response::Response;
use chrono::{Duration, Utc};
use guardian_backend::container::Container;
use guardian_backend::create_app::create_app;
use guardian_backend::infrastructure::config::{
    AppConfig, DatabaseConfig, JwtConfig, LoggingConfig, MiddlewareConfig, RedisConfig, SeedConfig,
    ServerConfig,
};
use http_body_util::BodyExt;
use jsonwebtoken::{Algorithm, EncodingKey, Header as JwtHeader, encode};
use serde_json::{Value, json};
use tower::ServiceExt;
use tracing_appender::rolling::Rotation;
use uuid::Uuid;

const TEST_JWT_SECRET: &str = "api-test-secret-that-is-long-enough";

/// 每个测试独立装配一套容器与临时 SQLite 数据库，互不干扰。
async fn setup_app() -> Router {
    setup_app_with_cors_origins(vec!["https://example.com".to_string()]).await
}

/// 与 `setup_app` 相同，但可指定 CORS 来源白名单。
async fn setup_app_with_cors_origins(cors_allowed_origins: Vec<String>) -> Router {
    let database_url = format!(
        "sqlite://{}?mode=rwc",
        std::env::temp_dir()
            .join(format!("guardian-api-test-{}.db", Uuid::now_v7()))
            .display()
    );

    let config = AppConfig {
        app_name: "guardian-backend".to_string(),
        app_env: "development".to_string(),
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
        },
        database: DatabaseConfig {
            url: database_url,
            min_connections: 1,
            max_connections: 2,
            connect_timeout_secs: 5,
            idle_secs: 30,
            sqlx_logging: false,
        },
        jwt: JwtConfig {
            secret: TEST_JWT_SECRET.to_string(),
            issuer: "test-issuer".to_string(),
            audience: "test-audience".to_string(),
            access_token_ttl_minutes: 120,
        },
        redis: RedisConfig {
            url: "redis://127.0.0.1:6379".to_string(),
            password: None,
        },
        seed: SeedConfig {
            admin_password: "admin123456".to_string(),
            ..SeedConfig::default()
        },
        logging: LoggingConfig {
            filter: "info".to_string(),
            utc_offset_hour: 8,
            utc_offset_minute: 0,
            utc_offset_second: 0,
            filename_prefix: "api-test".to_string(),
            filename_suffix: "log".to_string(),
            rotation: Rotation::NEVER,
            max_log_files: 1,
            out_dir: std::env::temp_dir().to_string_lossy().to_string(),
            batch_max_events: 50,
            batch_flush_interval_secs: 3,
        },
        middleware: MiddlewareConfig {
            request_timeout_secs: 10,
            max_body_bytes: 2 * 1024 * 1024,
            max_concurrency: 256,
            backpressure_queue: 256,
            rate_limit_requests: 1000,
            rate_limit_period_secs: 1,
            hsts_enabled: false,
            cors_allowed_origins,
            trusted_proxies: Vec::new(),
            client_max_concurrency: 32,
            client_rate_limit_requests: 1024,
            client_rate_limit_period_secs: 1,
            client_max_entries: 16384,
            client_idle_secs: 60,
            slow_request_ms: 1000,
        },
        ip_location_db_path: None,
        ip_location_v6_db_path: None,
        cache: Default::default(),
        two_factor_key: Some("BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=".into()),
    };

    // 会话用内存实现：集成测试不依赖外部 Redis。
    let container = Container::bootstrap_with_auth_store(config, Container::memory_auth_store())
        .await
        .expect("容器装配成功");
    create_app(Arc::new(container))
}

async fn send_response(app: Router, mut request: Request<Body>) -> Response {
    // 请求保护层按真实 TCP 对端分配额度：`oneshot` 不会自动注入，这里显式补一个本地对端。
    request.extensions_mut().insert(axum::extract::ConnectInfo(
        "127.0.0.1:12345"
            .parse::<std::net::SocketAddr>()
            .expect("测试对端地址应合法"),
    ));
    app.oneshot(request).await.expect("请求应成功返回")
}

async fn read_json(response: Response) -> Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("响应体可读取")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("响应体应为合法 JSON")
}

async fn send(app: Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = send_response(app, request).await;
    let status = response.status();
    (status, read_json(response).await)
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("构造 GET 请求")
}

fn get_with_bearer(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .expect("构造带令牌的 GET 请求")
}

fn post_json(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("构造 POST 请求")
}

// 仅被 debug 构建注册的 dev-transfer 用例使用，release 下需要一并裁剪。
#[cfg(debug_assertions)]
fn post_json_with_bearer(uri: &str, token: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .expect("构造带令牌的 POST 请求")
}

/// 通过调试登录接口获取访问令牌（dev-login 路由仅在调试构建注册）。
#[cfg(debug_assertions)]
async fn issue_dev_token(app: &Router) -> String {
    let (status, body) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/dev-login",
            json!({ "username": "demo-admin" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    body["data"]["access_token"]
        .as_str()
        .expect("应返回访问令牌")
        .to_string()
}

#[tokio::test]
async fn root_returns_service_info() {
    let app = setup_app().await;
    let (status, body) = send(app, get("/")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["code"], 200);
    assert_eq!(body["data"]["service_name"], "guardian-backend");
    assert_eq!(body["data"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(body["data"]["docs_enabled"], cfg!(feature = "docs"));
}

#[tokio::test]
async fn health_reports_ok() {
    let app = setup_app().await;
    let (status, body) = send(app, get("/api/v1/system/health")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["code"], 200);
    assert_eq!(body["message"], "Health check completed");
    assert_eq!(body["data"]["status"], "ok");
    assert_eq!(body["data"]["database_status"], "not_checked");
}

#[tokio::test]
async fn ready_confirms_database() {
    let app = setup_app().await;
    let (status, body) = send(app, get("/api/v1/system/ready")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Service ready");
    assert!(body.get("data").is_none(), "无数据时不应输出 data 字段");
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn dev_login_issues_bearer_token() {
    let app = setup_app().await;
    let (status, body) = send(
        app,
        post_json(
            "/api/v1/auth/dev-login",
            json!({ "username": "demo-admin" }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Debug token issued");
    assert_eq!(body["data"]["token_type"], "Bearer");
    assert_eq!(body["data"]["expires_in_seconds"], 120 * 60);
    assert!(
        !body["data"]["access_token"]
            .as_str()
            .unwrap_or("")
            .is_empty()
    );
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn dev_login_rejects_blank_username() {
    let app = setup_app().await;
    let (status, body) = send(
        app,
        post_json("/api/v1/auth/dev-login", json!({ "username": "   " })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], 400);
    assert!(body["message"].as_str().unwrap_or("").contains("username"));
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn dev_login_applies_default_role_and_generated_user_id() {
    let app = setup_app().await;
    let token = issue_dev_token(&app).await;
    let (status, body) = send(app, get_with_bearer("/api/v1/auth/session", &token)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["username"], "demo-admin");
    assert_eq!(body["data"]["roles"], json!(["developer"]));
    assert!(!body["data"]["user_id"].as_str().unwrap_or("").is_empty());
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn dev_login_keeps_explicit_user_id_and_roles() {
    let app = setup_app().await;
    let (status, body) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/dev-login",
            json!({ "username": "ops", "user_id": "fixed-id", "roles": ["admin"] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = body["data"]["access_token"]
        .as_str()
        .expect("应返回访问令牌");

    let (status, body) = send(app, get_with_bearer("/api/v1/auth/session", token)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["user_id"], "fixed-id");
    assert_eq!(body["data"]["roles"], json!(["admin"]));
}

#[tokio::test]
async fn me_requires_authorization_header() {
    let app = setup_app().await;
    let (status, body) = send(app, get("/api/v1/auth/session")).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(
        body["message"]
            .as_str()
            .unwrap_or("")
            .contains("Authorization")
    );
}

#[tokio::test]
async fn me_rejects_non_bearer_scheme() {
    let app = setup_app().await;
    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/session")
        .header(header::AUTHORIZATION, "Basic dXNlcjpwYXNz")
        .body(Body::empty())
        .expect("构造请求");
    let (status, _) = send(app, request).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn me_rejects_invalid_token() {
    let app = setup_app().await;
    let (status, _) = send(app, get_with_bearer("/api/v1/auth/session", "not-a-jwt")).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn echo_creates_example() {
    let app = setup_app().await;
    let (status, body) = send(
        app,
        post_json(
            "/api/v1/examples/echo",
            json!({ "title": "Set up a new service", "note": "Start with logging and JWT" }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["title"], "Set up a new service");
    assert_eq!(body["data"]["note"], "Start with logging and JWT");
    assert_eq!(body["data"]["source"], "example-service");
    assert!(!body["data"]["id"].as_str().unwrap_or("").is_empty());
}

#[tokio::test]
async fn echo_accepts_missing_note() {
    let app = setup_app().await;
    let (status, body) = send(
        app,
        post_json("/api/v1/examples/echo", json!({ "title": "x" })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["data"]["note"].is_null());
}

#[tokio::test]
async fn echo_rejects_blank_title() {
    let app = setup_app().await;
    let (status, body) = send(
        app,
        post_json("/api/v1/examples/echo", json!({ "title": "   " })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["message"].as_str().unwrap_or("").contains("title"));
}

#[tokio::test]
async fn list_examples_applies_filters() {
    let app = setup_app().await;
    let (status, body) = send(app, get("/api/v1/examples?page=2&size=3&keyword=Swagger")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["page"], 2);
    assert_eq!(body["data"]["size"], 3);
    assert_eq!(body["data"]["keyword"], "Swagger");
    assert_eq!(body["data"]["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(body["data"]["items"][0]["id"], "example_003");
}

#[tokio::test]
async fn list_examples_uses_defaults() {
    let app = setup_app().await;
    let (status, body) = send(app, get("/api/v1/examples")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["page"], 1);
    assert_eq!(body["data"]["size"], 10);
    assert_eq!(body["data"]["items"].as_array().map(Vec::len), Some(3));
}

#[tokio::test]
async fn list_examples_rejects_out_of_range_paging() {
    let app = setup_app().await;

    for query in ["page=0", "page=10001", "size=0", "size=101"] {
        let (status, _) = send(app.clone(), get(&format!("/api/v1/examples?{query}"))).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}");
    }

    // 边界值仍然可用。
    let (status, _) = send(app, get("/api/v1/examples?page=10000&size=100")).await;
    assert_eq!(status, StatusCode::OK);
}

/// release 构建不注册示例路由：确认这些接口不会在正式包里对外暴露。
#[cfg(not(debug_assertions))]
#[tokio::test]
async fn example_routes_are_absent_in_release_builds() {
    let app = setup_app().await;

    for (method, path) in [
        ("GET", "/api/v1/examples"),
        ("GET", "/api/v1/examples/example_001"),
        ("POST", "/api/v1/examples/echo"),
    ] {
        let (status, _) = send(app.clone(), request(method, path, None, None)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {path}");
    }
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn example_detail_requires_bearer_token() {
    let app = setup_app().await;
    let (status, _) = send(app, get("/api/v1/examples/example_001")).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn example_detail_returns_requester_info() {
    let app = setup_app().await;
    let token = issue_dev_token(&app).await;
    let (status, body) = send(app, get_with_bearer("/api/v1/examples/example_001", &token)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], "example_001");
    assert_eq!(body["data"]["requested_by"], "demo-admin");
    assert_eq!(body["data"]["roles"], json!(["developer"]));
}

#[cfg(feature = "docs")]
#[tokio::test]
async fn swagger_ui_is_mounted_when_docs_feature_is_on() {
    let app = setup_app().await;
    let status = send_response(app, get("/swagger-ui")).await.status();

    // Swagger UI 挂载在 /swagger-ui，未带斜杠访问时会 303 重定向到 /swagger-ui/。
    assert!(
        status == StatusCode::OK || status == StatusCode::SEE_OTHER,
        "swagger-ui 应可访问，实际状态码: {status}"
    );
}

#[cfg(feature = "docs")]
#[tokio::test]
async fn openapi_document_is_served() {
    let app = setup_app().await;
    let (status, body) = send(app, get("/api-doc/openapi.json")).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["openapi"].as_str().is_some());
    assert!(
        body["paths"]
            .as_object()
            .map(|paths| !paths.is_empty())
            .unwrap_or(false)
    );

    // 事务示例的接口应当出现在 OpenAPI 文档里；dev-transfer 只在调试构建注册，随构建类型裁剪。
    let paths = body["paths"].as_object().expect("paths 对象");
    #[cfg(debug_assertions)]
    let expected = [
        "/api/v1/transactions",
        "/api/v1/transactions/{id}",
        "/api/v1/transactions/dev-transfer",
    ];
    #[cfg(not(debug_assertions))]
    let expected = ["/api/v1/transactions", "/api/v1/transactions/{id}"];
    for path in expected {
        assert!(paths.contains_key(path), "OpenAPI 缺少路径: {path}");
    }
    let tags = body["tags"].as_array().expect("tags 数组");
    assert!(
        tags.iter()
            .any(|tag| tag["name"] == "Transaction" && tag.get("description").is_some()),
        "OpenAPI 缺少 Transaction 标签"
    );
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let app = setup_app().await;
    let status = send_response(app, get("/api/v1/unknown")).await.status();

    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// dev-transfer 路由仅在调试构建注册（见 transaction_controller.rs）。
#[cfg(debug_assertions)]
const DEV_TRANSFER: &str = "/api/v1/transactions/dev-transfer";

#[cfg(debug_assertions)]
fn transfer_body(amount_cents: i64) -> Value {
    json!({
        "from_account_id": "acc_alice",
        "to_account_id": "acc_bob",
        "amount_cents": amount_cents,
        "remark": "集成测试转账",
    })
}

#[cfg(debug_assertions)]
fn transfer_body_with_request_id(amount_cents: i64, request_id: &str) -> Value {
    let mut payload = transfer_body(amount_cents);
    payload["request_id"] = json!(request_id);
    payload
}

/// 同一事务写入的余额、流水与审计日志，提交后应当全部可见。
#[cfg(debug_assertions)]
#[tokio::test]
async fn transfer_commits_balances_record_and_audits() {
    let app = setup_app().await;
    let token = issue_dev_token(&app).await;

    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            DEV_TRANSFER,
            &token,
            transfer_body_with_request_id(25000, "req-http-0001"),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Transfer transaction committed");
    assert_eq!(body["data"]["amount_cents"], 25000);
    assert_eq!(body["data"]["from_balance_after_cents"], 75000);
    assert_eq!(body["data"]["to_balance_after_cents"], 125000);
    assert_eq!(body["data"]["committed"], true);
    assert_eq!(body["data"]["rolled_back"], false);

    assert_eq!(body["data"]["request_id"], "req-http-0001");

    let record_id = body["data"]["record_id"].as_str().expect("返回流水 ID");
    let (status, detail) = send(
        app.clone(),
        get_with_bearer(&format!("/api/v1/transactions/{record_id}"), &token),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["data"]["record"]["amount_cents"], 25000);
    let audits = detail["data"]["audits"].as_array().expect("审计日志数组");
    assert_eq!(audits.len(), 2);
    assert_eq!(audits[0]["action"], "BALANCE_UPDATED");
    assert_eq!(audits[1]["action"], "RECORD_CREATED");
}

/// 跨表写入过程中命中幂等键唯一约束：整体回滚并返回 409。
#[cfg(debug_assertions)]
#[tokio::test]
async fn duplicate_request_id_returns_conflict_without_extra_writes() {
    let app = setup_app().await;
    let token = issue_dev_token(&app).await;

    let (status, _) = send(
        app.clone(),
        post_json_with_bearer(
            DEV_TRANSFER,
            &token,
            transfer_body_with_request_id(20000, "req-dup-http"),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            DEV_TRANSFER,
            &token,
            transfer_body_with_request_id(5000, "req-dup-http"),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let message = body["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("Conflict"),
        "409 应透出可读原因，实际: {body}"
    );
    assert!(
        !message.contains("UNIQUE") && !message.contains("transfer_records"),
        "409 不得透出表名/列名等数据库内部结构，实际: {message}"
    );

    // 第二次的余额更新与流水写入都被回滚，库里只剩首次结果。
    let (status, list) = send(app.clone(), get_with_bearer("/api/v1/transactions", &token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list["data"]["total"], 1);
    assert_eq!(list["data"]["items"][0]["amount_cents"], 20000);

    let (status, second) = send(
        app,
        post_json_with_bearer(DEV_TRANSFER, &token, transfer_body(1000)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(second["data"]["from_balance_after_cents"], 79000);
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn transfer_requires_bearer_token() {
    let app = setup_app().await;

    let (status, _) = send(app, post_json(DEV_TRANSFER, transfer_body(1000))).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn transfer_rejects_insufficient_balance_without_writing() {
    let app = setup_app().await;
    let token = issue_dev_token(&app).await;

    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(DEV_TRANSFER, &token, transfer_body(100001)),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.get("data").is_none());

    let (status, list) = send(app, get_with_bearer("/api/v1/transactions", &token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list["data"]["total"], 0);
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn transfer_rejects_unknown_account_with_404() {
    let app = setup_app().await;
    let token = issue_dev_token(&app).await;

    let mut payload = transfer_body(1000);
    payload["to_account_id"] = json!("acc_missing");
    let (status, _) = send(app, post_json_with_bearer(DEV_TRANSFER, &token, payload)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// 调试开关：事务内写入全部完成后强制失败，回滚必须让数据库回到原状。
#[cfg(debug_assertions)]
#[tokio::test]
async fn transfer_rolls_back_every_write_when_force_fail_is_on() {
    let app = setup_app().await;
    let token = issue_dev_token(&app).await;

    let mut payload = transfer_body(30000);
    payload["force_fail"] = json!(true);
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(DEV_TRANSFER, &token, payload),
    )
    .await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["message"], "系统内部错误");

    // 回滚后流水分页应为空，说明已写入的账户、流水、审计都被撤销。
    let (status, list) = send(app.clone(), get_with_bearer("/api/v1/transactions", &token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list["data"]["total"], 0);

    // 用一次正常转账反证余额仍为初始值：100000 - 20000 = 80000。
    let (status, body) = send(
        app,
        post_json_with_bearer(DEV_TRANSFER, &token, transfer_body(20000)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["from_balance_after_cents"], 80000);
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn transfer_list_paginates_and_rejects_bad_paging() {
    let app = setup_app().await;
    let token = issue_dev_token(&app).await;

    for amount in [1000, 2000] {
        let (status, _) = send(
            app.clone(),
            post_json_with_bearer(DEV_TRANSFER, &token, transfer_body(amount)),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    let (status, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/transactions?page=1&size=1", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 2);
    assert_eq!(body["data"]["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(body["data"]["items"][0]["amount_cents"], 2000);

    let (status, _) = send(
        app.clone(),
        get_with_bearer("/api/v1/transactions?size=101", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = send(app, get_with_bearer("/api/v1/transactions?page=0", &token)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn unknown_transfer_record_returns_404() {
    let app = setup_app().await;
    let token = issue_dev_token(&app).await;

    let (status, _) = send(
        app,
        get_with_bearer("/api/v1/transactions/missing-record", &token),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// 统一响应结构：`code` 派生自 HTTP 状态码，`message` 与 `timestamp` 恒定存在，响应体固定为 JSON。
fn assert_unified_envelope(status: StatusCode, headers: &HeaderMap, body: &Value) {
    assert_eq!(
        body["code"].as_u64(),
        Some(u64::from(status.as_u16())),
        "code 应与 HTTP 状态码一致: {body}"
    );
    assert!(body["message"].is_string(), "message 应为字符串: {body}");
    assert!(
        body["timestamp"].as_i64().is_some_and(|value| value > 0),
        "timestamp 应为毫秒时间戳: {body}"
    );
    assert_eq!(
        headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/json"),
        "统一响应结构应为 JSON: {headers:?}"
    );
}

#[tokio::test]
async fn success_response_follows_the_unified_envelope() {
    let app = setup_app().await;
    let response = send_response(app, get("/")).await;
    let status = response.status();
    let headers = response.headers().clone();
    let body = read_json(response).await;

    assert_eq!(status, StatusCode::OK);
    assert_unified_envelope(status, &headers, &body);
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn error_responses_follow_the_unified_envelope() {
    let app = setup_app().await;
    let token = issue_dev_token(&app).await;
    let cases = [
        (StatusCode::BAD_REQUEST, get("/api/v1/examples?page=0")),
        (StatusCode::UNAUTHORIZED, get("/api/v1/auth/session")),
        (
            StatusCode::NOT_FOUND,
            get_with_bearer("/api/v1/transactions/missing-record", &token),
        ),
    ];

    for (expected, request) in cases {
        let response = send_response(app.clone(), request).await;
        let status = response.status();
        let headers = response.headers().clone();
        let body = read_json(response).await;

        assert_eq!(status, expected, "响应状态码不符: {body}");
        assert_unified_envelope(status, &headers, &body);
        assert!(body.get("data").is_none(), "错误响应不应带 data: {body}");
    }
}

/// 请求体解析失败由提取器直接拒绝，响应是 axum 产生的纯文本，这里只锁定状态码契约。
#[tokio::test]
async fn malformed_json_body_is_rejected_with_400() {
    let app = setup_app().await;
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/examples/echo")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{not json}"))
        .expect("构造请求");

    assert_eq!(
        send_response(app, request).await.status(),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn missing_json_content_type_is_rejected_with_415() {
    let app = setup_app().await;
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/examples/echo")
        .body(Body::from(r#"{"title":"x"}"#))
        .expect("构造请求");

    assert_eq!(
        send_response(app, request).await.status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
}

#[tokio::test]
async fn json_body_missing_required_field_is_rejected_with_422() {
    let app = setup_app().await;
    let status = send_response(app, post_json("/api/v1/examples/echo", json!({})))
        .await
        .status();

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn non_numeric_query_param_is_rejected_with_400() {
    let app = setup_app().await;
    let status = send_response(app, get("/api/v1/examples?size=abc"))
        .await
        .status();

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn unsupported_method_returns_405_with_allow_header() {
    let app = setup_app().await;
    let request = Request::builder()
        .method("PUT")
        .uri("/api/v1/system/health")
        .body(Body::empty())
        .expect("构造请求");
    let response = send_response(app, request).await;

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(
        response
            .headers()
            .get(header::ALLOW)
            .and_then(|value| value.to_str().ok()),
        Some("GET,HEAD")
    );
}

#[tokio::test]
async fn cors_preflight_allows_configured_origin_only() {
    let app = setup_app().await;
    let response = send_response(app, cors_preflight("https://example.com")).await;

    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    assert_eq!(
        headers
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|value| value.to_str().ok()),
        Some("https://example.com")
    );
    assert_eq!(
        headers
            .get(header::ACCESS_CONTROL_ALLOW_METHODS)
            .and_then(|value| value.to_str().ok()),
        Some("*")
    );
}

/// 未列入白名单的来源不得拿到任何跨域放行头。
#[tokio::test]
async fn cors_preflight_rejects_unlisted_origin() {
    let app = setup_app().await;
    let response = send_response(app, cors_preflight("https://evil.example")).await;

    assert!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none(),
        "未列入白名单的来源不应收到 allow-origin"
    );
}

/// 白名单为空时不下发任何 CORS 头（默认配置）。
#[tokio::test]
async fn cors_headers_are_absent_when_whitelist_is_empty() {
    let app = setup_app_with_cors_origins(Vec::new()).await;
    let response = send_response(app, cors_preflight("https://example.com")).await;

    assert!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none(),
        "空白名单不应放行任何来源"
    );
}

fn cors_preflight(origin: &str) -> Request<Body> {
    Request::builder()
        .method("OPTIONS")
        .uri("/api/v1/examples/echo")
        .header(header::ORIGIN, origin)
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .body(Body::empty())
        .expect("构造请求")
}

#[tokio::test]
async fn transaction_endpoints_require_bearer_token() {
    let app = setup_app().await;

    for uri in ["/api/v1/transactions", "/api/v1/transactions/any-record"] {
        let (status, body) = send(app.clone(), get(uri)).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}");
        assert_eq!(body["code"], 401, "{uri}");
    }
}

#[tokio::test]
async fn malformed_authorization_header_is_rejected_with_401() {
    let app = setup_app().await;

    for value in ["Bearer", "Bearer   "] {
        let request = Request::builder()
            .method("GET")
            .uri("/api/v1/auth/session")
            .header(header::AUTHORIZATION, value)
            .body(Body::empty())
            .expect("构造请求");
        let (status, body) = send(app.clone(), request).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED, "{value}");
        assert!(
            body["message"]
                .as_str()
                .is_some_and(|message| message.contains("Bearer <token>")),
            "{value} 应提示 Bearer 格式: {body}"
        );
    }
}

/// 与 `JwtService` 同密钥、同签发方签发的令牌，但已过期 5 分钟（jsonwebtoken 默认 60 秒 leeway）。
fn expired_token() -> String {
    let now = Utc::now();
    let claims = json!({
        "sub": "expired-user",
        "username": "expired",
        "roles": ["developer"],
        "iss": "test-issuer",
        "aud": "test-audience",
        "iat": (now - Duration::minutes(10)).timestamp() as usize,
        "exp": (now - Duration::minutes(5)).timestamp() as usize,
    });

    encode(
        &JwtHeader::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
    )
    .expect("签发过期令牌")
}

#[tokio::test]
async fn expired_token_is_rejected_with_401() {
    let app = setup_app().await;
    let (status, body) = send(
        app,
        get_with_bearer("/api/v1/auth/session", &expired_token()),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(
        body["message"]
            .as_str()
            .is_some_and(|message| message.contains("expired")),
        "过期令牌应给出可读原因: {body}"
    );
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn dev_login_with_empty_roles_falls_back_to_default_role() {
    let app = setup_app().await;
    let (status, body) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/dev-login",
            json!({ "username": "demo-admin", "roles": [] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = body["data"]["access_token"]
        .as_str()
        .expect("应返回访问令牌");

    let (status, me) = send(app, get_with_bearer("/api/v1/auth/session", token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["data"]["roles"], json!(["developer"]));
}
