use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tower::{BoxError, load_shed::error::Overloaded};

use crate::response::ApiResponse;

/// 并发或排队已超出容量（`LoadShed` 触发）时快速返回 503。
///
/// 限流/背压触发时是批量拒绝，逐条告警无信息量，这里不打日志；
/// 请求观测层已为每次请求记录一条访问日志。
pub async fn overload(err: BoxError) -> Response {
    if err.is::<Overloaded>() {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Service is busy, please retry later",
        )
        .into_response();
    }
    internal(err)
}

/// 限流窗口内额度已用尽（`LoadShed` 触发）时快速返回 429。
///
/// 限流/背压触发时是批量拒绝，逐条告警无信息量，这里不打日志；
/// 请求观测层已为每次请求记录一条访问日志。
pub async fn rate_limited(err: BoxError) -> Response {
    if err.is::<Overloaded>() {
        return ApiResponse::<()>::error(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many requests, please retry later",
        )
        .into_response();
    }
    internal(err)
}

/// 请求处理 panic 的隔离：保留内部错误日志，对外只暴露统一的安全 500。
pub(crate) fn panic_response(err: Box<dyn std::any::Any + Send>) -> Response {
    let detail = err
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| err.downcast_ref::<&str>().copied())
        .unwrap_or("non-string panic");
    tracing::error!(panic = detail, "request handler panicked");
    ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, "系统内部错误").into_response()
}

fn internal(err: BoxError) -> Response {
    tracing::error!("middleware stack internal error: {err}");
    ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, "系统内部错误").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn overloaded_is_mapped_by_caller() {
        let overloaded = || -> BoxError { Box::new(Overloaded::new()) };

        assert_eq!(
            overload(overloaded()).await.status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            rate_limited(overloaded()).await.status(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[tokio::test]
    async fn unexpected_error_is_internal() {
        let unexpected = || -> BoxError { Box::new(std::io::Error::other("boom")) };

        assert_eq!(
            overload(unexpected()).await.status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            rate_limited(unexpected()).await.status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
