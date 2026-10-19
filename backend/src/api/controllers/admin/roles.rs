use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, RawQuery, State},
    routing::{get, put},
};
use serde::Serialize;

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
            system_admin::{IdResponse, PermissionGraph, RoleRow, RoleWriteCommand, StatusQuery},
        },
    },
    response::{ApiResponse, csv_attachment},
};

/// 角色已授权菜单与自定义数据范围：`{"menuIds": [...], "deptIds": [...]}`。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoleMenusResponse {
    menu_ids: Vec<i64>,
    dept_ids: Vec<i64>,
}

pub fn router() -> Router<Arc<Container>> {
    Router::new()
        .route("/system/roles", get(list).post(create))
        .route("/system/roles/export", get(export))
        .route("/system/permissions/graph", get(graph))
        .route("/system/roles/{id}/menus", get(menus))
        .route("/system/roles/{id}", put(update).delete(remove))
}

async fn list(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<ApiResponse<PageResult<RoleRow>>, AppError> {
    user.require("system:role:list")?;
    let query = QueryString::parse(raw.as_deref());
    let rows = container
        .admin_service
        .list_roles(&status_query(&query)?)
        .await?;

    Ok(ok(rows))
}

async fn export(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<axum::response::Response, AppError> {
    user.require("system:role:list")?;
    let query = QueryString::parse(raw.as_deref());
    let csv = container
        .admin_service
        .export_roles(query.text("keyword").as_deref(), query.int("status")?)
        .await?;

    Ok(csv_attachment("roles.csv", csv))
}

/// 权限关系图：用户节点只对有用户查询权限的账号返回，并遵守数据权限。
async fn graph(
    State(container): State<Arc<Container>>,
    user: AdminUser,
) -> Result<ApiResponse<PermissionGraph>, AppError> {
    user.require("system:role:list")?;
    let scope = user.data_scope(&container).await?;
    let graph = container
        .admin_service
        .permission_graph(&scope, user.has_permission("system:user:list"))
        .await?;

    Ok(ok(graph))
}

async fn menus(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<RoleMenusResponse>, AppError> {
    user.require("system:role:list")?;
    let menu_ids = container.admin_service.role_menus(id).await?;
    let dept_ids = container.admin_service.role_depts(id).await?;

    Ok(ok(RoleMenusResponse { menu_ids, dept_ids }))
}

async fn create(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    ApiJson(command): ApiJson<RoleWriteCommand>,
) -> Result<ApiResponse<IdResponse>, AppError> {
    user.require("system:role:add")?;
    let created = container
        .admin_service
        .create_role(command, user.username())
        .await?;

    Ok(ok(created))
}

async fn update(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
    ApiJson(command): ApiJson<RoleWriteCommand>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:role:edit")?;
    container
        .admin_service
        .update_role(id, command, user.username())
        .await?;

    Ok(done())
}

async fn remove(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:role:remove")?;
    container
        .admin_service
        .delete_role(id, user.username())
        .await?;

    Ok(done())
}

pub(crate) fn status_query(query: &QueryString) -> Result<StatusQuery, AppError> {
    Ok(StatusQuery {
        page: query.page()?,
        keyword: query.text("keyword"),
        status: query.int("status")?,
    })
}
