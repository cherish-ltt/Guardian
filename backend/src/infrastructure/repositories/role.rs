use async_trait::async_trait;
use sea_orm::{ConnectionTrait, Value};

use crate::{
    domain::models::{
        pagination::Page,
        system_admin::{IdNameOption, RoleRow, RoleWriteCommand},
    },
    error::AppError,
    infrastructure::repositories::sql::{
        execute, insert, int_or_zero, query_all, query_count, text, text_or_none,
    },
    util::time,
};

/// 角色与角色-菜单关联的读写。
#[async_trait]
pub trait RoleRepository: Send + Sync {
    async fn count_roles<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn list_roles<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
        page: Page,
    ) -> Result<Vec<RoleRow>, AppError>
    where
        C: ConnectionTrait;

    /// 用户页的角色下拉：只含启用角色。
    async fn list_role_options<C>(&self, db: &C) -> Result<Vec<IdNameOption>, AppError>
    where
        C: ConnectionTrait;

    async fn count_role_code<C>(&self, db: &C, code: &str) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn insert_role<C>(
        &self,
        db: &C,
        command: &RoleWriteCommand,
        operator: &str,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn update_role<C>(
        &self,
        db: &C,
        id: i64,
        command: &RoleWriteCommand,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn soft_delete_role<C>(&self, db: &C, id: i64, operator: &str) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn count_role_users<C>(&self, db: &C, role_id: i64) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn role_menu_ids<C>(&self, db: &C, role_id: i64) -> Result<Vec<i64>, AppError>
    where
        C: ConnectionTrait;

    async fn replace_role_menus<C>(
        &self,
        db: &C,
        role_id: i64,
        menu_ids: &[i64],
    ) -> Result<(), AppError>
    where
        C: ConnectionTrait;

    async fn delete_role_menus<C>(&self, db: &C, role_id: i64) -> Result<(), AppError>
    where
        C: ConnectionTrait;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SeaOrmRoleRepository;

/// 数据范围编码在列表里展示成中文名称，与 Spring 版一致。
const ROLE_SELECT: &str = "SELECT r.id, r.role_name AS name, r.role_code AS code, \
     CASE r.data_scope \
       WHEN 'ALL' THEN '全部数据权限' \
       WHEN 'DEPT_AND_CHILD' THEN '本部门及以下' \
       WHEN 'DEPT' THEN '本部门数据权限' \
       WHEN 'CUSTOM' THEN '自定义数据权限' \
       ELSE '仅本人数据权限' END AS data_scope, \
     (SELECT COUNT(*) FROM sys_user_role ur \
        JOIN sys_user cu ON cu.id = ur.user_id AND cu.deleted = 0 \
        WHERE ur.role_id = r.id) AS user_count, \
     r.sort, r.status, r.builtin, substr(r.create_time, 1, 16) AS create_time, r.remark \
     FROM sys_role r";

#[async_trait]
impl RoleRepository for SeaOrmRoleRepository {
    async fn count_roles<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, values) = filter_of(keyword, status);
        let sql = format!("SELECT COUNT(*) AS value FROM sys_role r WHERE {filter}");

        query_count(db, &sql, values).await
    }

    async fn list_roles<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
        page: Page,
    ) -> Result<Vec<RoleRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, mut values) = filter_of(keyword, status);
        let sql = format!("{ROLE_SELECT} WHERE {filter} ORDER BY r.sort, r.id LIMIT ? OFFSET ?");
        values.push(page.limit().into());
        values.push(page.offset().into());
        let rows = query_all(db, &sql, values).await?;
        let ids: Vec<i64> = rows.iter().map(|row| int_or_zero(row, "id")).collect();
        let menu_ids = menu_ids_of(db, &ids).await?;

        Ok(rows
            .iter()
            .map(|row| {
                let id = int_or_zero(row, "id");

                RoleRow {
                    id,
                    name: text(row, "name").unwrap_or_default(),
                    code: text(row, "code").unwrap_or_default(),
                    data_scope: text(row, "data_scope").unwrap_or_default(),
                    user_count: int_or_zero(row, "user_count"),
                    sort: int_or_zero(row, "sort"),
                    status: int_or_zero(row, "status"),
                    builtin: int_or_zero(row, "builtin") == 1,
                    create_time: text_or_none(row, "create_time"),
                    menu_ids: menu_ids.get(&id).cloned().unwrap_or_default(),
                    remark: text_or_none(row, "remark"),
                }
            })
            .collect())
    }

    async fn list_role_options<C>(&self, db: &C) -> Result<Vec<IdNameOption>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT id, role_name AS name FROM sys_role WHERE deleted = 0 AND status = 1 \
             ORDER BY sort, id LIMIT 5000",
            vec![],
        )
        .await?;

        Ok(rows
            .iter()
            .map(|row| IdNameOption {
                id: int_or_zero(row, "id"),
                name: text(row, "name").unwrap_or_default(),
            })
            .collect())
    }

    async fn count_role_code<C>(&self, db: &C, code: &str) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_role WHERE role_code = ? AND deleted = 0",
            vec![code.into()],
        )
        .await
    }

    async fn insert_role<C>(
        &self,
        db: &C,
        command: &RoleWriteCommand,
        operator: &str,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let now = time::now();

        insert(
            db,
            "INSERT INTO sys_role (role_name, role_code, data_scope, sort, status, builtin, remark, \
             create_by, update_by, create_time, update_time) \
             VALUES (?, ?, ?, ?, ?, 0, ?, ?, ?, ?, ?)",
            vec![
                command.name.clone().unwrap_or_default().into(),
                command.code.clone().unwrap_or_default().into(),
                command
                    .data_scope
                    .clone()
                    .unwrap_or_else(|| "SELF".to_string())
                    .into(),
                command.sort.unwrap_or(0).into(),
                command.status.unwrap_or(1).into(),
                command.remark.clone().into(),
                operator.into(),
                operator.into(),
                now.clone().into(),
                now.into(),
            ],
        )
        .await
    }

    async fn update_role<C>(
        &self,
        db: &C,
        id: i64,
        command: &RoleWriteCommand,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE sys_role SET role_name = COALESCE(?, role_name), \
             data_scope = COALESCE(?, data_scope), sort = COALESCE(?, sort), \
             status = COALESCE(?, status), remark = COALESCE(?, remark), \
             update_by = ?, update_time = ? WHERE id = ? AND deleted = 0 AND builtin = 0",
            vec![
                command.name.clone().into(),
                command.data_scope.clone().into(),
                command.sort.into(),
                command.status.into(),
                command.remark.clone().into(),
                operator.into(),
                time::now().into(),
                id.into(),
            ],
        )
        .await
    }

    async fn soft_delete_role<C>(&self, db: &C, id: i64, operator: &str) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE sys_role SET deleted = 1, update_by = ?, update_time = ? \
             WHERE id = ? AND deleted = 0 AND builtin = 0",
            vec![operator.into(), time::now().into(), id.into()],
        )
        .await
    }

    async fn count_role_users<C>(&self, db: &C, role_id: i64) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_user_role ur JOIN sys_user u ON u.id = ur.user_id \
             WHERE ur.role_id = ? AND u.deleted = 0",
            vec![role_id.into()],
        )
        .await
    }

    async fn role_menu_ids<C>(&self, db: &C, role_id: i64) -> Result<Vec<i64>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT menu_id FROM sys_role_menu WHERE role_id = ? ORDER BY menu_id",
            vec![role_id.into()],
        )
        .await?;

        Ok(rows.iter().map(|row| int_or_zero(row, "menu_id")).collect())
    }

    async fn replace_role_menus<C>(
        &self,
        db: &C,
        role_id: i64,
        menu_ids: &[i64],
    ) -> Result<(), AppError>
    where
        C: ConnectionTrait,
    {
        self.delete_role_menus(db, role_id).await?;
        for menu_id in menu_ids {
            execute(
                db,
                "INSERT OR IGNORE INTO sys_role_menu (role_id, menu_id) VALUES (?, ?)",
                vec![role_id.into(), (*menu_id).into()],
            )
            .await?;
        }

        Ok(())
    }

    async fn delete_role_menus<C>(&self, db: &C, role_id: i64) -> Result<(), AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "DELETE FROM sys_role_menu WHERE role_id = ?",
            vec![role_id.into()],
        )
        .await?;

        Ok(())
    }
}

