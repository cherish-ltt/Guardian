//! 分批写入验证单独成进程：`logging::init` 会安装全局日志订阅者，同一进程只能
//! 初始化一次；该测试要观察「未满批次不落盘」的中间状态，必须独占一个进程。

use std::path::Path;

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
            filename_prefix: "logging-batch".to_string(),
            filename_suffix: "log".to_string(),
            rotation: Rotation::NEVER,
            max_log_files: 2,
            out_dir: out_dir.to_string(),
            batch_max_events: 5,
            // 定时刷盘间隔拉到 1 小时，确保本测试只由「条数」触发落盘
            batch_flush_interval_secs: 3600,
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
fn file_logs_are_flushed_in_batches() {
    let out_dir = std::env::temp_dir().join(format!("guardian-log-batch-{}", Uuid::now_v7()));
    std::fs::create_dir_all(&out_dir).expect("创建临时日志目录");

    let config = test_config(&out_dir.to_string_lossy());
    let guard = match logging::init(&config) {
        Ok(guard) => guard,
        Err(e) => panic!("日志初始化应成功: {e}"),
    };

    // 未满批次阈值：事件只进内存缓冲，不应落盘
    for i in 0..4 {
        tracing::info!("batch log entry {i}");
    }
    assert_eq!(
        read_logs(&out_dir),
        "",
        "未满 LOG_BATCH_MAX_EVENTS=5 条时不应写入文件"
    );

    // 满 5 条：同步触发一次落盘
    tracing::info!("batch log entry 5");
    assert_eq!(
        read_logs(&out_dir).lines().count(),
        5,
        "满批次后应落盘 5 条日志"
    );

    // 继续写入，剩余 3 条在 guard drop 时落盘
    for i in 6..9 {
        tracing::info!("batch log entry {i}");
    }
    drop(guard);

    let logs = read_logs(&out_dir);
    assert_eq!(
        logs.lines().count(),
        8,
        "drop 后全部 8 条日志应已落盘:\n{logs}"
    );
}
