use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode, response::IntoResponse,
};

use crate::dto::{
    CreatePermissionRequest, PermissionListQuery, PermissionListResponse, PermissionResponse,
    PermissionTreeResponse, UpdatePermissionRequest,
};
use crate::response::Response;
use crate::router::AppState;
use crate::service::permission_service::*;
use uuid::Uuid;

pub async fn list_permission(
    State(state): State<AppState>,
    Query(query): Query<PermissionListQuery>,
) -> impl IntoResponse {
    match list_permission_service(state, query).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Response::failed(e.to_string())),
        ),
    }
}

pub async fn get_permission_tree(
    State(state): State<AppState>,
) -> impl IntoResponse {
    match get_permission_tree_service(state).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Response::failed(e.to_string())),
        ),
    }
}

pub async fn get_permission(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match get_permission_service(state, id).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}

pub async fn create_permission(
    State(state): State<AppState>,
    Json(payload): Json<CreatePermissionRequest>,
) -> impl IntoResponse {
    match create_permission_service(state, payload).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}

pub async fn update_permission(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdatePermissionRequest>,
) -> impl IntoResponse {
    match update_permission_service(state, id, payload).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}

pub async fn delete_permission(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match delete_permission_service(state, id).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}
