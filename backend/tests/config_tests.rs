use std::env;
use std::net::IpAddr;
use std::{
    path::Path,
    sync::{Mutex, MutexGuard, OnceLock},
};

use guardian_backend::infrastructure::config::{AppConfig, RedisConfig, ServerConfig};

/// 环境变量是进程级全局状态，所有依赖环境变量的测试必须串行执行，避免相互污染。
static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn set_var(key: &str, value: &str) {
    unsafe { env::set_var(key, value) };
}

fn remove_var(key: &str) {
    unsafe { env::remove_var(key) };
}

/// 断言结束（含 panic）时恢复环境变量的守卫：panic 展开也会执行 Drop。
struct EnvRestore {
    saved: Vec<(String, Option<String>)>,
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            match value {
                Some(value) => set_var(key, value),
                None => remove_var(key),
            }
        }
    }
}

/// 临时设置环境变量后执行断言，结束时（含断言 panic）恢复原始值。
fn with_env<F: FnOnce()>(vars: &[(&str, Option<&str>)], assertion: F) {
    let _lock = env_lock();
    let saved: Vec<(String, Option<String>)> = vars
        .iter()
        .map(|(key, _)| (key.to_string(), env::var(key).ok()))
        .collect();

    // 先挂上恢复守卫，再改环境变量：无论 assertion 是否 panic，都会恢复。
    let _restore = EnvRestore { saved };

    for (key, value) in vars {
        match value {
            Some(value) => set_var(key, value),
            None => remove_var(key),
        }
    }

    assertion();
}

const EXAMPLE_SECRET: &str = "change-me-to-a-random-string-with-at-least-32-characters";
const CUSTOM_SECRET: &str = "unit-test-secret-that-is-definitely-long-enough";

/// Rotation 的 Debug 输出为 PascalCase（如 Daily），统一小写后比较。
fn rotation_debug(rotation: &tracing_appender::rolling::Rotation) -> String {
    format!("{rotation:?}").to_lowercase()
}

#[test]
fn security_and_cache_config_validation() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("TWO_FACTOR_KEY", None),
        ],
        || {
            let config = AppConfig::from_env().unwrap();
            assert!(config.two_factor_key.is_none());
            assert_eq!(config.cache.max_loads, 4);
        },
    );
    for key in [
        "CACHE_TTL_SECS",
        "CACHE_NEGATIVE_TTL_SECS",
        "CACHE_TIMEOUT_MS",
        "CACHE_MAX_LOADS",
        "CACHE_MAX_ENTRIES",
        "CACHE_MAX_VALUE_BYTES",
    ] {
        with_env(
            &[("JWT_SECRET", Some(CUSTOM_SECRET)), (key, Some("0"))],
            || assert!(AppConfig::from_env().is_err()),
        );
    }
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("TWO_FACTOR_KEY", Some("invalid")),
        ],
        || assert!(AppConfig::from_env().is_err()),
    );
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            (
                "TWO_FACTOR_KEY",
                Some("BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc="),
            ),
            ("CACHE_TTL_SECS", Some("20")),
            ("CACHE_NEGATIVE_TTL_SECS", Some("5")),
            ("CACHE_MAX_LOADS", Some("2")),
        ],
        || {
            let config = AppConfig::from_env().unwrap();
            assert!(config.two_factor_key.is_some());
            assert_eq!(config.cache.ttl_secs, 20);
            assert_eq!(config.cache.negative_ttl_secs, 5);
            assert_eq!(config.cache.max_loads, 2);
        },
    );
}

