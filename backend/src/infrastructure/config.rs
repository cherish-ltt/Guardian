use std::{
    env,
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use anyhow::{Context, Result, anyhow};
use axum::http::HeaderValue;
use ipnet::IpNet;
use tracing_appender::rolling::Rotation;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_name: String,
    pub app_env: String,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
    pub redis: RedisConfig,
    pub seed: SeedConfig,
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

/// 演示账号与口令：来自 `.env`，不写进迁移文件与种子 SQL。
#[derive(Debug, Clone)]
pub struct SeedConfig {
    /// 演示账号的初始口令，同时作为新增用户未传口令时的默认值。
    pub admin_password: String,
    /// 播种的账号，按顺序取 ID `1..=N`。
    pub users: Vec<SeedUser>,
}

impl Default for SeedConfig {
    fn default() -> Self {
        Self {
            admin_password: DEFAULT_ADMIN_PASSWORD.to_string(),
            users: default_seed_users(),
        }
    }
}

/// 一个播种账号，对应 `SEED_USERS` 里的一项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedUser {
    pub username: String,
    pub nickname: String,
    pub email: String,
    pub dept_id: i64,
}

const DEFAULT_ADMIN_PASSWORD: &str = "admin123456";

/// 内置演示账号：`SEED_USERS` 未设置时使用。
fn default_seed_users() -> Vec<SeedUser> {
    [
        ("admin", "陈立诚", "chenlc@example.com", 5),
        ("linjh", "林嘉豪", "linjh@example.com", 5),
        ("heyj", "何雅静", "heyj@example.com", 6),
    ]
    .into_iter()
    .map(|(username, nickname, email, dept_id)| SeedUser {
        username: username.to_string(),
        nickname: nickname.to_string(),
        email: email.to_string(),
        dept_id,
    })
    .collect()
}

/// 解析 `SEED_USERS`：`用户名:昵称:邮箱:部门ID`，多个账号用逗号分隔。
///
/// 邮箱里不允许出现冒号，所以按 `:` 切分是安全的；字段数不对或部门 ID 不是整数时拒绝启动。
fn parse_seed_users(raw: &str) -> Result<Vec<SeedUser>> {
    let mut users = Vec::new();
    for entry in raw
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let fields: Vec<&str> = entry.split(':').map(str::trim).collect();
        let [username, nickname, email, dept_id] = fields[..] else {
            return Err(anyhow!(
                "invalid SEED_USERS entry `{entry}`: expected username:nickname:email:deptId"
            ));
        };
        if username.is_empty() || nickname.is_empty() || email.is_empty() {
            return Err(anyhow!(
                "invalid SEED_USERS entry `{entry}`: username, nickname and email must not be empty"
            ));
        }
        let dept_id = dept_id
            .parse::<i64>()
            .map_err(|error| anyhow!("invalid SEED_USERS entry `{entry}`: deptId {error}"))?;

        users.push(SeedUser {
            username: username.to_string(),
            nickname: nickname.to_string(),
            email: email.to_string(),
            dept_id,
        });
    }

    if users.is_empty() {
        return Err(anyhow!("SEED_USERS must contain at least one account"));
    }

    Ok(users)
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
    /// 从进入请求保护层到响应头生成（含全局排队）的超时，响应流发送不在范围内。
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
    /// 显式可信代理：只有匹配这些 IP/CIDR 的直连对端，其 `x-forwarded-for` 才被采信。
    pub trusted_proxies: Vec<IpNet>,
    /// 单个来源 IP 的在途请求上限（含全局排队）。
    pub client_max_concurrency: usize,
    /// 单个来源 IP 在每个固定窗口内允许的请求数。
    pub client_rate_limit_requests: u64,
    /// 客户端限流的固定窗口长度（秒）。
    pub client_rate_limit_period_secs: u64,
    /// 客户端注册表容量上限，满了以后新来源直接拒绝。
    pub client_max_entries: usize,
    /// 客户端注册表的空闲保留期（秒），不得小于限流窗口。
    pub client_idle_secs: u64,
    /// 超过该耗时的请求额外输出阶段诊断日志（毫秒）。
    pub slow_request_ms: u64,
}

impl AppConfig {
    /// 运行期前置校验：`main` 与 `create_app` 都必须调用，嵌入自定义入口也不能绕过。
    pub fn validate_runtime(&self) -> Result<()> {
        if self.app_env.eq_ignore_ascii_case("production") && cfg!(debug_assertions) {
            return Err(anyhow!(
                "production requires a release build: debug authentication routes are unsafe"
            ));
        }
        Ok(())
    }

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

