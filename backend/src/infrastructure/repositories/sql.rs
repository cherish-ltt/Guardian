//! 原生 SQL 的小工具。
//!
//! 后台管理的列表、统计与聚合大量使用 SQLite 专有函数（`strftime`、`substr`、
//! `group_concat`）与动态过滤条件，用查询构造器反而不如一条可读的 SQL 直接；
//! 简单的增删改仍走 `entities` 与查询构造器。
//! 这里只做「执行 + 取值」两件事，不放业务语义。

use sea_orm::{ConnectionTrait, DbBackend, QueryResult, Statement, Value};

use crate::error::AppError;

pub fn statement(sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Sqlite, sql, values)
}

pub async fn query_all<C>(
    db: &C,
    sql: &str,
    values: Vec<Value>,
) -> Result<Vec<QueryResult>, AppError>
where
    C: ConnectionTrait,
{
    db.query_all_raw(statement(sql, values))
        .await
        .map_err(Into::into)
}

pub async fn query_opt<C>(
    db: &C,
    sql: &str,
    values: Vec<Value>,
) -> Result<Option<QueryResult>, AppError>
where
    C: ConnectionTrait,
{
    db.query_one_raw(statement(sql, values))
        .await
        .map_err(Into::into)
}

/// 取第一行第一列的整数，没有结果时返回 0。
pub async fn query_count<C>(db: &C, sql: &str, values: Vec<Value>) -> Result<i64, AppError>
where
    C: ConnectionTrait,
{
    let row = query_opt(db, sql, values).await?;

    Ok(row
        .as_ref()
        .and_then(|row| row.try_get::<i64>("", "value").ok())
        .unwrap_or(0))
}

/// 取第一行第一列的文本。
pub async fn query_text<C>(
    db: &C,
    sql: &str,
    values: Vec<Value>,
) -> Result<Option<String>, AppError>
where
    C: ConnectionTrait,
{
    let row = query_opt(db, sql, values).await?;

    Ok(row.as_ref().and_then(|row| text(row, "value")))
}

/// 执行写语句，返回影响行数。
pub async fn execute<C>(db: &C, sql: &str, values: Vec<Value>) -> Result<u64, AppError>
where
    C: ConnectionTrait,
{
    let result = db.execute_raw(statement(sql, values)).await?;

    Ok(result.rows_affected())
}

/// 执行插入并返回自增主键。
pub async fn insert<C>(db: &C, sql: &str, values: Vec<Value>) -> Result<i64, AppError>
where
    C: ConnectionTrait,
{
    let result = db.execute_raw(statement(sql, values)).await?;

    Ok(result.last_insert_id() as i64)
}

pub fn text(row: &QueryResult, column: &str) -> Option<String> {
    row.try_get::<Option<String>>("", column).ok().flatten()
}

pub fn int(row: &QueryResult, column: &str) -> Option<i64> {
    row.try_get::<Option<i64>>("", column).ok().flatten()
}

pub fn int_or_zero(row: &QueryResult, column: &str) -> i64 {
    int(row, column).unwrap_or(0)
}

pub fn float(row: &QueryResult, column: &str) -> f64 {
    row.try_get::<Option<f64>>("", column)
        .ok()
        .flatten()
        .unwrap_or(0.0)
}

/// 空的字符串列统一读成 `None`，避免前端收到 `""` 与 `null` 两种空值。
pub fn text_or_none(row: &QueryResult, column: &str) -> Option<String> {
    text(row, column).filter(|value| !value.is_empty())
}

/// 可空整数列的文本形式（用于 `1`/`0` 标记）。
pub fn flag(row: &QueryResult, column: &str) -> bool {
    int_or_zero(row, column) == 1
}
