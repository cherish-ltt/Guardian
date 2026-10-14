use async_trait::async_trait;
use sea_orm::{ConnectionTrait, Value};

use crate::{
    domain::models::{
        identity::DataScope,
        system_admin::{GraphMenu, GraphUser, UserRoleLink},
    },
    error::AppError,
    infrastructure::repositories::sql::{int_or_zero, query_all, text, text_or_none},
};

/// 权限关系图（用户 / 角色 / 菜单 / 关联对）所需的读操作。
#[async_trait]
pub trait PermissionGraphRepository: Send + Sync {
    async fn graph_users<C>(&self, db: &C, scope: &DataScope) -> Result<Vec<GraphUser>, AppError>
    where
        C: ConnectionTrait;

    async fn user_role_links<C>(&self, db: &C) -> Result<Vec<UserRoleLink>, AppError>
    where
        C: ConnectionTrait;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SeaOrmPermissionGraphRepository;

/// 菜单节点在关系图里只带展示所需字段。
pub const GRAPH_MENU_SELECT: &str = "SELECT m.id, m.parent_id, m.menu_name, m.menu_type, m.icon, \
     m.path, m.permission, m.sort, m.visible, m.status FROM sys_menu m \
     ORDER BY m.parent_id, m.sort, m.id";

#[async_trait]
impl PermissionGraphRepository for SeaOrmPermissionGraphRepository {
    async fn graph_users<C>(&self, db: &C, scope: &DataScope) -> Result<Vec<GraphUser>, AppError>
    where
        C: ConnectionTrait,
    {
        let mut filter = "u.deleted = 0".to_string();
        let mut values: Vec<Value> = Vec::new();
        if !scope.all {
            let mut condition = "u.id = ?".to_string();
            values.push(scope.user_id_or_default().into());
            if !scope.dept_ids.is_empty() {
                let placeholders = vec!["?"; scope.dept_ids.len()].join(", ");
                condition.push_str(&format!(" OR u.dept_id IN ({placeholders})"));
                for dept_id in &scope.dept_ids {
                    values.push((*dept_id).into());
                }
            }
            filter.push_str(&format!(" AND ({condition})"));
        }

        let sql = format!(
            "SELECT u.id, u.username, u.nickname, d.dept_name, u.status FROM sys_user u \
             LEFT JOIN sys_dept d ON d.id = u.dept_id WHERE {filter} ORDER BY u.id LIMIT 2000"
        );
        let rows = query_all(db, &sql, values).await?;

        Ok(rows
            .iter()
            .map(|row| GraphUser {
                id: int_or_zero(row, "id"),
                username: text(row, "username").unwrap_or_default(),
                nickname: text(row, "nickname").unwrap_or_default(),
                dept_name: text_or_none(row, "dept_name"),
                status: int_or_zero(row, "status"),
            })
            .collect())
    }

    async fn user_role_links<C>(&self, db: &C) -> Result<Vec<UserRoleLink>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT ur.user_id, ur.role_id FROM sys_user_role ur \
             JOIN sys_user u ON u.id = ur.user_id AND u.deleted = 0 \
             JOIN sys_role r ON r.id = ur.role_id AND r.deleted = 0 \
             ORDER BY ur.user_id, ur.role_id",
            vec![],
        )
        .await?;

        Ok(rows
            .iter()
            .map(|row| UserRoleLink {
                user_id: int_or_zero(row, "user_id"),
                role_id: int_or_zero(row, "role_id"),
            })
            .collect())
    }
}

/// 关系图里的菜单节点。
pub fn map_graph_menu(row: &sea_orm::QueryResult) -> GraphMenu {
    GraphMenu {
        id: int_or_zero(row, "id"),
        parent_id: int_or_zero(row, "parent_id"),
        name: text(row, "menu_name").unwrap_or_default(),
        type_: text(row, "menu_type").unwrap_or_default(),
        icon: text_or_none(row, "icon"),
        path: text_or_none(row, "path"),
        permission: text_or_none(row, "permission"),
        sort: int_or_zero(row, "sort"),
        visible: int_or_zero(row, "visible"),
        status: int_or_zero(row, "status"),
    }
}
