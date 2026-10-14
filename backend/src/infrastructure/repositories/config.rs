use async_trait::async_trait;
use sea_orm::{ConnectionTrait, Value};

use crate::{
    domain::models::{
        pagination::Page,
        system_admin::{ConfigRow, ConfigWriteCommand},
    },
    error::AppError,
    infrastructure::repositories::sql::{
        execute, insert, int_or_zero, query_all, query_count, query_text, text, text_or_none,
    },
    util::time,
};

/// 参数配置的读写。内置参数（`builtin = 1`）不允许修改与删除。
#[async_trait]
pub trait ConfigRepository: Send + Sync {
    async fn count_configs<C>(&self, db: &C, keyword: Option<&str>) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn list_configs<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        page: Page,
    ) -> Result<Vec<ConfigRow>, AppError>
    where
        C: ConnectionTrait;

    /// 账号初始密码等运行时读取的参数值。
    async fn config_value<C>(&self, db: &C, key: &str) -> Result<Option<String>, AppError>
    where
        C: ConnectionTrait;

    async fn count_config_key<C>(&self, db: &C, key: &str) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn insert_config<C>(
        &self,
        db: &C,
        command: &ConfigWriteCommand,
        operator: &str,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn update_config<C>(
        &self,
        db: &C,
        id: i64,
        command: &ConfigWriteCommand,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn delete_config<C>(&self, db: &C, id: i64) -> Result<u64, AppError>
    where
        C: ConnectionTrait;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SeaOrmConfigRepository;

const CONFIG_SELECT: &str = "SELECT c.id, c.config_name, c.config_key, c.config_value, \
     c.config_type, c.builtin, substr(c.create_time, 1, 16) AS create_time, c.remark \
     FROM sys_config c";

#[async_trait]
impl ConfigRepository for SeaOrmConfigRepository {
    async fn count_configs<C>(&self, db: &C, keyword: Option<&str>) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, values) = filter_of(keyword);
        let sql = format!("SELECT COUNT(*) AS value FROM sys_config c WHERE {filter}");

        query_count(db, &sql, values).await
    }

    async fn list_configs<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        page: Page,
    ) -> Result<Vec<ConfigRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, mut values) = filter_of(keyword);
        let sql = format!("{CONFIG_SELECT} WHERE {filter} ORDER BY c.id LIMIT ? OFFSET ?");
        values.push(page.limit().into());
        values.push(page.offset().into());
        let rows = query_all(db, &sql, values).await?;

        Ok(rows
            .iter()
            .map(|row| ConfigRow {
                id: int_or_zero(row, "id"),
                name: text(row, "config_name").unwrap_or_default(),
                key: text(row, "config_key").unwrap_or_default(),
                value: text(row, "config_value").unwrap_or_default(),
                type_: text(row, "config_type").unwrap_or_default(),
                builtin: int_or_zero(row, "builtin") == 1,
                create_time: text_or_none(row, "create_time"),
                remark: text_or_none(row, "remark"),
            })
            .collect())
    }

    async fn config_value<C>(&self, db: &C, key: &str) -> Result<Option<String>, AppError>
    where
        C: ConnectionTrait,
    {
        query_text(
            db,
            "SELECT config_value AS value FROM sys_config WHERE config_key = ? LIMIT 1",
            vec![key.into()],
        )
        .await
    }

    async fn count_config_key<C>(&self, db: &C, key: &str) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_config WHERE config_key = ?",
            vec![key.into()],
        )
        .await
    }

    async fn insert_config<C>(
        &self,
        db: &C,
        command: &ConfigWriteCommand,
        operator: &str,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let now = time::now();

        insert(
            db,
            "INSERT INTO sys_config (config_name, config_key, config_value, config_type, builtin, \
             remark, create_by, update_by, create_time, update_time) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            vec![
                command.name.clone().unwrap_or_default().into(),
                command.key.clone().unwrap_or_default().into(),
                command.value.clone().unwrap_or_default().into(),
                command
                    .config_type
                    .clone()
                    .unwrap_or_else(|| "N".to_string())
                    .into(),
                i64::from(command.builtin.unwrap_or(false)).into(),
                command.remark.clone().into(),
                operator.into(),
                operator.into(),
                now.clone().into(),
                now.into(),
            ],
        )
        .await
    }

    async fn update_config<C>(
        &self,
        db: &C,
        id: i64,
        command: &ConfigWriteCommand,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE sys_config SET config_name = COALESCE(?, config_name), \
             config_value = COALESCE(?, config_value), config_type = COALESCE(?, config_type), \
             builtin = COALESCE(?, builtin), remark = COALESCE(?, remark), update_by = ?, \
             update_time = ? WHERE id = ? AND builtin = 0",
            vec![
                command.name.clone().into(),
                command.value.clone().into(),
                command.config_type.clone().into(),
                command.builtin.map(i64::from).into(),
                command.remark.clone().into(),
                operator.into(),
                time::now().into(),
                id.into(),
            ],
        )
        .await
    }

    async fn delete_config<C>(&self, db: &C, id: i64) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "DELETE FROM sys_config WHERE id = ? AND builtin = 0",
            vec![id.into()],
        )
        .await
    }
}

fn filter_of(keyword: Option<&str>) -> (String, Vec<Value>) {
    let mut clauses = vec!["1 = 1".to_string()];
    let mut values = Vec::new();

    if let Some(keyword) = keyword.filter(|value| !value.is_empty()) {
        clauses.push("(c.config_name LIKE ? OR c.config_key LIKE ?)".to_string());
        let pattern = format!("%{keyword}%");
        values.push(pattern.clone().into());
        values.push(pattern.into());
    }

    (clauses.join(" AND "), values)
}
