use std::{
    io::{self, Write},
    mem,
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::{Context, Result};
use time::{UtcOffset, macros::format_description};
use tracing::{Event, Subscriber};
use tracing_appender::rolling::RollingFileAppender;
use tracing_subscriber::{
    EnvFilter,
    fmt::{
        self, FmtContext, FormatFields, MakeWriter,
        format::{Format, FormatEvent, Full, Writer},
        time::{FormatTime, OffsetTime},
    },
    layer::SubscriberExt,
    registry::LookupSpan,
    util::SubscriberInitExt,
};

use crate::infrastructure::config::AppConfig;

/// 初始化全局日志。
///
/// 日志初始化应尽量早，这样启动阶段的配置错误与数据库错误也能被记录下来。
///
/// 文件层采用分批写入：事件先进入内存缓冲，满 `LOG_BATCH_MAX_EVENTS` 条或每
/// `LOG_BATCH_FLUSH_INTERVAL_SECS` 秒落盘一次，避免高频场景下逐条写文件浪费 CPU；
/// 控制台层保持实时输出。返回的 `BatchGuard` 在 drop 时停止刷盘线程并落盘剩余缓冲。
pub fn init(config: &AppConfig) -> Result<BatchGuard> {
    // 1. 配置时区和时间格式（东八区 UTC+8）
    let offset = UtcOffset::from_hms(
        config.logging.utc_offset_hour,
        config.logging.utc_offset_minute,
        config.logging.utc_offset_second,
    )
    .context("invalid log timezone offset configuration")?;
    let timer = OffsetTime::new(
        offset,
        // 自定义时间格式：年-月-日 时:分:秒.毫秒
        format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:5]"),
    );

    // 2. 配置日志过滤级别（来自 .env）。先于文件句柄校验，配置非法时不留下任何副作用。
    let env_filter = EnvFilter::try_new(config.logging.filter.clone())
        .with_context(|| format!("invalid LOG_FILTER: {}", config.logging.filter))?;

    // 3. 配置日志滚动策略
    let file_appender = RollingFileAppender::builder()
        // 按天切割（同时支持按大小切割，见下文）
        .rotation(config.logging.rotation.clone())
        // 日志文件名前缀（生成的文件如：app.2026-04-22.log）
        .filename_prefix(config.logging.filename_prefix.clone())
        // 日志文件名后缀
        .filename_suffix(config.logging.filename_suffix.clone())
        // 最多保留 60 个日志文件（按时间倒序保留最近 60 天）
        .max_log_files(config.logging.max_log_files)
        // .latest_symlink("app.latest.log") // 需要对应平台权限
        .build(config.logging.out_dir.clone())
        .context("failed to create log file appender")?;

    // 4. 配置格式化层（输出到文件，分批写入）
    let file_writer = BatchFileWriter::new(
        file_appender,
        config.logging.batch_max_events,
        Duration::from_secs(config.logging.batch_flush_interval_secs),
    );
    let fmt_layer = fmt::layer()
        // 先格式化整行再写入，保证每条日志恰好触发一次底层写入，分批计数才精确
        .event_format(LineFormat::new(
            fmt::format()
                .with_ansi(false) // 关闭文件中的 ANSI 颜色码（避免乱码）
                .with_timer(timer.clone()),
        ))
        .with_writer(file_writer.clone());

    // 5. 控制台输出层（带颜色，实时输出）
    let console_layer = fmt::layer().with_writer(std::io::stdout).with_timer(timer);

    // 6. 初始化全局订阅者
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(console_layer)
        .try_init()
        .context("failed to initialize global log subscriber")?;

    // 7. 启动定时刷盘线程
    Ok(file_writer.start_flusher())
}

