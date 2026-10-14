//! 后台管理接口集成测试的公共装置。
//!
//! 每个测试独立装配容器与临时 SQLite 数据库；会话用内存实现，不依赖外部 Redis。
//! 三个测试二进制各自编译一份本模块，未用到的辅助函数允许空置。

#![allow(dead_code)]

use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    response::Response,
};
use guardian_backend::{
    container::Container,
    create_app::create_app,
    infrastructure::config::{
        AppConfig, DatabaseConfig, JwtConfig, LoggingConfig, MiddlewareConfig, RedisConfig,
        SeedConfig, ServerConfig,
    },
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use tracing_appender::rolling::Rotation;
use uuid::Uuid;

const TEST_JWT_SECRET: &str = "admin-api-test-secret-long-enough";

/// 演示口令，与种子数据一致。
pub const DEMO_PASSWORD: &str = "admin123456";

pub async fn setup_app() -> Router {
    let database_url = format!(
        "sqlite://{}?mode=rwc",
        std::env::temp_dir()
            .join(format!("guardian-admin-test-{}.db", Uuid::now_v7()))
            .display()
    );
    let config = AppConfig {
        app_name: "guardian-backend".to_string(),
        app_env: "test".to_string(),
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
            admin_password: DEMO_PASSWORD.to_string(),
            ..SeedConfig::default()
        },
        logging: LoggingConfig {
            filter: "info".to_string(),
            utc_offset_hour: 8,
            utc_offset_minute: 0,
            utc_offset_second: 0,
            filename_prefix: "admin-api-test".to_string(),
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
            rate_limit_requests: 10_000,
            rate_limit_period_secs: 1,
            hsts_enabled: false,
            cors_allowed_origins: Vec::new(),
        },
    };

    let container = Container::bootstrap_with_auth_store(config, Container::memory_auth_store())
        .await
        .expect("容器装配成功");

    create_app(Arc::new(container))
}

pub async fn send(app: Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.oneshot(request).await.expect("请求应成功返回");
    let status = response.status();

    (status, read_json(response).await)
}

/// 原样返回状态码、响应头与响应体，供 CSV / ZIP 等非 JSON 响应断言。
pub async fn send_raw(
    app: Router,
    request: Request<Body>,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let response = app.oneshot(request).await.expect("请求应成功返回");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = read_bytes(response).await;

    (status, headers, bytes)
}

pub async fn read_json(response: Response) -> Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("响应体可读取")
        .to_bytes();

    serde_json::from_slice(&bytes).expect("响应体应为合法 JSON")
}

pub async fn read_bytes(response: Response) -> Vec<u8> {
    response
        .into_body()
        .collect()
        .await
        .expect("响应体可读取")
        .to_bytes()
        .to_vec()
}

pub fn get(uri: &str) -> Request<Body> {
    request("GET", uri, None, None)
}

pub fn get_with_bearer(uri: &str, token: &str) -> Request<Body> {
    request("GET", uri, None, Some(token))
}

pub fn post_json(uri: &str, body: Value) -> Request<Body> {
    request("POST", uri, Some(body), None)
}

pub fn post_json_with_bearer(uri: &str, token: &str, body: Value) -> Request<Body> {
    request("POST", uri, Some(body), Some(token))
}

pub fn put_json_with_bearer(uri: &str, token: &str, body: Value) -> Request<Body> {
    request("PUT", uri, Some(body), Some(token))
}

pub fn delete_with_bearer(uri: &str, token: &str) -> Request<Body> {
    request("DELETE", uri, None, Some(token))
}

fn request(method: &str, uri: &str, body: Option<Value>, token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = match body {
        Some(body) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");

            Body::from(body.to_string())
        },
        None => Body::empty(),
    };

    builder.body(body).expect("构造请求")
}

/// 走完整登录流程：取验证码（页面明文回显）→ 登录 → 返回令牌。
pub async fn login(app: &Router, username: &str, password: &str) -> String {
    let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    let uuid = captcha["data"]["uuid"]
        .as_str()
        .expect("验证码标识")
        .to_string();
    let code = captcha["data"]["code"]
        .as_str()
        .expect("验证码明文")
        .to_string();
    let (status, body) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({
                "username": username,
                "password": password,
                "captcha": code,
                "captchaUuid": uuid,
            }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "登录失败: {body}");

    body["data"]["token"]
        .as_str()
        .expect("登录令牌")
        .to_string()
}

/// 管理员令牌：绝大多数用例的起点。
pub async fn admin_token(app: &Router) -> String {
    login(app, "admin", DEMO_PASSWORD).await
}
