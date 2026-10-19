use async_trait::async_trait;
use sea_orm::{ConnectionTrait, QueryResult, Value};

use crate::{
    domain::models::system_admin::{DeptFlatRow, DeptOptionRow, DeptUpdate, DeptWriteCommand},
    error::AppError,
    infrastructure::repositories::sql::{
        execute, insert, int_or_zero, query_all, query_count, query_opt, text, text_or_none,
    },
    util::time,
};

/// 部门的读写与树形结构所需字段。
#[async_trait]
pub trait DeptRepository: Send + Sync {
    async fn list_depts<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<Vec<DeptFlatRow>, AppError>
    where
        C: ConnectionTrait;

    /// 用户页下拉用的部门结构：只有 id / parentId / name / status。
    async fn list_dept_options<C>(&self, db: &C) -> Result<Vec<DeptOptionRow>, AppError>
    where
        C: ConnectionTrait;

    async fn insert_dept<C>(
        &self,
        db: &C,
        command: &DeptWriteCommand,
        ancestors: &str,
        operator: &str,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn update_dept<C>(
        &self,
        db: &C,
        id: i64,
        update: &DeptUpdate<'_>,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn soft_delete_dept<C>(&self, db: &C, id: i64, operator: &str) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn count_dept_children<C>(&self, db: &C, dept_id: i64) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    /// 给定 ID 中实际存在（未删除）的部门数量，用于校验关联是否悬空。
    async fn count_dept_ids<C>(&self, db: &C, ids: &[i64]) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn count_dept_users<C>(&self, db: &C, dept_id: i64) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn dept_ancestors<C>(&self, db: &C, dept_id: i64) -> Result<Option<String>, AppError>
    where
        C: ConnectionTrait;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SeaOrmDeptRepository;

const DEPT_SELECT: &str = "SELECT d.id, d.parent_id, d.dept_name, d.leader, d.phone, d.email, \
     d.sort, d.status, (SELECT COUNT(*) FROM sys_user u WHERE u.dept_id = d.id AND u.deleted = 0) \
     AS member_count FROM sys_dept d";

#[async_trait]
impl DeptRepository for SeaOrmDeptRepository {
    async fn list_depts<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<Vec<DeptFlatRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, values) = filter_of(keyword, status);
        let sql = format!("{DEPT_SELECT} WHERE {filter} ORDER BY d.parent_id, d.sort, d.id");
        let rows = query_all(db, &sql, values).await?;

        Ok(rows.iter().map(map_dept).collect())
    }

    async fn list_dept_options<C>(&self, db: &C) -> Result<Vec<DeptOptionRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT d.id, d.parent_id, d.dept_name, d.status FROM sys_dept d WHERE d.deleted = 0 \
             ORDER BY d.parent_id, d.sort, d.id",
            vec![],
        )
        .await?;

        Ok(rows
            .iter()
            .map(|row| DeptOptionRow {
                id: int_or_zero(row, "id"),
                parent_id: int_or_zero(row, "parent_id"),
                name: text(row, "dept_name").unwrap_or_default(),
                status: int_or_zero(row, "status"),
            })
            .collect())
    }

    async fn insert_dept<C>(
        &self,
        db: &C,
        command: &DeptWriteCommand,
        ancestors: &str,
        operator: &str,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let now = time::now();

        insert(
            db,
            "INSERT INTO sys_dept (parent_id, ancestors, dept_name, leader, phone, email, sort, \
             status, create_by, update_by, create_time, update_time) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            vec![
                command.parent_id.unwrap_or(0).into(),
                ancestors.into(),
                command.name.clone().unwrap_or_default().into(),
                command.leader.clone().into(),
                command.phone.clone().into(),
                command.email.clone().into(),
                command.sort.unwrap_or(0).into(),
                command.status.unwrap_or(1).into(),
                operator.into(),
                operator.into(),
                now.clone().into(),
                now.into(),
            ],
        )
        .await
    }

    async fn update_dept<C>(
        &self,
        db: &C,
        id: i64,
        update: &DeptUpdate<'_>,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE sys_dept SET parent_id = COALESCE(?, parent_id), \
             ancestors = COALESCE(?, ancestors), dept_name = COALESCE(?, dept_name), \
             leader = COALESCE(?, leader), phone = COALESCE(?, phone), \
             email = COALESCE(?, email), sort = COALESCE(?, sort), \
             status = COALESCE(?, status), update_by = ?, update_time = ? \
             WHERE id = ? AND deleted = 0",
            vec![
                update.command.parent_id.into(),
                update.ancestors.into(),
                update.command.name.clone().into(),
                update.command.leader.clone().into(),
                update.command.phone.clone().into(),
                update.command.email.clone().into(),
                update.command.sort.into(),
                update.command.status.into(),
                update.operator.into(),
                time::now().into(),
                id.into(),
            ],
        )
        .await
    }

    async fn soft_delete_dept<C>(&self, db: &C, id: i64, operator: &str) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE sys_dept SET deleted = 1, update_by = ?, update_time = ? \
             WHERE id = ? AND deleted = 0",
            vec![operator.into(), time::now().into(), id.into()],
        )
        .await
    }

    async fn count_dept_children<C>(&self, db: &C, dept_id: i64) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_dept WHERE parent_id = ? AND deleted = 0",
            vec![dept_id.into()],
        )
        .await
    }

    async fn count_dept_ids<C>(&self, db: &C, ids: &[i64]) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = format!(
            "SELECT COUNT(*) AS value FROM sys_dept WHERE deleted = 0 AND id IN ({placeholders})"
        );
        let values = ids.iter().map(|id| (*id).into()).collect();

        query_count(db, &sql, values).await
    }

    async fn count_dept_users<C>(&self, db: &C, dept_id: i64) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_user WHERE dept_id = ? AND deleted = 0",
            vec![dept_id.into()],
        )
        .await
    }

    async fn dept_ancestors<C>(&self, db: &C, dept_id: i64) -> Result<Option<String>, AppError>
    where
        C: ConnectionTrait,
    {
        let row = query_opt(
            db,
            "SELECT ancestors FROM sys_dept WHERE id = ? AND deleted = 0",
            vec![dept_id.into()],
        )
        .await?;

        Ok(row
            .as_ref()
            .map(|row| text(row, "ancestors").unwrap_or_default()))
    }
}

fn filter_of(keyword: Option<&str>, status: Option<i64>) -> (String, Vec<Value>) {
    let mut clauses = vec!["d.deleted = 0".to_string()];
    let mut values = Vec::new();

    if let Some(keyword) = keyword.filter(|value| !value.is_empty()) {
        clauses.push("(d.dept_name LIKE ? OR d.leader LIKE ?)".to_string());
        let pattern = format!("%{keyword}%");
        values.push(pattern.clone().into());
        values.push(pattern.into());
    }
    if let Some(status) = status {
        clauses.push("d.status = ?".to_string());
        values.push(status.into());
    }

    (clauses.join(" AND "), values)
}

fn map_dept(row: &QueryResult) -> DeptFlatRow {
    DeptFlatRow {
        id: int_or_zero(row, "id"),
        parent_id: int_or_zero(row, "parent_id"),
        name: text(row, "dept_name").unwrap_or_default(),
        leader: text_or_none(row, "leader"),
        phone: text_or_none(row, "phone"),
        email: text_or_none(row, "email"),
        sort: int_or_zero(row, "sort"),
        status: int_or_zero(row, "status"),
        member_count: int_or_zero(row, "member_count"),
    }
}
