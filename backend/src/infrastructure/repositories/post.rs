use async_trait::async_trait;
use sea_orm::{ConnectionTrait, Value};

use crate::{
    domain::models::{
        pagination::Page,
        system_admin::{IdNameOption, PostRow, PostWriteCommand},
    },
    error::AppError,
    infrastructure::repositories::sql::{
        execute, insert, int_or_zero, query_all, query_count, text, text_or_none,
    },
    util::time,
};

/// 岗位的读写。
#[async_trait]
pub trait PostRepository: Send + Sync {
    async fn count_posts<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn list_posts<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
        page: Page,
    ) -> Result<Vec<PostRow>, AppError>
    where
        C: ConnectionTrait;

    /// 用户页的岗位下拉：不过滤启用状态，与 Spring 版一致。
    async fn list_post_options<C>(&self, db: &C) -> Result<Vec<IdNameOption>, AppError>
    where
        C: ConnectionTrait;

    /// 给定 ID 中实际存在且未删除的岗位数：用户保存岗位时用它拦下无效关联。
    async fn count_post_ids<C>(&self, db: &C, ids: &[i64]) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn count_post_code<C>(&self, db: &C, post_code: &str) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn insert_post<C>(
        &self,
        db: &C,
        command: &PostWriteCommand,
        operator: &str,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn update_post<C>(
        &self,
        db: &C,
        id: i64,
        command: &PostWriteCommand,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn soft_delete_post<C>(&self, db: &C, id: i64, operator: &str) -> Result<u64, AppError>
    where
        C: ConnectionTrait;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SeaOrmPostRepository;

const POST_SELECT: &str = "SELECT p.id, p.post_code, p.post_name, p.sort, p.status, p.remark, \
     substr(p.create_time, 1, 16) AS create_time, \
     (SELECT COUNT(*) FROM sys_user_post up \
        JOIN sys_user cu ON cu.id = up.user_id AND cu.deleted = 0 \
        WHERE up.post_id = p.id) AS user_count \
     FROM sys_post p";

#[async_trait]
impl PostRepository for SeaOrmPostRepository {
    async fn count_posts<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, values) = filter_of(keyword, status);
        let sql = format!("SELECT COUNT(*) AS value FROM sys_post p WHERE {filter}");

        query_count(db, &sql, values).await
    }

    async fn list_posts<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
        page: Page,
    ) -> Result<Vec<PostRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, mut values) = filter_of(keyword, status);
        let sql = format!("{POST_SELECT} WHERE {filter} ORDER BY p.sort, p.id LIMIT ? OFFSET ?");
        values.push(page.limit().into());
        values.push(page.offset().into());
        let rows = query_all(db, &sql, values).await?;

        Ok(rows
            .iter()
            .map(|row| PostRow {
                id: int_or_zero(row, "id"),
                post_code: text(row, "post_code").unwrap_or_default(),
                post_name: text(row, "post_name").unwrap_or_default(),
                sort: int_or_zero(row, "sort"),
                status: int_or_zero(row, "status"),
                remark: text_or_none(row, "remark"),
                create_time: text_or_none(row, "create_time"),
                user_count: int_or_zero(row, "user_count"),
            })
            .collect())
    }

    async fn list_post_options<C>(&self, db: &C) -> Result<Vec<IdNameOption>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT id, post_name AS name FROM sys_post WHERE deleted = 0 ORDER BY sort, id \
             LIMIT 5000",
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

    async fn count_post_ids<C>(&self, db: &C, ids: &[i64]) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = format!(
            "SELECT COUNT(*) AS value FROM sys_post WHERE deleted = 0 AND id IN ({placeholders})"
        );
        let values = ids.iter().map(|id| (*id).into()).collect();

        query_count(db, &sql, values).await
    }

    async fn count_post_code<C>(&self, db: &C, post_code: &str) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_post WHERE post_code = ? AND deleted = 0",
            vec![post_code.into()],
        )
        .await
    }

    async fn insert_post<C>(
        &self,
        db: &C,
        command: &PostWriteCommand,
        operator: &str,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let now = time::now();

        insert(
            db,
            "INSERT INTO sys_post (post_code, post_name, sort, status, remark, create_by, \
             update_by, create_time, update_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            vec![
                command.post_code.clone().unwrap_or_default().into(),
                command.post_name.clone().unwrap_or_default().into(),
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

    async fn update_post<C>(
        &self,
        db: &C,
        id: i64,
        command: &PostWriteCommand,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE sys_post SET post_name = COALESCE(?, post_name), sort = COALESCE(?, sort), \
             status = COALESCE(?, status), remark = COALESCE(?, remark), update_by = ?, \
             update_time = ? WHERE id = ? AND deleted = 0",
            vec![
                command.post_name.clone().into(),
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

    async fn soft_delete_post<C>(&self, db: &C, id: i64, operator: &str) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE sys_post SET deleted = 1, update_by = ?, update_time = ? \
             WHERE id = ? AND deleted = 0",
            vec![operator.into(), time::now().into(), id.into()],
        )
        .await
    }
}

fn filter_of(keyword: Option<&str>, status: Option<i64>) -> (String, Vec<Value>) {
    let mut clauses = vec!["p.deleted = 0".to_string()];
    let mut values = Vec::new();

    if let Some(keyword) = keyword.filter(|value| !value.is_empty()) {
        clauses.push("(p.post_name LIKE ? OR p.post_code LIKE ?)".to_string());
        let pattern = format!("%{keyword}%");
        values.push(pattern.clone().into());
        values.push(pattern.into());
    }
    if let Some(status) = status {
        clauses.push("p.status = ?".to_string());
        values.push(status.into());
    }

    (clauses.join(" AND "), values)
}
