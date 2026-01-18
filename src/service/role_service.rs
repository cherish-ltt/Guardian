use anyhow::{Result, anyhow};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};

use crate::dto::{
    CreateRoleRequest, PermissionSimple, RoleDetailResponse, RoleListQuery, RoleListResponse,
    RoleResponse, UpdateRoleRequest,
};
use crate::entities::{permissions, prelude::*, role_permissions, roles};
use crate::response::Response;
use crate::router::AppState;

pub async fn list_role_service(
    state: AppState,
    query: RoleListQuery,
) -> Result<Response<RoleListResponse>> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let mut select = Roles::find();

    if let Some(keyword) = &query.keyword {
        select = select.filter(
            roles::Column::Name
                .contains(keyword)
                .or(roles::Column::Code.contains(keyword)),
        );
    }

    let total = select.clone().count(&state.conn).await?;

    let roles_list = select
        .order_by_desc(roles::Column::CreatedAt)
        .limit(page_size)
        .offset(offset)
        .all(&state.conn)
        .await?;

    let list: Vec<RoleResponse> = roles_list
        .into_iter()
        .map(|role| RoleResponse {
            id: role.id,
            code: role.code,
            name: role.name,
            description: role.description,
            is_system: role.is_system.unwrap_or(false),
            created_at: role
                .created_at
                .map(|dt| dt.into())
                .unwrap_or_else(chrono::Local::now),
            updated_at: role
                .updated_at
                .map(|dt| dt.into())
                .unwrap_or_else(chrono::Local::now),
        })
        .collect();

    Ok(Response::ok_data(RoleListResponse {
        total,
        page,
        page_size,
        list,
    }))
}

pub async fn get_role_service(
    state: AppState,
    id: uuid::Uuid,
) -> Result<Response<RoleDetailResponse>> {
    let role = Roles::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("角色不存在"))?;

    let role_permissions_list = role_permissions::Entity::find()
        .filter(role_permissions::Column::RoleId.eq(id))
        .find_also_related(permissions::Entity)
        .all(&state.conn)
        .await?;

    let permissions: Vec<PermissionSimple> = role_permissions_list
        .into_iter()
        .filter_map(|rp| rp.1)
        .map(|perm| PermissionSimple {
            id: perm.id,
            code: perm.code,
            name: perm.name,
        })
        .collect();

    Ok(Response::ok_data(RoleDetailResponse {
        id: role.id,
        code: role.code,
        name: role.name,
        description: role.description,
        is_system: role.is_system.unwrap_or(false),
        permissions,
        created_at: role
            .created_at
            .map(|dt| dt.into())
            .unwrap_or_else(chrono::Local::now),
        updated_at: role
            .updated_at
            .map(|dt| dt.into())
            .unwrap_or_else(chrono::Local::now),
    }))
}

pub async fn create_role_service(
    state: AppState,
    payload: CreateRoleRequest,
) -> Result<Response<RoleResponse>> {
    let existing = roles::Entity::find()
        .filter(roles::Column::Code.eq(&payload.code))
        .one(&state.conn)
        .await?;

    if existing.is_some() {
        return Ok(Response::failed("角色代码已存在".to_string()));
    }

    let payload_clone = payload.clone();

    state
        .conn
        .transaction::<_, roles::Model, sea_orm::DbErr>(|txn| {
            Box::pin(async move {
                let role = roles::ActiveModel {
                    id: Set(uuid::Uuid::new_v4()),
                    code: Set(payload_clone.code.clone()),
                    name: Set(payload_clone.name.clone()),
                    description: Set(payload_clone.description.clone()),
                    is_system: Set(Some(false)),
                    created_at: Set(Some(chrono::Local::now().into())),
                    updated_at: Set(Some(chrono::Local::now().into())),
                };

                let role = role.insert(txn).await?;

                if let Some(permission_ids) = payload_clone.permission_ids.clone() {
                    for permission_id in permission_ids {
                        let role_perm = role_permissions::ActiveModel {
                            role_id: Set(role.id),
                            permission_id: Set(permission_id),
                            ..Default::default()
                        };
                        role_perm.insert(txn).await?;
                    }
                }

                Ok::<_, sea_orm::DbErr>(role)
            })
        })
        .await
        .map_err(|e| anyhow!("创建角色失败: {}", e))?;

    let role = roles::Entity::find()
        .filter(roles::Column::Code.eq(&payload.code))
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("角色创建后查询失败"))?;

    Ok(Response::ok(
        Some("创建成功".to_string()),
        RoleResponse {
            id: role.id,
            code: role.code,
            name: role.name,
            description: role.description,
            is_system: role.is_system.unwrap_or(false),
            created_at: role
                .created_at
                .map(|dt| dt.into())
                .unwrap_or_else(chrono::Local::now),
            updated_at: role
                .updated_at
                .map(|dt| dt.into())
                .unwrap_or_else(chrono::Local::now),
        },
    ))
}

