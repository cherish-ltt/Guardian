use std::time::Duration;

use axum::{
    Router,
    body::Body,
    error_handling::HandleErrorLayer,
    http::{HeaderName, HeaderValue, header},
    response::Response,
};
use tower::{
    ServiceBuilder,
    buffer::BufferLayer,
    limit::{ConcurrencyLimitLayer, RateLimitLayer},
    load_shed::LoadShedLayer,
};
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    cors::{AllowOrigin, Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    set_header::{HeaderMetadata, response::SetMultipleResponseHeadersLayer},
};

use crate::{
    infrastructure::config::MiddlewareConfig,
    middleware::{
        client_ip,
        client_limits::{self, ClientLimits},
        diagnostics, error_response, trace,
    },
};

const REQUEST_ID_HEADER: &str = "x-request-id";

/// 用 `ServiceBuilder` 把中间件栈包在整个应用之外。
///
/// 必须包住整个 Router，不能用 `Router::layer`：后者会把有状态中间件逐路由复制一份，
/// 全局并发上限与全局限流会因此失效。
pub fn apply(app: Router, config: &MiddlewareConfig) -> Router {
    let app = app.route_layer(axum::middleware::from_fn(diagnostics::route));
    let stack = ServiceBuilder::new()
        .layer(axum::middleware::from_fn(client_ip::sanitize_request_id))
        .layer(SetRequestIdLayer::new(request_id_header(), MakeRequestUuid))
        .layer(PropagateRequestIdLayer::new(request_id_header()))
        .layer(axum::middleware::from_fn_with_state(
            config.clone(),
            client_ip::identify,
        ))
        .layer(SetMultipleResponseHeadersLayer::overriding(
            security_headers(config),
        ))
        .layer(axum::middleware::from_fn_with_state(
            config.clone(),
            diagnostics::observe,
        ))
        .map_response(|response: axum::http::Response<_>| response.map(Body::new))
        .layer(CatchPanicLayer::custom(error_response::panic_response))
        .layer(axum::middleware::from_fn_with_state(
            ClientLimits::new(config),
            client_limits::protect,
        ))
        .layer(HandleErrorLayer::new(error_response::overload))
        .layer(LoadShedLayer::new())
        .layer(BufferLayer::new(config.backpressure_queue))
        .layer(ConcurrencyLimitLayer::new(config.max_concurrency))
        .layer(HandleErrorLayer::new(error_response::rate_limited))
        .layer(BufferLayer::new(config.backpressure_queue))
        .layer(LoadShedLayer::new())
        .layer(RateLimitLayer::new(
            config.rate_limit_requests,
            Duration::from_secs(config.rate_limit_period_secs),
        ))
        .layer(CompressionLayer::new())
        .layer(axum::middleware::from_fn(diagnostics::enter))
        // 必须在 `RequestBodyLimitLayer` 之外：限流层会把请求体换成 `Limited<Body>`，
        // 而中间件函数只接受 `Request<Body>`；同时要留在 `CompressionLayer` 之内，
        // 才能读到未压缩的 JSON 响应体。
        .layer(axum::middleware::from_fn(trace::inject_trace_id))
        .layer(RequestBodyLimitLayer::new(config.max_body_bytes))
        .layer(cors_layer(config))
        .service(app);

    Router::new().fallback_service(stack)
}

/// CORS 与其余中间件一起装配，不再由 controller 或 `create_app` 各自添加。
///
/// 默认（白名单为空）不下发任何跨域放行头；白名单含 `*` 时放行全部来源。
fn cors_layer(config: &MiddlewareConfig) -> CorsLayer {
    if config
        .cors_allowed_origins
        .iter()
        .any(|origin| origin == "*")
    {
        return CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
    }

    let allowed: Vec<HeaderValue> = config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect();

    if allowed.is_empty() {
        return CorsLayer::new();
    }

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed))
        .allow_methods(Any)
        .allow_headers(Any)
}

fn request_id_header() -> HeaderName {
    HeaderName::from_static(REQUEST_ID_HEADER)
}

fn security_headers(config: &MiddlewareConfig) -> Vec<HeaderMetadata<Response<Body>>> {
    let mut headers: Vec<HeaderMetadata<Response<Body>>> = vec![
        (
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        )
            .into(),
        (header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY")).into(),
        (
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        )
            .into(),
    ];

    if config.hsts_enabled {
        headers.push(
            (
                header::STRICT_TRANSPORT_SECURITY,
                HeaderValue::from_static("max-age=31536000; includeSubDomains"),
            )
                .into(),
        );
    }

    headers
}
