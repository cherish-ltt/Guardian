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
            system_admin::{ConfigRow, ConfigWriteCommand, IdResponse, KeywordQuery},
        },
    },
    response::{ApiResponse, csv_attachment},
};

pub fn router() -> Router<Arc<Container>> {
    Router::new()
        .route("/system/configs", get(list).post(create))
        .route("/system/configs/export", get(export))
        .route("/system/configs/{id}", put(update).delete(remove))
}

async fn list(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<ApiResponse<PageResult<ConfigRow>>, AppError> {
    user.require("system:config:list")?;
    let query = QueryString::parse(raw.as_deref());
    let rows = container
        .admin_service
        .list_configs(&KeywordQuery {
            page: query.page()?,
            keyword: query.text("keyword"),
        })
        .await?;

    Ok(ok(rows))
}

async fn export(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<axum::response::Response, AppError> {
    user.require("system:config:list")?;
    let query = QueryString::parse(raw.as_deref());
    let csv = container
        .admin_service
        .export_configs(query.text("keyword").as_deref())
        .await?;

    Ok(csv_attachment("configs.csv", csv))
}

async fn create(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    ApiJson(command): ApiJson<ConfigWriteCommand>,
) -> Result<ApiResponse<IdResponse>, AppError> {
    user.require("system:config:add")?;
    let created = container
        .admin_service
        .create_config(command, user.username())
        .await?;

    Ok(ok(created))
}

async fn update(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
    ApiJson(command): ApiJson<ConfigWriteCommand>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:config:edit")?;
    container
        .admin_service
        .update_config(id, command, user.username())
        .await?;

    Ok(done())
}

async fn remove(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:config:remove")?;
    container.admin_service.delete_config(id).await?;

    Ok(done())
}
