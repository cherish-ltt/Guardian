use std::time::{Duration, Instant};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    routing::{get, post},
};
use guardian_backend::{infrastructure::config::MiddlewareConfig, middleware};
use tower::ServiceExt;
use uuid::Uuid;

fn base_config() -> MiddlewareConfig {
    MiddlewareConfig {
        request_timeout_secs: 10,
        max_body_bytes: 2 * 1024 * 1024,
        max_concurrency: 256,
        backpressure_queue: 256,
        rate_limit_requests: 1000,
        rate_limit_period_secs: 1,
        hsts_enabled: false,
        cors_allowed_origins: vec!["https://example.com".to_string()],
    }
}

fn ping_app(config: &MiddlewareConfig) -> Router {
    middleware::apply(
        Router::new().route("/ping", get(|| async { "pong" })),
        config,
    )
}

fn get_request(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .body(Body::empty())
        .expect("构造请求")
}

async fn send(app: Router, request: Request<Body>) -> axum::response::Response {
    app.oneshot(request).await.expect("请求应成功返回")
}

async fn slow_handler() -> &'static str {
    tokio::time::sleep(Duration::from_secs(5)).await;
    "slow"
}

async fn hold_handler() -> &'static str {
    tokio::time::sleep(Duration::from_millis(300)).await;
    "held"
}

#[tokio::test]
async fn request_over_timeout_is_rejected_with_408() {
    let mut config = base_config();
    config.request_timeout_secs = 1;

    let app = middleware::apply(Router::new().route("/slow", get(slow_handler)), &config);

    let started = Instant::now();
    let response = send(app, get_request("/slow")).await;

    assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "超时应在 handler 完成前返回，实际耗时 {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn oversized_body_is_rejected_with_413() {
    let mut config = base_config();
    config.max_body_bytes = 1024;

    let app = middleware::apply(
        Router::new().route(
            "/upload",
            post(|body: axum::body::Bytes| async move { body.len().to_string() }),
        ),
        &config,
    );

    let oversized = Request::builder()
        .method("POST")
        .uri("/upload")
        .header(header::CONTENT_LENGTH, "4096")
        .body(Body::from(vec![0_u8; 4096]))
        .expect("构造请求");
    let response = send(app.clone(), oversized).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);

    let accepted = Request::builder()
        .method("POST")
        .uri("/upload")
        .header(header::CONTENT_LENGTH, "4")
        .body(Body::from(vec![0_u8; 4]))
        .expect("构造请求");
    assert_eq!(send(app, accepted).await.status(), StatusCode::OK);
}

#[tokio::test]
async fn requests_over_the_rate_limit_are_rejected_with_429() {
    let mut config = base_config();
    config.rate_limit_requests = 2;
    config.rate_limit_period_secs = 60;

    let app = ping_app(&config);

    for expected in [
        StatusCode::OK,
        StatusCode::OK,
        StatusCode::TOO_MANY_REQUESTS,
    ] {
        let response = send(app.clone(), get_request("/ping")).await;
        assert_eq!(response.status(), expected);
    }

    // 窗口未重置前持续快速拒绝
    assert_eq!(
        send(app, get_request("/ping")).await.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn concurrency_overload_is_shed_with_503() {
    let mut config = base_config();
    config.max_concurrency = 1;
    config.backpressure_queue = 1;

    let app = middleware::apply(Router::new().route("/hold", get(hold_handler)), &config);

    let started = Instant::now();
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let app = app.clone();
        tasks.push(tokio::spawn(async move {
            send(app, get_request("/hold")).await.status()
        }));
    }

    let mut statuses = Vec::new();
    for task in tasks {
        statuses.push(task.await.expect("请求任务不应 panic"));
    }
    let elapsed = started.elapsed();

    assert!(
        statuses.contains(&StatusCode::OK),
        "容量内请求应正常处理: {statuses:?}"
    );
    assert!(
        statuses.contains(&StatusCode::SERVICE_UNAVAILABLE),
        "超出容量的请求应快速返回 503: {statuses:?}"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "超出排队长度的请求不得继续堆积，实际耗时 {elapsed:?}"
    );
}

#[tokio::test]
async fn request_id_is_generated_and_propagated() {
    let app = ping_app(&base_config());

    let response = send(app.clone(), get_request("/ping")).await;
    let generated = response
        .headers()
        .get("x-request-id")
        .expect("响应应带 x-request-id")
        .to_str()
        .expect("x-request-id 可读");
    assert!(
        Uuid::parse_str(generated).is_ok(),
        "未透传时应生成 UUID: {generated}"
    );

    let request = Request::builder()
        .uri("/ping")
        .header("x-request-id", "client-trace-id")
        .body(Body::empty())
        .expect("构造请求");
    let response = send(app, request).await;
    assert_eq!(
        response
            .headers()
            .get("x-request-id")
            .expect("响应应带 x-request-id"),
        "client-trace-id"
    );
}

#[tokio::test]
async fn security_headers_follow_config() {
    let response = send(ping_app(&base_config()), get_request("/ping")).await;
    let headers = response.headers();
    assert_eq!(
        headers.get("x-content-type-options").expect("缺少 nosniff"),
        "nosniff"
    );
    assert_eq!(headers.get("x-frame-options").expect("缺少 DENY"), "DENY");
    assert_eq!(
        headers
            .get("referrer-policy")
            .expect("缺少 referrer-policy"),
        "strict-origin-when-cross-origin"
    );
    assert!(
        headers.get("strict-transport-security").is_none(),
        "非 HTTPS 环境不应下发 HSTS"
    );

    let mut config = base_config();
    config.hsts_enabled = true;
    let response = send(ping_app(&config), get_request("/ping")).await;
    assert!(
        response
            .headers()
            .get("strict-transport-security")
            .is_some(),
        "开启后应下发 HSTS"
    );
}

#[tokio::test]
async fn compressible_response_is_gzipped_when_accepted() {
    // 默认压缩谓词只压缩大于 32 字节的响应体
    let app = middleware::apply(
        Router::new().route("/ping", get(|| async { "a".repeat(64) })),
        &base_config(),
    );
    let request = Request::builder()
        .uri("/ping")
        .header(header::ACCEPT_ENCODING, "gzip")
        .body(Body::empty())
        .expect("构造请求");

    let response = send(app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_ENCODING)
            .expect("应压缩响应体"),
        "gzip"
    );
}

/// CORS 由中间件栈统一装配：白名单内的来源放行，未列入的直接无 CORS 头。
#[tokio::test]
async fn cors_layer_follows_allowed_origins_config() {
    let app = ping_app(&base_config());

    let allowed = send(app.clone(), cors_preflight("https://example.com")).await;
    assert_eq!(
        allowed
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|value| value.to_str().ok()),
        Some("https://example.com")
    );

    let rejected = send(app, cors_preflight("https://evil.example")).await;
    assert!(
        rejected
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none(),
        "未列入白名单的来源不应收到 allow-origin"
    );
}

fn cors_preflight(origin: &str) -> Request<Body> {
    Request::builder()
        .method("OPTIONS")
        .uri("/ping")
        .header(header::ORIGIN, origin)
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
        .body(Body::empty())
        .expect("构造请求")
}
