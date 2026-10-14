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
        models::{
            pagination::PageResult,
            system_admin::{IdResponse, PostRow, PostWriteCommand},
        },
    },
    response::{ApiResponse, csv_attachment},
};

use super::roles::status_query;

pub fn router() -> Router<Arc<Container>> {
    Router::new()
        .route("/system/posts", get(list).post(create))
        .route("/system/posts/export", get(export))
        .route("/system/posts/{id}", put(update).delete(remove))
}

async fn list(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<ApiResponse<PageResult<PostRow>>, AppError> {
    user.require("system:post:list")?;
    let query = QueryString::parse(raw.as_deref());
    let rows = container
        .admin_service
        .list_posts(&status_query(&query)?)
        .await?;

    Ok(ok(rows))
}

async fn export(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<axum::response::Response, AppError> {
    user.require("system:post:list")?;
    let query = QueryString::parse(raw.as_deref());
    let csv = container
        .admin_service
        .export_posts(query.text("keyword").as_deref(), query.int("status")?)
        .await?;

    Ok(csv_attachment("posts.csv", csv))
}

async fn create(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    ApiJson(command): ApiJson<PostWriteCommand>,
) -> Result<ApiResponse<IdResponse>, AppError> {
    user.require("system:post:add")?;
    let created = container
        .admin_service
        .create_post(command, user.username())
        .await?;

    Ok(ok(created))
}

async fn update(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
    ApiJson(command): ApiJson<PostWriteCommand>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:post:edit")?;
    container
        .admin_service
        .update_post(id, command, user.username())
        .await?;

    Ok(done())
}

async fn remove(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:post:remove")?;
    container
        .admin_service
        .delete_post(id, user.username())
        .await?;

    Ok(done())
}
