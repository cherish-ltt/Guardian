use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::dto::{
    AdminDetailResponse, AdminListQuery, AdminListResponse, AdminResponse, CreateAdminRequest,
    UpdateAdminRequest,
};
use crate::response::Response;
use crate::router::AppState;
use crate::service::admin_service::*;

pub async fn list_admin(
    State(state): State<AppState>,
    Query(query): Query<AdminListQuery>,
) -> impl IntoResponse {
    match list_admin_service(state, query).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Response::failed(e.to_string())),
        ),
    }
}

pub async fn get_admin(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    match get_admin_service(state, id).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}

pub async fn create_admin(
    State(state): State<AppState>,
    Json(payload): Json<CreateAdminRequest>,
) -> impl IntoResponse {
    match create_admin_service(state, payload).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}

pub async fn update_admin(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(payload): Json<UpdateAdminRequest>,
) -> impl IntoResponse {
    match update_admin_service(state, id, payload).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}

pub async fn delete_admin(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    match delete_admin_service(state, id).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}
