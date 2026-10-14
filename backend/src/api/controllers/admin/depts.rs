use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, RawQuery, State},
    routing::{get, put},
};

use crate::{
    api::{
        done,
        dto::admin::QueryString,
        extractors::{admin_user::AdminUser, api_json::ApiJson},
        ok,
    },
    container::Container,
    domain::{
        error::AppError,
        models::system_admin::{DeptNode, DeptWriteCommand, IdResponse},
    },
    response::{ApiResponse, csv_attachment},
};

use super::roles::status_query;

pub fn router() -> Router<Arc<Container>> {
    Router::new()
        .route("/system/depts/tree", get(tree))
        .route("/system/depts/export", get(export))
        .route("/system/depts", axum::routing::post(create))
        .route("/system/depts/{id}", put(update).delete(remove))
}

async fn tree(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<ApiResponse<Vec<DeptNode>>, AppError> {
    user.require("system:dept:list")?;
    let query = QueryString::parse(raw.as_deref());
    let nodes = container
        .admin_service
        .list_depts(&status_query(&query)?)
        .await?;

    Ok(ok(nodes))
}

async fn export(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<axum::response::Response, AppError> {
    user.require("system:dept:list")?;
    let query = QueryString::parse(raw.as_deref());
    let csv = container
        .admin_service
        .export_depts(query.text("keyword").as_deref(), query.int("status")?)
        .await?;

    Ok(csv_attachment("depts.csv", csv))
}

async fn create(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    ApiJson(command): ApiJson<DeptWriteCommand>,
) -> Result<ApiResponse<IdResponse>, AppError> {
    user.require("system:dept:add")?;
    let created = container
        .admin_service
        .create_dept(command, user.username())
        .await?;

    Ok(ok(created))
}

async fn update(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
    ApiJson(command): ApiJson<DeptWriteCommand>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:dept:edit")?;
    container
        .admin_service
        .update_dept(id, command, user.username())
        .await?;

    Ok(done())
}

async fn remove(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:dept:remove")?;
    container
        .admin_service
        .delete_dept(id, user.username())
        .await?;

    Ok(done())
}
