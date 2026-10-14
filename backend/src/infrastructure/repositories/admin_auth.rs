use async_trait::async_trait;
use sea_orm::{ConnectionTrait, QueryResult};

use crate::{
    domain::models::identity::{AuthUserRow, MenuRow},
    error::AppError,
    infrastructure::repositories::sql::{
        execute, int, int_or_zero, query_all, query_opt, text, text_or_none,
    },
    util::time,
};

/// 登录鉴权与数据权限所需的读操作。
///
/// 每个方法都接收 `C: ConnectionTrait`，由调用方决定是否在事务内执行。
#[async_trait]
pub trait AdminAuthRepository: Send + Sync {
    async fn find_user_by_username<C>(
        &self,
        db: &C,
        username: &str,
    ) -> Result<Option<AuthUserRow>, AppError>
    where
        C: ConnectionTrait;

    /// 启用且未删除的角色编码，按排序号返回。
    async fn role_codes<C>(&self, db: &C, user_id: i64) -> Result<Vec<String>, AppError>
    where
        C: ConnectionTrait;

    async fn permissions<C>(&self, db: &C, user_id: i64) -> Result<Vec<String>, AppError>
    where
        C: ConnectionTrait;

    /// 超级管理员用：全部启用的权限码。
    async fn all_permissions<C>(&self, db: &C) -> Result<Vec<String>, AppError>
    where
        C: ConnectionTrait;

    async fn route_menus<C>(&self, db: &C, user_id: i64) -> Result<Vec<MenuRow>, AppError>
    where
        C: ConnectionTrait;

    /// 超级管理员用：全部启用且可见的目录与菜单。
    async fn all_route_menus<C>(&self, db: &C) -> Result<Vec<MenuRow>, AppError>
    where
        C: ConnectionTrait;

    async fn insert_login_log<C>(
        &self,
        db: &C,
        username: &str,
        status: i64,
        message: &str,
    ) -> Result<(), AppError>
    where
        C: ConnectionTrait;

    async fn touch_last_login<C>(&self, db: &C, user_id: i64) -> Result<(), AppError>
    where
        C: ConnectionTrait;

    async fn update_password<C>(
        &self,
        db: &C,
        user_id: i64,
        username: &str,
        password_hash: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    /// 数据权限解析：启用角色的 `(角色 ID, 数据范围)`。
    async fn active_role_scopes<C>(
        &self,
        db: &C,
        user_id: i64,
    ) -> Result<Vec<(i64, String)>, AppError>
    where
        C: ConnectionTrait;

    /// 数据权限解析：启用用户的 `(ID, 部门 ID)`。
    async fn active_user_profile<C>(
        &self,
        db: &C,
        username: &str,
    ) -> Result<Option<(i64, Option<i64>)>, AppError>
    where
        C: ConnectionTrait;

    /// 本部门及全部下级部门（`ancestors` 是一段以逗号分隔的上级链）。
    async fn dept_and_child_ids<C>(&self, db: &C, dept_id: i64) -> Result<Vec<i64>, AppError>
    where
        C: ConnectionTrait;

    async fn role_dept_ids<C>(&self, db: &C, role_id: i64) -> Result<Vec<i64>, AppError>
    where
        C: ConnectionTrait;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SeaOrmAdminAuthRepository;

const FIND_USER: &str = "SELECT u.id, u.username, u.password, u.nickname, u.status, u.dept_id, \
     d.dept_name AS dept_name \
     FROM sys_user u LEFT JOIN sys_dept d ON d.id = u.dept_id \
     WHERE u.username = ? AND u.deleted = 0 LIMIT 1";

const ROLE_CODES: &str = "SELECT r.role_code FROM sys_user_role ur \
     JOIN sys_role r ON r.id = ur.role_id \
     WHERE ur.user_id = ? AND r.status = 1 AND r.deleted = 0 ORDER BY r.sort, r.id";

const USER_PERMISSIONS: &str = "SELECT DISTINCT m.permission FROM sys_user_role ur \
     JOIN sys_role r ON r.id = ur.role_id AND r.status = 1 AND r.deleted = 0 \
     JOIN sys_role_menu rm ON rm.role_id = ur.role_id \
     JOIN sys_menu m ON m.id = rm.menu_id \
     WHERE ur.user_id = ? AND m.status = 1 AND m.permission IS NOT NULL AND m.permission <> '' \
     ORDER BY m.permission";

const ALL_PERMISSIONS: &str = "SELECT DISTINCT permission FROM sys_menu \
     WHERE status = 1 AND permission IS NOT NULL AND permission <> '' ORDER BY permission";

const USER_ROUTE_MENUS: &str = "SELECT DISTINCT m.id, m.parent_id, m.menu_name, m.menu_type, m.icon, \
     m.path, m.component, m.permission, m.sort \
     FROM sys_user_role ur \
     JOIN sys_role r ON r.id = ur.role_id AND r.status = 1 AND r.deleted = 0 \
     JOIN sys_role_menu rm ON rm.role_id = ur.role_id \
     JOIN sys_menu m ON m.id = rm.menu_id \
     WHERE ur.user_id = ? AND m.status = 1 AND m.visible = 1 AND m.menu_type IN ('dir', 'menu') \
     ORDER BY m.parent_id, m.sort, m.id";

const ALL_ROUTE_MENUS: &str = "SELECT id, parent_id, menu_name, menu_type, icon, path, component, \
     permission, sort FROM sys_menu \
     WHERE status = 1 AND visible = 1 AND menu_type IN ('dir', 'menu') \
     ORDER BY parent_id, sort, id";

#[async_trait]
impl AdminAuthRepository for SeaOrmAdminAuthRepository {
    async fn find_user_by_username<C>(
        &self,
        db: &C,
        username: &str,
    ) -> Result<Option<AuthUserRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let row = query_opt(db, FIND_USER, vec![username.into()]).await?;

        Ok(row.map(|row| AuthUserRow {
            id: int_or_zero(&row, "id"),
            username: text(&row, "username").unwrap_or_default(),
            password: text(&row, "password").unwrap_or_default(),
            nickname: text(&row, "nickname").unwrap_or_default(),
            status: int_or_zero(&row, "status"),
            dept_id: int(&row, "dept_id"),
            dept_name: text_or_none(&row, "dept_name"),
        }))
    }

    async fn role_codes<C>(&self, db: &C, user_id: i64) -> Result<Vec<String>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(db, ROLE_CODES, vec![user_id.into()]).await?;

        Ok(rows
            .iter()
            .filter_map(|row| text(row, "role_code"))
            .collect())
    }

