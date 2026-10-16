use std::{sync::Arc, time::Instant};

use axum::{
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderMap, Method, StatusCode},
    middleware::Next,
    response::Response,
};
use http_body_util::BodyExt;
use serde_json::Value;
use tracing::{error, warn};

use crate::{
    container::Container, domain::models::audit_log::OperationLogEntry, middleware::client_ip,
};

/// 需要留痕的写操作：路径按 `nest("/api/v1")` 之后的段匹配，`:id` 匹配任意一段。
const AUDIT_ROUTES: [(Method, &[&str], &str, &str); 31] = [
    (Method::PUT, &["auth", "password"], "账号安全", "修改密码"),
    (Method::POST, &["system", "users"], "用户管理", "新增"),
    (Method::PUT, &["system", "users", ":id"], "用户管理", "修改"),
    (
        Method::PUT,
        &["system", "users", ":id", "status"],
        "用户管理",
        "状态变更",
    ),
    (
        Method::PUT,
        &["system", "users", ":id", "password"],
        "用户管理",
        "重置密码",
    ),
    (
        Method::DELETE,
        &["system", "users", ":id"],
        "用户管理",
        "删除",
    ),
    (Method::POST, &["system", "roles"], "角色管理", "新增"),
    (Method::PUT, &["system", "roles", ":id"], "角色管理", "修改"),
    (
        Method::DELETE,
        &["system", "roles", ":id"],
        "角色管理",
        "删除",
    ),
    (Method::POST, &["system", "menus"], "菜单管理", "新增"),
    (Method::PUT, &["system", "menus", ":id"], "菜单管理", "修改"),
    (
        Method::DELETE,
        &["system", "menus", ":id"],
        "菜单管理",
        "删除",
    ),
    (Method::POST, &["system", "depts"], "部门管理", "新增"),
    (Method::PUT, &["system", "depts", ":id"], "部门管理", "修改"),
    (
        Method::DELETE,
        &["system", "depts", ":id"],
        "部门管理",
        "删除",
    ),
    (Method::POST, &["system", "posts"], "岗位管理", "新增"),
    (Method::PUT, &["system", "posts", ":id"], "岗位管理", "修改"),
    (
        Method::DELETE,
        &["system", "posts", ":id"],
        "岗位管理",
        "删除",
    ),
    (
        Method::POST,
        &["system", "dict", "types"],
        "字典管理",
        "新增字典类型",
    ),
    (
        Method::PUT,
        &["system", "dict", "types", ":id"],
        "字典管理",
        "修改字典类型",
    ),
    (
        Method::DELETE,
        &["system", "dict", "types", ":id"],
        "字典管理",
        "删除字典类型",
    ),
    (
        Method::POST,
        &["system", "dict", "data"],
        "字典管理",
        "新增字典数据",
    ),
    (
        Method::PUT,
        &["system", "dict", "data", ":id"],
        "字典管理",
        "修改字典数据",
    ),
    (
        Method::DELETE,
        &["system", "dict", "data", ":id"],
        "字典管理",
        "删除字典数据",
    ),
    (Method::POST, &["system", "configs"], "参数配置", "新增"),
    (
        Method::PUT,
        &["system", "configs", ":id"],
        "参数配置",
        "修改",
    ),
    (
        Method::DELETE,
        &["system", "configs", ":id"],
        "参数配置",
        "删除",
    ),
    (
        Method::POST,
        &["tool", "generator", "import"],
        "代码生成",
        "导入表",
    ),
    (
        Method::PUT,
        &["tool", "generator", "tables", ":id"],
        "代码生成",
        "修改配置",
    ),
    (
        Method::POST,
        &["tool", "generator", "tables", ":id", "generate"],
        "代码生成",
        "生成代码",
    ),
    (
        Method::GET,
        &["tool", "generator", "tables", ":id", "download"],
        "代码生成",
        "下载代码",
    ),
];

/// 写操作留痕中间件：命中 [`AUDIT_ROUTES`] 时记录操作日志。
///
/// 与 Spring 版 `@OperLog` 切面的差异：这里记录的是请求体（切面记录的是方法入参），
/// 且认证 / 权限被拒（401、403）时不写日志——那两种情况下请求根本没进业务方法。
pub async fn record_operation(
    State(container): State<Arc<Container>>,
    request: Request,
    next: Next,
) -> Response {
    let Some((module, action)) = audit_action(request.method(), request.uri().path()) else {
        return next.run(request).await;
    };

    let method = request.method().to_string();
    let url = request.uri().path().to_string();
    let operator = operator_of(&container, request.headers());
    let ip = client_ip::source_of(request.extensions(), request.headers()).ip;
    let limit = container.config.middleware.max_body_bytes;
    let (request, params) = buffered_request(request, limit, &url).await;

    let started = Instant::now();
    let response = next.run(request).await;
    let cost_ms = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
    let status = response.status();
    if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
        return response;
    }

    let (response, error_message) = extract_error(status, response).await;
    let entry = OperationLogEntry {
        module: module.to_string(),
        action: action.to_string(),
        request_method: Some(method),
        request_url: Some(url.clone()),
        request_params: params,
        operator,
        ip,
        cost_ms,
        result: i64::from(status.is_success()),
        error_message,
    };
    if let Err(write_error) = container.audit_service.record(entry).await {
        // 留痕写入失败不能影响业务响应，但必须在日志里显式暴露。
        error!(%write_error, %url, "failed to record operation log");
    }

    response
}

