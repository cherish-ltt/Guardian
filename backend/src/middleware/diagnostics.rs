use std::{
    collections::BTreeMap,
    future::Future,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    extract::{MatchedPath, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use tracing::Instrument;

use super::{
    client_ip::{ClientIp, IdentityError},
    trace,
};
use crate::{infrastructure::config::MiddlewareConfig, response::ApiResponse};

tokio::task_local! {
    static CURRENT: Diagnostics;
}

#[derive(Clone, Default)]
struct Diagnostics(Arc<Mutex<Details>>);

#[derive(Default)]
struct Details {
    route: Option<String>,
    queue_ms: Option<u64>,
    stages: BTreeMap<&'static str, Stage>,
}

#[derive(Debug, Default)]
struct Stage {
    elapsed_ms: u64,
    calls: u64,
    interrupted: u64,
}

struct Timer {
    diagnostics: Diagnostics,
    name: &'static str,
    start: Instant,
    completed: bool,
}

impl Drop for Timer {
    fn drop(&mut self) {
        let mut details = self.diagnostics.0.lock().unwrap_or_else(|e| e.into_inner());
        let stage = details.stages.entry(self.name).or_default();
        stage.elapsed_ms += millis(self.start.elapsed());
        stage.calls += 1;
        stage.interrupted += u64::from(!self.completed);
    }
}

/// 测量阶段但不逐阶段打印日志；必须使用固定名称，不能带用户输入或 SQL。
pub(crate) async fn measure<F: Future>(name: &'static str, future: F) -> F::Output {
    let mut timer = CURRENT
        .try_with(|diagnostics| Timer {
            diagnostics: diagnostics.clone(),
            name,
            start: Instant::now(),
            completed: false,
        })
        .ok();
    let result = future.await;
    if let Some(timer) = timer.as_mut() {
        timer.completed = true;
    }
    result
}

/// 到达保护层的时刻，用于在进入处理层时算出全局排队耗时。
#[derive(Clone)]
struct Arrival(Instant);

/// 请求观测与总超时：含排队计时、框架错误归一化，以及每请求一条完成日志。
pub(crate) async fn observe(
    State(config): State<MiddlewareConfig>,
    mut request: Request,
    next: Next,
) -> Response {
    let started = Instant::now();
    let is_head = request.method() == axum::http::Method::HEAD;
    let is_auth = request.uri().path().starts_with("/api/v1/auth/");
    let diagnostics = Diagnostics::default();
    request.extensions_mut().insert(diagnostics.clone());
    request.extensions_mut().insert(Arrival(started));
    // 追踪 ID 只解析一次并放进请求上下文：出栈各层共用同一个值，不会出现头与体不一致。
    let trace_id = trace::TraceId(trace::resolve_trace_id(
        request
            .headers()
            .get(trace::TRACE_ID_HEADER)
            .and_then(|value| value.to_str().ok()),
    ));
    request.extensions_mut().insert(trace_id.clone());
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let client_ip = request
        .extensions()
        .get::<Result<ClientIp, IdentityError>>()
        .and_then(|value| value.as_ref().ok())
        .map(|ip| ip.0.to_string())
        .unwrap_or_default();
    let span = tracing::info_span!(
        target: "http_request",
        "http_request",
        method = %request.method(),
        path = %request.uri().path(),
        request_id = %request_id,
        client_ip = %client_ip,
    );
    async move {
        let response = match tokio::time::timeout(
            Duration::from_secs(config.request_timeout_secs),
            next.run(request),
        )
        .await
        {
            Ok(response) => normalize(response, &trace_id.0).await,
            Err(_) => {
                trace::inject(
                    ApiResponse::<()>::error(StatusCode::REQUEST_TIMEOUT, "请求超时")
                        .into_response(),
                    &trace_id.0,
                )
                .await
            },
        };
        // 限流、背压与 panic 的响应由本层之外生成，内层注入不到，这里补上响应头。
        let mut response = trace::attach_header(response, &trace_id.0);
        if is_auth {
            response.headers_mut().insert(
                axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"),
            );
        }
        if is_head {
            *response.body_mut() = axum::body::Body::empty();
        }
        let elapsed_ms = millis(started.elapsed());
        let details = diagnostics.0.lock().unwrap_or_else(|e| e.into_inner());
        let status = response.status().as_u16();
        let route = details.route.as_deref().unwrap_or("unmatched");
        let queue_ms = details.queue_ms.unwrap_or(elapsed_ms);
        if elapsed_ms >= config.slow_request_ms
            || status == 408
            || (status >= 500 && (status != 503 || details.route.is_some()))
        {
            tracing::warn!(
                status,
                elapsed_ms,
                route,
                queue_ms,
                stages = ?details.stages,
                "request diagnostic"
            );
        } else {
            tracing::info!(status, elapsed_ms, route, "request completed");
        }
        response
    }
    .instrument(span)
    .await
}

/// 记录进入处理层前的排队耗时，并把阶段测量上下文传进 handler。
pub(crate) async fn enter(mut request: Request, next: Next) -> Response {
    let diagnostics = request
        .extensions_mut()
        .get::<Diagnostics>()
        .cloned()
        .unwrap_or_default();
    if let Some(arrival) = request.extensions().get::<Arrival>() {
        diagnostics
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .queue_ms = Some(millis(arrival.0.elapsed()));
    }
    CURRENT.scope(diagnostics, next.run(request)).await
}

/// 匹配完成后记录路由模板；无状态，不分配独立额度。
pub(crate) async fn route(request: Request, next: Next) -> Response {
    if let (Some(diagnostics), Some(path)) = (
        request.extensions().get::<Diagnostics>(),
        request.extensions().get::<MatchedPath>(),
    ) {
        diagnostics
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .route = Some(path.as_str().to_owned());
    }
    next.run(request).await
}

/// 把框架生成的纯文本错误换成统一响应结构：不回显解析细节，保留其余响应头。
async fn normalize(response: Response, trace_id: &str) -> Response {
    let message = match response.status() {
        StatusCode::BAD_REQUEST => "请求格式错误",
        StatusCode::NOT_FOUND => "资源不存在",
        StatusCode::METHOD_NOT_ALLOWED => "请求方法不支持",
        StatusCode::PAYLOAD_TOO_LARGE => "请求体过大",
        StatusCode::UNSUPPORTED_MEDIA_TYPE => "请求内容类型不支持",
        StatusCode::UNPROCESSABLE_ENTITY => "请求参数校验失败",
        _ => return response,
    };
    if response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"))
    {
        return response;
    }
    let mut normalized = ApiResponse::<()>::error(response.status(), message).into_response();
    for (name, value) in response.headers() {
        if name != axum::http::header::CONTENT_LENGTH
            && name != axum::http::header::CONTENT_TYPE
            && name != axum::http::header::CONTENT_ENCODING
            && name != axum::http::header::TRANSFER_ENCODING
        {
            normalized.headers_mut().insert(name.clone(), value.clone());
        }
    }
    trace::inject(normalized, trace_id).await
}

fn millis(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stages_aggregate_and_record_cancellation_without_extra_events() {
        let diagnostics = Diagnostics::default();
        CURRENT
            .scope(diagnostics.clone(), async {
                measure("db.query", async {}).await;
                measure("db.query", async {}).await;
                assert!(
                    tokio::time::timeout(
                        Duration::from_millis(5),
                        measure("db.wait", std::future::pending::<()>())
                    )
                    .await
                    .is_err()
                );
            })
            .await;
        let details = diagnostics.0.lock().unwrap();
        assert_eq!(details.stages["db.query"].calls, 2);
        assert_eq!(details.stages["db.query"].interrupted, 0);
        assert_eq!(details.stages["db.wait"].interrupted, 1);
        assert!(details.stages["db.wait"].elapsed_ms >= 5);
    }

    #[tokio::test]
    async fn measurements_work_without_request_context() {
        assert_eq!(measure("outside", async { 42 }).await, 42);
    }

    #[tokio::test]
    async fn plain_text_errors_are_normalized_but_json_is_kept() {
        let plain = (StatusCode::PAYLOAD_TOO_LARGE, "body too large").into_response();
        let normalized = normalize(plain, "trace-1").await;
        assert_eq!(normalized.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let bytes = axum::body::to_bytes(normalized.into_body(), 4096)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["code"], 413);
        assert_eq!(body["message"], "请求体过大");
        assert_eq!(body["traceId"], "trace-1");

        let json = ApiResponse::<()>::error(StatusCode::NOT_FOUND, "资源不存在").into_response();
        let kept = normalize(json, "trace-2").await;
        let bytes = axum::body::to_bytes(kept.into_body(), 4096).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["message"], "资源不存在");
        assert!(body.get("traceId").is_none());
    }
}
