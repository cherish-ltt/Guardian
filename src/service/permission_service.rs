use anyhow::{Result, anyhow};
use sea_orm::ActiveValue::NotSet;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

use crate::dto::{
    CreatePermissionRequest, PermissionListQuery, PermissionListResponse, PermissionResponse,
    PermissionTreeResponse, UpdatePermissionRequest,
};
use crate::entities::{permissions, prelude::*};
use crate::response::Response;
use crate::router::AppState;

pub async fn list_permission_service(
    state: AppState,
    query: PermissionListQuery,
) -> Result<Response<PermissionListResponse>> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let mut select = Permissions::find();

    if let Some(resource_type) = &query.resource_type {
        select = select.filter(permissions::Column::ResourceType.eq(resource_type));
    }

    if let Some(keyword) = &query.keyword {
        select = select.filter(
            permissions::Column::Name
                .contains(keyword)
                .or(permissions::Column::Code.contains(keyword)),
        );
    }

    let paginator = select
        .order_by_asc(permissions::Column::SortOrder)
        .paginate(&state.conn, page_size);

    let total = paginator
        .num_items_and_pages()
        .await
        .map(|(items)| items.number_of_items)
        .unwrap_or(0);
    let permissions_list = paginator.fetch_page(page - 1).await?;

    let list: Vec<PermissionResponse> = permissions_list
        .into_iter()
        .map(|perm| PermissionResponse {
            id: perm.id,
            code: perm.code,
            name: perm.name,
            description: perm.description,
            resource_type: perm.resource_type,
            http_method: perm.http_method,
            resource_path: perm.resource_path,
            parent_id: perm.parent_id,
            sort_order: perm.sort_order.unwrap_or(0),
            is_system: perm.is_system.unwrap_or(false),
            created_at: perm
                .created_at
                .map(|dt| dt.into())
                .unwrap_or_else(chrono::Local::now),
        })
        .collect();

    Ok(Response::ok_data(PermissionListResponse {
        total,
        page,
        page_size,
        list,
    }))
}

pub async fn get_permission_tree_service(
    state: AppState,
) -> Result<Response<Vec<PermissionTreeResponse>>> {
    let all_permissions = Permissions::find()
        .order_by_asc(permissions::Column::SortOrder)
        .all(&state.conn)
        .await?;

    let tree = build_permission_tree(&all_permissions, None);

    Ok(Response::ok_data(tree))
}

fn build_permission_tree(
    permissions: &[permissions::Model],
    parent_id: Option<uuid::Uuid>,
) -> Vec<PermissionTreeResponse> {
    let mut tree = Vec::new();

    for perm in permissions {
        if perm.parent_id == parent_id {
            let children = build_permission_tree(permissions, Some(perm.id));
            tree.push(PermissionTreeResponse {
                id: perm.id,
                code: perm.code.clone(),
                name: perm.name.clone(),
                description: perm.description.clone(),
                resource_type: perm.resource_type.clone(),
                http_method: perm.http_method.clone(),
                resource_path: perm.resource_path.clone(),
                sort_order: perm.sort_order.unwrap_or(0),
                is_system: perm.is_system.unwrap_or(false),
                children,
            });
        }
    }

    tree
}

pub async fn get_permission_service(
    state: AppState,
    id: uuid::Uuid,
) -> Result<Response<PermissionResponse>> {
    let permission = Permissions::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("权限不存在"))?;

    Ok(Response::ok_data(PermissionResponse {
        id: permission.id,
        code: permission.code,
        name: permission.name,
        description: permission.description,
        resource_type: permission.resource_type,
        http_method: permission.http_method,
        resource_path: permission.resource_path,
        parent_id: permission.parent_id,
        sort_order: permission.sort_order.unwrap_or(0),
        is_system: permission.is_system.unwrap_or(false),
        created_at: permission
            .created_at
            .map(|dt| dt.into())
            .unwrap_or_else(chrono::Local::now),
    }))
}