pub async fn update_role_service(
    state: AppState,
    id: uuid::Uuid,
    payload: UpdateRoleRequest,
) -> Result<Response<RoleResponse>> {
    let role = Roles::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("角色不存在"))?;

    if role.is_system.unwrap_or(false) {
        return Ok(Response::failed("系统内置角色不可修改".to_string()));
    }

    state
        .conn
        .transaction::<_, roles::Model, sea_orm::DbErr>(|txn| {
            Box::pin(async move {
                let mut role_model: roles::ActiveModel = role.into_active_model();

                if let Some(name) = &payload.name {
                    role_model.name = Set(name.clone());
                }

                if let Some(description) = &payload.description {
                    role_model.description = Set(Some(description.clone()));
                }

                if let Some(permission_ids) = &payload.permission_ids {
                    RolePermissions::delete_many()
                        .filter(role_permissions::Column::RoleId.eq(id))
                        .exec(txn)
                        .await?;

                    for permission_id in permission_ids {
                        let role_perm = role_permissions::ActiveModel {
                            role_id: Set(id),
                            permission_id: Set(*permission_id),
                            ..Default::default()
                        };
                        role_perm.insert(txn).await?;
                    }
                }

                role_model.updated_at = Set(Some(chrono::Local::now().into()));
                let role = role_model.update(txn).await?;

                Ok::<_, sea_orm::DbErr>(role)
            })
        })
        .await
        .map_err(|e| anyhow!("更新角色失败: {}", e))?;

    let role = Roles::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("角色更新后查询失败"))?;

    Ok(Response::ok(
        Some("更新成功".to_string()),
        RoleResponse {
            id: role.id,
            code: role.code,
            name: role.name,
            description: role.description,
            is_system: role.is_system.unwrap_or(false),
            created_at: role
                .created_at
                .map(|dt| dt.into())
                .unwrap_or_else(chrono::Local::now),
            updated_at: role
                .updated_at
                .map(|dt| dt.into())
                .unwrap_or_else(chrono::Local::now),
        },
    ))
}

pub async fn delete_role_service(state: AppState, id: uuid::Uuid) -> Result<Response<()>> {
    let role = Roles::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("角色不存在"))?;

    if role.is_system.unwrap_or(false) {
        return Ok(Response::failed("系统内置角色不可删除".to_string()));
    }

    role_permissions::Entity::delete_many()
        .filter(role_permissions::Column::RoleId.eq(id))
        .exec(&state.conn)
        .await?;

    let role_model: roles::ActiveModel = role.into_active_model();
    role_model.delete(&state.conn).await?;

    Ok(Response::ok_msg(Some("删除成功".to_string())))
}

pub async fn assign_permissions_service(
    state: AppState,
    id: uuid::Uuid,
    permission_ids: Vec<uuid::Uuid>,
) -> Result<Response<()>> {
    let role = Roles::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("角色不存在"))?;

    state
        .conn
        .transaction::<_, (), sea_orm::DbErr>(|txn| {
            Box::pin(async move {
                RolePermissions::delete_many()
                    .filter(role_permissions::Column::RoleId.eq(id))
                    .exec(txn)
                    .await?;

                for permission_id in permission_ids {
                    let role_perm = role_permissions::ActiveModel {
                        role_id: Set(id),
                        permission_id: Set(permission_id),
                        ..Default::default()
                    };
                    role_perm.insert(txn).await?;
                }

                Ok::<_, sea_orm::DbErr>(())
            })
        })
        .await
        .map_err(|e| anyhow!("分配权限失败: {}", e))?;

    Ok(Response::ok_msg(Some("权限分配成功".to_string())))
}
