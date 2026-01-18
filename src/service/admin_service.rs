use anyhow::{Result, anyhow};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, NotSet, PaginatorTrait,
    QueryFilter, QueryOrder, RelationTrait, Set,
};

use crate::dto::{
    AdminDetailResponse, AdminListQuery, AdminListResponse, AdminResponse, CreateAdminRequest,
    RoleSimple, UpdateAdminRequest,
};
use crate::entities::prelude::*;
use crate::entities::{admin_roles, admins};
use crate::response::Response;
use crate::router::AppState;
use crate::utils::hash_password;

pub async fn list_admin_service(
    state: AppState,
    query: AdminListQuery,
) -> Result<Response<AdminListResponse>> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let mut select = Admins::find();

    if let Some(status) = query.status {
        select = select.filter(admins::Column::Status.eq(Some(status)));
    }

    if let Some(keyword) = &query.keyword {
        select = select.filter(admins::Column::Username.contains(keyword));
    }

    let paginator = select
        .order_by_desc(admins::Column::CreatedAt)
        .paginate(&state.conn, page_size);

    let total = paginator
        .num_items_and_pages()
        .await
        .map(|(items)| items.number_of_items)
        .unwrap_or(0);
    let admins_list = paginator.fetch_page(page - 1).await?;

    let list: Vec<AdminResponse> = admins_list
        .into_iter()
        .map(|admin| AdminResponse {
            id: admin.id,
            username: admin.username,
            is_super_admin: admin.is_super_admin.unwrap_or(false),
            status: admin.status.unwrap_or(1),
            last_login_at: admin.last_login_at.map(|dt| dt.into()),
            created_at: admin
                .created_at
                .map(|dt| dt.into())
                .unwrap_or_else(chrono::Local::now),
            updated_at: admin
                .updated_at
                .map(|dt| dt.into())
                .unwrap_or_else(chrono::Local::now),
        })
        .collect();

    Ok(Response::ok_data(AdminListResponse {
        total,
        page,
        page_size,
        list,
    }))
}

pub async fn get_admin_service(
    state: AppState,
    id: uuid::Uuid,
) -> Result<Response<AdminDetailResponse>> {
    let admin = Admins::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("管理员不存在"))?;

    let roles = admin.find_related(Roles).all(&state.conn).await?;

    let roles_vec: Vec<RoleSimple> = roles
        .into_iter()
        .map(|role| RoleSimple {
            id: role.id,
            code: role.code,
            name: role.name,
        })
        .collect();

    Ok(Response::ok_data(AdminDetailResponse {
        id: admin.id,
        username: admin.username,
        is_super_admin: admin.is_super_admin.unwrap_or(false),
        status: admin.status.unwrap_or(1),
        last_login_at: admin.last_login_at.map(|dt| dt.into()),
        login_attempts: admin.login_attempts.unwrap_or(0),
        locked_until: admin.locked_until.map(|dt| dt.into()),
        created_at: admin
            .created_at
            .map(|dt| dt.into())
            .unwrap_or_else(chrono::Local::now),
        updated_at: admin
            .updated_at
            .map(|dt| dt.into())
            .unwrap_or_else(chrono::Local::now),
        roles: roles_vec,
    }))
}