pub async fn create_permission_service(
    state: AppState,
    payload: CreatePermissionRequest,
) -> Result<Response<PermissionResponse>> {
    let existing = Permissions::find()
        .filter(permissions::Column::Code.eq(&payload.code))
        .one(&state.conn)
        .await?;

    if existing.is_some() {
        return Ok(Response::failed("权限代码已存在".to_string()));
    }

    let permission = permissions::ActiveModel {
        id: NotSet,
        code: Set(payload.code),
        name: Set(payload.name),
        description: Set(payload.description),
        resource_type: Set(payload.resource_type),
        http_method: Set(payload.http_method),
        resource_path: Set(payload.resource_path),
        parent_id: Set(payload.parent_id),
        sort_order: Set(payload.sort_order),
        is_system: Set(Some(false)),
        created_at: Set(Some(chrono::Local::now().into())),
        updated_at: Set(Some(chrono::Local::now().into())),
    };

    let permission = permission.insert(&state.conn).await?;

    Ok(Response::ok_data(PermissionResponse {
        id: permission.id,
        code: permission.code,
        name: permission.name,
        description: permission.description,
        resource_type: permission.resource_type,
        http_method: permission.http_method,
        resource_path: permission.resource_path,
        parent_id: permission.parent_id,
        sort_order: permission.sort_order.unwrap_or(0),
        is_system: permission.is_system.unwrap_or(false),
        created_at: permission
            .created_at
            .map(|dt| dt.into())
            .unwrap_or_else(chrono::Local::now),
    }))
}

pub async fn update_permission_service(
    state: AppState,
    id: uuid::Uuid,
    payload: UpdatePermissionRequest,
) -> Result<Response<PermissionResponse>> {
    let permission = Permissions::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("权限不存在"))?;

    if permission.is_system.unwrap_or(false) {
        return Ok(Response::failed("系统内置权限不可修改".to_string()));
    }

    let mut perm_model: permissions::ActiveModel = permission.into_active_model();

    if let Some(name) = payload.name {
        perm_model.name = Set(name);
    }

    if let Some(description) = payload.description {
        perm_model.description = Set(Some(description));
    }

    if let Some(resource_type) = payload.resource_type {
        perm_model.resource_type = Set(resource_type);
    }

    if let Some(http_method) = payload.http_method {
        perm_model.http_method = Set(Some(http_method));
    }

    if let Some(resource_path) = payload.resource_path {
        perm_model.resource_path = Set(Some(resource_path));
    }

    if let Some(parent_id) = payload.parent_id {
        perm_model.parent_id = Set(Some(parent_id));
    }

    if let Some(sort_order) = payload.sort_order {
        perm_model.sort_order = Set(Some(sort_order));
    }

    perm_model.updated_at = Set(Some(chrono::Local::now().into()));
    let permission = perm_model.update(&state.conn).await?;

    Ok(Response::ok_data(PermissionResponse {
        id: permission.id,
        code: permission.code,
        name: permission.name,
        description: permission.description,
        resource_type: permission.resource_type,
        http_method: permission.http_method,
        resource_path: permission.resource_path,
        parent_id: permission.parent_id,
        sort_order: permission.sort_order.unwrap_or(0),
        is_system: permission.is_system.unwrap_or(false),
        created_at: permission
            .created_at
            .map(|dt| dt.into())
            .unwrap_or_else(chrono::Local::now),
    }))
}

pub async fn delete_permission_service(state: AppState, id: uuid::Uuid) -> Result<Response<()>> {
    let permission = permissions::Entity::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("权限不存在"))?;

    if permission.is_system.unwrap_or(false) {
        return Ok(Response::failed("系统内置权限不可删除".to_string()));
    }

    let perm_model: permissions::ActiveModel = permission.into_active_model();
    perm_model.delete(&state.conn).await?;

    Ok(Response::ok_msg(Some("删除成功".to_string())))
}
