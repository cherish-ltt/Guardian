use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use super::client_ip::{ClientIp, IdentityError};
use crate::{infrastructure::config::MiddlewareConfig, response::ApiResponse};

enum Rejection {
    Capacity,
    Rate(u64),
    Concurrency,
}

impl IntoResponse for Rejection {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::Capacity => (StatusCode::SERVICE_UNAVAILABLE, "系统内部错误"),
            Self::Rate(_) => (StatusCode::TOO_MANY_REQUESTS, "请求过于频繁"),
            Self::Concurrency => (StatusCode::TOO_MANY_REQUESTS, "同一来源并发请求过多"),
        };
        let mut response = ApiResponse::<()>::error(status, message).into_response();
        if let Self::Rate(seconds) = self {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, axum::http::HeaderValue::from(seconds));
        }
        response
    }
}

struct Entry {
    window: Instant,
    requests: u64,
    active: usize,
    last_seen: Instant,
}

struct Registry {
    entries: HashMap<IpAddr, Entry>,
    next_cleanup: Instant,
}

/// 进程级客户端额度：按规范化来源 IP 分配并发许可与固定窗口速率计数，跨路由共享。
#[derive(Clone)]
pub(crate) struct ClientLimits {
    registry: Arc<Mutex<Registry>>,
    config: MiddlewareConfig,
}

impl ClientLimits {
    pub(crate) fn new(config: &MiddlewareConfig) -> Self {
        Self {
            registry: Arc::new(Mutex::new(Registry {
                entries: HashMap::new(),
                next_cleanup: Instant::now(),
            })),
            config: config.clone(),
        }
    }

    fn acquire(&self, ip: IpAddr, now: Instant) -> Result<Permit, Rejection> {
        let mut registry = self.registry.lock().unwrap_or_else(|e| e.into_inner());
        let idle = Duration::from_secs(self.config.client_idle_secs);
        if now >= registry.next_cleanup {
            // 在途请求不能驱逐，否则同一 IP 会重新拿到一份全新的额度。
            registry
                .entries
                .retain(|_, entry| entry.active > 0 || now.duration_since(entry.last_seen) < idle);
            registry.next_cleanup = now + idle;
        }
        if !registry.entries.contains_key(&ip)
            && registry.entries.len() >= self.config.client_max_entries
        {
            return Err(Rejection::Capacity);
        }
        let entry = registry.entries.entry(ip).or_insert(Entry {
            window: now,
            requests: 0,
            active: 0,
            last_seen: now,
        });
        entry.last_seen = now;
        let period = Duration::from_secs(self.config.client_rate_limit_period_secs);
        if now.duration_since(entry.window) >= period {
            entry.window = now;
            entry.requests = 0;
        }
        if entry.requests >= self.config.client_rate_limit_requests {
            let wait = period.saturating_sub(now.duration_since(entry.window));
            let seconds = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
            return Err(Rejection::Rate(seconds));
        }
        entry.requests += 1;
        if entry.active >= self.config.client_max_concurrency {
            return Err(Rejection::Concurrency);
        }
        entry.active += 1;
        Ok(Permit {
            limits: self.clone(),
            ip,
        })
    }
}

struct Permit {
    limits: ClientLimits,
    ip: IpAddr,
}

/// 许可随响应释放：正常完成、超时取消与异常退出都会走到这里。
impl Drop for Permit {
    fn drop(&mut self) {
        let mut registry = self
            .limits
            .registry
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = registry.entries.get_mut(&self.ip) {
            entry.active -= 1;
            entry.last_seen = Instant::now();
        }
    }
}

pub(crate) async fn protect(
    State(limits): State<ClientLimits>,
    request: Request,
    next: Next,
) -> Response {
    let ip = match request
        .extensions()
        .get::<Result<ClientIp, IdentityError>>()
    {
        Some(Ok(ip)) => ip.0,
        Some(Err(IdentityError::InvalidForwardedFor)) => {
            return ApiResponse::<()>::error(
                StatusCode::BAD_REQUEST,
                "请求头 X-Forwarded-For 格式错误",
            )
            .into_response();
        },
        _ => {
            tracing::error!("client identity unavailable: serve with ConnectInfo<SocketAddr>");
            return ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, "系统内部错误")
                .into_response();
        },
    };
    let _permit = match limits.acquire(ip, Instant::now()) {
        Ok(permit) => permit,
        Err(rejection) => return rejection.into_response(),
    };
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> MiddlewareConfig {
        MiddlewareConfig {
            request_timeout_secs: 10,
            max_body_bytes: 1024,
            max_concurrency: 256,
            backpressure_queue: 256,
            rate_limit_requests: 1000,
            rate_limit_period_secs: 1,
            hsts_enabled: false,
            cors_allowed_origins: Vec::new(),
            trusted_proxies: Vec::new(),
            client_max_concurrency: 1,
            client_rate_limit_requests: 2,
            client_rate_limit_period_secs: 1,
            client_max_entries: 1,
            client_idle_secs: 1,
            slow_request_ms: 1000,
        }
    }

    #[test]
    fn registry_is_bounded_and_idle_entries_expire() {
        let limits = ClientLimits::new(&config());
        let now = Instant::now();
        let first = "192.0.2.1".parse().unwrap();
        let second = "192.0.2.2".parse().unwrap();
        let permit = limits.acquire(first, now).ok().unwrap();
        assert_eq!(
            limits
                .acquire(second, now)
                .err()
                .unwrap()
                .into_response()
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert!(
            limits
                .acquire(second, now + Duration::from_secs(2))
                .is_err()
        );
        drop(permit);
        assert!(limits.acquire(second, now + Duration::from_secs(4)).is_ok());
    }

    #[test]
    fn concurrency_and_rate_are_independent_and_reset() {
        let limits = ClientLimits::new(&config());
        let now = Instant::now();
        let ip = "192.0.2.1".parse().unwrap();
        let permit = limits.acquire(ip, now).ok().unwrap();
        assert_eq!(
            limits
                .acquire(ip, now)
                .err()
                .unwrap()
                .into_response()
                .status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        drop(permit);
        assert_eq!(
            limits
                .acquire(ip, now)
                .err()
                .unwrap()
                .into_response()
                .headers()[header::RETRY_AFTER],
            "1"
        );
        assert!(limits.acquire(ip, now + Duration::from_secs(1)).is_ok());
    }
}