        let seed = SeedConfig {
            admin_password: get_env_or("SEED_ADMIN_PASSWORD", DEFAULT_ADMIN_PASSWORD),
            users: match env::var("SEED_USERS") {
                Ok(raw) => parse_seed_users(&raw)?,
                Err(_) => default_seed_users(),
            },
        };
        if seed.admin_password.trim().is_empty() {
            return Err(anyhow!("SEED_ADMIN_PASSWORD must not be empty"));
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

        let trusted_proxies = parse_proxies(&get_env_or("MIDDLEWARE_TRUSTED_PROXIES", ""))?;
        // 信任所有对端等于把转发头交给客户端自报，按来源 IP 分配的额度随之失效；
        // 前缀长度为 0 的网段（`*` 与手写的 0.0.0.0/0）在生产环境直接拒绝启动。
        if app_env.eq_ignore_ascii_case("production")
            && trusted_proxies.iter().any(|net| net.prefix_len() == 0)
        {
            return Err(anyhow!(
                "MIDDLEWARE_TRUSTED_PROXIES must not trust every peer in production"
            ));
        }

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
            trusted_proxies,
            client_max_concurrency: parse_env_or("MIDDLEWARE_CLIENT_MAX_CONCURRENCY", 32_usize)?,
            client_rate_limit_requests: parse_env_or(
                "MIDDLEWARE_CLIENT_RATE_LIMIT_REQUESTS",
                1024_u64,
            )?,
            client_rate_limit_period_secs: parse_env_or(
                "MIDDLEWARE_CLIENT_RATE_LIMIT_PERIOD_SECS",
                1_u64,
            )?,
            client_max_entries: parse_env_or("MIDDLEWARE_CLIENT_MAX_ENTRIES", 16384_usize)?,
            client_idle_secs: parse_env_or("MIDDLEWARE_CLIENT_IDLE_SECS", 60_u64)?,
            slow_request_ms: parse_env_or("MIDDLEWARE_SLOW_REQUEST_MS", 1000_u64)?,
        };

        if middleware.request_timeout_secs == 0
            || middleware.max_body_bytes == 0
            || middleware.max_concurrency == 0
            || middleware.backpressure_queue == 0
            || middleware.rate_limit_requests == 0
            || middleware.rate_limit_period_secs == 0
            || middleware.client_max_concurrency == 0
            || middleware.client_rate_limit_requests == 0
            || middleware.client_rate_limit_period_secs == 0
            || middleware.client_max_entries == 0
            || middleware.client_idle_secs == 0
            || middleware.slow_request_ms == 0
        {
            return Err(anyhow!(
                "MIDDLEWARE capacity parameters must be greater than 0"
            ));
        }

        // 空闲项在窗口内不能清理，否则刚被限流的来源会被当成新来源重新计时。
        if middleware.client_idle_secs < middleware.client_rate_limit_period_secs {
            return Err(anyhow!(
                "MIDDLEWARE_CLIENT_IDLE_SECS must be at least the client rate limit period"
            ));
        }

        Ok(Self {
            app_name,
            app_env,
            server,
            database,
            jwt,
            redis,
            seed,
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

/// `*` 的展开形式：包含判断只看网段，同时信任全部 IPv4 与 IPv6 即等价于信任所有直连对端。
const TRUST_ALL_PROXIES: [&str; 2] = ["0.0.0.0/0", "::/0"];

/// 解析可信代理列表：接受 CIDR 或单个 IP，`*` 表示信任所有直连对端。
///
/// `*` 与具体条目混写会拒绝启动，避免把「多信任一个网段」误读成放行全部。
fn parse_proxies(raw: &str) -> Result<Vec<IpNet>> {
    let entries: Vec<&str> = raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect();
    match entries.as_slice() {
        ["*"] => TRUST_ALL_PROXIES
            .iter()
            .map(|net| parse_proxy(net))
            .collect(),
        entries if entries.contains(&"*") => Err(anyhow!(
            "MIDDLEWARE_TRUSTED_PROXIES: `*` cannot be combined with explicit entries"
        )),
        entries => entries.iter().map(|value| parse_proxy(value)).collect(),
    }
}

fn parse_proxy(value: &str) -> Result<IpNet> {
    value
        .parse::<IpNet>()
        .or_else(|_| value.parse::<IpAddr>().map(IpNet::from))
        .map_err(|_| anyhow!("invalid MIDDLEWARE_TRUSTED_PROXIES entry: {value}"))
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