#[test]
fn valid_config_loads_with_defaults() {
    with_env(
        &[
            ("APP_NAME", None),
            ("APP_ENV", None),
            ("SERVER_HOST", None),
            ("SERVER_PORT", None),
            ("DATABASE_URL", None),
            ("DATABASE_MIN_CONNECTIONS", None),
            ("DATABASE_MAX_CONNECTIONS", None),
            ("DATABASE_CONNECT_TIMEOUT_SECS", None),
            ("DATABASE_IDLE_SECS", None),
            ("DATABASE_SQLX_LOGGING", None),
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("JWT_ISSUER", None),
            ("JWT_AUDIENCE", None),
            ("JWT_ACCESS_TOKEN_TTL_MINUTES", None),
            ("LOG_FILTER", None),
            ("LOG_UTC_OFFSET_HOUR", None),
            ("LOG_UTC_OFFSET_MINUTE", None),
            ("LOG_UTC_OFFSET_SECOND", None),
            ("LOG_FILENAME_PREFIX", None),
            ("LOG_FILENAME_SUFFIX", None),
            ("LOG_ROTATION", None),
            ("LOG_MAX_LOG_FILES", None),
            ("LOG_OUT_DIR", None),
            ("LOG_BATCH_MAX_EVENTS", None),
            ("LOG_BATCH_FLUSH_INTERVAL_SECS", None),
            ("MIDDLEWARE_REQUEST_TIMEOUT_SECS", None),
            ("MIDDLEWARE_MAX_BODY_BYTES", None),
            ("MIDDLEWARE_MAX_CONCURRENCY", None),
            ("MIDDLEWARE_BACKPRESSURE_QUEUE", None),
            ("MIDDLEWARE_RATE_LIMIT_REQUESTS", None),
            ("MIDDLEWARE_RATE_LIMIT_PERIOD_SECS", None),
            ("MIDDLEWARE_HSTS_ENABLED", None),
            ("MIDDLEWARE_CORS_ALLOWED_ORIGINS", None),
            ("MIDDLEWARE_TRUSTED_PROXIES", None),
            ("MIDDLEWARE_CLIENT_MAX_CONCURRENCY", None),
            ("MIDDLEWARE_CLIENT_RATE_LIMIT_REQUESTS", None),
            ("MIDDLEWARE_CLIENT_RATE_LIMIT_PERIOD_SECS", None),
            ("MIDDLEWARE_CLIENT_MAX_ENTRIES", None),
            ("MIDDLEWARE_CLIENT_IDLE_SECS", None),
            ("MIDDLEWARE_SLOW_REQUEST_MS", None),
            ("IP_LOCATION_DB_PATH", None),
            ("IP_LOCATION_V6_DB_PATH", None),
        ],
        || {
            let config = AppConfig::from_env().expect("默认配置应可加载");

            assert_eq!(config.app_name, "guardian-backend");
            assert_eq!(config.app_env, "development");
            assert_eq!(config.server.host, "127.0.0.1");
            assert_eq!(config.server.port, 8080);
            assert_eq!(config.database.url, "sqlite://guardian.db?mode=rwc");
            assert_eq!(config.database.min_connections, 1);
            assert_eq!(config.database.max_connections, 10);
            assert_eq!(config.database.connect_timeout_secs, 8);
            assert_eq!(config.database.idle_secs, 60);
            assert!(!config.database.sqlx_logging);
            assert_eq!(config.jwt.issuer, "guardian-backend");
            assert_eq!(config.jwt.audience, "guardian-clients");
            assert_eq!(config.jwt.access_token_ttl_minutes, 120);
            assert_eq!(config.logging.filter, "info,tower_http=info");
            assert_eq!(config.logging.filename_prefix, "app");
            assert_eq!(config.logging.filename_suffix, "log");
            assert_eq!(config.logging.max_log_files, 60);
            assert_eq!(config.logging.out_dir, "./log");
            assert_eq!(config.logging.batch_max_events, 50);
            assert_eq!(config.logging.batch_flush_interval_secs, 3);
            assert!(rotation_debug(&config.logging.rotation).contains("daily"));

            assert_eq!(config.middleware.request_timeout_secs, 10);
            assert_eq!(config.middleware.max_body_bytes, 2 * 1024 * 1024);
            assert_eq!(config.middleware.max_concurrency, 256);
            assert_eq!(config.middleware.backpressure_queue, 256);
            assert_eq!(config.middleware.rate_limit_requests, 32768);
            assert_eq!(config.middleware.rate_limit_period_secs, 1);
            assert!(!config.middleware.hsts_enabled);
            assert!(config.middleware.cors_allowed_origins.is_empty());
            assert!(config.middleware.trusted_proxies.is_empty());
            assert_eq!(config.middleware.client_max_concurrency, 32);
            assert_eq!(config.middleware.client_rate_limit_requests, 1024);
            assert_eq!(config.middleware.client_rate_limit_period_secs, 1);
            assert_eq!(config.middleware.client_max_entries, 16384);
            assert_eq!(config.middleware.client_idle_secs, 60);
            assert_eq!(config.middleware.slow_request_ms, 1000);
            assert!(
                config.ip_location_db_path.is_none(),
                "未配置时不查库，地点只区分本机 / 内网 / 外网"
            );
            assert!(config.ip_location_v6_db_path.is_none());
        },
    );
}