/// 读走请求体用于留痕，再按原字节重建请求；读取失败时按空体重放。
async fn buffered_request(request: Request, limit: usize, url: &str) -> (Request, Option<String>) {
    let (parts, body) = request.into_parts();
    match to_bytes(body, limit).await {
        Ok(bytes) => {
            let params = masked_params(&bytes);
            (Request::from_parts(parts, Body::from(bytes)), params)
        },
        Err(rejection) => {
            warn!(%rejection, %url, "failed to buffer request body for operation log");
            (Request::from_parts(parts, Body::empty()), None)
        },
    }
}

/// 失败响应要取走统一响应体里的 `message`，再把响应原样交还调用链。
async fn extract_error(status: StatusCode, response: Response) -> (Response, Option<String>) {
    if status.is_success() {
        return (response, None);
    }

    let (response, message) = take_error_message(response).await;

    (response, message)
}

/// 命中留痕规则时返回 `(模块, 操作)`。
fn audit_action(method: &Method, path: &str) -> Option<(&'static str, &'static str)> {
    let segments = path
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    AUDIT_ROUTES
        .iter()
        .find(|(expected_method, pattern, _, _)| {
            *expected_method == method
                && pattern.len() == segments.len()
                && pattern
                    .iter()
                    .zip(&segments)
                    .all(|(expected, actual)| *expected == ":id" || expected == actual)
        })
        .map(|(_, _, module, action)| (*module, *action))
}

/// 记录操作人：只解 JWT，不再查库（处理器已经做过完整鉴权）。
fn operator_of(container: &Container, headers: &HeaderMap) -> Option<String> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, token)| token.trim())
        .filter(|token| !token.is_empty())?;

    container
        .jwt_service
        .verify_access_token(token)
        .ok()
        .map(|claims| claims.username)
}

/// 请求体先做敏感字段脱敏，再按 4000 字符截断。
fn masked_params(body: &[u8]) -> Option<String> {
    if body.is_empty() {
        return None;
    }

    let text = match serde_json::from_slice::<Value>(body) {
        Ok(mut value) => {
            mask_sensitive(&mut value);
            value.to_string()
        },
        Err(_) => String::from_utf8_lossy(body).to_string(),
    };

    Some(OperationLogEntry::truncate(Some(text)).unwrap_or_default())
}

fn mask_sensitive(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            for (key, field) in fields.iter_mut() {
                if is_sensitive(key) {
                    *field = Value::String("******".to_string());
                } else {
                    mask_sensitive(field);
                }
            }
        },
        Value::Array(items) => {
            for item in items {
                mask_sensitive(item);
            }
        },
        _ => {},
    }
}

fn is_sensitive(field: &str) -> bool {
    let lowered = field.to_lowercase();

    ["password", "token", "secret", "authorization"]
        .iter()
        .any(|keyword| lowered.contains(keyword))
}

/// 读取失败响应的 `message`，并把响应体放回响应里。
async fn take_error_message(response: Response) -> (Response, Option<String>) {
    let (parts, body) = response.into_parts();
    let Ok(collected) = body.collect().await else {
        return (Response::from_parts(parts, Body::empty()), None);
    };
    let bytes = collected.to_bytes();
    let message = serde_json::from_slice::<Value>(&bytes)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .and_then(|message| OperationLogEntry::truncate(Some(message)));

    (Response::from_parts(parts, Body::from(bytes)), message)
}

#[cfg(test)]
mod tests {
    use axum::http::Method;
    use serde_json::json;

    use super::{audit_action, masked_params};

    #[test]
    fn audit_routes_match_by_method_and_segments() {
        assert_eq!(
            audit_action(&Method::POST, "/system/users"),
            Some(("用户管理", "新增"))
        );
        assert_eq!(
            audit_action(&Method::PUT, "/system/users/12/status"),
            Some(("用户管理", "状态变更"))
        );
        assert_eq!(
            audit_action(&Method::DELETE, "/system/dict/data/7"),
            Some(("字典管理", "删除字典数据"))
        );
        assert_eq!(
            audit_action(&Method::GET, "/tool/generator/tables/3/download"),
            Some(("代码生成", "下载代码"))
        );
    }

    #[test]
    fn read_routes_are_not_audited() {
        assert_eq!(audit_action(&Method::GET, "/system/users"), None);
        assert_eq!(audit_action(&Method::POST, "/auth/login"), None);
        assert_eq!(
            audit_action(&Method::PUT, "/system/users/12/nickname"),
            None
        );
    }

    #[test]
    fn sensitive_fields_are_masked_recursively() {
        let body = serde_json::to_vec(&json!({
            "username": "admin",
            "password": "admin123",
            "nested": { "newPassword": "x", "keep": "v" },
            "list": [{ "authorization": "Bearer t" }]
        }))
        .expect("序列化请求体");

        let params = masked_params(&body).expect("留痕参数");
        assert!(params.contains("\"username\":\"admin\""));
        assert!(!params.contains("admin123"));
        assert!(!params.contains("Bearer t"));
        assert!(params.contains("\"keep\":\"v\""));
        assert_eq!(params.matches("\"******\"").count(), 3);
    }

    #[test]
    fn empty_body_has_no_params() {
        assert_eq!(masked_params(b""), None);
    }
}
