use std::sync::Arc;

use axum::{
    Router,
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use tracing::warn;

use crate::{api, container::Container, middleware, response::ApiResponse};

pub fn create_app(container: Arc<Container>) -> Router {
    // 自定义嵌入入口也必须阻止 production 配置运行开发认证路由。
    if let Err(error) = container.config.validate_runtime() {
        panic!("{error}");
    }
    let api = api::router().layer(axum::middleware::from_fn_with_state(
        Arc::clone(&container),
        middleware::audit::record_operation,
    ));
    let app = Router::new()
        .route("/", get(api::controllers::system_controller::root))
        .nest("/api/v1", api)
        .fallback(not_found)
        .method_not_allowed_fallback(method_not_allowed)
        .with_state(Arc::clone(&container));

    #[cfg(feature = "docs")]
    let app = crate::docs::mount(app);

    middleware::apply(app, &container.config.middleware)
}

/// 未匹配到任何路由：与 Spring 版的 `NoResourceFoundException` 文案一致。
///
/// 只记 path 不记完整 URI：未匹配的请求也可能带着令牌、验证码等 query 参数，
/// 日志里不应该出现这些值。
async fn not_found(request: Request) -> Response {
    warn!(path = %request.uri().path(), "no route matched");

    ApiResponse::<()>::error(StatusCode::NOT_FOUND, "资源不存在").into_response()
}

/// 路径存在但方法不支持：与 Spring 版的 `HttpRequestMethodNotSupportedException` 文案一致。
async fn method_not_allowed(method: axum::http::Method) -> Response {
    ApiResponse::<()>::error(
        StatusCode::METHOD_NOT_ALLOWED,
        format!("不支持的请求方法: {method}"),
    )
    .into_response()
}