    async fn permissions<C>(&self, db: &C, user_id: i64) -> Result<Vec<String>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(db, USER_PERMISSIONS, vec![user_id.into()]).await?;

        Ok(rows
            .iter()
            .filter_map(|row| text(row, "permission"))
            .collect())
    }

    async fn all_permissions<C>(&self, db: &C) -> Result<Vec<String>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(db, ALL_PERMISSIONS, vec![]).await?;

        Ok(rows
            .iter()
            .filter_map(|row| text(row, "permission"))
            .collect())
    }

    async fn route_menus<C>(&self, db: &C, user_id: i64) -> Result<Vec<MenuRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(db, USER_ROUTE_MENUS, vec![user_id.into()]).await?;

        Ok(rows.iter().map(map_menu_row).collect())
    }

    async fn all_route_menus<C>(&self, db: &C) -> Result<Vec<MenuRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(db, ALL_ROUTE_MENUS, vec![]).await?;

        Ok(rows.iter().map(map_menu_row).collect())
    }

    async fn insert_login_log<C>(
        &self,
        db: &C,
        username: &str,
        status: i64,
        message: &str,
    ) -> Result<(), AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "INSERT INTO sys_login_log (username, status, msg, login_time) VALUES (?, ?, ?, ?)",
            vec![
                username.into(),
                status.into(),
                message.into(),
                time::now().into(),
            ],
        )
        .await?;

        Ok(())
    }

    async fn touch_last_login<C>(&self, db: &C, user_id: i64) -> Result<(), AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE sys_user SET last_login_time = ? WHERE id = ? AND deleted = 0",
            vec![time::now().into(), user_id.into()],
        )
        .await?;

        Ok(())
    }

    async fn update_password<C>(
        &self,
        db: &C,
        user_id: i64,
        username: &str,
        password_hash: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE sys_user SET password = ?, update_by = ?, update_time = ? \
             WHERE id = ? AND deleted = 0",
            vec![
                password_hash.into(),
                username.into(),
                time::now().into(),
                user_id.into(),
            ],
        )
        .await
    }

    async fn active_role_scopes<C>(
        &self,
        db: &C,
        user_id: i64,
    ) -> Result<Vec<(i64, String)>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT r.id, r.data_scope FROM sys_user_role ur JOIN sys_role r ON r.id = ur.role_id \
             WHERE ur.user_id = ? AND r.status = 1 AND r.deleted = 0 ORDER BY r.sort, r.id",
            vec![user_id.into()],
        )
        .await?;

        Ok(rows
            .iter()
            .map(|row| {
                (
                    int_or_zero(row, "id"),
                    text(row, "data_scope").unwrap_or_else(|| "SELF".to_string()),
                )
            })
            .collect())
    }

    async fn active_user_profile<C>(
        &self,
        db: &C,
        username: &str,
    ) -> Result<Option<(i64, Option<i64>)>, AppError>
    where
        C: ConnectionTrait,
    {
        let row = query_opt(
            db,
            "SELECT u.id, u.dept_id FROM sys_user u \
             WHERE u.username = ? AND u.deleted = 0 AND u.status = 1 LIMIT 1",
            vec![username.into()],
        )
        .await?;

        Ok(row.map(|row| (int_or_zero(&row, "id"), int(&row, "dept_id"))))
    }

    async fn dept_and_child_ids<C>(&self, db: &C, dept_id: i64) -> Result<Vec<i64>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT d.id FROM sys_dept d WHERE d.deleted = 0 \
             AND (d.id = ? OR (',' || d.ancestors || ',') LIKE ('%,' || ? || ',%')) \
             ORDER BY d.id",
            vec![dept_id.into(), dept_id.into()],
        )
        .await?;

        Ok(rows.iter().map(|row| int_or_zero(row, "id")).collect())
    }

    async fn role_dept_ids<C>(&self, db: &C, role_id: i64) -> Result<Vec<i64>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT dept_id FROM sys_role_dept WHERE role_id = ? ORDER BY dept_id",
            vec![role_id.into()],
        )
        .await?;

        Ok(rows.iter().map(|row| int_or_zero(row, "dept_id")).collect())
    }
}

fn map_menu_row(row: &QueryResult) -> MenuRow {
    MenuRow {
        id: int_or_zero(row, "id"),
        parent_id: int_or_zero(row, "parent_id"),
        menu_name: text(row, "menu_name").unwrap_or_default(),
        menu_type: text(row, "menu_type").unwrap_or_default(),
        icon: text_or_none(row, "icon"),
        path: text_or_none(row, "path"),
        component: text_or_none(row, "component"),
        permission: text_or_none(row, "permission"),
        sort: int_or_zero(row, "sort"),
    }
}
