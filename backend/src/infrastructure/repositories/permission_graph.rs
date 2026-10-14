use async_trait::async_trait;
use sea_orm::{ConnectionTrait, Value};

use crate::{
    domain::models::{
        identity::DataScope,
        system_admin::{GraphUser, UserRoleLink},
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
