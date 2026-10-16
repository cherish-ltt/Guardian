use guardian_backend::infrastructure::config::{
    AppConfig, DatabaseConfig, JwtConfig, LoggingConfig, MiddlewareConfig, RedisConfig, SeedConfig,
    ServerConfig,
};
use guardian_backend::logging;
use tracing_appender::rolling::Rotation;
use uuid::Uuid;

const TEST_JWT_SECRET: &str = "logging-test-secret-that-is-long-enough";

fn test_config(out_dir: &str, utc_offset_hour: i8, utc_offset_minute: i8) -> AppConfig {
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
            utc_offset_hour,
            utc_offset_minute,
            utc_offset_second: 0,
            filename_prefix: "logging-test".to_string(),
            filename_suffix: "log".to_string(),
            rotation: Rotation::NEVER,
            max_log_files: 2,
            out_dir: out_dir.to_string(),
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

#[test]
fn logging_init_creates_log_file_in_configured_dir() {
    let out_dir = std::env::temp_dir().join(format!("guardian-log-test-{}", Uuid::now_v7()));
    std::fs::create_dir_all(&out_dir).expect("创建临时日志目录");

    let config = test_config(&out_dir.to_string_lossy(), 8, 0);
    let guard = match logging::init(&config) {
        Ok(guard) => guard,
        Err(e) => panic!("日志初始化应成功: {e}"),
    };

    tracing::info!("logging init test message");
    drop(guard);

    // 不只断言文件存在，还要断言写入的内容确实落盘（BatchGuard drop 时落盘剩余缓冲）。
    let mut contents = String::new();
    for entry in std::fs::read_dir(&out_dir).expect("读取日志目录").flatten() {
        if entry.path().is_file() {
            contents.push_str(&std::fs::read_to_string(entry.path()).expect("读取日志文件"));
        }
    }
    assert!(
        contents.contains("logging init test message"),
        "日志文件应包含写入的日志行，实际内容:\n{contents}"
    );
}

#[test]
fn logging_init_rejects_invalid_utc_offset() {
    // time crate 的 UtcOffset 分量合法范围为 ±25 小时、±59 分秒，99 分钟必然非法。
    let config = test_config("/tmp/guardian-invalid-offset", 0, 99);

    let error = match logging::init(&config) {
        Ok(_) => panic!("非法时区偏移应被拒绝"),
        Err(e) => e,
    };
    assert!(
        error
            .to_string()
            .contains("invalid log timezone offset configuration")
    );
}

/// LOG_FILTER 是唯一真值来源：写错时必须拒绝启动，而不是静默换一套过滤规则。
#[test]
fn logging_init_rejects_invalid_filter() {
    let mut config = test_config("/tmp/guardian-invalid-filter", 8, 0);
    config.logging.filter = "info,missing=notalevel".to_string();

    match logging::init(&config) {
        Ok(_) => panic!("非法 LOG_FILTER 应被拒绝"),
        Err(error) => assert!(
            error.to_string().contains("invalid LOG_FILTER"),
            "报错信息不符合预期: {error}"
        ),
    }
}
