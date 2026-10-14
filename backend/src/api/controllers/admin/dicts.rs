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
            system_admin::{
                DictDataRow, DictDataWriteCommand, DictOption, DictTypeRow, DictTypeWriteCommand,
                IdResponse,
            },
        },
    },
    response::{ApiResponse, csv_attachment},
};

use super::roles::status_query;

pub fn router() -> Router<Arc<Container>> {
    Router::new()
        .route("/system/dict/types", get(list).post(create_type))
        .route("/system/dict/types/export", get(export))
        .route("/system/dict/data", axum::routing::post(create_data))
        .route("/system/dict/options/{dict_type}", get(options))
        .route(
            "/system/dict/types/{id}",
            put(update_type).delete(remove_type),
        )
        // GET 传字典类型、PUT / DELETE 传主键：axum 要求同一位置的路径参数同名，
        // 因此两处都叫 `dict_type`，取值为数字时按主键解析。
        .route(
            "/system/dict/data/{dict_type}",
            get(data).put(update_data).delete(remove_data),
        )
}

async fn list(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<ApiResponse<PageResult<DictTypeRow>>, AppError> {
    user.require("system:dict:list")?;
    let query = QueryString::parse(raw.as_deref());
    let rows = container
        .admin_service
        .list_dict_types(&status_query(&query)?)
        .await?;

    Ok(ok(rows))
}

async fn export(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<axum::response::Response, AppError> {
    user.require("system:dict:list")?;
    let query = QueryString::parse(raw.as_deref());
    let csv = container
        .admin_service
        .export_dict_types(query.text("keyword").as_deref(), query.int("status")?)
        .await?;

    Ok(csv_attachment("dict-types.csv", csv))
}

/// 业务下拉：只要求已登录，不要求字典管理权限（与 Spring 版一致）。
async fn options(
    State(container): State<Arc<Container>>,
    _user: AdminUser,
    Path(dict_type): Path<String>,
) -> Result<ApiResponse<Vec<DictOption>>, AppError> {
    let options = container.admin_service.dict_options(&dict_type).await?;

    Ok(ok(options))
}

async fn data(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(dict_type): Path<String>,
) -> Result<ApiResponse<Vec<DictDataRow>>, AppError> {
    user.require("system:dict:list")?;
    let rows = container.admin_service.list_dict_data(&dict_type).await?;

    Ok(ok(rows))
}

async fn create_type(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    ApiJson(command): ApiJson<DictTypeWriteCommand>,
) -> Result<ApiResponse<IdResponse>, AppError> {
    user.require("system:dict:add")?;
    let created = container
        .admin_service
        .create_dict_type(command, user.username())
        .await?;

    Ok(ok(created))
}

async fn update_type(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
    ApiJson(command): ApiJson<DictTypeWriteCommand>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:dict:edit")?;
    container
        .admin_service
        .update_dict_type(id, command, user.username())
        .await?;

    Ok(done())
}

async fn remove_type(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:dict:remove")?;
    container.admin_service.delete_dict_type(id).await?;

    Ok(done())
}

async fn create_data(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    ApiJson(command): ApiJson<DictDataWriteCommand>,
) -> Result<ApiResponse<IdResponse>, AppError> {
    user.require("system:dict:add")?;
    let created = container
        .admin_service
        .create_dict_data(command, user.username())
        .await?;

    Ok(ok(created))
}

async fn update_data(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
    ApiJson(command): ApiJson<DictDataWriteCommand>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:dict:edit")?;
    container
        .admin_service
        .update_dict_data(id, command, user.username())
        .await?;

    Ok(done())
}

async fn remove_data(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:dict:remove")?;
    container.admin_service.delete_dict_data(id).await?;

    Ok(done())
}
