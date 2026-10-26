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
                IdResponse, UserOptions, UserPasswordCommand, UserQuery, UserRow,
                UserStatusCommand, UserWriteCommand,
            },
        },
    },
    response::{ApiResponse, csv_attachment},
};

pub fn router() -> Router<Arc<Container>> {
    Router::new()
        .route("/system/users", get(list).post(create))
        .route("/system/users/options", get(options))
        .route("/system/users/export", get(export))
        .route("/system/users/{id}", put(update).delete(remove))
        .route("/system/users/{id}/status", put(update_status))
        .route("/system/users/{id}/password", put(reset_password))
}

async fn list(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<ApiResponse<PageResult<UserRow>>, AppError> {
    user.require("system:user:list")?;
    let query = QueryString::parse(raw.as_deref());
    let scope = user.data_scope(&container).await?;
    let rows = container
        .admin_service
        .list_users(&filter_of(&query)?, &scope)
        .await?;

    Ok(ok(rows))
}

/// 用户页所需的引用选项，避免为只读用户申请角色、岗位和参数管理权限。
async fn options(
    State(container): State<Arc<Container>>,
    user: AdminUser,
) -> Result<ApiResponse<UserOptions>, AppError> {
    user.require("system:user:list")?;
    let can_edit =
        user.has_permission("system:user:add") || user.has_permission("system:user:edit");
    let can_set_password =
        user.has_permission("system:user:add") || user.has_permission("system:user:resetPwd");
    let options = container
        .admin_service
        .user_options(can_edit, can_set_password)
        .await?;

    Ok(ok(options))
}

async fn export(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<axum::response::Response, AppError> {
    user.require("system:user:list")?;
    let query = QueryString::parse(raw.as_deref());
    let scope = user.data_scope(&container).await?;
    let csv = container
        .admin_service
        .export_users(&filter_of(&query)?, &scope)
        .await?;

    Ok(csv_attachment("users.csv", csv))
}

async fn create(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    ApiJson(command): ApiJson<UserWriteCommand>,
) -> Result<ApiResponse<IdResponse>, AppError> {
    user.require("system:user:add")?;
    let created = container
        .admin_service
        .create_user(command, user.username(), user.is_super_admin())
        .await?;

    Ok(ok(created))
}

async fn update(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
    ApiJson(command): ApiJson<UserWriteCommand>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:user:edit")?;
    container
        .admin_service
        .update_user(id, command, user.username(), user.is_super_admin())
        .await?;

    Ok(done())
}

async fn update_status(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
    ApiJson(command): ApiJson<UserStatusCommand>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:user:edit")?;
    container
        .admin_service
        .update_user_status(id, command.status, user.username())
        .await?;

    Ok(done())
}

async fn reset_password(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
    ApiJson(command): ApiJson<UserPasswordCommand>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:user:resetPwd")?;
    container
        .admin_service
        .reset_user_password(id, command.password, user.username())
        .await?;

    Ok(done())
}

async fn remove(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:user:remove")?;
    container
        .admin_service
        .delete_user(id, user.username())
        .await?;

    Ok(done())
}

fn filter_of(query: &QueryString) -> Result<UserQuery, AppError> {
    Ok(UserQuery {
        page: query.page()?,
        keyword: query.text("keyword"),
        status: query.int("status")?,
        dept_id: query.int("deptId")?,
        post_id: query.int("postId")?,
    })
}
