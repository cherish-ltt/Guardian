//! 客户端保护层（可信来源 IP、按 IP 额度）的黑盒用例。

use std::{net::SocketAddr, sync::Arc};

use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    routing::get,
};
use guardian_backend::{infrastructure::config::MiddlewareConfig, middleware};
use tower::ServiceExt;

fn config() -> MiddlewareConfig {
    MiddlewareConfig {
        request_timeout_secs: 1,
        max_body_bytes: 1024,
        max_concurrency: 256,
        backpressure_queue: 256,
        rate_limit_requests: 1000,
        rate_limit_period_secs: 1,
        hsts_enabled: false,
        cors_allowed_origins: Vec::new(),
        trusted_proxies: Vec::new(),
        client_max_concurrency: 1,
        client_rate_limit_requests: 2,
        client_rate_limit_period_secs: 60,
        client_max_entries: 100,
        client_idle_secs: 60,
        slow_request_ms: 1000,
    }
}

fn request(peer: &str, forwarded: Option<&str>) -> Request<Body> {
    let mut request = Request::builder()
        .uri("/ping")
        .body(Body::empty())
        .expect("构造请求");
    request.extensions_mut().insert(ConnectInfo(
        peer.parse::<SocketAddr>().expect("测试对端地址应合法"),
    ));
    if let Some(value) = forwarded {
        request
            .headers_mut()
            .insert("x-forwarded-for", value.parse().expect("构造请求头"));
    }
    request
}

fn app(config: &MiddlewareConfig) -> Router {
    middleware::apply(
        Router::new().route("/ping", get(|| async { "pong" })),
        config,
    )
}

async fn message(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .expect("响应体可读取");
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("响应体应为合法 JSON");

    body["message"].as_str().expect("message 字段").to_string()
}

