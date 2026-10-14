use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

pub use crate::domain::error::AppError;
use crate::response::ApiResponse;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.http_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let message = response_message(&self, status);

        // 服务端错误只在日志里留原始原因，响应体用稳定文案（已含在 AppError 的 Display 里）。
        if status.is_server_error() {
            error!(status = status.as_u16(), error = %self, "request handling failed");
        }

        ApiResponse::<()>::error(status, message).into_response()
    }
}

/// 客户端可见的文案：4xx 透出业务原因，503 与 5xx 只给稳定文案。
///
/// 5xx 的文案与后台前端契约一致（Spring 版的「系统内部错误」）。
fn response_message(error: &AppError, status: StatusCode) -> String {
    if status.is_client_error() {
        return error.to_string();
    }
    if status == StatusCode::SERVICE_UNAVAILABLE {
        return "Service temporarily unavailable".to_string();
    }

    "系统内部错误".to_string()
}

#[cfg(test)]
mod tests {
    use super::AppError;
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[test]
    fn each_error_maps_to_expected_status() {
        let cases = [
            (AppError::bad_request("invalid parameter"), 400),
            (AppError::unauthorized("unauthorized"), 401),
            (AppError::not_found("not found"), 404),
            (AppError::conflict("duplicate idempotency key"), 409),
            (AppError::unavailable("database not ready"), 503),
            (AppError::internal("unexpected failure"), 500),
            (AppError::Config("missing configuration".to_string()), 500),
        ];

        for (error, expected) in cases {
            let response = error.into_response();
            assert_eq!(response.status().as_u16(), expected);
        }
    }

    #[tokio::test]
    async fn client_errors_keep_message() {
        let response = AppError::bad_request("title must not be empty").into_response();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("读取响应体");
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("解析响应体");

        assert_eq!(body["code"], 400);
        assert_eq!(body["message"], "Bad request: title must not be empty");
    }

    #[tokio::test]
    async fn server_errors_are_masked() {
        let errors = [
            AppError::unavailable("database not ready"),
            AppError::internal("unexpected failure"),
        ];

        for error in errors {
            let raw_message = error.to_string();
            let response = error.into_response();
            let bytes = to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("读取响应体");
            let body: serde_json::Value = serde_json::from_slice(&bytes).expect("解析响应体");
            let message = body["message"].as_str().expect("message 字段为字符串");

            assert!(
                !message.contains(&raw_message),
                "服务端错误不应透出内部信息"
            );
            assert!(body.get("data").is_none());
        }
    }

    #[test]
    fn unavailable_maps_to_service_unavailable() {
        let response = AppError::unavailable("database not ready").into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
