use axum::{
    Json,
    extract::{FromRequest, Request, rejection::JsonRejection},
};
use serde::de::DeserializeOwned;

/// 请求体提取器：把 `Json` 的拒绝文案统一成 Spring 版的「请求体格式错误」。
///
/// axum 默认的 `Json` 拒绝会返回 422 与纯文本响应体，前端无法解析 `code` 字段，
/// 这里统一收敛为 400 + 统一响应体。
pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = crate::domain::error::AppError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) =
            Json::<T>::from_request(request, state)
                .await
                .map_err(|rejection: JsonRejection| {
                    tracing::debug!(%rejection, "request body rejected");

                    crate::domain::error::AppError::business("请求体格式错误")
                })?;

        Ok(Self(value))
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        response::IntoResponse,
        routing::post,
    };
    use tower::ServiceExt;

    use super::ApiJson;

    #[derive(Debug, serde::Deserialize)]
    struct Payload {
        name: String,
    }

    #[tokio::test]
    async fn valid_body_is_accepted() {
        let app = Router::new().route(
            "/demo",
            post(|ApiJson(payload): ApiJson<Payload>| async move { payload.name }),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/demo")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"alice"}"#))
                    .expect("构造请求"),
            )
            .await
            .expect("调用处理器");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn malformed_body_becomes_bad_request_with_unified_envelope() {
        let app = Router::new().route(
            "/demo",
            post(|ApiJson(_payload): ApiJson<Payload>| async {}),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/demo")
                    .header("content-type", "application/json")
                    .body(Body::from("{not json"))
                    .expect("构造请求"),
            )
            .await
            .expect("调用处理器");
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("读取响应体");
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("解析响应体");

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], 400);
        assert_eq!(body["message"], "请求体格式错误");
        let _ = ().into_response();
    }
}