/// 打印本次启动实际生效的配置，便于事后追溯启动那一刻的参数。
///
/// 以手绘 ASCII 列表框输出，按配置分组；多行消息由 fmt 层逐行打印，
/// 控制台与文件层行为一致。数据库连接串中的口令与 JWT 密钥只输出脱敏结果。
pub fn log_startup_config(config: &AppConfig) {
    const WIDTH: usize = 64;
    let equals = format!("+{}+", "=".repeat(WIDTH));
    let dashes = format!("+{}+", "-".repeat(WIDTH));
    let mut lines = vec![
        String::new(),
        equals.clone(),
        format!("|{:^width$}|", "", width = WIDTH),
        format!("|{:^width$}|", "startup configuration", width = WIDTH),
        format!("|{:^width$}|", "", width = WIDTH),
        equals,
    ];

    let database = &config.database;
    let jwt = &config.jwt;
    let logger = &config.logging;
    let middleware = &config.middleware;
    let redis = &config.redis;
    let cache = &config.cache;
    let seed = &config.seed;
    let server = &config.server;

    push_section(
        &mut lines,
        &dashes,
        "application",
        vec![
            format!("app_name={}", config.app_name),
            format!("app_env={}", config.app_env),
            format!("version={}", env!("CARGO_PKG_VERSION")),
        ],
    );
    push_section(
        &mut lines,
        &dashes,
        "server",
        vec![
            format!("host={}", server.host),
            format!("port={}", server.port),
        ],
    );
    push_section(
        &mut lines,
        &dashes,
        "database",
        vec![
            format!("url={}", mask_url(&database.url)),
            format!("min_connections={}", database.min_connections),
            format!("max_connections={}", database.max_connections),
            format!("connect_timeout_secs={}", database.connect_timeout_secs),
            format!("idle_secs={}", database.idle_secs),
            format!("sqlx_logging={}", database.sqlx_logging),
        ],
    );
    push_section(
        &mut lines,
        &dashes,
        "jwt",
        vec![
            format!("secret={}", mask_secret(&jwt.secret)),
            format!("issuer={}", jwt.issuer),
            format!("audience={}", jwt.audience),
            format!("access_token_ttl_minutes={}", jwt.access_token_ttl_minutes),
        ],
    );
    push_section(
        &mut lines,
        &dashes,
        "two_factor",
        vec![format!(
            "two_factor_key={}",
            if config.two_factor_key.is_some() {
                "(configured)"
            } else {
                "(not set)"
            }
        )],
    );
    push_section(
        &mut lines,
        &dashes,
        "redis",
        vec![
            format!("url={}", redis.url),
            format!(
                "password={}",
                redis
                    .password
                    .as_deref()
                    .map(mask_secret)
                    .unwrap_or_else(|| "(not set)".to_string())
            ),
        ],
    );
    push_section(
        &mut lines,
        &dashes,
        "cache",
        vec![
            format!("ttl_secs={}", cache.ttl_secs),
            format!("negative_ttl_secs={}", cache.negative_ttl_secs),
            format!("timeout_ms={}", cache.timeout_ms),
            format!("max_loads={}", cache.max_loads),
            format!("max_entries={}", cache.max_entries),
            format!("max_value_bytes={}", cache.max_value_bytes),
        ],
    );
    push_section(&mut lines, &dashes, "seed", {
        let mut rows = vec![
            format!("admin_password={}", mask_secret(&seed.admin_password)),
            format!("users={}", seed.users.len()),
        ];
        for (index, user) in seed.users.iter().enumerate() {
            let position = index + 1;
            rows.push(format!("user[{position}].username={}", user.username));
            rows.push(format!("user[{position}].nickname={}", user.nickname));
            rows.push(format!("user[{position}].email={}", user.email));
            rows.push(format!("user[{position}].dept_id={}", user.dept_id));
        }
        rows
    });
    push_section(
        &mut lines,
        &dashes,
        "ip_location",
        vec![
            format!(
                "ip_location_db_path={}",
                config
                    .ip_location_db_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "(not set)".to_string())
            ),
            format!(
                "ip_location_v6_db_path={}",
                config
                    .ip_location_v6_db_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "(not set)".to_string())
            ),
        ],
    );
    push_section(
        &mut lines,
        &dashes,
        "logging",
        vec![
            format!("filter={}", logger.filter),
            format!("utc_offset_hour={}", logger.utc_offset_hour),
            format!("utc_offset_minute={}", logger.utc_offset_minute),
            format!("utc_offset_second={}", logger.utc_offset_second),
            format!("out_dir={}", logger.out_dir),
            format!("filename_prefix={}", logger.filename_prefix),
            format!("filename_suffix={}", logger.filename_suffix),
            format!("rotation={:?}", logger.rotation),
            format!("max_log_files={}", logger.max_log_files),
            format!("batch_max_events={}", logger.batch_max_events),
            format!(
                "batch_flush_interval_secs={}",
                logger.batch_flush_interval_secs
            ),
        ],
    );
    push_section(
        &mut lines,
        &dashes,
        "middleware",
        vec![
            format!("request_timeout_secs={}", middleware.request_timeout_secs),
            format!("max_body_bytes={}", middleware.max_body_bytes),
            format!("max_concurrency={}", middleware.max_concurrency),
            format!("backpressure_queue={}", middleware.backpressure_queue),
            format!("rate_limit_requests={}", middleware.rate_limit_requests),
            format!(
                "rate_limit_period_secs={}",
                middleware.rate_limit_period_secs
            ),
            format!("hsts_enabled={}", middleware.hsts_enabled),
            format!("cors_allowed_origins={:?}", middleware.cors_allowed_origins),
            format!("trusted_proxies={:?}", middleware.trusted_proxies),
            format!(
                "client_max_concurrency={}",
                middleware.client_max_concurrency
            ),
            format!(
                "client_rate_limit_requests={}",
                middleware.client_rate_limit_requests
            ),
            format!(
                "client_rate_limit_period_secs={}",
                middleware.client_rate_limit_period_secs
            ),
            format!("client_max_entries={}", middleware.client_max_entries),
            format!("client_idle_secs={}", middleware.client_idle_secs),
            format!("slow_request_ms={}", middleware.slow_request_ms),
        ],
    );

    tracing::info!("{}", lines.join("\n"));
}

