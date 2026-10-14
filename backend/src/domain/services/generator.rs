use async_trait::async_trait;

use crate::domain::{
    error::AppError,
    models::{
        generator::{
            GenColumnRow, GenGenerateResult, GenImportResult, GenPreview, GenTableRow,
            GenTableUpdateCommand, ImportableTable,
        },
        pagination::PageResult,
        system_admin::KeywordQuery,
    },
};

/// 代码生成用例：导入数据库表、预览模板产物、下载 ZIP 与维护生成配置。
#[async_trait]
pub trait GeneratorUseCase: Send + Sync {
    async fn tables(&self, query: &KeywordQuery) -> Result<PageResult<GenTableRow>, AppError>;

    async fn importable_tables(
        &self,
        keyword: Option<&str>,
    ) -> Result<Vec<ImportableTable>, AppError>;

    async fn import_tables(
        &self,
        table_names: Vec<String>,
        operator: &str,
    ) -> Result<GenImportResult, AppError>;

    async fn columns(&self, table_id: i64) -> Result<Vec<GenColumnRow>, AppError>;

    async fn preview(&self, table_id: i64) -> Result<GenPreview, AppError>;

    async fn update_table(
        &self,
        table_id: i64,
        command: GenTableUpdateCommand,
        operator: &str,
    ) -> Result<(), AppError>;

    async fn generate(&self, table_id: i64) -> Result<GenGenerateResult, AppError>;

    /// 下载包：返回 `(建议文件名, ZIP 字节)`。
    async fn download(&self, table_id: i64) -> Result<(String, Vec<u8>), AppError>;
}
