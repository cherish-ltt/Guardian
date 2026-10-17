//! 定时刷盘路径验证（独立进程）：`logging::init` 每进程只能安装一次全局订阅者，
//! 与 `tests/logging_batch_tests.rs` 分开，各自独占进程观察落盘时机。

use std::path::Path;
use std::thread;
use std::time::Duration;

use guardian_backend::infrastructure::config::{
    AppConfig, DatabaseConfig, JwtConfig, LoggingConfig, MiddlewareConfig, RedisConfig, SeedConfig,
    ServerConfig,
};
use guardian_backend::logging;
use tracing_appender::rolling::Rotation;
use uuid::Uuid;

const TEST_JWT_SECRET: &str = "logging-test-secret-that-is-long-enough";

fn test_config(out_dir: &str) -> AppConfig {
    AppConfig {
        app_name: "guardian-backend".to_string(),
        app_env: "development".to_string(),
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
        },
        database: DatabaseConfig {
            url: "sqlite://:memory:".to_string(),
            min_connections: 1,
            max_connections: 1,
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
            filename_prefix: "logging-interval".to_string(),
            filename_suffix: "log".to_string(),
            rotation: Rotation::NEVER,
            max_log_files: 2,
            out_dir: out_dir.to_string(),
            // 条数阈值拉高，确保本测试只由定时刷盘触发落盘
            batch_max_events: 100,
            batch_flush_interval_secs: 1,
        },
        middleware: MiddlewareConfig {
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
        },
        ip_location_db_path: None,
        cache: Default::default(),
        two_factor_key: Some("BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=".into()),
    }
}

/// 读取日志目录下所有文件的拼接内容；尚未创建文件时返回空串。
fn read_logs(out_dir: &Path) -> String {
    let mut contents = String::new();
    if let Ok(entries) = std::fs::read_dir(out_dir) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                contents.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
            }
        }
    }
    contents
}

#[test]
fn logs_are_flushed_on_fixed_interval() {
    let out_dir = std::env::temp_dir().join(format!("guardian-log-interval-{}", Uuid::now_v7()));
    std::fs::create_dir_all(&out_dir).expect("创建临时日志目录");

    let config = test_config(&out_dir.to_string_lossy());
    let guard = match logging::init(&config) {
        Ok(guard) => guard,
        Err(e) => panic!("日志初始化应成功: {e}"),
    };

    // 顺带验证启动配置日志输出了批量参数（log_startup_config 会写入文件）
    logging::log_startup_config(&config);

    for i in 0..3 {
        tracing::info!("interval flush entry {i}");
    }
    // 未满 100 条、间隔 1s：轮询等待定时线程落盘，避免固定 sleep 造成的时序抖动。
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut logs;
    loop {
        logs = read_logs(&out_dir);
        if logs.contains("interval flush entry 2") || std::time::Instant::now() > deadline {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(
        logs.contains("interval flush entry 0")
            && logs.contains("interval flush entry 1")
            && logs.contains("interval flush entry 2"),
        "定时刷盘线程应在间隔到期后落盘缓冲日志:\n{logs}"
    );
    assert!(
        logs.contains("batch_max_events=100") && logs.contains("batch_flush_interval_secs=1"),
        "启动配置日志应输出批量参数:\n{logs}"
    );
    // 版本号取自 Cargo.toml，断言用编译期同一来源比对，不硬编码字面量。
    assert!(
        logs.contains(&format!("version={}", env!("CARGO_PKG_VERSION"))),
        "启动配置日志应输出版本号:\n{logs}"
    );
    drop(guard);
}
