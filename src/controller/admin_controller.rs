use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
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
) -> (StatusCode, Json<Response<AdminListResponse>>) {
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
) -> (StatusCode, Json<Response<AdminDetailResponse>>) {
    match get_admin_service(state, id).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}

pub async fn create_admin(
    State(state): State<AppState>,
    Json(payload): Json<CreateAdminRequest>,
) -> (StatusCode, Json<Response<AdminResponse>>) {
    match create_admin_service(state, payload).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}

pub async fn update_admin(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(payload): Json<UpdateAdminRequest>,
) -> (StatusCode, Json<Response<AdminResponse>>) {
    match update_admin_service(state, id, payload).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}

pub async fn delete_admin(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> (StatusCode, Json<Response<()>>) {
    match delete_admin_service(state, id).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}
