use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct RateLimiter {
    clients: Arc<DashMap<IpAddr, RateLimitState>>,
    max_requests: u64,
    window_secs: u64,
}

#[derive(Clone)]
struct RateLimitState {
    count: u64,
    window_start: Instant,
}

impl RateLimiter {
    pub fn new(max_requests: u64, window_secs: u64) -> Self {
        Self {
            clients: Arc::new(DashMap::new()),
            max_requests,
            window_secs,
        }
    }

    pub fn is_allowed(&self, client_ip: IpAddr) -> bool {
        let now = Instant::now();
        let window_duration = Duration::from_secs(self.window_secs);

        let mut entry = self.clients.entry(client_ip).or_insert(RateLimitState {
            count: 0,
            window_start: now,
        });

        let state = entry.value_mut();

        if now.duration_since(state.window_start) >= window_duration {
            state.count = 1;
            state.window_start = now;
            return true;
        }

        if state.count >= self.max_requests {
            return false;
        }

        state.count += 1;
        true
    }

    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        let window_duration = Duration::from_secs(self.window_secs);

        self.clients
            .retain(|_, state| now.duration_since(state.window_start) < window_duration);
    }
}

fn create_rate_limiter() -> RateLimiter {
    let max_requests = std::env::var("RATE_LIMIT_MAX_REQUESTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    let window_secs = std::env::var("RATE_LIMIT_WINDOW_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    RateLimiter::new(max_requests, window_secs)
}

pub async fn rate_limit_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
    use std::sync::OnceLock;

    static RATE_LIMITER: OnceLock<RateLimiter> = OnceLock::new();

    let rate_limiter = RATE_LIMITER.get_or_init(create_rate_limiter);

    let client_ip = extract_client_ip(&request);

    if !rate_limiter.is_allowed(client_ip) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

fn extract_client_ip(request: &Request) -> IpAddr {
    request
        .headers()
        .get("X-Forwarded-For")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse().ok())
        .or_else(|| {
            request
                .headers()
                .get("X-Real-IP")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or_else(|| "127.0.0.1".parse().unwrap())
}