/// 追加一个配置分节：标题行、键值行与分隔线。
fn push_section(lines: &mut Vec<String>, dashes: &str, title: &str, rows: Vec<String>) {
    lines.push(format!("| [{title}]"));
    for row in rows {
        lines.push(format!("|   {row}"));
    }
    lines.push(dashes.to_string());
}

/// 把默认 `Format` 的输出先聚合成一行再交给底层 writer。
///
/// tracing 的 fmt 层对一条事件会多次调用 writer（时间戳、级别、字段等分段写入），
/// 直接计数会失真；这里先格式化为完整一行，保证每次事件恰好触发一次 `write`，
/// 分批写入器才能以「条数」精确触发落盘。
#[derive(Clone)]
struct LineFormat<T>(Format<Full, T>);

impl<T> LineFormat<T> {
    fn new(inner: Format<Full, T>) -> Self {
        Self(inner)
    }
}

impl<S, T, N> FormatEvent<S, N> for LineFormat<T>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
    T: FormatTime,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let mut line = String::new();
        self.0.format_event(ctx, Writer::new(&mut line), event)?;
        writer.write_str(&line)
    }
}

/// 文件日志分批写入器：事件先进入内存缓冲，满 `batch_max_events` 条或每
/// `flush_interval` 秒写一次文件，降低高频场景下逐条写文件的 CPU 开销。
///
/// 实现 `MakeWriter` 供 tracing fmt 层使用；每次事件由 `make_writer` 创建
/// 一个 [`BatchWriter`] 追加缓冲。
#[derive(Clone)]
pub struct BatchFileWriter {
    inner: Arc<Mutex<RollingFileAppender>>,
    state: Arc<Mutex<BatchState>>,
    batch_max_events: usize,
    flush_interval: Duration,
}

/// 批量缓冲状态：已格式化的日志字节与累计事件条数。
#[derive(Default)]
struct BatchState {
    buf: Vec<u8>,
    events: usize,
}

impl BatchFileWriter {
    fn new(inner: RollingFileAppender, batch_max_events: usize, flush_interval: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(inner)),
            state: Arc::new(Mutex::new(BatchState::default())),
            batch_max_events,
            flush_interval,
        }
    }

    /// 把缓冲内容一次性写入文件并清空。
    ///
    /// 磁盘 IO 在锁外进行：先在锁内取走缓冲并重置计数，再释放锁写文件。
    /// 否则定时刷盘或满批落盘时，所有写日志的线程会被写盘时间串行阻塞。
    fn flush(&self) -> io::Result<()> {
        let (mut pending, pending_events) = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.buf.is_empty() {
                return Ok(());
            }
            (
                mem::take(&mut state.buf),
                mem::replace(&mut state.events, 0),
            )
        };

        let write_result = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner.write_all(&pending).and_then(|()| inner.flush())
        };

        if let Err(error) = write_result {
            // 写盘失败时把内容放回缓冲队首，避免丢日志。
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            pending.extend_from_slice(&state.buf);
            state.buf = pending;
            state.events += pending_events;
            return Err(error);
        }

        Ok(())
    }

    /// 启动后台定时刷盘线程，返回持有停止信号与线程句柄的 guard。
    fn start_flusher(&self) -> BatchGuard {
        let (stop, rx) = mpsc::channel::<()>();
        let flusher = self.clone();
        let interval = flusher.flush_interval;
        let thread = thread::spawn(move || {
            while rx.recv_timeout(interval).is_err() {
                let _ = flusher.flush();
            }
        });
        BatchGuard {
            stop: Some(stop),
            thread: Some(thread),
            writer: self.clone(),
        }
    }
}

