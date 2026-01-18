use crate::entities::{admin_roles, admins, permissions, role_permissions, roles};
use crate::router::AppState;
use crate::utils::hash_password;
use anyhow::{Result, anyhow};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, QueryFilter, Set, TransactionTrait,
};

pub async fn init_system(state: AppState) -> Result<String> {
    let existing_admin = admins::Entity::find()
        .filter(admins::Column::Username.eq("admin"))
        .one(&state.conn)
        .await;

    if let Ok(Some(_)) = existing_admin {
        return Ok("系统已初始化".to_string());
    }

    let password_hash = hash_password("123456");

    state
        .conn
        .transaction::<_, (), sea_orm::DbErr>(|txn| {
            Box::pin(async move {
                let admin = admins::ActiveModel {
                    id: NotSet,
                    username: Set("admin".to_string()),
                    password_hash: Set(password_hash.clone()),
                    is_super_admin: Set(Some(true)),
                    status: Set(Some(1)),
                    created_at: NotSet,
                    updated_at: NotSet,
                    ..Default::default()
                };
                let admin = admin.insert(txn).await?;

                let super_admin_role = roles::ActiveModel {
                    id: NotSet,
                    code: Set("super_admin".to_string()),
                    name: Set("超级管理员".to_string()),
                    description: Set(Some("拥有所有权限".to_string())),
                    is_system: Set(Some(true)),
                    created_at: NotSet,
                    updated_at: NotSet,
                };
                let super_admin_role = super_admin_role.insert(txn).await?;

                let admin_role = roles::ActiveModel {
                    id: NotSet,
                    code: Set("admin".to_string()),
                    name: Set("普通管理员".to_string()),
                    description: Set(Some("拥有部分权限".to_string())),
                    is_system: Set(Some(false)),
                    created_at: NotSet,
                    updated_at: NotSet,
                };
                let admin_role = admin_role.insert(txn).await?;

                let user_create_permission = permissions::ActiveModel {
                    id: NotSet,
                    code: Set("user:create".to_string()),
                    name: Set("创建用户".to_string()),
                    description: Set(Some("创建新用户".to_string())),
                    resource_type: Set("api".to_string()),
                    http_method: Set(Some("POST".to_string())),
                    resource_path: Set(Some("/api/v1/users".to_string())),
                    parent_id: Set(None),
                    sort_order: Set(Some(1)),
                    is_system: Set(Some(true)),
                    created_at: NotSet,
                    updated_at: NotSet,
                };
                let user_create_permission = user_create_permission.insert(txn).await?;

                let user_read_permission = permissions::ActiveModel {
                    id: NotSet,
                    code: Set("user:read".to_string()),
                    name: Set("查看用户".to_string()),
                    description: Set(Some("查看用户信息".to_string())),
                    resource_type: Set("api".to_string()),
                    http_method: Set(Some("GET".to_string())),
                    resource_path: Set(Some("/api/v1/users".to_string())),
                    parent_id: Set(None),
                    sort_order: Set(Some(2)),
                    is_system: Set(Some(true)),
                    created_at: NotSet,
                    updated_at: NotSet,
                };
                let user_read_permission = user_read_permission.insert(txn).await?;

                let admin_role_map = admin_roles::ActiveModel {
                    admin_id: Set(admin.id),
                    role_id: Set(super_admin_role.id),
                    created_at: NotSet,
                };
                admin_role_map.insert(txn).await?;

                let rp1 = role_permissions::ActiveModel {
                    role_id: Set(super_admin_role.id),
                    permission_id: Set(user_create_permission.id),
                    created_at: NotSet,
                };
                rp1.insert(txn).await?;

                let rp2 = role_permissions::ActiveModel {
                    role_id: Set(super_admin_role.id),
                    permission_id: Set(user_read_permission.id),
                    created_at: NotSet,
                };
                rp2.insert(txn).await?;

                Ok::<_, sea_orm::DbErr>(())
            })
        })
        .await
        .map_err(|e| anyhow!("系统初始化失败: {}", e))?;

    Ok("系统初始化完成".to_string())
}
