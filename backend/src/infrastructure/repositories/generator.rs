use async_trait::async_trait;
use sea_orm::{ConnectionTrait, Value};

use crate::{
    domain::models::{
        generator::{
            DatabaseColumn, DatabaseTable, GenColumnRow, GenTableDetail, GenTableRow,
            GenTableUpdateCommand, ImportableTable, NewGenColumn, NewGenTable,
        },
        pagination::Page,
    },
    error::AppError,
    infrastructure::repositories::sql::{
        execute, insert, int_or_zero, query_all, query_count, query_opt, text, text_or_none,
    },
    util::time,
};

/// 代码生成配置与数据库元数据的读写。
#[async_trait]
pub trait GeneratorRepository: Send + Sync {
    async fn count_tables<C>(&self, db: &C, keyword: Option<&str>) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn list_tables<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        page: Page,
    ) -> Result<Vec<GenTableRow>, AppError>
    where
        C: ConnectionTrait;

    async fn list_importable_tables<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
    ) -> Result<Vec<ImportableTable>, AppError>
    where
        C: ConnectionTrait;

    async fn database_table<C>(
        &self,
        db: &C,
        table_name: &str,
    ) -> Result<Option<DatabaseTable>, AppError>
    where
        C: ConnectionTrait;

    async fn database_columns<C>(
        &self,
        db: &C,
        table_name: &str,
    ) -> Result<Vec<DatabaseColumn>, AppError>
    where
        C: ConnectionTrait;

    async fn table_id_by_name<C>(&self, db: &C, table_name: &str) -> Result<Option<i64>, AppError>
    where
        C: ConnectionTrait;

    async fn insert_table<C>(&self, db: &C, table: &NewGenTable<'_>) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn insert_column<C>(&self, db: &C, column: &NewGenColumn) -> Result<(), AppError>
    where
        C: ConnectionTrait;

    async fn table_detail<C>(&self, db: &C, id: i64) -> Result<Option<GenTableDetail>, AppError>
    where
        C: ConnectionTrait;

    async fn columns<C>(&self, db: &C, table_id: i64) -> Result<Vec<GenColumnRow>, AppError>
    where
        C: ConnectionTrait;

    async fn update_table<C>(
        &self,
        db: &C,
        id: i64,
        command: &GenTableUpdateCommand,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SeaOrmGeneratorRepository;

/// SQLite 没有注释，表说明留空；`_sqlx_migrations` 属于框架表，不出现在候选列表里。
const IMPORTABLE_SELECT: &str = "SELECT t.name AS table_name, \
     CASE WHEN g.id IS NULL THEN 0 ELSE 1 END AS imported \
     FROM sqlite_master t LEFT JOIN gen_table g ON g.table_name = t.name \
     WHERE t.type = 'table' AND t.name NOT LIKE 'sqlite_%' AND t.name <> '_sqlx_migrations'";

#[async_trait]
impl GeneratorRepository for SeaOrmGeneratorRepository {
    async fn count_tables<C>(&self, db: &C, keyword: Option<&str>) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, values) = filter_of(keyword);
        let sql = format!("SELECT COUNT(*) AS value FROM gen_table WHERE {filter}");

        query_count(db, &sql, values).await
    }

    async fn list_tables<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
        page: Page,
    ) -> Result<Vec<GenTableRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, mut values) = filter_of(keyword);
        let sql = format!(
            "SELECT id, table_name, table_comment, class_name, module_name, business_name, synced, \
             substr(create_time, 1, 16) AS create_time FROM gen_table WHERE {filter} \
             ORDER BY update_time DESC, id DESC LIMIT ? OFFSET ?"
        );
        values.push(page.limit().into());
        values.push(page.offset().into());
        let rows = query_all(db, &sql, values).await?;

        Ok(rows
            .iter()
            .map(|row| GenTableRow {
                id: int_or_zero(row, "id"),
                table_name: text(row, "table_name").unwrap_or_default(),
                table_comment: text_or_none(row, "table_comment"),
                class_name: text(row, "class_name").unwrap_or_default(),
                module: text(row, "module_name").unwrap_or_default(),
                business_name: text(row, "business_name").unwrap_or_default(),
                synced: int_or_zero(row, "synced") == 1,
                create_time: text_or_none(row, "create_time"),
            })
            .collect())
    }

    async fn list_importable_tables<C>(
        &self,
        db: &C,
        keyword: Option<&str>,
    ) -> Result<Vec<ImportableTable>, AppError>
    where
        C: ConnectionTrait,
    {
        let mut sql = IMPORTABLE_SELECT.to_string();
        let mut values: Vec<Value> = Vec::new();
        if let Some(keyword) = keyword.filter(|value| !value.is_empty()) {
            sql.push_str(" AND t.name LIKE ?");
            values.push(format!("%{keyword}%").into());
        }
        sql.push_str(" ORDER BY t.name");
        let rows = query_all(db, &sql, values).await?;

        Ok(rows
            .iter()
            .map(|row| ImportableTable {
                table_name: text(row, "table_name").unwrap_or_default(),
                table_comment: None,
                imported: int_or_zero(row, "imported") == 1,
            })
            .collect())
    }

    async fn database_table<C>(
        &self,
        db: &C,
        table_name: &str,
    ) -> Result<Option<DatabaseTable>, AppError>
    where
        C: ConnectionTrait,
    {
        let row = query_opt(
            db,
            "SELECT name AS table_name FROM sqlite_master WHERE type = 'table' AND name = ?",
            vec![table_name.into()],
        )
        .await?;

        Ok(row.map(|row| DatabaseTable {
            table_name: text(&row, "table_name").unwrap_or_default(),
            table_comment: None,
        }))
    }

    async fn database_columns<C>(
        &self,
        db: &C,
        table_name: &str,
    ) -> Result<Vec<DatabaseColumn>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT p.name AS column_name, p.type AS column_type, p.\"notnull\" AS not_null, \
             p.cid AS sort FROM pragma_table_info(?) p ORDER BY p.cid",
            vec![table_name.into()],
        )
        .await?;

        Ok(rows
            .iter()
            .map(|row| {
                let column_type = text(row, "column_type").unwrap_or_default();

                DatabaseColumn {
                    column_name: text(row, "column_name").unwrap_or_default(),
                    data_type: base_type(&column_type),
                    column_type,
                    column_comment: None,
                    nullable: if int_or_zero(row, "not_null") == 1 {
                        "NO".to_string()
                    } else {
                        "YES".to_string()
                    },
                    sort: int_or_zero(row, "sort"),
                }
            })
            .collect())
    }

    async fn table_id_by_name<C>(&self, db: &C, table_name: &str) -> Result<Option<i64>, AppError>
    where
        C: ConnectionTrait,
    {
        let row = query_opt(
            db,
            "SELECT id AS value FROM gen_table WHERE table_name = ?",
            vec![table_name.into()],
        )
        .await?;

        Ok(row.as_ref().map(|row| int_or_zero(row, "value")))
    }

    async fn insert_table<C>(&self, db: &C, table: &NewGenTable<'_>) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let now = time::now();

        insert(
            db,
            "INSERT INTO gen_table (table_name, table_comment, class_name, module_name, \
             business_name, synced, create_by, update_by, create_time, update_time) \
             VALUES (?, ?, ?, ?, ?, 1, ?, ?, ?, ?)",
            vec![
                table.table_name.into(),
                table.table_comment.into(),
                table.class_name.into(),
                table.module.into(),
                table.business_name.into(),
                table.operator.into(),
                table.operator.into(),
                now.clone().into(),
                now.into(),
            ],
        )
        .await
    }

    async fn insert_column<C>(&self, db: &C, column: &NewGenColumn) -> Result<(), AppError>
    where
        C: ConnectionTrait,
    {
        insert(
            db,
            "INSERT INTO gen_table_column (table_id, column_name, rust_field, rust_type, jdbc_type, \
             column_comment, is_insert, is_edit, is_list, is_query, query_type, form_type, required, \
             nullable, sort) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            vec![
                column.table_id.into(),
                column.column_name.clone().into(),
                column.rust_field.clone().into(),
                column.rust_type.clone().into(),
                column.jdbc_type.clone().into(),
                column.column_comment.clone().into(),
                column.insert.into(),
                column.edit.into(),
                column.list.into(),
                column.query.into(),
                column.query_type.clone().into(),
                column.form_type.clone().into(),
                column.required.into(),
                column.nullable.into(),
                column.sort.into(),
            ],
        )
        .await?;

        Ok(())
    }

    async fn table_detail<C>(&self, db: &C, id: i64) -> Result<Option<GenTableDetail>, AppError>
    where
        C: ConnectionTrait,
    {
        let row = query_opt(
            db,
            "SELECT id, table_name, table_comment, class_name, module_name, business_name, synced \
             FROM gen_table WHERE id = ?",
            vec![id.into()],
        )
        .await?;

        Ok(row.map(|row| GenTableDetail {
            id: int_or_zero(&row, "id"),
            table_name: text(&row, "table_name").unwrap_or_default(),
            table_comment: text_or_none(&row, "table_comment"),
            class_name: text(&row, "class_name").unwrap_or_default(),
            module: text(&row, "module_name").unwrap_or_default(),
            business_name: text(&row, "business_name").unwrap_or_default(),
            synced: int_or_zero(&row, "synced"),
        }))
    }

    async fn columns<C>(&self, db: &C, table_id: i64) -> Result<Vec<GenColumnRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT id, column_name, rust_field, rust_type, jdbc_type, column_comment, is_insert, \
             is_edit, is_list, is_query, query_type, form_type, required, nullable \
             FROM gen_table_column WHERE table_id = ? ORDER BY sort, id",
            vec![table_id.into()],
        )
        .await?;

        Ok(rows
            .iter()
            .map(|row| GenColumnRow {
                id: int_or_zero(row, "id"),
                col: text(row, "column_name").unwrap_or_default(),
                rust_field: text(row, "rust_field").unwrap_or_default(),
                rust_type: text(row, "rust_type").unwrap_or_default(),
                jdbc_type: text(row, "jdbc_type").unwrap_or_default(),
                comment: text_or_none(row, "column_comment"),
                insert: int_or_zero(row, "is_insert") == 1,
                edit: int_or_zero(row, "is_edit") == 1,
                list: int_or_zero(row, "is_list") == 1,
                query: int_or_zero(row, "is_query") == 1,
                query_type: text(row, "query_type").unwrap_or_default(),
                form_type: text(row, "form_type").unwrap_or_default(),
                required: int_or_zero(row, "required") == 1,
                nullable: int_or_zero(row, "nullable") == 1,
            })
            .collect())
    }

    async fn update_table<C>(
        &self,
        db: &C,
        id: i64,
        command: &GenTableUpdateCommand,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE gen_table SET table_comment = COALESCE(?, table_comment), \
             synced = COALESCE(?, synced), update_by = ?, update_time = ? WHERE id = ?",
            vec![
                command.table_comment.clone().into(),
                command.synced.map(i64::from).into(),
                operator.into(),
                time::now().into(),
                id.into(),
            ],
        )
        .await
    }
}

fn filter_of(keyword: Option<&str>) -> (String, Vec<Value>) {
    let mut clauses = vec!["1 = 1".to_string()];
    let mut values = Vec::new();

    if let Some(keyword) = keyword.filter(|value| !value.is_empty()) {
        clauses
            .push("(table_name LIKE ? OR table_comment LIKE ? OR class_name LIKE ?)".to_string());
        let pattern = format!("%{keyword}%");
        for _ in 0..3 {
            values.push(pattern.clone().into());
        }
    }

    (clauses.join(" AND "), values)
}

/// `VARCHAR(64)` → `varchar`，与 MySQL `information_schema.columns.data_type` 的语义对齐。
fn base_type(column_type: &str) -> String {
    column_type
        .split('(')
        .next()
        .unwrap_or_default()
        .trim()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::base_type;

    #[test]
    fn base_type_strips_length_and_lowercases() {
        assert_eq!(base_type("VARCHAR(64)"), "varchar");
        assert_eq!(base_type("INTEGER"), "integer");
        assert_eq!(base_type(" TEXT "), "text");
        assert_eq!(base_type(""), "");
    }
}
