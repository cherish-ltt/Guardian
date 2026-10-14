use std::{env, net::SocketAddr, str::FromStr};

use anyhow::{Context, Result, anyhow};
use axum::http::HeaderValue;
use tracing_appender::rolling::Rotation;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_name: String,
    pub app_env: String,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
    pub redis: RedisConfig,
    pub logging: LoggingConfig,
    pub middleware: MiddlewareConfig,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl ServerConfig {
    pub fn socket_addr(&self) -> Result<SocketAddr> {
        let address = format!("{}:{}", self.host, self.port);
        address
            .parse::<SocketAddr>()
            .with_context(|| format!("failed to parse server listen address: {address}"))
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub min_connections: u32,
    pub max_connections: u32,
    pub connect_timeout_secs: u64,
    pub idle_secs: u64,
    pub sqlx_logging: bool,
}

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub issuer: String,
    pub audience: String,
    pub access_token_ttl_minutes: i64,
}

/// 缓存连接配置：验证码与登录会话撤销都写在这里。
#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
    /// 空字符串视为「未设置密码」。
    pub password: Option<String>,
}

impl RedisConfig {
    /// 带认证信息的连接串；`redis` 客户端要求密码作为 userinfo 的一部分。
    pub fn connection_url(&self) -> Result<String> {
        let Some(password) = self.password.as_deref() else {
            return Ok(self.url.clone());
        };

        let rest = self
            .url
            .split_once("://")
            .map(|(_, rest)| rest)
            .ok_or_else(|| anyhow!("invalid REDIS_URL: {}", self.url))?;
        let scheme = &self.url[..self.url.len() - rest.len() - 3];

        Ok(format!("{scheme}://:{password}@{rest}"))
    }
}

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub filter: String,
    pub utc_offset_hour: i8,
    pub utc_offset_minute: i8,
    pub utc_offset_second: i8,
    pub filename_prefix: String,
    pub filename_suffix: String,
    pub rotation: Rotation,
    pub max_log_files: usize,
    pub out_dir: String,
    /// 文件日志批量落盘：缓冲满 N 条触发一次写入。
    pub batch_max_events: usize,
    /// 文件日志批量落盘：最长 N 秒强制写入一次。
    pub batch_flush_interval_secs: u64,
}

/// HTTP 中间件容量参数，全部集中在 `.env`，业务代码不得内联这些数字。
#[derive(Debug, Clone)]
pub struct MiddlewareConfig {
    /// 单个请求从进入服务到响应完成的整体超时（秒）。
    pub request_timeout_secs: u64,
    /// 请求体最大字节数。
    pub max_body_bytes: usize,
    /// 同时进入 handler 的最大请求数。
    pub max_concurrency: usize,
    /// 并发已满时允许排队等待的请求数，超出部分快速拒绝。
    pub backpressure_queue: usize,
    /// 全局限流窗口内允许的请求数。
    pub rate_limit_requests: u64,
    /// 全局限流窗口长度（秒）。
    pub rate_limit_period_secs: u64,
    /// 是否下发 HSTS 响应头，默认跟随 `APP_ENV`。
    pub hsts_enabled: bool,
    /// 允许跨域访问的来源白名单（如 `https://a.example`）。
    ///
    /// 空列表表示不下发任何 CORS 放行头；单元素为 `*` 表示放行所有来源。
    /// 条目在 `AppConfig::from_env` 阶段校验为合法 header 值。
    pub cors_allowed_origins: Vec<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let app_name = get_env_or("APP_NAME", "guardian-backend");
        let app_env = get_env_or("APP_ENV", "development");

        let server = ServerConfig {
            host: get_env_or("SERVER_HOST", "127.0.0.1"),
            port: parse_env_or("SERVER_PORT", 8080u16)?,
        };

        let database = DatabaseConfig {
            url: get_env_or("DATABASE_URL", "sqlite://guardian.db?mode=rwc"),
            min_connections: parse_env_or("DATABASE_MIN_CONNECTIONS", 1u32)?,
            max_connections: parse_env_or("DATABASE_MAX_CONNECTIONS", 10u32)?,
            connect_timeout_secs: parse_env_or("DATABASE_CONNECT_TIMEOUT_SECS", 8u64)?,
            idle_secs: parse_env_or("DATABASE_IDLE_SECS", 60u64)?,
            sqlx_logging: parse_env_or("DATABASE_SQLX_LOGGING", false)?,
        };

        let jwt = JwtConfig {
            secret: get_required_env("JWT_SECRET")?,
            issuer: get_env_or("JWT_ISSUER", "guardian-backend"),
            audience: get_env_or("JWT_AUDIENCE", "guardian-clients"),
            access_token_ttl_minutes: parse_env_or("JWT_ACCESS_TOKEN_TTL_MINUTES", 120i64)?,
        };

        if jwt.secret.len() < 32 {
            return Err(anyhow!("JWT_SECRET must be at least 32 characters long"));
        }
        if app_env.eq_ignore_ascii_case("production")
            && jwt.secret == "change-me-to-a-random-string-with-at-least-32-characters"
        {
            return Err(anyhow!(
                "example JWT_SECRET is not allowed in production environment"
            ));
        }
        if jwt.access_token_ttl_minutes <= 0 {
            return Err(anyhow!(
                "JWT_ACCESS_TOKEN_TTL_MINUTES must be greater than 0"
            ));
        }
        if database.max_connections == 0
            || database.min_connections == 0
            || database.min_connections > database.max_connections
        {
            return Err(anyhow!(
                "invalid database pool parameters: expected 0 < min_connections <= max_connections"
            ));
        }
        // 迁移文件与补列逻辑都依赖 SQLite 语法（pragma_table_info / sqlite 后端），
        // 其他后端会在启动时以难以定位的方式失败，所以在这里直接拒绝。
        let scheme = database.url.split(':').next().unwrap_or_default();
        if scheme != "sqlite" {
            return Err(anyhow!(
                "unsupported DATABASE_URL scheme `{scheme}`: this service only supports sqlite:// URLs"
            ));
        }
        if database.connect_timeout_secs == 0 || database.idle_secs == 0 {
            return Err(anyhow!(
                "database connect timeout and idle timeout must be greater than 0"
            ));
        }

