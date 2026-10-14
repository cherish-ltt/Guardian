use async_trait::async_trait;
use sea_orm::{ConnectionTrait, Value};

use crate::{
    domain::models::system_admin::{MenuFlatRow, MenuWriteCommand},
    error::AppError,
    infrastructure::repositories::sql::{
        execute, int_or_zero, query_all, query_count, text, text_or_none,
    },
    util::time,
};

/// 菜单的读写。菜单 ID 由应用侧按 `MAX(id) + 1` 分配，与 Spring 版一致。
#[async_trait]
pub trait MenuRepository: Send + Sync {
    async fn list_menus<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<Vec<MenuFlatRow>, AppError>
    where
        C: ConnectionTrait;

    async fn next_menu_id<C>(&self, db: &C) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn insert_menu<C>(
        &self,
        db: &C,
        id: i64,
        command: &MenuWriteCommand,
        operator: &str,
    ) -> Result<(), AppError>
    where
        C: ConnectionTrait;

    async fn update_menu<C>(
        &self,
        db: &C,
        id: i64,
        command: &MenuWriteCommand,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn delete_menu<C>(&self, db: &C, id: i64) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn count_menu_children<C>(&self, db: &C, menu_id: i64) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn count_menu_roles<C>(&self, db: &C, menu_id: i64) -> Result<i64, AppError>
    where
        C: ConnectionTrait;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SeaOrmMenuRepository;

const MENU_SELECT: &str = "SELECT m.id, m.parent_id, m.menu_name, m.menu_type, m.icon, m.path, \
     m.component, m.permission, m.sort, m.visible, m.status FROM sys_menu m";

#[async_trait]
impl MenuRepository for SeaOrmMenuRepository {
    async fn list_menus<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<Vec<MenuFlatRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, values) = filter_of(keyword, status);
        let sql = format!("{MENU_SELECT} WHERE {filter} ORDER BY m.parent_id, m.sort, m.id");
        let rows = query_all(db, &sql, values).await?;

        Ok(rows.iter().map(map_menu).collect())
    }

    async fn next_menu_id<C>(&self, db: &C) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let row = super::sql::query_opt(
            db,
            "SELECT COALESCE(MAX(id), 0) + 1 AS value FROM sys_menu",
            vec![],
        )
        .await?;

        Ok(row.as_ref().map_or(1, |row| int_or_zero(row, "value")))
    }

    async fn insert_menu<C>(
        &self,
        db: &C,
        id: i64,
        command: &MenuWriteCommand,
        operator: &str,
    ) -> Result<(), AppError>
    where
        C: ConnectionTrait,
    {
        let now = time::now();
        execute(
            db,
            "INSERT INTO sys_menu (id, parent_id, menu_name, menu_type, icon, path, component, \
             permission, sort, visible, status, create_by, update_by, create_time, update_time) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            vec![
                id.into(),
                command.parent_id.unwrap_or(0).into(),
                command.name.clone().unwrap_or_default().into(),
                command
                    .type_
                    .clone()
                    .unwrap_or_else(|| "menu".to_string())
                    .into(),
                command.icon.clone().into(),
                command.path.clone().into(),
                command.component.clone().into(),
                command.permission.clone().into(),
                command.sort.unwrap_or(0).into(),
                command.visible.unwrap_or(1).into(),
                command.status.unwrap_or(1).into(),
                operator.into(),
                operator.into(),
                now.clone().into(),
                now.into(),
            ],
        )
        .await?;

        Ok(())
    }

    async fn update_menu<C>(
        &self,
        db: &C,
        id: i64,
        command: &MenuWriteCommand,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE sys_menu SET parent_id = COALESCE(?, parent_id), \
             menu_name = COALESCE(?, menu_name), menu_type = COALESCE(?, menu_type), \
             icon = COALESCE(?, icon), path = COALESCE(?, path), \
             component = COALESCE(?, component), permission = COALESCE(?, permission), \
             sort = COALESCE(?, sort), visible = COALESCE(?, visible), \
             status = COALESCE(?, status), update_by = ?, update_time = ? WHERE id = ?",
            vec![
                command.parent_id.into(),
                command.name.clone().into(),
                command.type_.clone().into(),
                command.icon.clone().into(),
                command.path.clone().into(),
                command.component.clone().into(),
                command.permission.clone().into(),
                command.sort.into(),
                command.visible.into(),
                command.status.into(),
                operator.into(),
                time::now().into(),
                id.into(),
            ],
        )
        .await
    }

    async fn delete_menu<C>(&self, db: &C, id: i64) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(db, "DELETE FROM sys_menu WHERE id = ?", vec![id.into()]).await
    }

    async fn count_menu_children<C>(&self, db: &C, menu_id: i64) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_menu WHERE parent_id = ?",
            vec![menu_id.into()],
        )
        .await
    }

    async fn count_menu_roles<C>(&self, db: &C, menu_id: i64) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_role_menu WHERE menu_id = ?",
            vec![menu_id.into()],
        )
        .await
    }
}

fn filter_of(keyword: Option<&str>, status: Option<i64>) -> (String, Vec<Value>) {
    let mut clauses = vec!["1 = 1".to_string()];
    let mut values = Vec::new();

    if let Some(keyword) = keyword.filter(|value| !value.is_empty()) {
        clauses.push("(m.menu_name LIKE ? OR m.permission LIKE ?)".to_string());
        let pattern = format!("%{keyword}%");
        values.push(pattern.clone().into());
        values.push(pattern.into());
    }
    if let Some(status) = status {
        clauses.push("m.status = ?".to_string());
        values.push(status.into());
    }

    (clauses.join(" AND "), values)
}

fn map_menu(row: &sea_orm::QueryResult) -> MenuFlatRow {
    MenuFlatRow {
        id: int_or_zero(row, "id"),
        parent_id: int_or_zero(row, "parent_id"),
        name: text(row, "menu_name").unwrap_or_default(),
        type_: text(row, "menu_type").unwrap_or_default(),
        icon: text_or_none(row, "icon"),
        path: text_or_none(row, "path"),
        component: text_or_none(row, "component"),
        permission: text_or_none(row, "permission"),
        sort: int_or_zero(row, "sort"),
        visible: int_or_zero(row, "visible"),
        status: int_or_zero(row, "status"),
    }
}
