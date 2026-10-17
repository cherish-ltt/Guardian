//! 请求日志关联单独成进程：`tracing` 的 callsite interest 缓存是进程级的，
//! 其他测试线程若在没有订阅者时命中过 tower-http 的 span callsite，会把该 callsite
//! 缓存为 `never`，而 `set_global_default` 并不重建该缓存。

use std::sync::{Arc, Mutex};

use axum::{Router, body::Body, extract::ConnectInfo, http::Request, routing::get};
use guardian_backend::{
    infrastructure::config::{
        AppConfig, DatabaseConfig, JwtConfig, LoggingConfig, MiddlewareConfig, RedisConfig,
        SeedConfig, ServerConfig,
    },
    middleware,
};
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
        // 只有事件才产生日志行，span 创建/关闭事件会让「每请求一条」的断言失真。
        .with_span_events(FmtSpan::NONE)
        .finish();
    // set_global_default 进程级只能安装一次，panic 后无法恢复：本文件必须独占进程运行
    //（顶层注释已说明 callsite interest 缓存同样是进程级的）。
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
        trusted_proxies: Vec::new(),
        client_max_concurrency: 32,
        client_rate_limit_requests: 1024,
        client_rate_limit_period_secs: 1,
        client_max_entries: 16384,
        client_idle_secs: 60,
        slow_request_ms: 1000,
    }
}

/// `oneshot` 不经过真实监听，测试里显式补一个本地对端。
fn with_peer(app: Router) -> Router {
    app.layer(axum::Extension(ConnectInfo(
        "127.0.0.1:12345"
            .parse::<std::net::SocketAddr>()
            .expect("测试对端地址应合法"),
    )))
}

#[tokio::test]
async fn request_logs_are_correlated_and_low_noise() {
    let logs = capture_logs();
    access_log_records_route_and_client_ip(&logs).await;
    slow_request_is_reported_once(&logs).await;
    interrupted_database_stage_is_reported(&logs).await;
}

/// 正常请求只有一条访问日志，带路由模板与来源 IP，且不记录 query。
async fn access_log_records_route_and_client_ip(logs: &Arc<LogBuffer>) {
    let app = with_peer(middleware::apply(
        Router::new().route("/ping", get(|| async { "pong" })),
        &base_config(),
    ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ping?secret=do-not-log")
                .header("x-request-id", "log-correlation-id")
                .body(Body::empty())
                .expect("构造请求"),
        )
        .await
        .expect("请求应成功返回");
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let captured = logs.contents();
    assert_eq!(
        captured.lines().count(),
        1,
        "正常请求只能一条访问日志: {captured}"
    );
    assert!(
        captured.contains("http_request"),
        "日志应来自 http_request span: {captured}"
    );
    assert!(
        captured.contains("request_id=log-correlation-id"),
        "日志应携带请求的 request_id: {captured}"
    );
    assert!(
        captured.contains("client_ip=127.0.0.1"),
        "日志应记录可信来源 IP: {captured}"
    );
    assert!(
        captured.contains(r#"route="/ping""#),
        "日志应记录路由模板: {captured}"
    );
    assert!(
        !captured.contains("do-not-log"),
        "访问日志不得记录 query: {captured}"
    );
}

/// 慢请求升级成一条诊断日志，不再另发一条普通访问日志。
async fn slow_request_is_reported_once(logs: &Arc<LogBuffer>) {
    let config = MiddlewareConfig {
        slow_request_ms: 1,
        ..base_config()
    };
    let app = with_peer(middleware::apply(
        Router::new().route(
            "/slow/{id}",
            get(|| async {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                "slow"
            }),
        ),
        &config,
    ));
    app.oneshot(
        Request::builder()
            .uri("/slow/123?secret=hidden")
            .body(Body::empty())
            .expect("构造请求"),
    )
    .await
    .expect("请求应成功返回");

    let captured = logs.contents();
    assert_eq!(
        captured.lines().count(),
        2,
        "慢请求不能再加一条普通访问日志: {captured}"
    );
    assert!(captured.contains("request diagnostic"));
    assert!(captured.contains(r#"route="/slow/{id}""#));
    assert!(captured.contains("queue_ms="));
    assert!(!captured.contains("hidden"));
}

/// 真实数据库连接池被占满时，`db.ready` 阶段应被标记为中断，而不是只看到一条超时日志。
///
/// 与上一段共用同一个进程（全局订阅者只能安装一次），因此不是独立的 `#[tokio::test]`。
async fn interrupted_database_stage_is_reported(logs: &Arc<LogBuffer>) {
    let config = AppConfig {
        app_name: "guardian-backend".to_string(),
        app_env: "test".to_string(),
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
        },
        database: DatabaseConfig {
            url: format!(
                "sqlite://{}?mode=rwc",
                std::env::temp_dir()
                    .join(format!("guardian-diagnostic-{}.db", uuid::Uuid::now_v7()))
                    .display()
            ),
            min_connections: 1,
            max_connections: 1,
            connect_timeout_secs: 5,
            idle_secs: 30,
            sqlx_logging: false,
        },
        jwt: JwtConfig {
            secret: "diagnostic-test-secret-that-is-long-enough".to_string(),
            issuer: "test-issuer".to_string(),
            audience: "test-audience".to_string(),
            access_token_ttl_minutes: 120,
        },
        redis: RedisConfig {
            url: "redis://127.0.0.1:6379".to_string(),
            password: None,
        },
        seed: SeedConfig::default(),
        logging: LoggingConfig {
            filter: "info".to_string(),
            utc_offset_hour: 8,
            utc_offset_minute: 0,
            utc_offset_second: 0,
            filename_prefix: "middleware-log-test".to_string(),
            filename_suffix: "log".to_string(),
            rotation: tracing_appender::rolling::Rotation::NEVER,
            max_log_files: 1,
            out_dir: std::env::temp_dir().to_string_lossy().to_string(),
            batch_max_events: 50,
            batch_flush_interval_secs: 3,
        },
        middleware: MiddlewareConfig {
            request_timeout_secs: 1,
            ..base_config()
        },
        ip_location_db_path: None,
        cache: Default::default(),
        two_factor_key: Some("BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=".into()),
    };

    let container = Arc::new(
        guardian_backend::container::Container::bootstrap_with_auth_store(
            config,
            guardian_backend::container::Container::memory_auth_store(),
        )
        .await
        .expect("容器装配成功"),
    );
    // 占住唯一的池连接：后续 ping 只能排队等连接，直到请求总超时把阶段判为中断。
    let held = container
        .database
        .get_sqlite_connection_pool()
        .acquire()
        .await
        .expect("占用池连接");
    let app = with_peer(guardian_backend::create_app::create_app(container.clone()));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/system/ready")
                .header("x-request-id", "pool-timeout-id")
                .body(Body::empty())
                .expect("构造请求"),
        )
        .await
        .expect("请求应成功返回");
    assert_eq!(response.status(), axum::http::StatusCode::REQUEST_TIMEOUT);
    drop(held);

    let captured = logs.contents();
    let timeout_line = captured
        .lines()
        .find(|line| line.contains("pool-timeout-id"))
        .expect("超时请求应有诊断日志");
    assert!(
        timeout_line.contains("db.ready"),
        "数据库阶段要跨排队层被采集到: {timeout_line}"
    );
    assert!(timeout_line.contains("interrupted: 1"));
    assert!(timeout_line.contains("request diagnostic"));
    assert!(timeout_line.contains("status=408"));
}