        let redis = RedisConfig {
            url: get_env_or("REDIS_URL", "redis://127.0.0.1:6379"),
            password: env::var("REDIS_PASSWORD")
                .ok()
                .filter(|value| !value.trim().is_empty()),
        };
        let redis_scheme = redis.url.split_once("://").map(|(scheme, _)| scheme);
        if !matches!(redis_scheme, Some("redis" | "rediss")) {
            return Err(anyhow!("unsupported REDIS_URL: {}", redis.url));
        }

        let max_log_files = parse_env_or("LOG_MAX_LOG_FILES", 60_usize)?;
        if max_log_files == 0 {
            return Err(anyhow!("LOG_MAX_LOG_FILES must be greater than 0"));
        }

        let batch_max_events = parse_env_or("LOG_BATCH_MAX_EVENTS", 50_usize)?;
        let batch_flush_interval_secs = parse_env_or("LOG_BATCH_FLUSH_INTERVAL_SECS", 3_u64)?;
        if batch_max_events == 0 || batch_flush_interval_secs == 0 {
            return Err(anyhow!("log batch parameters must be greater than 0"));
        }

        let logging = LoggingConfig {
            filter: get_env_or("LOG_FILTER", "info,tower_http=info"),
            utc_offset_hour: parse_env_or("LOG_UTC_OFFSET_HOUR", 0_i8)?,
            utc_offset_minute: parse_env_or("LOG_UTC_OFFSET_MINUTE", 0_i8)?,
            utc_offset_second: parse_env_or("LOG_UTC_OFFSET_SECOND", 0_i8)?,
            filename_prefix: get_env_or("LOG_FILENAME_PREFIX", "app"),
            filename_suffix: get_env_or("LOG_FILENAME_SUFFIX", "log"),
            rotation: match get_env_or("LOG_ROTATION", "DAILY").as_str() {
                "DAILY" | "Rotation::DAILY" => Rotation::DAILY,
                "HOURLY" | "Rotation::HOURLY" => Rotation::HOURLY,
                "MINUTELY" | "Rotation::MINUTELY" => Rotation::MINUTELY,
                "NEVER" | "Rotation::NEVER" => Rotation::NEVER,
                "WEEKLY" | "Rotation::WEEKLY" => Rotation::WEEKLY,
                value => return Err(anyhow!("unsupported log rotation strategy: {value}")),
            },
            max_log_files,
            out_dir: get_env_or("LOG_OUT_DIR", "./log"),
            batch_max_events,
            batch_flush_interval_secs,
        };

        let middleware = MiddlewareConfig {
            request_timeout_secs: parse_env_or("MIDDLEWARE_REQUEST_TIMEOUT_SECS", 10_u64)?,
            max_body_bytes: parse_env_or("MIDDLEWARE_MAX_BODY_BYTES", 2 * 1024 * 1024_usize)?,
            max_concurrency: parse_env_or("MIDDLEWARE_MAX_CONCURRENCY", 256_usize)?,
            backpressure_queue: parse_env_or("MIDDLEWARE_BACKPRESSURE_QUEUE", 256_usize)?,
            rate_limit_requests: parse_env_or("MIDDLEWARE_RATE_LIMIT_REQUESTS", 32768_u64)?,
            rate_limit_period_secs: parse_env_or("MIDDLEWARE_RATE_LIMIT_PERIOD_SECS", 1_u64)?,
            hsts_enabled: parse_env_or(
                "MIDDLEWARE_HSTS_ENABLED",
                app_env.eq_ignore_ascii_case("production"),
            )?,
            cors_allowed_origins: parse_origins(&get_env_or(
                "MIDDLEWARE_CORS_ALLOWED_ORIGINS",
                "",
            ))?,
        };

        if middleware.request_timeout_secs == 0
            || middleware.max_body_bytes == 0
            || middleware.max_concurrency == 0
            || middleware.backpressure_queue == 0
            || middleware.rate_limit_requests == 0
            || middleware.rate_limit_period_secs == 0
        {
            return Err(anyhow!(
                "MIDDLEWARE capacity parameters must be greater than 0"
            ));
        }

        Ok(Self {
            app_name,
            app_env,
            server,
            database,
            jwt,
            redis,
            logging,
            middleware,
        })
    }
}

fn get_required_env(key: &str) -> Result<String> {
    env::var(key).with_context(|| format!("missing required environment variable: {key}"))
}

fn get_env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

/// 把逗号分隔的来源白名单解析成列表，非法条目直接拒绝启动。
fn parse_origins(raw: &str) -> Result<Vec<String>> {
    let mut origins = Vec::new();
    for origin in raw
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        HeaderValue::from_str(origin).map_err(|error| {
            anyhow!("invalid MIDDLEWARE_CORS_ALLOWED_ORIGINS entry {origin}: {error}")
        })?;
        origins.push(origin.to_string());
    }

    Ok(origins)
}

fn parse_env_or<T>(key: &str, default: T) -> Result<T>
where
    T: FromStr + Clone,
    <T as FromStr>::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(value) => value
            .parse::<T>()
            .map_err(|error| anyhow!("failed to parse environment variable {key}: {error}")),
        Err(_) => Ok(default),
    }
}
