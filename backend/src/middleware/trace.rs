use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use uuid::Uuid;

use crate::response::ApiResponse;

/// 追踪 ID 的请求与响应头，与 Spring 版 `TraceIdFilter` 保持同名。
pub const TRACE_ID_HEADER: &str = "x-trace-id";
/// 统一响应体里的追踪字段名。
pub const TRACE_ID_FIELD: &str = "traceId";

/// 沿用调用方传入的追踪 ID，缺失或空白时生成 32 位十六进制。
pub fn resolve_trace_id(existing: Option<&str>) -> String {
    match existing.map(str::trim) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => Uuid::now_v7().simple().to_string(),
    }
}

/// 给每个响应补上 `X-Trace-Id` 头与响应体里的 `traceId` 字段。
///
/// 统一响应体由 `ApiResponse` 的 `Serialize` 实现产出，无法读到请求上下文，
/// 因此追踪 ID 在响应出栈时统一注入：这里只改写 JSON 响应体，
/// CSV / ZIP 等文件响应只加响应头。
pub async fn inject_trace_id(request: Request, next: Next) -> Response {
    let trace_id = resolve_trace_id(
        request
            .headers()
            .get(TRACE_ID_HEADER)
            .and_then(|value| value.to_str().ok()),
    );
    let mut response = next.run(request).await;

    let Ok(header_value) = HeaderValue::from_str(&trace_id) else {
        tracing::warn!(%trace_id, "trace id is not a valid header value, response keeps untraced");
        return response;
    };
    response.headers_mut().insert(
        header::HeaderName::from_static(TRACE_ID_HEADER),
        header_value,
    );

    inject_body(response, &trace_id).await
}

/// 只在 JSON 响应体里补写 `traceId`；其余类型（CSV / ZIP 等）只保留响应头。
async fn inject_body(response: Response, trace_id: &str) -> Response {
    if !is_json(response.headers()) {
        return response;
    }

    let (mut parts, body) = response.into_parts();
    let collected = match body.collect().await {
        Ok(collected) => collected,
        Err(error) => {
            tracing::error!(%error, "failed to buffer json response body for trace id injection");
            return ApiResponse::<()>::error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "系统内部错误",
            )
            .into_response();
        },
    };

    let rewritten = attach_trace_id(&collected.to_bytes(), trace_id);
    parts.headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&rewritten.len().to_string())
            .unwrap_or(HeaderValue::from_static("0")),
    );

    Response::from_parts(parts, Body::from(rewritten))
}

/// 把 `traceId` 写进 JSON 对象；不是对象（数组、标量）或解析失败时原样返回。
fn attach_trace_id(bytes: &[u8], trace_id: &str) -> Vec<u8> {
    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return bytes.to_vec();
    };
    if let Some(object) = value.as_object_mut() {
        object.insert(
            TRACE_ID_FIELD.to_string(),
            serde_json::Value::String(trace_id.to_string()),
        );
        return serde_json::to_vec(&value).unwrap_or_else(|_| bytes.to_vec());
    }

    bytes.to_vec()
}

fn is_json(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"))
}

#[cfg(test)]
mod tests {
    use super::{attach_trace_id, resolve_trace_id};

    #[test]
    fn incoming_trace_id_is_reused() {
        assert_eq!(resolve_trace_id(Some("abc123")), "abc123");
        assert_eq!(resolve_trace_id(Some("  abc123  ")), "abc123");
    }

    #[test]
    fn missing_trace_id_is_generated_as_32_hex_chars() {
        for source in [None, Some(""), Some("   ")] {
            let trace_id = resolve_trace_id(source);
            assert_eq!(trace_id.len(), 32);
            assert!(trace_id.chars().all(|ch| ch.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn json_object_gets_trace_id_field() {
        let rewritten = attach_trace_id(br#"{"code":200,"message":"Success"}"#, "t-1");
        let value: serde_json::Value = serde_json::from_slice(&rewritten).expect("解析响应体");

        assert_eq!(value["traceId"], "t-1");
        assert_eq!(value["code"], 200);
    }

    #[test]
    fn non_object_json_is_kept_as_is() {
        assert_eq!(attach_trace_id(b"[1,2]", "t-1"), b"[1,2]");
        assert_eq!(attach_trace_id(b"not json", "t-1"), b"not json");
    }
}
