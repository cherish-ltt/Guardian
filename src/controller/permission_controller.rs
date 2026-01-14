use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
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
) -> (StatusCode, Json<Response<PermissionListResponse>>) {
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
) -> (StatusCode, Json<Response<Vec<PermissionTreeResponse>>>) {
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
) -> (StatusCode, Json<Response<PermissionResponse>>) {
    match get_permission_service(state, id).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}

pub async fn create_permission(
    State(state): State<AppState>,
    Json(payload): Json<CreatePermissionRequest>,
) -> (StatusCode, Json<Response<PermissionResponse>>) {
    match create_permission_service(state, payload).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}

pub async fn update_permission(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdatePermissionRequest>,
) -> (StatusCode, Json<Response<PermissionResponse>>) {
    match update_permission_service(state, id, payload).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}

pub async fn delete_permission(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> (StatusCode, Json<Response<()>>) {
    match delete_permission_service(state, id).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}
