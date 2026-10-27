use async_trait::async_trait;
use sea_orm::{ConnectionTrait, Value};

use crate::{
    domain::models::{
        audit_log::{LoginLogRow, OperLogRow, OperationLogEntry},
        pagination::Page,
    },
    error::AppError,
    infrastructure::repositories::sql::{
        execute, insert, int_or_zero, query_all, query_count, text, text_or_none,
    },
    util::time,
};

/// 操作日志与登录日志的读写。
#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    async fn count_oper_logs<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        result: Option<i64>,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn list_oper_logs<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        result: Option<i64>,
        page: Page,
    ) -> Result<Vec<OperLogRow>, AppError>
    where
        C: ConnectionTrait;

    async fn count_login_logs<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn list_login_logs<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
        page: Page,
    ) -> Result<Vec<LoginLogRow>, AppError>
    where
        C: ConnectionTrait;

    async fn insert_oper_log<C>(&self, db: &C, entry: &OperationLogEntry) -> Result<(), AppError>
    where
        C: ConnectionTrait;

    async fn delete_oper_log<C>(&self, db: &C, id: i64) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn clear_oper_logs<C>(&self, db: &C) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn delete_login_log<C>(&self, db: &C, id: i64) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn clear_login_logs<C>(&self, db: &C) -> Result<u64, AppError>
    where
        C: ConnectionTrait;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SeaOrmAuditLogRepository;

#[async_trait]
impl AuditLogRepository for SeaOrmAuditLogRepository {
    async fn count_oper_logs<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        result: Option<i64>,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, values) = oper_filter(keyword, result);
        let sql = format!("SELECT COUNT(*) AS value FROM sys_oper_log l WHERE {filter}");

        query_count(db, &sql, values).await
    }

    async fn list_oper_logs<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        result: Option<i64>,
        page: Page,
    ) -> Result<Vec<OperLogRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, mut values) = oper_filter(keyword, result);
        let sql = format!(
            "SELECT l.id, l.module, l.action, l.request_method, l.request_url, l.request_params, \
             l.oper_name, l.dept_name, l.ip, l.cost, l.result, l.error_msg, \
             l.oper_time FROM sys_oper_log l WHERE {filter} \
             ORDER BY l.oper_time DESC, l.id DESC LIMIT ? OFFSET ?"
        );
        values.push(page.limit().into());
        values.push(page.offset().into());
        let rows = query_all(db, &sql, values).await?;

        Ok(rows
            .iter()
            .map(|row| OperLogRow {
                id: int_or_zero(row, "id"),
                module: text(row, "module").unwrap_or_default(),
                action: text(row, "action").unwrap_or_default(),
                method: text_or_none(row, "request_method"),
                url: text_or_none(row, "request_url"),
                params: text_or_none(row, "request_params"),
                oper_name: text_or_none(row, "oper_name"),
                dept: text_or_none(row, "dept_name"),
                ip: text_or_none(row, "ip").unwrap_or_default(),
                cost: super::sql::int(row, "cost"),
                result: int_or_zero(row, "result"),
                error_msg: text_or_none(row, "error_msg"),
                oper_time: text_or_none(row, "oper_time"),
            })
            .collect())
    }

    async fn count_login_logs<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, values) = login_filter(keyword, status);
        let sql = format!("SELECT COUNT(*) AS value FROM sys_login_log l WHERE {filter}");

        query_count(db, &sql, values).await
    }

    async fn list_login_logs<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
        page: Page,
    ) -> Result<Vec<LoginLogRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, mut values) = login_filter(keyword, status);
        let sql = format!(
            "SELECT l.id, l.username, l.ip, l.browser, l.os, l.status, l.msg, \
             l.login_time FROM sys_login_log l WHERE {filter} \
             ORDER BY l.login_time DESC, l.id DESC LIMIT ? OFFSET ?"
        );
        values.push(page.limit().into());
        values.push(page.offset().into());
        let rows = query_all(db, &sql, values).await?;

        Ok(rows
            .iter()
            .map(|row| LoginLogRow {
                id: int_or_zero(row, "id"),
                username: text_or_none(row, "username"),
                ip: text_or_none(row, "ip").unwrap_or_default(),
                browser: text_or_none(row, "browser"),
                os: text_or_none(row, "os"),
                status: int_or_zero(row, "status"),
                msg: text_or_none(row, "msg"),
                login_time: text_or_none(row, "login_time"),
            })
            .collect())
    }

    async fn insert_oper_log<C>(&self, db: &C, entry: &OperationLogEntry) -> Result<(), AppError>
    where
        C: ConnectionTrait,
    {
        insert(
            db,
            "INSERT INTO sys_oper_log (module, action, request_method, request_url, request_params, \
             oper_name, dept_name, ip, cost, result, error_msg, oper_time) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            vec![
                entry.module.clone().into(),
                entry.action.clone().into(),
                entry.request_method.clone().into(),
                entry.request_url.clone().into(),
                entry.request_params.clone().into(),
                entry.operator.clone().into(),
                Option::<String>::None.into(),
                entry.ip.clone().into(),
                entry.cost_ms.into(),
                entry.result.into(),
                entry.error_message.clone().into(),
                time::now().into(),
            ],
        )
        .await?;

        Ok(())
    }

    async fn delete_oper_log<C>(&self, db: &C, id: i64) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(db, "DELETE FROM sys_oper_log WHERE id = ?", vec![id.into()]).await
    }

    async fn clear_oper_logs<C>(&self, db: &C) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(db, "DELETE FROM sys_oper_log", vec![]).await
    }

    async fn delete_login_log<C>(&self, db: &C, id: i64) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "DELETE FROM sys_login_log WHERE id = ?",
            vec![id.into()],
        )
        .await
    }

    async fn clear_login_logs<C>(&self, db: &C) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(db, "DELETE FROM sys_login_log", vec![]).await
    }
}

fn oper_filter(keyword: Option<&str>, result: Option<i64>) -> (String, Vec<Value>) {
    let mut clauses = vec!["1 = 1".to_string()];
    let mut values = Vec::new();

    if let Some(keyword) = keyword.filter(|value| !value.is_empty()) {
        clauses.push(
            "(l.module LIKE ? OR l.action LIKE ? OR l.oper_name LIKE ? OR l.request_url LIKE ? \
             OR l.request_params LIKE ?)"
                .to_string(),
        );
        let pattern = format!("%{keyword}%");
        for _ in 0..5 {
            values.push(pattern.clone().into());
        }
    }
    if let Some(result) = result {
        clauses.push("l.result = ?".to_string());
        values.push(result.into());
    }

    (clauses.join(" AND "), values)
}

fn login_filter(keyword: Option<&str>, status: Option<i64>) -> (String, Vec<Value>) {
    let mut clauses = vec!["1 = 1".to_string()];
    let mut values = Vec::new();

    if let Some(keyword) = keyword.filter(|value| !value.is_empty()) {
        clauses.push("(l.username LIKE ? OR l.ip LIKE ? OR l.msg LIKE ?)".to_string());
        let pattern = format!("%{keyword}%");
        for _ in 0..3 {
            values.push(pattern.clone().into());
        }
    }
    if let Some(status) = status {
        clauses.push("l.status = ?".to_string());
        values.push(status.into());
    }

    (clauses.join(" AND "), values)
}
