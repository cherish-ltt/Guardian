use aws_lc_rs::digest::{SHA256, digest};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};

use crate::{
    domain::{
        error::AppError,
        models::generator::{
            DatabaseColumn, GenColumnChange, GenColumnRow, GenSyncCommand, GenSyncPreview,
            GenSyncResult, GenTableDetail, NewGenColumn,
        },
    },
    infrastructure::repositories::generator::{GeneratorRepository, SeaOrmGeneratorRepository},
    services::{
        generator::{column_row, validate_primary_key},
        support::in_immediate_transaction,
    },
};

struct Snapshot {
    table: GenTableDetail,
    columns: Vec<GenColumnRow>,
    database_columns: Vec<DatabaseColumn>,
}

impl Snapshot {
    async fn load<C: ConnectionTrait>(database: &C, table_id: i64) -> Result<Self, AppError> {
        let repository = SeaOrmGeneratorRepository;
        let table = repository
            .table_detail(database, table_id)
            .await?
            .ok_or_else(|| AppError::business("代码生成表不存在"))?;
        if repository
            .database_table(database, &table.table_name)
            .await?
            .is_none()
        {
            return Err(AppError::business(format!(
                "数据库表不存在：{}",
                table.table_name
            )));
        }
        let columns = repository.columns(database, table_id).await?;
        let database_columns = repository
            .database_columns(database, &table.table_name)
            .await?;
        validate_primary_key(&table.table_name, &database_columns)?;
        Ok(Self {
            table,
            columns,
            database_columns,
        })
    }

    fn revision(&self) -> Result<String, AppError> {
        let bytes = serde_json::to_vec(&(&self.table, &self.columns, &self.database_columns))
            .map_err(|error| AppError::internal(error.to_string()))?;
        Ok(URL_SAFE_NO_PAD.encode(digest(&SHA256, &bytes)))
    }

    fn changes(&self) -> Vec<GenColumnChange> {
        let mut changes = Vec::new();
        for column in &self.database_columns {
            let defaults = draft_column(column_row(self.table.id, column));
            let before = self
                .columns
                .iter()
                .find(|old| old.col == column.column_name);
            if let Some(before) = before {
                let (after, changed_attributes) = refresh_column(before, defaults);
                if !changed_attributes.is_empty() {
                    changes.push(GenColumnChange {
                        column_name: column.column_name.clone(),
                        kind: "changed",
                        before: Some(before.clone()),
                        after: Some(after),
                        changed_attributes,
                    });
                }
            } else {
                changes.push(GenColumnChange {
                    column_name: column.column_name.clone(),
                    kind: "added",
                    before: None,
                    after: Some(defaults),
                    changed_attributes: Vec::new(),
                });
            }
        }
        for before in &self.columns {
            if !self
                .database_columns
                .iter()
                .any(|column| column.column_name == before.col)
            {
                changes.push(GenColumnChange {
                    column_name: before.col.clone(),
                    kind: "removed",
                    before: Some(before.clone()),
                    after: None,
                    changed_attributes: Vec::new(),
                });
            }
        }
        changes
    }
}

pub(super) async fn preview(
    database: &DatabaseConnection,
    table_id: i64,
) -> Result<GenSyncPreview, AppError> {
    // DEFERRED 保证预览读到同一快照，同时不提前占用写锁。
    let transaction = database.begin().await?;
    let result = async {
        let snapshot = Snapshot::load(&transaction, table_id).await?;
        Ok(GenSyncPreview {
            table_name: snapshot.table.table_name.clone(),
            revision: snapshot.revision()?,
            changes: snapshot.changes(),
        })
    }
    .await;
    match result {
        Ok(value) => {
            transaction.commit().await?;
            Ok(value)
        },
        Err(error) => {
            if let Err(rollback_error) = transaction.rollback().await {
                tracing::warn!(%rollback_error, "transaction rollback failed");
            }
            Err(error)
        },
    }
}

pub(super) async fn synchronize(
    database: &DatabaseConnection,
    table_id: i64,
    command: GenSyncCommand,
    operator: &str,
) -> Result<GenSyncResult, AppError> {
    if command.revision.is_empty() {
        return Err(AppError::business("请先预览结构差异"));
    }
    let operator = operator.to_string();
    in_immediate_transaction(database, |transaction| {
        Box::pin(async move {
            let repository = SeaOrmGeneratorRepository;
            let snapshot = Snapshot::load(transaction, table_id).await?;
            if command.revision != snapshot.revision()? {
                return Err(AppError::conflict(
                    "数据库结构或生成配置已变化，请重新预览结构差异",
                ));
            }
            let changes = snapshot.changes();
            for change in &changes {
                match (&change.before, &change.after) {
                    (None, Some(after)) => {
                        let column = snapshot
                            .database_columns
                            .iter()
                            .find(|column| column.column_name == after.col)
                            .ok_or_else(|| AppError::internal("同步列元数据缺失"))?;
                        repository
                            .insert_column(transaction, &column_row(table_id, column))
                            .await?;
                    },
                    (Some(_), Some(after)) => {
                        repository
                            .update_column(transaction, table_id, after)
                            .await?
                    },
                    (Some(before), None) => {
                        repository
                            .delete_column(transaction, table_id, before.id)
                            .await?
                    },
                    (None, None) => return Err(AppError::internal("同步列差异无效")),
                }
            }
            if !changes.is_empty() {
                repository
                    .finish_sync(transaction, &snapshot.table, &operator)
                    .await?;
            }
            Ok(GenSyncResult {
                table_name: snapshot.table.table_name,
                changed: changes.len(),
            })
        })
    })
    .await
}

fn draft_column(column: NewGenColumn) -> GenColumnRow {
    GenColumnRow {
        id: 0,
        col: column.column_name,
        rust_field: column.rust_field,
        rust_type: column.rust_type,
        jdbc_type: column.jdbc_type,
        column_type: column.column_type,
        comment: column.column_comment,
        insert: column.insert == 1,
        edit: column.edit == 1,
        list: column.list == 1,
        query: column.query == 1,
        query_type: column.query_type,
        form_type: column.form_type,
        required: column.required == 1,
        nullable: column.nullable == 1,
        primary_key: column.primary_key == 1,
        sort: column.sort,
    }
}

fn refresh_column(
    before: &GenColumnRow,
    defaults: GenColumnRow,
) -> (GenColumnRow, Vec<&'static str>) {
    let mut after = before.clone();
    let mut changed_attributes = Vec::new();
    if !before
        .column_type
        .eq_ignore_ascii_case(&defaults.column_type)
    {
        after.column_type = defaults.column_type;
        after.jdbc_type = defaults.jdbc_type;
        after.rust_type = defaults.rust_type;
        after.form_type = defaults.form_type;
        changed_attributes.push("columnType");
    }
    if before.primary_key != defaults.primary_key {
        after.primary_key = defaults.primary_key;
        after.insert = defaults.insert;
        after.edit = defaults.edit;
        after.list = defaults.list;
        after.required = defaults.required;
        changed_attributes.push("primaryKey");
    }
    if before.nullable != defaults.nullable {
        after.nullable = defaults.nullable;
        after.required = defaults.required;
        changed_attributes.push("nullable");
    }
    if before.sort != defaults.sort {
        after.sort = defaults.sort;
        changed_attributes.push("sort");
    }
    (after, changed_attributes)
}
