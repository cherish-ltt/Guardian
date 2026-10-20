use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenTableRow {
    pub id: i64,
    pub table_name: String,
    pub table_comment: Option<String>,
    pub class_name: String,
    pub module: String,
    pub business_name: String,
    pub synced: bool,
    pub create_time: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenColumnRow {
    pub id: i64,
    pub col: String,
    pub rust_field: String,
    pub rust_type: String,
    pub jdbc_type: String,
    pub comment: Option<String>,
    pub insert: bool,
    pub edit: bool,
    pub list: bool,
    pub query: bool,
    pub query_type: String,
    pub form_type: String,
    pub required: bool,
    /// 数据库列是否允许 NULL：生成物用 `Option<T>` 表示可空列。
    pub nullable: bool,
}

/// 可导入的数据库表：`imported` 表示已导入过生成配置。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportableTable {
    pub table_name: String,
    pub table_comment: Option<String>,
    pub imported: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenFile {
    pub name: String,
    pub path: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenPreview {
    pub files: Vec<GenFile>,
    pub active_file: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenImportResult {
    pub imported: usize,
    pub ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenGenerateResult {
    pub table_name: String,
    pub message: String,
}

/// 生成配置里的一行：表信息 + 字段列表一次取回，供预览与下载复用。
#[derive(Debug, Clone)]
pub struct GenTableDetail {
    pub id: i64,
    pub table_name: String,
    pub table_comment: Option<String>,
    pub class_name: String,
    pub module: String,
    pub business_name: String,
    pub synced: i64,
}

/// 数据库元数据里的表：SQLite 从 `sqlite_master` 读。
#[derive(Debug, Clone)]
pub struct DatabaseTable {
    pub table_name: String,
    pub table_comment: Option<String>,
}

/// 数据库元数据里的列。
#[derive(Debug, Clone)]
pub struct DatabaseColumn {
    pub column_name: String,
    pub data_type: String,
    pub column_type: String,
    pub column_comment: Option<String>,
    /// `YES` / `NO`，与 MySQL `information_schema.columns.is_nullable` 语义一致。
    pub nullable: String,
    pub sort: i64,
}

/// 新导入的表：类名、模块与业务名由表名推导后一次传入。
#[derive(Debug, Clone, Copy)]
pub struct NewGenTable<'a> {
    pub table_name: &'a str,
    pub table_comment: Option<&'a str>,
    pub class_name: &'a str,
    pub module: &'a str,
    pub business_name: &'a str,
    pub operator: &'a str,
}

/// 写入生成配置的字段行。
#[derive(Debug, Clone)]
pub struct NewGenColumn {
    pub table_id: i64,
    pub column_name: String,
    pub rust_field: String,
    pub rust_type: String,
    pub jdbc_type: String,
    pub column_comment: Option<String>,
    pub insert: i64,
    pub edit: i64,
    pub list: i64,
    pub query: i64,
    pub query_type: String,
    pub form_type: String,
    pub required: i64,
    pub nullable: i64,
    pub sort: i64,
}

/// 生成配置的待保存内容：表说明与确认标记可改，其余只读。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenTableUpdateCommand {
    pub table_comment: Option<String>,
    pub synced: Option<bool>,
}