#[test]
fn ip_location_database_path_is_optional_and_trimmed() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("IP_LOCATION_DB_PATH", Some("  /data/ip2region_v4.xdb  ")),
            ("IP_LOCATION_V6_DB_PATH", Some("  /data/ip2region_v6.xdb  ")),
        ],
        || {
            let config = AppConfig::from_env().expect("配置应可加载");

            assert_eq!(
                config.ip_location_db_path.as_deref(),
                Some(Path::new("/data/ip2region_v4.xdb"))
            );
            assert_eq!(
                config.ip_location_v6_db_path.as_deref(),
                Some(Path::new("/data/ip2region_v6.xdb"))
            );
        },
    );

    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("IP_LOCATION_DB_PATH", Some("   ")),
            ("IP_LOCATION_V6_DB_PATH", Some("   ")),
        ],
        || {
            let config = AppConfig::from_env().expect("空白视为未配置");

            assert!(
                config.ip_location_db_path.is_none(),
                "显式写成空白表示不启用"
            );
            assert!(config.ip_location_v6_db_path.is_none());
        },
    );
}

#[test]
fn custom_overrides_are_applied() {
    with_env(
        &[
            ("APP_NAME", Some("custom-app")),
            ("APP_ENV", Some("staging")),
            ("SERVER_PORT", Some("9000")),
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("JWT_ACCESS_TOKEN_TTL_MINUTES", Some("30")),
            ("DATABASE_SQLX_LOGGING", Some("true")),
            ("LOG_ROTATION", Some("HOURLY")),
            ("LOG_MAX_LOG_FILES", Some("7")),
            ("LOG_BATCH_MAX_EVENTS", Some("100")),
            ("LOG_BATCH_FLUSH_INTERVAL_SECS", Some("5")),
        ],
        || {
            let config = AppConfig::from_env().expect("自定义配置应可加载");

            assert_eq!(config.app_name, "custom-app");
            assert_eq!(config.app_env, "staging");
            assert_eq!(config.server.port, 9000);
            assert_eq!(config.jwt.access_token_ttl_minutes, 30);
            assert!(config.database.sqlx_logging);
            assert_eq!(config.logging.max_log_files, 7);
            assert_eq!(config.logging.batch_max_events, 100);
            assert_eq!(config.logging.batch_flush_interval_secs, 5);
            assert!(rotation_debug(&config.logging.rotation).contains("hourly"));
        },
    );
}

#[test]
fn supported_log_rotation_variants_parse() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("LOG_ROTATION", Some("DAILY")),
        ],
        || {
            for variant in ["daily", "hourly", "minutely", "weekly", "never"] {
                set_var("LOG_ROTATION", &variant.to_uppercase());
                let config = AppConfig::from_env().expect("支持的轮转策略应可解析");
                assert!(rotation_debug(&config.logging.rotation).contains(variant));
            }

            for variant in ["Rotation::DAILY", "Rotation::WEEKLY"] {
                set_var("LOG_ROTATION", variant);
                let config = AppConfig::from_env().expect("兼容写法应可解析");
                let expected = variant.rsplit("::").next().unwrap_or("").to_lowercase();
                assert!(rotation_debug(&config.logging.rotation).contains(&expected));
            }
        },
    );
}

