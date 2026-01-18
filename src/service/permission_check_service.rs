use anyhow::{Ok, Result};
use log::info;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::entities::{admin_roles, permissions, prelude::*};
use crate::middleware::auth::AuthContext;
use crate::router::AppState;

pub async fn check_api_permission(
    state: AppState,
    auth_context: AuthContext,
    method: String,
    path: String,
) -> Result<bool> {
    info!(
        "=== 权限检查 ===\n  Admin ID: {}\n  Username: {}\n  Is Super Admin: {}\n  Method: {}\n  Path: {}",
        auth_context.admin_id, auth_context.username, auth_context.is_super_admin, method, path
    );

    if auth_context.is_super_admin {
        info!("  -> 超级管理员，跳过权限检查，返回 true");
        return Ok(true);
    }

    let admin_roles_list = AdminRoles::find()
        .filter(admin_roles::Column::AdminId.eq(auth_context.admin_id))
        .find_also_related(Roles)
        .all(&state.conn)
        .await?;

    if admin_roles_list.is_empty() {
        return Ok(false);
    }

    let role_ids: Vec<uuid::Uuid> = admin_roles_list
        .iter()
        .filter_map(|(_, role)| role.as_ref().map(|r| r.id))
        .collect();

    let role_permissions_list = RolePermissions::find()
        .filter(
            <crate::entities::role_permissions::Entity as sea_orm::EntityTrait>::Column::RoleId
                .is_in(role_ids),
        )
        .find_also_related(Permissions)
        .all(&state.conn)
        .await?;

    let has_permission = role_permissions_list.iter().any(|(_, permission)| {
        if let Some(perm) = permission {
            if perm.resource_type != "api" {
                return false;
            }

            let method_match = match &perm.http_method {
                Some(m) => m.to_uppercase() == method,
                None => false,
            };

            let path_match = match &perm.resource_path {
                Some(p) => {
                    if p == "*" {
                        true
                    } else {
                        let pattern = p.replace("*", ".*").replace("{id}", "[^/]+");
                        let regex_result = regex::Regex::new(&format!("^{}$", pattern));
                        if regex_result.is_ok() {
                            let regex = regex_result.unwrap();
                            regex.is_match(&path)
                        } else {
                            false
                        }
                    }
                }
                None => false,
            };

            method_match && path_match
        } else {
            false
        }
    });

    Ok(has_permission)
}