pub async fn create_admin_service(
    state: AppState,
    payload: CreateAdminRequest,
) -> Result<Response<AdminResponse>> {
    let existing = Admins::find()
        .filter(admins::Column::Username.eq(&payload.username))
        .one(&state.conn)
        .await?;

    if existing.is_some() {
        return Ok(Response::failed("用户名已存在".to_string()));
    }

    let password_hash = hash_password(&payload.password);

    let admin = admins::ActiveModel {
        id: Set(uuid::Uuid::new_v4()),
        username: Set(payload.username),
        password_hash: Set(password_hash),
        is_super_admin: Set(payload.is_super_admin),
        status: Set(Some(1)),
        two_fa_secret: Set(None),
        last_login_at: Set(None),
        login_attempts: Set(Some(0)),
        locked_until: Set(None),
        created_at: Set(Some(chrono::Local::now().into())),
        updated_at: Set(Some(chrono::Local::now().into())),
    };

    let admin = admin.insert(&state.conn).await?;

    if let Some(role_ids) = payload.role_ids {
        for role_id in role_ids {
            let admin_role = admin_roles::ActiveModel {
                admin_id: Set(admin.id),
                role_id: Set(role_id),
                ..Default::default()
            };
            admin_role.insert(&state.conn).await?;
        }
    }

    Ok(Response::ok(
        Some("创建成功".to_string()),
        AdminResponse {
            id: admin.id,
            username: admin.username,
            is_super_admin: admin.is_super_admin.unwrap_or(false),
            status: admin.status.unwrap_or(1),
            last_login_at: admin.last_login_at.map(|dt| dt.into()),
            created_at: admin
                .created_at
                .map(|dt| dt.into())
                .unwrap_or_else(chrono::Local::now),
            updated_at: admin
                .updated_at
                .map(|dt| dt.into())
                .unwrap_or_else(chrono::Local::now),
        },
    ))
}

pub async fn update_admin_service(
    state: AppState,
    id: uuid::Uuid,
    payload: UpdateAdminRequest,
) -> Result<Response<AdminResponse>> {
    let admin = Admins::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("管理员不存在"))?;

    let mut admin_model: admins::ActiveModel = admin.into_active_model();

    if let Some(password) = payload.password {
        admin_model.password_hash = Set(hash_password(&password));
    }

    if let Some(status) = payload.status {
        admin_model.status = Set(Some(status));
    }

    if let Some(role_ids) = payload.role_ids {
        admin_roles::Entity::delete_many()
            .filter(admin_roles::Column::AdminId.eq(id))
            .exec(&state.conn)
            .await?;

        for role_id in role_ids {
            let admin_role = admin_roles::ActiveModel {
                created_at: NotSet,
                admin_id: Set(id),
                role_id: Set(role_id),
            };
            admin_role.insert(&state.conn).await?;
        }
    }

    admin_model.updated_at = Set(Some(chrono::Local::now().into()));
    let admin = admin_model.update(&state.conn).await?;

    Ok(Response::ok(
        Some("更新成功".to_string()),
        AdminResponse {
            id: admin.id,
            username: admin.username,
            is_super_admin: admin.is_super_admin.unwrap_or(false),
            status: admin.status.unwrap_or(1),
            last_login_at: admin.last_login_at.map(|dt| dt.into()),
            created_at: admin
                .created_at
                .map(|dt| dt.into())
                .unwrap_or_else(chrono::Local::now),
            updated_at: admin
                .updated_at
                .map(|dt| dt.into())
                .unwrap_or_else(chrono::Local::now),
        },
    ))
}

pub async fn delete_admin_service(state: AppState, id: uuid::Uuid) -> Result<Response<()>> {
    Ok(Response::ok_msg(Some("暂不支持".to_string())))
}

pub async fn assign_roles_service(
    state: AppState,
    id: uuid::Uuid,
    role_ids: Vec<uuid::Uuid>,
) -> Result<Response<()>> {
    let admin = Admins::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("管理员不存在"))?;

    if admin.is_super_admin.unwrap_or(false) {
        return Ok(Response::failed("超级管理员不可分配角色".to_string()));
    }

    admin_roles::Entity::delete_many()
        .filter(admin_roles::Column::AdminId.eq(id))
        .exec(&state.conn)
        .await?;

    for role_id in role_ids {
        let admin_role = admin_roles::ActiveModel {
            admin_id: Set(id),
            role_id: Set(role_id),
            ..Default::default()
        };
        admin_role.insert(&state.conn).await?;
    }

    Ok(Response::ok_msg(Some("角色分配成功".to_string())))
}