#[test]
fn production_accepts_custom_secret() {
    with_env(
        &[
            ("APP_ENV", Some("production")),
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
        ],
        || {
            let result = AppConfig::from_env();
            assert!(result.is_ok());
        },
    );
}

#[test]
fn production_rejects_example_jwt_secret() {
    with_env(
        &[
            ("APP_ENV", Some("production")),
            ("JWT_SECRET", Some(EXAMPLE_SECRET)),
        ],
        || {
            let result = AppConfig::from_env();
            assert!(result.is_err());
        },
    );
}

#[test]
fn missing_jwt_secret_is_rejected() {
    with_env(&[("JWT_SECRET", None)], || {
        let error = AppConfig::from_env().expect_err("缺少 JWT_SECRET 应被拒绝");
        assert!(
            error
                .to_string()
                .contains("missing required environment variable")
        );
    });
}

#[test]
fn short_jwt_secret_is_rejected() {
    with_env(&[("JWT_SECRET", Some("too-short"))], || {
        let error = AppConfig::from_env().expect_err("过短的 JWT_SECRET 应被拒绝");
        assert!(error.to_string().contains("at least 32 characters"));
    });
}

#[test]
fn non_positive_token_ttl_is_rejected() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("JWT_ACCESS_TOKEN_TTL_MINUTES", Some("0")),
        ],
        || {
            let error = AppConfig::from_env().expect_err("非正数 TTL 应被拒绝");
            assert!(error.to_string().contains("must be greater than 0"));
        },
    );
}

#[test]
fn zero_min_connections_is_rejected() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("DATABASE_MIN_CONNECTIONS", Some("0")),
        ],
        || {
            let error = AppConfig::from_env().expect_err("0 最小连接数应被拒绝");
            assert!(
                error
                    .to_string()
                    .contains("invalid database pool parameters")
            );
        },
    );
}

#[test]
fn min_connections_above_max_is_rejected() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("DATABASE_MIN_CONNECTIONS", Some("11")),
        ],
        || {
            let error = AppConfig::from_env().expect_err("最小连接数大于最大连接数应被拒绝");
            assert!(
                error
                    .to_string()
                    .contains("invalid database pool parameters")
            );
        },
    );
}

#[test]
fn zero_connect_timeout_is_rejected() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("DATABASE_CONNECT_TIMEOUT_SECS", Some("0")),
        ],
        || {
            let error = AppConfig::from_env().expect_err("0 连接超时应被拒绝");
            assert!(error.to_string().contains("timeout must be greater than 0"));
        },
    );
}

#[test]
fn zero_idle_timeout_is_rejected() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("DATABASE_IDLE_SECS", Some("0")),
        ],
        || {
            let error = AppConfig::from_env().expect_err("0 空闲超时应被拒绝");
            assert!(error.to_string().contains("timeout must be greater than 0"));
        },
    );
}

#[test]
fn zero_batch_params_are_rejected() {
    for key in ["LOG_BATCH_MAX_EVENTS", "LOG_BATCH_FLUSH_INTERVAL_SECS"] {
        with_env(
            &[("JWT_SECRET", Some(CUSTOM_SECRET)), (key, Some("0"))],
            || {
                let error = AppConfig::from_env().expect_err("批量参数为 0 应被拒绝");
                assert!(
                    error
                        .to_string()
                        .contains("log batch parameters must be greater than 0"),
                    "{key} 的报错信息不符合预期: {error}"
                );
            },
        );
    }
}

#[test]
fn zero_max_log_files_is_rejected() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("LOG_MAX_LOG_FILES", Some("0")),
        ],
        || {
            let error = AppConfig::from_env().expect_err("0 日志文件数应被拒绝");
            assert!(
                error
                    .to_string()
                    .contains("LOG_MAX_LOG_FILES must be greater than 0")
            );
        },
    );
}

#[test]
fn unsupported_log_rotation_is_rejected() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("LOG_ROTATION", Some("YEARLY")),
        ],
        || {
            let error = AppConfig::from_env().expect_err("不支持的轮转策略应被拒绝");
            assert!(
                error
                    .to_string()
                    .contains("unsupported log rotation strategy")
            );
        },
    );
}