fn filter_of(keyword: Option<&str>, status: Option<i64>) -> (String, Vec<Value>) {
    let mut clauses = vec!["r.deleted = 0".to_string()];
    let mut values = Vec::new();

    if let Some(keyword) = keyword.filter(|value| !value.is_empty()) {
        clauses.push("(r.role_name LIKE ? OR r.role_code LIKE ?)".to_string());
        let pattern = format!("%{keyword}%");
        values.push(pattern.clone().into());
        values.push(pattern.into());
    }
    if let Some(status) = status {
        clauses.push("r.status = ?".to_string());
        values.push(status.into());
    }

    (clauses.join(" AND "), values)
}

async fn menu_ids_of<C>(
    db: &C,
    role_ids: &[i64],
) -> Result<std::collections::HashMap<i64, Vec<i64>>, AppError>
where
    C: ConnectionTrait,
{
    let mut grouped: std::collections::HashMap<i64, Vec<i64>> = std::collections::HashMap::new();
    if role_ids.is_empty() {
        return Ok(grouped);
    }

    let placeholders = vec!["?"; role_ids.len()].join(", ");
    let sql = format!(
        "SELECT role_id, menu_id FROM sys_role_menu WHERE role_id IN ({placeholders}) \
         ORDER BY role_id, menu_id"
    );
    let values: Vec<Value> = role_ids.iter().map(|id| (*id).into()).collect();
    for row in query_all(db, &sql, values).await? {
        grouped
            .entry(int_or_zero(&row, "role_id"))
            .or_default()
            .push(int_or_zero(&row, "menu_id"));
    }

    Ok(grouped)
}
