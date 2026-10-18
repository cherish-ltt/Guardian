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
        ip_location_v6_db_path: None,
        cache: Default::default(),
        two_factor_key: Some("BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=".into()),
    }
}

/// 从 `AppConfig` 的 `Debug` 输出里取出所有字段名。
///
/// 用于「快照必须覆盖全部配置字段」的断言：新增字段时若忘了同步快照，这条断言会直接失败，
/// 而不是等到线上查日志才发现某个参数没打印。
fn field_names(debug: &str) -> Vec<String> {
    debug
        .split(": ")
        .filter_map(|segment| {
            let name: String = segment
                .chars()
                .rev()
                .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            name.chars()
                .next()
                .is_some_and(|ch| ch.is_alphabetic())
                .then_some(name)
        })
        .collect()
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
    // 快照必须覆盖 AppConfig 的每一个字段：字段名从 Debug 输出自动收集，新增字段漏打印会失败。
    let names = field_names(&format!("{config:?}"));
    assert!(!names.is_empty(), "Debug 输出解析失败:\n{config:?}");
    for name in &names {
        // 嵌套配置以 `| [分组名]` 出现，标量字段以 `name=值` 出现。
        assert!(
            logs.contains(&format!("{name}=")) || logs.contains(&format!("| [{name}]")),
            "启动配置日志缺少字段 {name}:\n{logs}"
        );
    }
    // 分节结构固定，缺一节说明分组被误删。
    for section in [
        "application",
        "server",
        "database",
        "jwt",
        "two_factor",
        "redis",
        "cache",
        "seed",
        "ip_location",
        "logging",
        "middleware",
    ] {
        assert!(
            logs.contains(&format!("| [{section}]")),
            "启动配置日志缺少 [{section}] 分节:\n{logs}"
        );
    }
    // 密钥与口令只以脱敏形式出现。
    for secret in [
        TEST_JWT_SECRET,
        &config.seed.admin_password,
        config
            .two_factor_key
            .as_deref()
            .expect("测试配置带 2FA 密钥"),
    ] {
        assert!(
            !logs.contains(secret),
            "启动配置日志泄漏了密钥原文 {secret}:\n{logs}"
        );
    }
    assert!(
        logs.contains("two_factor_key=(configured)"),
        "2FA 密钥只输出是否已配置:\n{logs}"
    );
    // 版本号取自 Cargo.toml，断言用编译期同一来源比对，不硬编码字面量。
    assert!(
        logs.contains(&format!("version={}", env!("CARGO_PKG_VERSION"))),
        "启动配置日志应输出版本号:\n{logs}"
    );
    drop(guard);
}