#[test]
fn invalid_server_port_is_rejected() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("SERVER_PORT", Some("not-a-number")),
        ],
        || {
            let error = AppConfig::from_env().expect_err("非法端口应被拒绝");
            assert!(error.to_string().contains("failed to parse"));
        },
    );
}

#[test]
fn socket_addr_parses_valid_host() {
    let server = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
    };
    let address = server.socket_addr().expect("合法地址应可解析");
    assert_eq!(address.to_string(), "127.0.0.1:8080");
}

#[test]
fn socket_addr_rejects_invalid_host() {
    let server = ServerConfig {
        host: "invalid host".to_string(),
        port: 8080,
    };
    assert!(server.socket_addr().is_err());
}

#[test]
fn middleware_overrides_are_applied() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("MIDDLEWARE_REQUEST_TIMEOUT_SECS", Some("3")),
            ("MIDDLEWARE_MAX_BODY_BYTES", Some("1024")),
            ("MIDDLEWARE_MAX_CONCURRENCY", Some("8")),
            ("MIDDLEWARE_BACKPRESSURE_QUEUE", Some("4")),
            ("MIDDLEWARE_RATE_LIMIT_REQUESTS", Some("50")),
            ("MIDDLEWARE_RATE_LIMIT_PERIOD_SECS", Some("2")),
            ("MIDDLEWARE_HSTS_ENABLED", Some("true")),
            (
                "MIDDLEWARE_TRUSTED_PROXIES",
                Some("10.0.0.0/8, ::1, 127.0.0.1"),
            ),
            ("MIDDLEWARE_CLIENT_MAX_CONCURRENCY", Some("4")),
            ("MIDDLEWARE_CLIENT_RATE_LIMIT_REQUESTS", Some("20")),
            ("MIDDLEWARE_CLIENT_RATE_LIMIT_PERIOD_SECS", Some("2")),
            ("MIDDLEWARE_CLIENT_MAX_ENTRIES", Some("100")),
            ("MIDDLEWARE_CLIENT_IDLE_SECS", Some("30")),
            ("MIDDLEWARE_SLOW_REQUEST_MS", Some("250")),
            (
                "MIDDLEWARE_CORS_ALLOWED_ORIGINS",
                Some("https://a.example, https://b.example ,"),
            ),
        ],
        || {
            let config = AppConfig::from_env().expect("中间件覆盖项应可加载");

            assert_eq!(config.middleware.request_timeout_secs, 3);
            assert_eq!(config.middleware.max_body_bytes, 1024);
            assert_eq!(config.middleware.max_concurrency, 8);
            assert_eq!(config.middleware.backpressure_queue, 4);
            assert_eq!(config.middleware.rate_limit_requests, 50);
            assert_eq!(config.middleware.rate_limit_period_secs, 2);
            assert_eq!(config.middleware.trusted_proxies.len(), 3);
            assert_eq!(config.middleware.client_max_concurrency, 4);
            assert_eq!(config.middleware.client_rate_limit_requests, 20);
            assert_eq!(config.middleware.client_rate_limit_period_secs, 2);
            assert_eq!(config.middleware.client_max_entries, 100);
            assert_eq!(config.middleware.client_idle_secs, 30);
            assert_eq!(config.middleware.slow_request_ms, 250);
            assert!(config.middleware.hsts_enabled);
            assert_eq!(
                config.middleware.cors_allowed_origins,
                vec![
                    "https://a.example".to_string(),
                    "https://b.example".to_string()
                ]
            );
        },
    );
}

#[test]
fn invalid_cors_origin_is_rejected() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            (
                "MIDDLEWARE_CORS_ALLOWED_ORIGINS",
                Some("https://a.example\nX-Evil: 1"),
            ),
        ],
        || {
            let error = AppConfig::from_env().expect_err("非法来源应被拒绝");
            assert!(
                error
                    .to_string()
                    .contains("invalid MIDDLEWARE_CORS_ALLOWED_ORIGINS entry"),
                "报错信息不符合预期: {error}"
            );
        },
    );
}

