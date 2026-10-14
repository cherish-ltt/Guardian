//! 请求日志关联单独成进程：`tracing` 的 callsite interest 缓存是进程级的，
//! 其他测试线程若在没有订阅者时命中过 tower-http 的 span callsite，会把该 callsite
//! 缓存为 `never`，而 `set_global_default` 并不重建该缓存。

use std::sync::{Arc, Mutex};

use axum::{Router, body::Body, http::Request, routing::get};
use guardian_backend::{infrastructure::config::MiddlewareConfig, middleware};
use tower::ServiceExt;
use tracing_subscriber::fmt::format::FmtSpan;

#[derive(Clone, Default)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl LogBuffer {
    fn contents(&self) -> String {
        String::from_utf8(self.0.lock().expect("日志缓冲可读").clone()).expect("日志为 UTF-8")
    }
}

impl std::io::Write for &LogBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("日志缓冲可写").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn capture_logs() -> Arc<LogBuffer> {
    let buffer = Arc::new(LogBuffer::default());
    let subscriber = tracing_subscriber::fmt()
        .with_writer(Arc::clone(&buffer))
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .with_span_events(FmtSpan::NEW)
        .finish();
    // set_global_default 进程级只能安装一次，panic 后无法恢复：本文件必须独占进程运行
    //（顶层注释已说明 callsite interest 缓存同样是进程级的，见 middleware_log_tests.rs 头部）。
    tracing::subscriber::set_global_default(subscriber).expect("安装全局日志订阅者");
    tracing::callsite::rebuild_interest_cache();
    buffer
}

fn base_config() -> MiddlewareConfig {
    MiddlewareConfig {
        request_timeout_secs: 10,
        max_body_bytes: 2 * 1024 * 1024,
        max_concurrency: 256,
        backpressure_queue: 256,
        rate_limit_requests: 1000,
        rate_limit_period_secs: 1,
        hsts_enabled: false,
        cors_allowed_origins: Vec::new(),
    }
}

#[tokio::test]
async fn request_id_is_correlated_in_logs() {
    let logs = capture_logs();
    let app = middleware::apply(
        Router::new().route("/ping", get(|| async { "pong" })),
        &base_config(),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ping")
                .header("x-request-id", "log-correlation-id")
                .body(Body::empty())
                .expect("构造请求"),
        )
        .await
        .expect("请求应成功返回");
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let captured = logs.contents();
    assert!(
        captured.contains("http_request"),
        "日志应来自 http_request span: {captured}"
    );
    assert!(
        captured.contains("request_id=log-correlation-id"),
        "日志应携带请求的 request_id: {captured}"
    );
}
