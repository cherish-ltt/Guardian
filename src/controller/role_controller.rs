use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};

use crate::dto::{
    CreateRoleRequest, RoleDetailResponse, RoleListQuery, RoleListResponse, RoleResponse,
    UpdateRoleRequest,
};
use crate::response::Response;
use crate::router::AppState;
use crate::service::role_service::*;
use uuid::Uuid;

pub async fn list_role(
    State(state): State<AppState>,
    Query(query): Query<RoleListQuery>,
) -> (StatusCode, Json<Response<RoleListResponse>>) {
    match list_role_service(state, query).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Response::failed(e.to_string())),
        ),
    }
}

pub async fn get_role(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> (StatusCode, Json<Response<RoleDetailResponse>>) {
    match get_role_service(state, id).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}

pub async fn create_role(
    State(state): State<AppState>,
    Json(payload): Json<CreateRoleRequest>,
) -> (StatusCode, Json<Response<RoleResponse>>) {
    match create_role_service(state, payload).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}

pub async fn update_role(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateRoleRequest>,
) -> (StatusCode, Json<Response<RoleResponse>>) {
    match update_role_service(state, id, payload).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}

pub async fn delete_role(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> (StatusCode, Json<Response<()>>) {
    match delete_role_service(state, id).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}

pub async fn assign_permissions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<Response<()>>) {
    let permission_ids: Vec<Uuid> = payload
        .get("permission_ids")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().and_then(|s| Uuid::parse_str(s).ok()))
                .collect::<Vec<_>>()
                .into()
        })
        .unwrap_or_default();

    match assign_permissions_service(state, id, permission_ids).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (StatusCode::OK, Json(Response::failed(e.to_string()))),
    }
}