#[test]
fn middleware_hsts_defaults_to_production_only() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("APP_ENV", Some("production")),
            ("MIDDLEWARE_HSTS_ENABLED", None),
        ],
        || {
            let config = AppConfig::from_env().expect("生产配置应可加载");
            assert!(config.middleware.hsts_enabled);
        },
    );
}

#[test]
fn zero_middleware_capacity_is_rejected() {
    for key in [
        "MIDDLEWARE_REQUEST_TIMEOUT_SECS",
        "MIDDLEWARE_MAX_BODY_BYTES",
        "MIDDLEWARE_MAX_CONCURRENCY",
        "MIDDLEWARE_BACKPRESSURE_QUEUE",
        "MIDDLEWARE_RATE_LIMIT_REQUESTS",
        "MIDDLEWARE_RATE_LIMIT_PERIOD_SECS",
        "MIDDLEWARE_CLIENT_MAX_CONCURRENCY",
        "MIDDLEWARE_CLIENT_RATE_LIMIT_REQUESTS",
        "MIDDLEWARE_CLIENT_RATE_LIMIT_PERIOD_SECS",
        "MIDDLEWARE_CLIENT_MAX_ENTRIES",
        "MIDDLEWARE_CLIENT_IDLE_SECS",
        "MIDDLEWARE_SLOW_REQUEST_MS",
    ] {
        with_env(
            &[("JWT_SECRET", Some(CUSTOM_SECRET)), (key, Some("0"))],
            || {
                let error = AppConfig::from_env().expect_err("容量参数为 0 应被拒绝");
                assert!(
                    error
                        .to_string()
                        .contains("MIDDLEWARE capacity parameters must be greater than 0"),
                    "{key} 的报错信息不符合预期: {error}"
                );
            },
        );
    }
}

#[test]
fn wildcard_trusted_proxies_cover_every_peer() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("MIDDLEWARE_TRUSTED_PROXIES", Some("*")),
        ],
        || {
            let config = AppConfig::from_env().expect("`*` 应可加载");
            let nets = &config.middleware.trusted_proxies;
            assert_eq!(nets.len(), 2);
            for peer in ["203.0.113.7", "2001:db8::1"] {
                let peer: IpAddr = peer.parse().expect("测试用地址应合法");
                assert!(
                    nets.iter().any(|net| net.contains(&peer)),
                    "`*` 应覆盖 {peer}"
                );
            }
        },
    );
}

/// 生产环境不能信任所有直连对端：转发头一旦完全采信，按 IP 的额度就形同虚设。
#[test]
fn production_rejects_trust_all_proxies() {
    for value in ["*", "0.0.0.0/0"] {
        with_env(
            &[
                ("APP_ENV", Some("production")),
                ("JWT_SECRET", Some(CUSTOM_SECRET)),
                ("MIDDLEWARE_TRUSTED_PROXIES", Some(value)),
            ],
            || {
                let error = AppConfig::from_env().expect_err("生产环境信任全部对端应被拒绝");
                assert!(
                    error
                        .to_string()
                        .contains("MIDDLEWARE_TRUSTED_PROXIES must not trust every peer"),
                    "{value} 的报错信息不符合预期: {error}"
                );
            },
        );
    }

    with_env(
        &[
            ("APP_ENV", Some("development")),
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("MIDDLEWARE_TRUSTED_PROXIES", Some("0.0.0.0/0")),
        ],
        || {
            assert!(AppConfig::from_env().is_ok());
        },
    );
}

