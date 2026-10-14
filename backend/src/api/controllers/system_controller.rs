use std::sync::Arc;

use axum::{Router, extract::State, routing::get};

use serde::Serialize;

use crate::{
    api::{
        dto::system::{HealthResponse, WelcomeResponse},
        ok,
    },
    container::Container,
    error::AppError,
    response::ApiResponse,
};

/// 前端健康探针的业务数据。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendHealthResponse {
    pub status: String,
    pub time: String,
    pub service_name: String,
}

#[cfg(feature = "docs")]
use crate::docs::{DocErrorResponse, DocHealthResponse, DocMessageResponse, DocWelcomeResponse};

pub fn router() -> Router<Arc<Container>> {
    Router::new()
        // 后台首页的健康探针：`status` 为 `UP` 时视为正常（与 Spring 版一致）。
        .route("/health", get(backend_health))
        .route("/system/health", get(health))
        .route("/system/ready", get(ready))
}

/// `/api/v1/health`：登录页与顶栏用的轻量探针，不查数据库。
async fn backend_health(
    State(container): State<Arc<Container>>,
) -> Result<ApiResponse<BackendHealthResponse>, AppError> {
    Ok(ok(BackendHealthResponse {
        status: "UP".to_string(),
        time: crate::util::time::now(),
        service_name: container.config.app_name.clone(),
    }))
}

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/",
    tag = "System",
    responses(
        (status = 200, description = "Welcome page returned", body = DocWelcomeResponse)
    )
))]
pub async fn root(
    State(container): State<Arc<Container>>,
) -> Result<ApiResponse<WelcomeResponse>, AppError> {
    let welcome = container.system_service.welcome().await?;
    Ok(ApiResponse::ok(welcome.into()))
}

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/api/v1/system/health",
    tag = "System",
    responses(
        (status = 200, description = "Health check passed", body = DocHealthResponse),
        (status = 503, description = "Service degraded or database unavailable", body = DocHealthResponse)
    )
))]
pub async fn health(
    State(container): State<Arc<Container>>,
) -> Result<ApiResponse<HealthResponse>, AppError> {
    let health = container.system_service.health().await?;

    Ok(ApiResponse::ok_with_message(
        "Health check completed",
        health.into(),
    ))
}

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/api/v1/system/ready",
    tag = "System",
    responses(
        (status = 200, description = "Service ready", body = DocMessageResponse),
        (status = 503, description = "Service not ready", body = DocErrorResponse)
    )
))]
pub async fn ready(State(container): State<Arc<Container>>) -> Result<ApiResponse<()>, AppError> {
    container.system_service.ready().await?;
    Ok(ApiResponse::message("Service ready"))
}