/// 额度跨路由共享（同一来源不能靠换路径绕过），但不同来源彼此独立。
#[tokio::test]
async fn client_rate_is_shared_across_routes_but_not_ips() {
    let app = middleware::apply(
        Router::new()
            .route("/ping", get(|| async { "pong" }))
            .route("/other", get(|| async { "other" })),
        &config(),
    );
    for _ in 0..2 {
        assert_eq!(
            app.clone()
                .oneshot(request("192.0.2.1:1000", None))
                .await
                .expect("请求应成功返回")
                .status(),
            StatusCode::OK
        );
    }

    let mut other = request("192.0.2.1:2000", None);
    *other.uri_mut() = "/other".parse().expect("构造路径");
    let rejected = app.clone().oneshot(other).await.expect("请求应成功返回");
    assert_eq!(rejected.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(
        rejected.headers().contains_key("retry-after"),
        "速率拒绝应给出重试时间"
    );
    assert_eq!(message(rejected).await, "请求过于频繁");

    assert_eq!(
        app.oneshot(request("192.0.2.2:1000", None))
            .await
            .expect("请求应成功返回")
            .status(),
        StatusCode::OK
    );
}

/// 不可信对端的转发头不影响额度：换 `X-Forwarded-For` 不能刷新额度。
#[tokio::test]
async fn untrusted_peers_cannot_change_identity_with_headers() {
    let app = app(&config());
    for forwarded in ["192.0.2.2", "192.0.2.3"] {
        assert_eq!(
            app.clone()
                .oneshot(request("192.0.2.1:1000", Some(forwarded)))
                .await
                .expect("请求应成功返回")
                .status(),
            StatusCode::OK
        );
    }

    assert_eq!(
        app.oneshot(request("192.0.2.1:1000", Some("192.0.2.4")))
            .await
            .expect("请求应成功返回")
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
}

/// 可信代理链从右向左解析：伪造的前缀不起作用，非法或重复转发头直接 400。
#[tokio::test]
async fn trusted_proxy_walks_chain_from_right_and_rejects_invalid_headers() {
    let mut config = config();
    config.trusted_proxies = vec![
        "10.0.0.0/8".parse().expect("构造网段"),
        "::1/128".parse().expect("构造网段"),
    ];
    let app = app(&config);
    for spoof in ["192.0.2.2", "192.0.2.3"] {
        let chain = format!("{spoof}, 198.51.100.1, 10.0.0.2");
        assert_eq!(
            app.clone()
                .oneshot(request("10.0.0.1:1000", Some(&chain)))
                .await
                .expect("请求应成功返回")
                .status(),
            StatusCode::OK
        );
    }

    assert_eq!(
        app.clone()
            .oneshot(request("10.0.0.1:1000", Some("198.51.100.1")))
            .await
            .expect("请求应成功返回")
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        app.clone()
            .oneshot(request("[::1]:1000", Some("2001:db8::1")))
            .await
            .expect("请求应成功返回")
            .status(),
        StatusCode::OK
    );

    let invalid = app
        .clone()
        .oneshot(request("10.0.0.1:1000", Some("unknown")))
        .await
        .expect("请求应成功返回");
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(message(invalid).await, "请求头 X-Forwarded-For 格式错误");

    let mut duplicated = request("10.0.0.1:1000", Some("192.0.2.9"));
    duplicated
        .headers_mut()
        .append("x-forwarded-for", "192.0.2.10".parse().expect("构造请求头"));
    assert_eq!(
        app.oneshot(duplicated)
            .await
            .expect("请求应成功返回")
            .status(),
        StatusCode::BAD_REQUEST
    );
}

/// IPv4 映射 IPv6 与纯 IPv4 共用同一份额度；缺少对端信息时失败关闭。
#[tokio::test]
async fn mapped_ipv6_shares_ipv4_quota_and_missing_peer_fails_closed() {
    let app = app(&config());
    for peer in ["192.0.2.1:1000", "[::ffff:192.0.2.1]:1000"] {
        assert_eq!(
            app.clone()
                .oneshot(request(peer, None))
                .await
                .expect("请求应成功返回")
                .status(),
            StatusCode::OK
        );
    }
    assert_eq!(
        app.clone()
            .oneshot(request("192.0.2.1:1000", None))
            .await
            .expect("请求应成功返回")
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ping")
                .body(Body::empty())
                .expect("构造请求"),
        )
        .await
        .expect("请求应成功返回");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(message(response).await, "系统内部错误");
}

/// 同一来源的并发许可用满只拒绝它自己，取消后必须释放许可。
#[tokio::test]
async fn concurrent_client_is_rejected_without_blocking_other_clients_and_releases_on_cancel() {
    let mut config = config();
    config.client_rate_limit_requests = 100;
    let entered = Arc::new(tokio::sync::Notify::new());
    let gate = Arc::new(tokio::sync::Notify::new());
    let app = middleware::apply(
        Router::new()
            .route(
                "/ping",
                get({
                    let entered = entered.clone();
                    let gate = gate.clone();
                    move || {
                        let entered = entered.clone();
                        let gate = gate.clone();
                        async move {
                            entered.notify_one();
                            gate.notified().await;
                            "done"
                        }
                    }
                }),
            )
            .route("/fast", get(|| async { "fast" })),
        &config,
    );

    let first = tokio::spawn(app.clone().oneshot(request("192.0.2.1:1000", None)));
    entered.notified().await;

    let rejected = app
        .clone()
        .oneshot(request("192.0.2.1:1000", None))
        .await
        .expect("请求应成功返回");
    assert_eq!(rejected.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(message(rejected).await, "同一来源并发请求过多");

    let mut other = request("192.0.2.2:1000", None);
    *other.uri_mut() = "/fast".parse().expect("构造路径");
    assert_eq!(
        app.clone()
            .oneshot(other)
            .await
            .expect("请求应成功返回")
            .status(),
        StatusCode::OK
    );

    first.abort();
    let _ = first.await;

    let mut next = request("192.0.2.1:1000", None);
    *next.uri_mut() = "/fast".parse().expect("构造路径");
    assert_eq!(
        app.oneshot(next).await.expect("请求应成功返回").status(),
        StatusCode::OK
    );
}