#[test]
fn invalid_client_configuration_is_rejected() {
    for value in ["*,10.0.0.0/8", "10.0.0.1/99", "not-an-ip"] {
        with_env(
            &[
                ("JWT_SECRET", Some(CUSTOM_SECRET)),
                ("MIDDLEWARE_TRUSTED_PROXIES", Some(value)),
            ],
            || {
                assert!(
                    AppConfig::from_env()
                        .expect_err("非法可信代理配置应被拒绝")
                        .to_string()
                        .contains("MIDDLEWARE_TRUSTED_PROXIES")
                );
            },
        );
    }

    // 空闲保留期比限流窗口还短，会让刚被限流的来源被当成新来源重新计时。
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("MIDDLEWARE_CLIENT_IDLE_SECS", Some("1")),
            ("MIDDLEWARE_CLIENT_RATE_LIMIT_PERIOD_SECS", Some("2")),
        ],
        || {
            assert!(
                AppConfig::from_env()
                    .expect_err("空闲保留期小于窗口应被拒绝")
                    .to_string()
                    .contains("MIDDLEWARE_CLIENT_IDLE_SECS")
            );
        },
    );
}

/// 开发构建带着可直接指定身份的开发登录接口，production 必须用 release 构建运行。
#[test]
fn production_runtime_requires_release() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("APP_ENV", Some("production")),
        ],
        || {
            let config = AppConfig::from_env().expect("生产配置应可加载");
            assert_eq!(config.validate_runtime().is_err(), cfg!(debug_assertions));
        },
    );

    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("APP_ENV", Some("development")),
        ],
        || {
            assert!(
                AppConfig::from_env()
                    .expect("开发配置应可加载")
                    .validate_runtime()
                    .is_ok()
            );
        },
    );
}

/// 脚手架只支持 SQLite：迁移文件与补列逻辑都依赖 SQLite 语法，其他后端必须启动即拒。
#[test]
fn non_sqlite_database_url_is_rejected() {
    for url in [
        "postgres://localhost/app",
        "mysql://localhost/app",
        "app.db",
    ] {
        with_env(
            &[
                ("JWT_SECRET", Some(CUSTOM_SECRET)),
                ("DATABASE_URL", Some(url)),
            ],
            || {
                let error = AppConfig::from_env().expect_err("非 SQLite 连接串应被拒绝");
                assert!(
                    error.to_string().contains("only supports sqlite:// URLs"),
                    "{url} 的报错信息不符合预期: {error}"
                );
            },
        );
    }
}

/// 未设置 `SEED_USERS` 时播种三个内置演示账号，口令取模板默认值。
#[test]
fn seed_defaults_to_demo_accounts() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("SEED_ADMIN_PASSWORD", None),
            ("SEED_USERS", None),
        ],
        || {
            let config = AppConfig::from_env().expect("默认配置应可加载");

            assert_eq!(config.seed.admin_password, "admin123456");
            assert_eq!(config.seed.users.len(), 3);
            assert_eq!(config.seed.users[0].username, "admin");
            assert_eq!(config.seed.users[2].email, "heyj@example.com");
        },
    );
}

/// `SEED_USERS` 覆盖内置演示账号；条目两侧的空白与空条目直接忽略。
#[test]
fn seed_users_override_demo_accounts() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("SEED_ADMIN_PASSWORD", Some("custom-password")),
            (
                "SEED_USERS",
                Some(" ops_admin : 运维管理员 :ops@example.com:2 , "),
            ),
        ],
        || {
            let config = AppConfig::from_env().expect("自定义播种账号应可加载");

            assert_eq!(config.seed.admin_password, "custom-password");
            assert_eq!(config.seed.users.len(), 1);
            assert_eq!(config.seed.users[0].username, "ops_admin");
            assert_eq!(config.seed.users[0].nickname, "运维管理员");
            assert_eq!(config.seed.users[0].dept_id, 2);
        },
    );
}

/// `SEED_USERS` 格式错误时启动即拒，避免带着半个账号清单跑起来。
#[test]
fn invalid_seed_users_is_rejected() {
    for (value, expected) in [
        (
            "admin:陈立诚:chenlc@example.com",
            "expected username:nickname:email:deptId",
        ),
        ("admin:陈立诚:chenlc@example.com:abc", "deptId"),
        ("admin::chenlc@example.com:5", "must not be empty"),
        ("   ", "at least one account"),
    ] {
        with_env(
            &[
                ("JWT_SECRET", Some(CUSTOM_SECRET)),
                ("SEED_USERS", Some(value)),
            ],
            || {
                let error = AppConfig::from_env().expect_err("非法 SEED_USERS 应被拒绝");
                assert!(
                    error.to_string().contains(expected),
                    "`{value}` 的报错信息不符合预期: {error}"
                );
            },
        );
    }
}

