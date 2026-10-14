use async_trait::async_trait;
use sea_orm::{ConnectionTrait, Value};

use crate::{
    domain::models::{
        pagination::Page,
        system_admin::{
            DictDataRow, DictDataWriteCommand, DictOption, DictTypeRow, DictTypeWriteCommand,
        },
    },
    error::AppError,
    infrastructure::repositories::sql::{
        execute, insert, int_or_zero, query_all, query_count, query_opt, text, text_or_none,
    },
    util::time,
};

/// 字典类型与字典数据的读写。
#[async_trait]
pub trait DictRepository: Send + Sync {
    async fn count_dict_types<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn list_dict_types<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
        page: Page,
    ) -> Result<Vec<DictTypeRow>, AppError>
    where
        C: ConnectionTrait;

    async fn count_dict_type<C>(&self, db: &C, dict_type: &str) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn insert_dict_type<C>(
        &self,
        db: &C,
        command: &DictTypeWriteCommand,
        operator: &str,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn update_dict_type<C>(
        &self,
        db: &C,
        id: i64,
        command: &DictTypeWriteCommand,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn delete_dict_type<C>(&self, db: &C, id: i64) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn dict_type_code<C>(&self, db: &C, id: i64) -> Result<Option<String>, AppError>
    where
        C: ConnectionTrait;

    async fn count_dict_data_by_type<C>(&self, db: &C, dict_type: &str) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    /// 管理页：`def` 转成布尔值。
    async fn list_dict_data<C>(
        &self,
        db: &C,
        dict_type: &str,
    ) -> Result<Vec<DictDataRow>, AppError>
    where
        C: ConnectionTrait;

    /// 业务下拉：只取启用字典里的启用项，`def` 保持 0/1。
    async fn list_dict_options<C>(
        &self,
        db: &C,
        dict_type: &str,
    ) -> Result<Vec<DictOption>, AppError>
    where
        C: ConnectionTrait;

    async fn insert_dict_data<C>(
        &self,
        db: &C,
        command: &DictDataWriteCommand,
        operator: &str,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn update_dict_data<C>(
        &self,
        db: &C,
        id: i64,
        command: &DictDataWriteCommand,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn delete_dict_data<C>(&self, db: &C, id: i64) -> Result<u64, AppError>
    where
        C: ConnectionTrait;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SeaOrmDictRepository;

const DICT_TYPE_SELECT: &str = "SELECT t.id, t.dict_name, t.dict_type, t.status, \
     substr(t.create_time, 1, 16) AS create_time, t.remark FROM sys_dict_type t";

#[async_trait]
impl DictRepository for SeaOrmDictRepository {
    async fn count_dict_types<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, values) = type_filter(keyword, status);
        let sql = format!("SELECT COUNT(*) AS value FROM sys_dict_type t WHERE {filter}");

        query_count(db, &sql, values).await
    }

    async fn list_dict_types<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        status: Option<i64>,
        page: Page,
    ) -> Result<Vec<DictTypeRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, mut values) = type_filter(keyword, status);
        let sql = format!("{DICT_TYPE_SELECT} WHERE {filter} ORDER BY t.id LIMIT ? OFFSET ?");
        values.push(page.limit().into());
        values.push(page.offset().into());
        let rows = query_all(db, &sql, values).await?;

        Ok(rows
            .iter()
            .map(|row| DictTypeRow {
                id: int_or_zero(row, "id"),
                dict_name: text(row, "dict_name").unwrap_or_default(),
                dict_type: text(row, "dict_type").unwrap_or_default(),
                status: int_or_zero(row, "status"),
                create_time: text_or_none(row, "create_time"),
                remark: text_or_none(row, "remark"),
            })
            .collect())
    }

    async fn count_dict_type<C>(&self, db: &C, dict_type: &str) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_dict_type WHERE dict_type = ?",
            vec![dict_type.into()],
        )
        .await
    }

    async fn insert_dict_type<C>(
        &self,
        db: &C,
        command: &DictTypeWriteCommand,
        operator: &str,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let now = time::now();

        insert(
            db,
            "INSERT INTO sys_dict_type (dict_name, dict_type, status, remark, create_by, update_by, \
             create_time, update_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            vec![
                command.dict_name.clone().unwrap_or_default().into(),
                command.dict_type.clone().unwrap_or_default().into(),
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

    async fn update_dict_type<C>(
        &self,
        db: &C,
        id: i64,
        command: &DictTypeWriteCommand,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE sys_dict_type SET dict_name = COALESCE(?, dict_name), \
             status = COALESCE(?, status), remark = COALESCE(?, remark), update_by = ?, \
             update_time = ? WHERE id = ?",
            vec![
                command.dict_name.clone().into(),
                command.status.into(),
                command.remark.clone().into(),
                operator.into(),
                time::now().into(),
                id.into(),
            ],
        )
        .await
    }

    async fn delete_dict_type<C>(&self, db: &C, id: i64) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "DELETE FROM sys_dict_type WHERE id = ?",
            vec![id.into()],
        )
        .await
    }

    async fn dict_type_code<C>(&self, db: &C, id: i64) -> Result<Option<String>, AppError>
    where
        C: ConnectionTrait,
    {
        let row = query_opt(
            db,
            "SELECT dict_type FROM sys_dict_type WHERE id = ?",
            vec![id.into()],
        )
        .await?;

        Ok(row.as_ref().and_then(|row| text(row, "dict_type")))
    }

    async fn count_dict_data_by_type<C>(&self, db: &C, dict_type: &str) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_dict_data WHERE dict_type = ?",
            vec![dict_type.into()],
        )
        .await
    }

    async fn list_dict_data<C>(&self, db: &C, dict_type: &str) -> Result<Vec<DictDataRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT d.id, d.dict_label, d.dict_value, d.sort, d.status, d.tone, d.is_default \
             FROM sys_dict_data d WHERE d.dict_type = ? ORDER BY d.sort, d.id",
            vec![dict_type.into()],
        )
        .await?;

        Ok(rows
            .iter()
            .map(|row| DictDataRow {
                id: int_or_zero(row, "id"),
                label: text(row, "dict_label").unwrap_or_default(),
                value: text(row, "dict_value").unwrap_or_default(),
                sort: int_or_zero(row, "sort"),
                status: int_or_zero(row, "status"),
                tone: text_or_none(row, "tone"),
                def: int_or_zero(row, "is_default") == 1,
            })
            .collect())
    }

    async fn list_dict_options<C>(
        &self,
        db: &C,
        dict_type: &str,
    ) -> Result<Vec<DictOption>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT d.dict_label, d.dict_value, d.tone, d.is_default, d.status \
             FROM sys_dict_data d \
             JOIN sys_dict_type t ON t.dict_type = d.dict_type AND t.status = 1 \
             WHERE d.dict_type = ? AND d.status = 1 ORDER BY d.sort, d.id",
            vec![dict_type.into()],
        )
        .await?;

        Ok(rows
            .iter()
            .map(|row| DictOption {
                label: text(row, "dict_label").unwrap_or_default(),
                value: text(row, "dict_value").unwrap_or_default(),
                tone: text_or_none(row, "tone"),
                def: int_or_zero(row, "is_default"),
                status: int_or_zero(row, "status"),
            })
            .collect())
    }

    async fn insert_dict_data<C>(
        &self,
        db: &C,
        command: &DictDataWriteCommand,
        operator: &str,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let now = time::now();

        insert(
            db,
            "INSERT INTO sys_dict_data (dict_type, dict_label, dict_value, sort, status, tone, \
             is_default, remark, create_by, update_by, create_time, update_time) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            vec![
                command.dict_type.clone().unwrap_or_default().into(),
                command.label.clone().unwrap_or_default().into(),
                command.value.clone().unwrap_or_default().into(),
                command.sort.unwrap_or(0).into(),
                command.status.unwrap_or(1).into(),
                command.tone.clone().into(),
                i64::from(command.def.unwrap_or(false)).into(),
                command.remark.clone().into(),
                operator.into(),
                operator.into(),
                now.clone().into(),
                now.into(),
            ],
        )
        .await
    }

    async fn update_dict_data<C>(
        &self,
        db: &C,
        id: i64,
        command: &DictDataWriteCommand,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE sys_dict_data SET dict_type = COALESCE(?, dict_type), \
             dict_label = COALESCE(?, dict_label), dict_value = COALESCE(?, dict_value), \
             sort = COALESCE(?, sort), status = COALESCE(?, status), tone = COALESCE(?, tone), \
             is_default = COALESCE(?, is_default), remark = COALESCE(?, remark), \
             update_by = ?, update_time = ? WHERE id = ?",
            vec![
                command.dict_type.clone().into(),
                command.label.clone().into(),
                command.value.clone().into(),
                command.sort.into(),
                command.status.into(),
                command.tone.clone().into(),
                command.def.map(i64::from).into(),
                command.remark.clone().into(),
                operator.into(),
                time::now().into(),
                id.into(),
            ],
        )
        .await
    }

    async fn delete_dict_data<C>(&self, db: &C, id: i64) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "DELETE FROM sys_dict_data WHERE id = ?",
            vec![id.into()],
        )
        .await
    }
}

fn type_filter(keyword: Option<&str>, status: Option<i64>) -> (String, Vec<Value>) {
    let mut clauses = vec!["1 = 1".to_string()];
    let mut values = Vec::new();

    if let Some(keyword) = keyword.filter(|value| !value.is_empty()) {
        clauses.push("(t.dict_name LIKE ? OR t.dict_type LIKE ?)".to_string());
        let pattern = format!("%{keyword}%");
        values.push(pattern.clone().into());
        values.push(pattern.into());
    }
    if let Some(status) = status {
        clauses.push("t.status = ?".to_string());
        values.push(status.into());
    }

    (clauses.join(" AND "), values)
}
