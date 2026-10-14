use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, RawQuery, State},
    routing::{get, post, put},
};

use crate::{
    api::{
        done,
        dto::admin::{GeneratorImportRequest, QueryString},
        extractors::{admin_user::AdminUser, api_json::ApiJson},
        ok,
    },
    container::Container,
    domain::{
        error::AppError,
        models::{
            generator::{
                GenColumnRow, GenGenerateResult, GenImportResult, GenPreview, GenTableRow,
                GenTableUpdateCommand, ImportableTable,
            },
            pagination::PageResult,
            system_admin::KeywordQuery,
        },
    },
    response::{ApiResponse, zip_attachment},
};

pub fn router() -> Router<Arc<Container>> {
    Router::new()
        .route("/tool/generator/tables", get(list))
        .route("/tool/generator/importable-tables", get(importable))
        .route("/tool/generator/import", post(import))
        .route("/tool/generator/tables/{id}/columns", get(columns))
        .route("/tool/generator/tables/{id}/preview", get(preview))
        .route("/tool/generator/tables/{id}/generate", post(generate))
        .route("/tool/generator/tables/{id}/download", get(download))
        .route("/tool/generator/tables/{id}", put(update))
}

async fn list(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<ApiResponse<PageResult<GenTableRow>>, AppError> {
    user.require("tool:gen:list")?;
    let query = QueryString::parse(raw.as_deref());
    let rows = container
        .generator_service
        .tables(&KeywordQuery {
            page: query.page(),
            keyword: query.text("keyword"),
        })
        .await?;

    Ok(ok(rows))
}

async fn importable(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<ApiResponse<Vec<ImportableTable>>, AppError> {
    user.require("tool:gen:list")?;
    let query = QueryString::parse(raw.as_deref());
    let tables = container
        .generator_service
        .importable_tables(query.text("keyword").as_deref())
        .await?;

    Ok(ok(tables))
}

async fn import(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    ApiJson(request): ApiJson<GeneratorImportRequest>,
) -> Result<ApiResponse<GenImportResult>, AppError> {
    user.require("tool:gen:list")?;
    let imported = container
        .generator_service
        .import_tables(request.table_names.unwrap_or_default(), user.username())
        .await?;

    Ok(ok(imported))
}

async fn columns(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<Vec<GenColumnRow>>, AppError> {
    user.require("tool:gen:list")?;
    let columns = container.generator_service.columns(id).await?;

    Ok(ok(columns))
}

async fn preview(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<GenPreview>, AppError> {
    user.require("tool:gen:list")?;
    let preview = container.generator_service.preview(id).await?;

    Ok(ok(preview))
}

async fn update(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
    ApiJson(command): ApiJson<GenTableUpdateCommand>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("tool:gen:list")?;
    container
        .generator_service
        .update_table(id, command, user.username())
        .await?;

    Ok(done())
}

async fn generate(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<GenGenerateResult>, AppError> {
    user.require("tool:gen:list")?;
    let result = container.generator_service.generate(id).await?;

    Ok(ok(result))
}

async fn download(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
) -> Result<axum::response::Response, AppError> {
    user.require("tool:gen:list")?;
    let (file_name, bytes) = container.generator_service.download(id).await?;

    Ok(zip_attachment(&file_name, bytes))
}