impl<'a> MakeWriter<'a> for BatchFileWriter {
    type Writer = BatchWriter;

    fn make_writer(&self) -> Self::Writer {
        BatchWriter(self.clone())
    }
}

/// 每次事件由 fmt 层创建一次；`write` 追加到共享缓冲，满 `batch_max_events` 条时落盘。
pub struct BatchWriter(BatchFileWriter);

impl Write for BatchWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut state = self.0.state.lock().unwrap_or_else(|e| e.into_inner());
        state.buf.extend_from_slice(buf);
        state.events += 1;
        if state.events >= self.0.batch_max_events {
            drop(state);
            self.0.flush()?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

/// `init` 返回的守卫：drop 时停止定时刷盘线程并落盘剩余缓冲。
pub struct BatchGuard {
    stop: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
    writer: BatchFileWriter,
}

impl Drop for BatchGuard {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        if let Err(e) = self.writer.flush() {
            tracing::error!("failed to flush logs on guard drop: {e}");
        }
    }
}

/// 隐藏连接串里的口令，保留其余结构以便核对目标库。
///
/// 同时处理 `scheme://user:password@host/db` 与 `scheme://host/db?password=xxx`
/// 两种写法：前者按最后一个 `@` 切分（口令里含 `@` 或 `:` 时也不会漏出），
/// 后者按参数名替换口令值。
fn mask_url(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };
    let end = rest.find(['/', '?']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);

    let authority = match authority.rsplit_once('@') {
        Some((userinfo, host)) => match userinfo.split_once(':') {
            Some((user, _)) => format!("{user}:***@{host}"),
            None => authority.to_owned(),
        },
        None => authority.to_owned(),
    };

    format!("{scheme}://{authority}{}", mask_query_password(tail))
}

/// 把查询参数里的口令值替换为 `***`。
fn mask_query_password(tail: &str) -> String {
    let Some((path, query)) = tail.split_once('?') else {
        return tail.to_owned();
    };
    let masked = query
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((key, _)) if is_password_key(key) => format!("{key}=***"),
            _ => pair.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("&");

    format!("{path}?{masked}")
}

fn is_password_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "password" | "passwd" | "pwd"
    )
}

/// 密钥只输出长度，不泄露任何内容。
fn mask_secret(secret: &str) -> String {
    format!("***({} chars)", secret.chars().count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_password_is_masked() {
        assert_eq!(
            mask_url("postgres://app:xxx@db:5432/orders"),
            "postgres://app:***@db:5432/orders"
        );
        assert_eq!(
            mask_url("mysql://root:xxx@db:3306/orders"),
            "mysql://root:***@db:3306/orders"
        );
        // 口令本身含 @ 或 : 时也不能漏出
        assert_eq!(
            mask_url("mysql://root:p@ss:word@db:3306/orders"),
            "mysql://root:***@db:3306/orders"
        );
        // 查询参数形式的口令
        assert_eq!(
            mask_url("postgres://db:5432/orders?user=app&password=xxx&sslmode=require"),
            "postgres://db:5432/orders?user=app&password=***&sslmode=require"
        );
        // 无口令的连接串保持原样
        assert_eq!(
            mask_url("mysql://root@db:3306/orders"),
            "mysql://root@db:3306/orders"
        );
        assert_eq!(
            mask_url("sqlite://guardian.db?mode=rwc"),
            "sqlite://guardian.db?mode=rwc"
        );
    }

    #[test]
    fn secret_content_is_not_printed() {
        let masked = mask_secret("change-me-change-me-change-me-32");

        assert_eq!(masked, "***(32 chars)");
        assert!(!masked.contains("change-me"));
    }
}
