use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::dto::{SystemInfoQuery, SystemInfoResponse};
use crate::response::Response;
use crate::router::AppState;
use crate::service::list_system_info_service;

pub async fn list_system_info(
    State(state): State<AppState>,
    Query(query): Query<SystemInfoQuery>,
) -> impl IntoResponse {
    match list_system_info_service(state, query).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Response::failed(e.to_string())),
        ),
    }
}