/// Redis 连接串：无口令时原样使用，有口令时把口令放进 userinfo。
#[test]
fn redis_connection_url_embeds_password_only_when_configured() {
    let plain = RedisConfig {
        url: "redis://127.0.0.1:6379".to_string(),
        password: None,
    };
    assert_eq!(
        plain.connection_url().expect("无口令无需改写"),
        "redis://127.0.0.1:6379"
    );

    let secured = RedisConfig {
        url: "redis://cache.internal:6380".to_string(),
        password: Some("s3cret".to_string()),
    };
    assert_eq!(
        secured.connection_url().expect("有口令时改写连接串"),
        "redis://:s3cret@cache.internal:6380"
    );

    let tls = RedisConfig {
        url: "rediss://cache.internal:6380".to_string(),
        password: Some("s3cret".to_string()),
    };
    assert_eq!(
        tls.connection_url().expect("rediss 同样支持"),
        "rediss://:s3cret@cache.internal:6380"
    );
}

/// 口令里的 userinfo 非法字符必须转义，否则会被当成 fragment / 路径分隔。
#[test]
fn redis_connection_url_percent_encodes_password() {
    let config = RedisConfig {
        url: "redis://127.0.0.1:6379/2".to_string(),
        password: Some("p#ss/w:rd@x".to_string()),
    };
    let url = config.connection_url().expect("编码后可用");

    assert_eq!(url, "redis://:p%23ss%2Fw%3Ard%40x@127.0.0.1:6379/2");

    let parsed = url::Url::parse(&url).expect("编码后的连接串必须可解析");
    assert_eq!(parsed.host_str(), Some("127.0.0.1"));
    assert_eq!(parsed.port(), Some(6379));
    assert_eq!(parsed.password(), Some("p%23ss%2Fw%3Ard%40x"));
    assert_eq!(parsed.path(), "/2");
}

/// 没有 scheme 的 REDIS_URL 插不进口令，必须报错而不是拼出坏连接串。
#[test]
fn redis_connection_url_rejects_urls_without_scheme() {
    let config = RedisConfig {
        url: "127.0.0.1:6379".to_string(),
        password: Some("s3cret".to_string()),
    };
    let error = config.connection_url().expect_err("缺少 scheme 应报错");

    assert!(error.to_string().contains("invalid REDIS_URL"), "{error}");
}

/// 只接受 `redis` / `rediss`：其他 scheme 启动即拒。
#[test]
fn non_redis_scheme_is_rejected() {
    for url in [
        "http://127.0.0.1:6379",
        "redis-sentinel://127.0.0.1:6379",
        "127.0.0.1:6379",
    ] {
        with_env(
            &[
                ("JWT_SECRET", Some(CUSTOM_SECRET)),
                ("REDIS_URL", Some(url)),
            ],
            || {
                let error = AppConfig::from_env().expect_err("非 redis scheme 应被拒绝");

                assert!(
                    error.to_string().contains("unsupported REDIS_URL"),
                    "{url} 的报错信息不符合预期: {error}"
                );
            },
        );
    }
}

/// 播种口令是演示账号与新增用户的默认口令，空白即拒启动。
#[test]
fn blank_seed_admin_password_is_rejected() {
    with_env(
        &[
            ("JWT_SECRET", Some(CUSTOM_SECRET)),
            ("SEED_ADMIN_PASSWORD", Some("   ")),
        ],
        || {
            let error = AppConfig::from_env().expect_err("空白播种口令应被拒绝");

            assert!(
                error
                    .to_string()
                    .contains("SEED_ADMIN_PASSWORD must not be empty"),
                "{error}"
            );
        },
    );
}
