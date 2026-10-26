use std::io::{Cursor, Write};

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use zip::{ZipWriter, write::SimpleFileOptions};

use crate::{
    domain::{
        error::AppError,
        models::{
            generator::{
                DatabaseColumn, GenColumnRow, GenFile, GenGenerateResult, GenImportResult,
                GenPreview, GenTableDetail, GenTableRow, GenTableUpdateCommand, ImportableTable,
                NewGenColumn, NewGenTable,
            },
            pagination::{Page, PageResult},
            system_admin::KeywordQuery,
        },
        services::generator::GeneratorUseCase,
    },
    infrastructure::repositories::generator::{GeneratorRepository, SeaOrmGeneratorRepository},
    services::support::in_immediate_transaction,
};

/// 预览里默认展开的文件：实体对应的 Controller。
const ACTIVE_FILE_SUFFIX: &str = "Controller";
/// 控制器产物路径：默认展开它，预览与下载文件名都从这里取。
const ACTIVE_FILE_PATH: &str = "backend/src/api/controllers";
/// 主键列名：`id` 列不参与新增 / 编辑 / 列表展示。
const PRIMARY_COLUMN: &str = "id";

pub struct GeneratorService {
    database: DatabaseConnection,
}

impl GeneratorService {
    pub fn new(database: DatabaseConnection) -> Self {
        Self { database }
    }

    async fn table_detail(&self, table_id: i64) -> Result<GenTableDetail, AppError> {
        SeaOrmGeneratorRepository
            .table_detail(&self.database, table_id)
            .await?
            .ok_or_else(|| AppError::business("代码生成表不存在"))
    }

    async fn rendered_files(&self, table: &GenTableDetail) -> Result<Vec<GenFile>, AppError> {
        let columns = SeaOrmGeneratorRepository
            .columns(&self.database, table.id)
            .await?;

        Ok(render_files(table, &columns))
    }

    /// 默认展开的控制器：路径为 `backend/src/api/controllers/<表名>.rs`。
    fn active_file_path(table_name: &str) -> String {
        format!("{ACTIVE_FILE_PATH}/{table_name}.rs")
    }
}

#[async_trait]
impl GeneratorUseCase for GeneratorService {
    async fn tables(&self, query: &KeywordQuery) -> Result<PageResult<GenTableRow>, AppError> {
        let page = Page::from_query(Some(&query.page))?;
        let keyword = query.keyword.as_deref();
        let total = SeaOrmGeneratorRepository
            .count_tables(&self.database, keyword)
            .await?;
        let rows = SeaOrmGeneratorRepository
            .list_tables(&self.database, keyword, page)
            .await?;

        Ok(PageResult::new(rows, total, page))
    }

    async fn importable_tables(
        &self,
        keyword: Option<&str>,
    ) -> Result<Vec<ImportableTable>, AppError> {
        SeaOrmGeneratorRepository
            .list_importable_tables(&self.database, keyword)
            .await
    }

    async fn import_tables(
        &self,
        table_names: Vec<String>,
        operator: &str,
    ) -> Result<GenImportResult, AppError> {
        if table_names.is_empty() {
            return Err(AppError::business("请选择要导入的数据库表"));
        }

        let operator = operator.to_string();
        let ids = in_immediate_transaction(&self.database, |transaction| {
            let table_names = table_names.clone();
            let operator = operator.clone();
            Box::pin(async move {
                let repository = SeaOrmGeneratorRepository;
                let mut ids = Vec::new();
                for table_name in table_names {
                    if let Some(existing) = repository
                        .table_id_by_name(transaction, &table_name)
                        .await?
                    {
                        ids.push(existing);
                        continue;
                    }

                    let Some(table) = repository.database_table(transaction, &table_name).await?
                    else {
                        return Err(AppError::business(format!("数据库表不存在：{table_name}")));
                    };
                    let table_id = repository
                        .insert_table(
                            transaction,
                            &NewGenTable {
                                table_name: &table.table_name,
                                table_comment: table.table_comment.as_deref(),
                                class_name: &upper_camel(&table.table_name),
                                module: &module_name(&table.table_name),
                                business_name: &business_name(&table.table_name),
                                operator: &operator,
                            },
                        )
                        .await?;
                    for column in repository
                        .database_columns(transaction, &table_name)
                        .await?
                    {
                        repository
                            .insert_column(transaction, &column_row(table_id, &column))
                            .await?;
                    }
                    ids.push(table_id);
                }

                Ok(ids)
            })
        })
        .await?;

        Ok(GenImportResult {
            imported: ids.len(),
            ids,
        })
    }

    async fn columns(&self, table_id: i64) -> Result<Vec<GenColumnRow>, AppError> {
        self.table_detail(table_id).await?;

        SeaOrmGeneratorRepository
            .columns(&self.database, table_id)
            .await
    }

    async fn preview(&self, table_id: i64) -> Result<GenPreview, AppError> {
        let table = self.table_detail(table_id).await?;
        let active_file = Self::active_file_path(&table.table_name);
        let files = self.rendered_files(&table).await?;
        let code = files
            .iter()
            .find(|file| file.path == active_file)
            .map(|file| file.code.clone())
            .unwrap_or_default();

        Ok(GenPreview {
            files,
            active_file,
            code,
        })
    }

    async fn update_table(
        &self,
        table_id: i64,
        command: GenTableUpdateCommand,
        operator: &str,
    ) -> Result<(), AppError> {
        if SeaOrmGeneratorRepository
            .update_table(&self.database, table_id, &command, operator)
            .await?
            == 0
        {
            return Err(AppError::business("代码生成表不存在"));
        }

        Ok(())
    }

    async fn generate(&self, table_id: i64) -> Result<GenGenerateResult, AppError> {
        let table = self.table_detail(table_id).await?;

        Ok(GenGenerateResult {
            table_name: table.table_name,
            message: "代码压缩包已准备就绪".to_string(),
        })
    }

    async fn download(&self, table_id: i64) -> Result<(String, Vec<u8>), AppError> {
        let table = self.table_detail(table_id).await?;
        let files = self.rendered_files(&table).await?;
        // ZIP 名取实体类名，去掉 Controller 后缀（`SysPost` → `SysPost.zip`）。
        let file_name = format!(
            "{}.zip",
            table
                .class_name
                .strip_suffix(ACTIVE_FILE_SUFFIX)
                .unwrap_or(&table.class_name)
        );

        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for file in &files {
            writer
                .start_file(file.path.as_str(), SimpleFileOptions::default())
                .map_err(|error| AppError::business(format!("代码压缩包生成失败：{error}")))?;
            writer
                .write_all(file.code.as_bytes())
                .map_err(|error| AppError::business(format!("代码压缩包生成失败：{error}")))?;
        }

        let bytes = writer
            .finish()
            .map_err(|error| AppError::business(format!("代码压缩包生成失败：{error}")))?
            .into_inner();

        Ok((file_name, bytes))
    }
}

/// 数据库列 → 生成配置行：主键不进表单与列表，名称/编码/状态/类型默认作为查询条件。
fn column_row(table_id: i64, column: &DatabaseColumn) -> NewGenColumn {
    let primary_like = column.column_name == PRIMARY_COLUMN;
    let query_field = query_field(&column.column_name);

    NewGenColumn {
        table_id,
        column_name: column.column_name.clone(),
        rust_field: snake_case(&column.column_name),
        rust_type: rust_type(&column.data_type).to_string(),
        jdbc_type: column.data_type.clone(),
        column_comment: column.column_comment.clone(),
        insert: i64::from(!primary_like),
        edit: i64::from(!primary_like),
        list: i64::from(!primary_like),
        query: i64::from(query_field),
        query_type: if query_field { "LIKE" } else { "=" }.to_string(),
        form_type: form_type(&column.data_type).to_string(),
        required: i64::from(column.nullable == "NO" && !primary_like),
        nullable: i64::from(column.nullable == "YES" && !primary_like),
        sort: column.sort,
    }
}

/// 十个模板产物：实体、领域模型、用例接口、仓储、服务、控制器、前端 api / 页面、菜单权限 SQL
/// 与一份接入说明。
fn render_files(table: &GenTableDetail, columns: &[GenColumnRow]) -> Vec<GenFile> {
    let module = table.module.as_str();
    let class_name = table.class_name.as_str();
    let table_name = table.table_name.as_str();
    let business = table.business_name.as_str();
    let entries = [
        (
            format!("backend/src/entities/{table_name}.rs"),
            entity_code(table_name, columns),
        ),
        (
            format!("backend/src/domain/models/{table_name}.rs"),
            domain_model_code(class_name, columns),
        ),
        (
            format!("backend/src/domain/services/{table_name}.rs"),
            domain_service_code(class_name, table_name, business),
        ),
        (
            format!("backend/src/infrastructure/repositories/{table_name}.rs"),
            repository_code(class_name, table_name, business, columns),
        ),
        (
            format!("backend/src/services/{table_name}.rs"),
            service_code(class_name, table_name, business),
        ),
        (
            format!("backend/src/api/controllers/{table_name}.rs"),
            controller_code(module, class_name, table_name, business),
        ),
        (
            format!("frontend/src/api/{business}.ts"),
            api_code(module, business),
        ),
        (
            format!("frontend/src/views/{module}/{class_name}.vue"),
            view_code(table_name, class_name, columns),
        ),
        (
            format!("sql/{table_name}_menu.sql"),
            menu_sql(class_name, module, business),
        ),
        (
            format!("docs/generated/{table_name}.md"),
            readme_code(module, class_name, table_name, business),
        ),
    ];

    entries
        .into_iter()
        .map(|(path, code)| {
            let name = path.rsplit('/').next().unwrap_or_default().to_string();

            GenFile { name, path, code }
        })
        .collect()
}

/// sea-orm 实体：表名取生成配置，主键列带 `primary_key` 标注，列注释转文档注释。
fn entity_code(table_name: &str, columns: &[GenColumnRow]) -> String {
    let mut fields = String::new();
    for column in columns {
        if let Some(comment) = column
            .comment
            .as_deref()
            .filter(|comment| !comment.is_empty())
        {
            fields.push_str(&format!("    /// {comment}\n"));
        }
        if column.col == PRIMARY_COLUMN {
            fields.push_str("    #[sea_orm(primary_key)]\n");
        }
        fields.push_str(&format!(
            "    pub {}: {},\n",
            column.rust_field,
            rust_field_type(column)
        ));
    }

    format!(
        "use sea_orm::entity::prelude::*;\n\n\
         #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]\n\
         #[sea_orm(table_name = \"{table_name}\")]\n\
         pub struct Model {{\n{fields}}}\n\n\
         #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]\n\
         pub enum Relation {{}}\n\n\
         impl ActiveModelBehavior for ActiveModel {{}}\n"
    )
}

/// 领域模型：列表行与写入命令，字段名按 camelCase 序列化给前端。
fn domain_model_code(class_name: &str, columns: &[GenColumnRow]) -> String {
    let row_fields = columns
        .iter()
        .map(|column| {
            format!(
                "    pub {}: {},\n",
                column.rust_field,
                rust_field_type(column)
            )
        })
        .collect::<String>();
    let command_fields = columns
        .iter()
        .filter(|column| column.col != PRIMARY_COLUMN)
        .map(|column| {
            format!(
                "    pub {}: Option<{}>,\n",
                column.rust_field, column.rust_type
            )
        })
        .collect::<String>();

    format!(
        "use serde::{{Deserialize, Serialize}};\n\n\
         /// {class_name} 的列表行。\n\
         #[derive(Debug, Clone, Serialize)]\n\
         #[serde(rename_all = \"camelCase\")]\n\
         pub struct {class_name}Row {{\n{row_fields}}}\n\n\
         /// {class_name} 的新增 / 编辑请求：未传的字段由服务层按缺省值处理。\n\
         #[derive(Debug, Clone, Default, Deserialize)]\n\
         #[serde(rename_all = \"camelCase\")]\n\
         pub struct {class_name}WriteCommand {{\n{command_fields}}}\n"
    )
}

/// 用例接口：控制器只依赖它，实现放在 `services` 层。
fn domain_service_code(class_name: &str, table_name: &str, business: &str) -> String {
    format!(
        "use async_trait::async_trait;\n\n\
         use crate::domain::{{\n    error::AppError,\n    models::{{\n        pagination::PageResult,\n        \
         system_admin::{{IdResponse, KeywordQuery}},\n        {table_name}::{{{class_name}Row, \
         {class_name}WriteCommand}},\n    }},\n}};\n\n\
         /// {class_name} 的用例：列表、新增、修改与删除。\n\
         #[async_trait]\n\
         pub trait {class_name}UseCase: Send + Sync {{\n    \
         async fn list_{business}(&self, query: &KeywordQuery) -> Result<PageResult<{class_name}Row>, \
         AppError>;\n\n    \
         async fn create_{business}(\n        &self,\n        command: {class_name}WriteCommand,\n        \
         operator: &str,\n    ) -> Result<IdResponse, AppError>;\n\n    \
         async fn update_{business}(\n        &self,\n        id: i64,\n        command: \
         {class_name}WriteCommand,\n        operator: &str,\n    ) -> Result<(), AppError>;\n\n    \
         async fn delete_{business}(&self, id: i64, operator: &str) -> Result<(), AppError>;\n}}\n"
    )
}

/// 仓储接口与适配器：列表与计数按 `column_name` 拼出可跑的 SQL，
/// 写操作留 `todo!`（生成器不知道软删除、唯一约束等业务约束，不猜）。
fn repository_code(
    class_name: &str,
    table_name: &str,
    business: &str,
    columns: &[GenColumnRow],
) -> String {
    let select = columns
        .iter()
        .map(|column| column.col.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    // 文本列参与 keyword 模糊匹配；没有文本列时退化成不过滤，避免生成空括号。
    let text_columns = columns
        .iter()
        .filter(|column| column.rust_type == "String")
        .map(|column| format!("{} LIKE ?", column.col))
        .collect::<Vec<_>>();
    let text_count = text_columns.len();
    let text_like = if text_columns.is_empty() {
        "1 = 1".to_string()
    } else {
        text_columns.join(" OR ")
    };
    let order = columns
        .iter()
        .find(|column| column.col == PRIMARY_COLUMN)
        .map_or_else(|| "1".to_string(), |column| column.col.clone());
    let row_build = columns
        .iter()
        .map(|column| format!("        {}: {},\n", column.rust_field, row_value_of(column)))
        .collect::<String>();

    // 只引入实际用到的取值助手，避免生成物带未使用 import。
    let mut sql_imports = vec!["query_all", "query_count"];
    if columns.iter().any(|column| column.rust_type == "String") {
        sql_imports.push("text");
    }
    if columns
        .iter()
        .any(|column| column.rust_type == "String" && column.nullable)
    {
        sql_imports.push("text_or_none");
    }
    if columns
        .iter()
        .any(|column| matches!(column.rust_type.as_str(), "i32" | "i64"))
    {
        sql_imports.push("int_or_zero");
    }
    if columns
        .iter()
        .any(|column| matches!(column.rust_type.as_str(), "i32" | "i64") && column.nullable)
    {
        sql_imports.push("int");
    }
    sql_imports.sort_unstable();
    let sql_imports = sql_imports.join(", ");

    format!(
        "use async_trait::async_trait;\n\
         use sea_orm::ConnectionTrait;\n\n\
         use crate::{{\n    domain::models::{{\n        pagination::Page,\n        \
         {table_name}::{{{class_name}Row, {class_name}WriteCommand}},\n    }},\n    error::AppError,\n    \
         infrastructure::repositories::sql::{{\n        {sql_imports},\n    }},\n}};\n\n\
         /// {class_name} 的读写。\n\
         #[async_trait]\n\
         pub trait {class_name}Repository: Send + Sync {{\n    \
         async fn count_{business}<C>(&self, db: &C, keyword: Option<&str>) -> Result<i64, AppError>\n    \
         where\n        C: ConnectionTrait;\n\n    \
         async fn list_{business}<C>(\n        &self,\n        db: &C,\n        keyword: Option<&str>,\n        \
         page: Page,\n    ) -> Result<Vec<{class_name}Row>, AppError>\n    where\n        C: ConnectionTrait;\n\n    \
         async fn insert_{business}<C>(\n        &self,\n        db: &C,\n        command: \
         &{class_name}WriteCommand,\n        operator: &str,\n    ) -> Result<i64, AppError>\n    where\n        \
         C: ConnectionTrait;\n\n    \
         async fn update_{business}<C>(\n        &self,\n        db: &C,\n        id: i64,\n        command: \
         &{class_name}WriteCommand,\n        operator: &str,\n    ) -> Result<u64, AppError>\n    where\n        \
         C: ConnectionTrait;\n\n    \
         async fn delete_{business}<C>(&self, db: &C, id: i64, operator: &str) -> Result<u64, AppError>\n    \
         where\n        C: ConnectionTrait;\n}}\n\n\
         #[derive(Debug, Default, Clone, Copy)]\n\
         pub struct SeaOrm{class_name}Repository;\n\n\
         /// 列表与计数用 `column_name` 直出，取值走 `sql` 模块的助手；\n\
         /// 写操作依赖业务约束（软删除、唯一校验），需要按表补完。\n\
         #[async_trait]\n\
         impl {class_name}Repository for SeaOrm{class_name}Repository {{\n    \
         async fn count_{business}<C>(&self, db: &C, keyword: Option<&str>) -> Result<i64, AppError>\n    \
         where\n        C: ConnectionTrait,\n    {{\n        \
         let (filter, values) = filter_of(keyword);\n        \
         let sql = format!(\"SELECT COUNT(*) AS value FROM {table_name} WHERE {{filter}}\");\n\n        \
         query_count(db, &sql, values).await\n    }}\n\n    \
         async fn list_{business}<C>(\n        &self,\n        db: &C,\n        keyword: Option<&str>,\n        \
         page: Page,\n    ) -> Result<Vec<{class_name}Row>, AppError>\n    where\n        C: ConnectionTrait,\n    \
         {{\n        let (filter, mut values) = filter_of(keyword);\n        \
         let sql = format!(\n            \"SELECT {select} FROM {table_name} WHERE {{filter}} \
         ORDER BY {order} LIMIT ? OFFSET ?\"\n        );\n        \
         values.push(page.limit().into());\n        values.push(page.offset().into());\n        \
         let rows = query_all(db, &sql, values).await?;\n\n        \
         Ok(rows.iter().map(row_of).collect())\n    }}\n\n    \
         async fn insert_{business}<C>(\n        &self,\n        _db: &C,\n        _command: \
         &{class_name}WriteCommand,\n        _operator: &str,\n    ) -> Result<i64, AppError>\n    where\n        \
         C: ConnectionTrait,\n    {{\n        todo!(\"按 {table_name} 的业务约束实现新增\")\n    }}\n\n    \
         async fn update_{business}<C>(\n        &self,\n        _db: &C,\n        _id: i64,\n        _command: \
         &{class_name}WriteCommand,\n        _operator: &str,\n    ) -> Result<u64, AppError>\n    where\n        \
         C: ConnectionTrait,\n    {{\n        todo!(\"按 {table_name} 的业务约束实现修改\")\n    }}\n\n    \
         async fn delete_{business}<C>(&self, _db: &C, _id: i64, _operator: &str) -> Result<u64, AppError>\n    \
         where\n        C: ConnectionTrait,\n    {{\n        todo!(\"按 {table_name} 的业务约束实现删除\")\n    }}\n}}\n\n\
         /// 关键词只按文本列模糊匹配，与后台列表的 `keyword` 语义一致。\n\
         fn filter_of(keyword: Option<&str>) -> (String, Vec<sea_orm::Value>) {{\n    \
         let mut clauses = vec![\"1 = 1\".to_string()];\n    let mut values = Vec::new();\n\n    \
         if let Some(keyword) = keyword.filter(|value| !value.is_empty()) {{\n        \
         clauses.push(format!(\"({text_like})\"));\n        \
         let pattern = format!(\"%{{keyword}}%\");\n        \
         for _ in 0..{text_count} {{\n            values.push(pattern.clone().into());\n        }}\n    }}\n\n    \
         (clauses.join(\" AND \"), values)\n}}\n\n\
         fn row_of(row: &sea_orm::QueryResult) -> {class_name}Row {{\n    {class_name}Row {{\n{row_build}    }}\n}}\n"
    )
}

/// 服务层：装配数据库连接并实现用例；未实现的写方法显式 `todo!` 而不是静默返回。
fn service_code(class_name: &str, table_name: &str, business: &str) -> String {
    format!(
        "use async_trait::async_trait;\n\
         use sea_orm::DatabaseConnection;\n\n\
         use crate::{{\n    domain::{{\n        error::AppError,\n        models::{{\n            \
         pagination::{{Page, PageResult}},\n            system_admin::{{IdResponse, KeywordQuery}},\n            \
         {table_name}::{{{class_name}Row, {class_name}WriteCommand}},\n        }},\n        \
         services::{table_name}::{class_name}UseCase,\n    }},\n    \
         infrastructure::repositories::{table_name}::{{SeaOrm{class_name}Repository, \
         {class_name}Repository}},\n}};\n\n\
         pub struct {class_name}Service {{\n    database: DatabaseConnection,\n}}\n\n\
         impl {class_name}Service {{\n    \
         pub fn new(database: DatabaseConnection) -> Self {{\n        Self {{ database }}\n    }}\n}}\n\n\
         #[async_trait]\n\
         impl {class_name}UseCase for {class_name}Service {{\n    \
         async fn list_{business}(&self, query: &KeywordQuery) -> Result<PageResult<{class_name}Row>, \
         AppError> {{\n        let page = Page::from_query(Some(&query.page))?;\n        \
         let keyword = query.keyword.as_deref();\n        \
         let total = SeaOrm{class_name}Repository\n            \
         .count_{business}(&self.database, keyword)\n            .await?;\n        \
         let rows = SeaOrm{class_name}Repository\n            \
         .list_{business}(&self.database, keyword, page)\n            .await?;\n\n        \
         Ok(PageResult::new(rows, total, page))\n    }}\n\n    \
         async fn create_{business}(\n        &self,\n        _command: {class_name}WriteCommand,\n        \
         _operator: &str,\n    ) -> Result<IdResponse, AppError> {{\n        \
         todo!(\"按 repositories/{table_name}.rs 的 SQL 实现新增\")\n    }}\n\n    \
         async fn update_{business}(\n        &self,\n        _id: i64,\n        _command: \
         {class_name}WriteCommand,\n        _operator: &str,\n    ) -> Result<(), AppError> {{\n        \
         todo!(\"按 repositories/{table_name}.rs 的 SQL 实现修改\")\n    }}\n\n    \
         async fn delete_{business}(&self, _id: i64, _operator: &str) -> Result<(), AppError> {{\n        \
         todo!(\"按 repositories/{table_name}.rs 的 SQL 实现删除\")\n    }}\n}}\n"
    )
}

/// axum 控制器：路由、权限码与响应体形状对齐本项目的后台管理接口。
fn controller_code(module: &str, class_name: &str, table_name: &str, business: &str) -> String {
    format!(
        "use std::sync::Arc;\n\n\
         use axum::{{\n    Router,\n    extract::{{Path, RawQuery, State}},\n    routing::{{get, put}},\n}};\n\n\
         use crate::{{\n    api::{{\n        done,\n        dto::admin::QueryString,\n        \
         extractors::{{admin_user::AdminUser, api_json::ApiJson}},\n        ok,\n    }},\n    \
         container::Container,\n    domain::{{\n        error::AppError,\n        models::{{\n            \
         pagination::PageResult,\n            system_admin::{{IdResponse, KeywordQuery}},\n            \
         {table_name}::{{{class_name}Row, {class_name}WriteCommand}},\n        }},\n    }},\n    \
         response::ApiResponse,\n}};\n\n\
         pub fn router() -> Router<Arc<Container>> {{\n    Router::new()\n        \
         .route(\"/{module}/{business}\", get(list).post(create))\n        \
         .route(\"/{module}/{business}/{{id}}\", put(update).delete(remove))\n}}\n\n\
         async fn list(\n    State(container): State<Arc<Container>>,\n    user: AdminUser,\n    \
         RawQuery(raw): RawQuery,\n) -> Result<ApiResponse<PageResult<{class_name}Row>>, AppError> {{\n    \
         user.require(\"{module}:{business}:list\")?;\n    let query = QueryString::parse(raw.as_deref());\n    \
         let rows = container\n        .{business}_service\n        .list_{business}(&KeywordQuery {{\n            \
         page: query.page()?,\n            keyword: query.text(\"keyword\"),\n        }})\n        .await?;\n\n    \
         Ok(ok(rows))\n}}\n\n\
         async fn create(\n    State(container): State<Arc<Container>>,\n    user: AdminUser,\n    \
         ApiJson(command): ApiJson<{class_name}WriteCommand>,\n) -> Result<ApiResponse<IdResponse>, AppError> {{\n    \
         user.require(\"{module}:{business}:add\")?;\n    let created = container\n        .{business}_service\n        \
         .create_{business}(command, user.username())\n        .await?;\n\n    Ok(ok(created))\n}}\n\n\
         async fn update(\n    State(container): State<Arc<Container>>,\n    user: AdminUser,\n    \
         Path(id): Path<i64>,\n    ApiJson(command): ApiJson<{class_name}WriteCommand>,\n) -> \
         Result<ApiResponse<()>, AppError> {{\n    user.require(\"{module}:{business}:edit\")?;\n    \
         container\n        .{business}_service\n        .update_{business}(id, command, user.username())\n        \
         .await?;\n\n    Ok(done())\n}}\n\n\
         async fn remove(\n    State(container): State<Arc<Container>>,\n    user: AdminUser,\n    \
         Path(id): Path<i64>,\n) -> Result<ApiResponse<()>, AppError> {{\n    \
         user.require(\"{module}:{business}:remove\")?;\n    container\n        .{business}_service\n        \
         .delete_{business}(id, user.username())\n        .await?;\n\n    Ok(done())\n}}\n"
    )
}

fn api_code(module: &str, business: &str) -> String {
    format!(
        "import type {{ HttpClient, QueryParams }} from './http'\n\
         import type {{ IdResponse, PageResult }} from './system'\n\n\
         export function create{class_name}Api(client: HttpClient) {{\n    return {{\n        \
         list: (query?: QueryParams) => client.get<PageResult<Record<string, unknown>>>('/{module}/{business}', \
         query),\n        \
         create: (body: Record<string, unknown>) => client.post<IdResponse>('/{module}/{business}', body),\n        \
         update: (id: number, body: Record<string, unknown>) =>\n            \
         client.put<void>(`/{module}/{business}/${{id}}`, body),\n        \
         remove: (id: number) => client.delete<void>(`/{module}/{business}/${{id}}`),\n    }}\n}}\n",
        class_name = upper_camel(business)
    )
}

fn view_code(table_name: &str, class_name: &str, columns: &[GenColumnRow]) -> String {
    let primary_field = columns
        .iter()
        .find(|column| column.list)
        .map(|column| column.rust_field.as_str())
        .unwrap_or(PRIMARY_COLUMN);

    format!(
        "<script setup lang=\"ts\">\nconst tableName = '{table_name}'\nconst className = '{class_name}'\n\
         const primaryField = '{primary_field}'\n</script>\n\n\
         <template>\n  <div>\n    <h1>{{{{ className }}}}</h1>\n    <p>{{{{ tableName }}}} · {{{{ primaryField }}}}</p>\n  </div>\n</template>\n"
    )
}

fn menu_sql(class_name: &str, module: &str, business: &str) -> String {
    format!(
        "-- {class_name} 菜单权限\n\
         INSERT INTO sys_menu (id, parent_id, menu_name, menu_type, path, component, permission, sort, visible, status, create_by, update_by)\n\
         SELECT COALESCE(MAX(id), 0) + 1, 0, '{class_name}管理', 'menu', '/{module}/{business}', \
         '{module}/{business}/index', '{module}:{business}:list', 1, 1, 1, 'system', 'system'\n\
         FROM sys_menu;\n"
    )
}

/// 接入说明：把生成的骨架落进项目需要改的装配点逐条列出。
fn readme_code(module: &str, class_name: &str, table_name: &str, business: &str) -> String {
    format!(
        "# {class_name} 接入步骤\n\n\
         由代码生成器按表 `{table_name}` 产出（模块 `{module}`、业务名 `{business}`）。\n\
         生成物是骨架：列表查询已接好，新增 / 修改 / 删除在服务层留了 `todo!`，仓储 SQL 需要自己补。\n\n\
         ## 1. 放入文件\n\n\
         | 生成文件 | 目标位置 |\n| --- | --- |\n\
         | `backend/src/entities/{table_name}.rs` | 同路径，sea-orm 实体 |\n\
         | `backend/src/domain/models/{table_name}.rs` | 同路径，列表行与写入命令 |\n\
         | `backend/src/domain/services/{table_name}.rs` | 同路径，`{class_name}UseCase` |\n\
         | `backend/src/infrastructure/repositories/{table_name}.rs` | 同路径，仓储接口与适配器 |\n\
         | `backend/src/services/{table_name}.rs` | 同路径，用例实现 |\n\
         | `backend/src/api/controllers/{table_name}.rs` | 同路径，axum 路由与处理器 |\n\
         | `frontend/src/api/{business}.ts` | 同路径，前端接口工厂 |\n\
         | `frontend/src/views/{module}/{class_name}.vue` | 同路径，页面骨架 |\n\n\
         ## 2. 注册模块\n\n\
         在下列文件的模块列表里各加一行（按字母序插入）：\n\n\
         ```rust\n\
         // backend/src/entities/mod.rs\n\
         pub mod {table_name};\n\
         // backend/src/domain/models/mod.rs\n\
         pub mod {table_name};\n\
         // backend/src/domain/services/mod.rs\n\
         pub mod {table_name};\n\
         // backend/src/infrastructure/repositories/mod.rs\n\
         pub mod {table_name};\n\
         // backend/src/services/mod.rs\n\
         pub mod {table_name};\n\
         // backend/src/api/controllers/mod.rs\n\
         pub mod {table_name};\n\
         ```\n\n\
         ## 3. 装配用例\n\n\
         `backend/src/container.rs` 里加字段与初始化：\n\n\
         ```rust\n\
         pub {business}_service: Arc<dyn {class_name}UseCase>,\n\
         let {business}_service: Arc<dyn {class_name}UseCase> =\n    \
         Arc::new({class_name}Service::new(database.clone()));\n\
         ```\n\n\
         ## 4. 挂载路由\n\n\
         `backend/src/api/mod.rs` 的 `router()` 里 merge：\n\n\
         ```rust\n\
         .merge(controllers::{table_name}::router())\n\
         ```\n\n\
         前端在 `frontend/src/api/index.ts` 里导出工厂并挂到 `systemApi` 或 `portalApi`：\n\n\
         ```ts\n\
         import {{ create{class_name}Api }} from './{business}.ts'\n\
         ```\n\n\
         ## 5. 补完仓储 SQL\n\n\
         `infrastructure/repositories/{table_name}.rs` 只生成 trait 与适配器声明，\n\
         五个方法的 SQL 按 `infrastructure/repositories/post.rs` 的写法补（`sql` 模块提供\n\
         `query_all` / `query_count` / `insert` / `execute` 与取值助手）。\n\n\
         ## 6. 权限与菜单\n\n\
         执行 `sql/{table_name}_menu.sql` 写入菜单，权限码为 `{module}:{business}:list`、\n\
         `{module}:{business}:add`、`{module}:{business}:edit`、`{module}:{business}:remove`，\n\
         需要同步授予角色。菜单的 `component` 是 `{module}/{business}/index`，与现有菜单同形；\n\
         前端要在 `frontend/src/composables/auth.ts` 的 `COMPONENT_MAP` 里登记这一项，\n\
         指向 `@/views/{module}/{class_name}.vue`，否则动态路由找不到页面。\n\n\
         ## 7. 操作留痕\n\n\
         在 `backend/src/middleware/audit.rs` 的 `AUDIT_ROUTES` 补「方法 + 路径段模式 + 模块 + 动作」，\n\
         并在 `tests/monitor_api_tests.rs` 里断言。\n"
    )
}

/// 仓储里从查询结果取值的表达式，只用 `sql` 模块已有的助手。
fn row_value_of(column: &GenColumnRow) -> String {
    let name = &column.rust_field;
    // 主键恒非空：SQLite 的 `pragma_table_info` 对 INTEGER PRIMARY KEY 报 notnull=0。
    let nullable = column.nullable && column.col != PRIMARY_COLUMN;
    match (column.rust_type.as_str(), nullable) {
        ("i64", false) => format!("int_or_zero(row, \"{name}\")"),
        ("i64", true) => format!("int(row, \"{name}\")"),
        ("i32", false) => format!("int_or_zero(row, \"{name}\") as i32"),
        ("i32", true) => format!("int(row, \"{name}\").map(|value| value as i32)"),
        // 项目没有浮点取值助手，硬转会把小数截断，因此显式留待手写。
        ("f64", _) => format!("todo!(\"按 {name} 的实际精度从查询结果取值\")"),
        (_, true) => format!("text_or_none(row, \"{name}\")"),
        (_, false) => format!("text(row, \"{name}\").unwrap_or_default()"),
    }
}

/// 列表行与实体里的字段类型：可空列包一层 `Option`，与数据库的 NULL 语义对齐。
fn rust_field_type(column: &GenColumnRow) -> String {
    if column.nullable && column.col != PRIMARY_COLUMN {
        format!("Option<{}>", column.rust_type)
    } else {
        column.rust_type.clone()
    }
}

fn module_name(table_name: &str) -> String {
    let module = table_name
        .split_once('_')
        .map_or("system", |(module, _)| module);

    if module == "sys" {
        "system".to_string()
    } else {
        module.to_string()
    }
}

fn business_name(table_name: &str) -> String {
    let raw = match table_name.split_once('_') {
        Some((_, rest)) => rest,
        None => table_name,
    };

    snake_case(raw)
}

fn upper_camel(value: &str) -> String {
    snake_case(value)
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| match part.chars().next() {
            Some(first) => format!("{}{}", first.to_uppercase(), &part[first.len_utf8()..]),
            None => String::new(),
        })
        .collect()
}

/// 列名 / 表名 → snake_case：Rust 字段与模块名只用小写加下划线。
fn snake_case(value: &str) -> String {
    value
        .to_lowercase()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

/// 数据库列类型 → Rust 类型。SQLite 只有 `TEXT` / `INTEGER` / `REAL`，时间与其余都按文本处理。
fn rust_type(data_type: &str) -> &'static str {
    match data_type {
        "bigint" | "integer" => "i64",
        "int" | "smallint" | "tinyint" => "i32",
        "decimal" | "numeric" | "real" => "f64",
        _ => "String",
    }
}
fn form_type(data_type: &str) -> &'static str {
    match data_type {
        "text" | "longtext" | "mediumtext" => "文本域",
        "int" | "integer" | "smallint" | "tinyint" | "bigint" | "decimal" | "numeric" => "数字框",
        "datetime" | "timestamp" | "date" | "time" => "日期框",
        _ => "文本框",
    }
}

fn query_field(column_name: &str) -> bool {
    let lowered = column_name.to_lowercase();

    ["name", "code", "status", "type"]
        .iter()
        .any(|keyword| lowered.contains(keyword))
}

#[cfg(test)]
mod tests {
    use super::{
        api_code, business_name, column_row, controller_code, domain_model_code,
        domain_service_code, entity_code, form_type, menu_sql, module_name, query_field,
        readme_code, repository_code, rust_type, service_code, snake_case, upper_camel, view_code,
    };
    use crate::domain::models::{
        generator::{DatabaseColumn, GenColumnRow},
        system_admin::KeywordQuery,
    };

    fn column(name: &str, data_type: &str, nullable: &str, sort: i64) -> DatabaseColumn {
        DatabaseColumn {
            column_name: name.to_string(),
            data_type: data_type.to_string(),
            column_type: data_type.to_string(),
            column_comment: None,
            nullable: nullable.to_string(),
            sort,
        }
    }

    fn column_row_of(name: &str, rust_type: &str, comment: Option<&str>) -> GenColumnRow {
        GenColumnRow {
            id: 1,
            col: name.to_string(),
            rust_field: snake_case(name),
            rust_type: rust_type.to_string(),
            jdbc_type: "text".to_string(),
            comment: comment.map(str::to_string),
            insert: true,
            edit: true,
            list: true,
            query: true,
            query_type: "LIKE".to_string(),
            form_type: "文本框".to_string(),
            required: true,
            nullable: false,
        }
    }

    #[test]
    fn naming_helpers_use_snake_case_and_upper_camel() {
        assert_eq!(module_name("sys_user"), "system");
        assert_eq!(module_name("biz_order"), "biz");
        assert_eq!(module_name("account"), "system");
        assert_eq!(business_name("sys_user_role"), "user_role");
        assert_eq!(business_name("account"), "account");
        assert_eq!(upper_camel("biz_order"), "BizOrder");
        assert_eq!(upper_camel(""), "");
        assert_eq!(snake_case("SYS_USER"), "sys_user");
        assert_eq!(snake_case("post__code"), "post_code");
    }

    #[test]
    fn type_mapping_produces_rust_types() {
        assert_eq!(rust_type("bigint"), "i64");
        assert_eq!(rust_type("integer"), "i64");
        assert_eq!(rust_type("int"), "i32");
        assert_eq!(rust_type("decimal"), "f64");
        assert_eq!(rust_type("real"), "f64");
        assert_eq!(rust_type("text"), "String");
        assert_eq!(rust_type("datetime"), "String");
        assert_eq!(form_type("text"), "文本域");
        assert_eq!(form_type("int"), "数字框");
        assert_eq!(form_type("date"), "日期框");
        assert_eq!(form_type("varchar"), "文本框");
    }

    #[test]
    fn query_field_matches_name_code_status_type() {
        assert!(query_field("user_name"));
        assert!(query_field("post_CODE"));
        assert!(query_field("status"));
        assert!(query_field("menu_type"));
        assert!(!query_field("create_time"));
    }

    #[test]
    fn primary_column_is_excluded_from_form_list_and_required() {
        let id = column_row(7, &column("id", "integer", "NO", 1));
        assert_eq!(id.rust_field, "id");
        assert_eq!(id.rust_type, "i64");
        assert_eq!((id.insert, id.edit, id.list), (0, 0, 0));
        assert_eq!(id.required, 0);

        let name = column_row(7, &column("user_name", "varchar", "NO", 2));
        assert_eq!(name.rust_field, "user_name");
        assert_eq!((name.insert, name.edit, name.list), (1, 1, 1));
        assert_eq!(name.query, 1);
        assert_eq!(name.query_type, "LIKE");
        assert_eq!(name.form_type, "文本框");
        assert_eq!(name.required, 1);

        let created = column_row(7, &column("create_time", "datetime", "YES", 3));
        assert_eq!(created.required, 0);
        assert_eq!(created.query, 0);
        assert_eq!(created.query_type, "=");
    }

    #[test]
    fn entity_template_emits_sea_orm_model() {
        let columns = vec![
            column_row_of("id", "i64", Some("主键")),
            column_row_of("amount", "f64", None),
        ];

        assert_eq!(
            entity_code("sys_order", &columns),
            "use sea_orm::entity::prelude::*;\n\n\
             #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]\n\
             #[sea_orm(table_name = \"sys_order\")]\n\
             pub struct Model {\n\
             \x20   /// 主键\n\
             \x20   #[sea_orm(primary_key)]\n\
             \x20   pub id: i64,\n\
             \x20   pub amount: f64,\n\
             }\n\n\
             #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]\n\
             pub enum Relation {}\n\n\
             impl ActiveModelBehavior for ActiveModel {}\n"
        );
    }

    #[test]
    fn domain_templates_declare_row_and_write_command() {
        let columns = vec![
            column_row_of("id", "i64", None),
            column_row_of("post_code", "String", None),
        ];

        let model = domain_model_code("SysPost", &columns);
        assert!(
            model.contains(
                "pub struct SysPostRow {\n    pub id: i64,\n    pub post_code: String,\n}"
            )
        );
        assert!(
            model.contains(
                "pub struct SysPostWriteCommand {\n    pub post_code: Option<String>,\n}"
            ),
            "主键不进写入命令: {model}"
        );

        let use_case = domain_service_code("SysPost", "sys_post", "post");
        assert!(use_case.contains("pub trait SysPostUseCase: Send + Sync {"));
        assert!(use_case.contains("async fn list_post(&self, query: &KeywordQuery)"));

        let repository = repository_code("SysPost", "sys_post", "post", &columns);
        assert!(repository.contains("pub trait SysPostRepository: Send + Sync {"));
        assert!(repository.contains("pub struct SeaOrmSysPostRepository;"));
        assert!(repository.contains("impl SysPostRepository for SeaOrmSysPostRepository {"));
        assert!(repository.contains("SELECT id, post_code FROM sys_post WHERE"));
        assert!(repository.contains("post_code LIKE ?"));

        let service = service_code("SysPost", "sys_post", "post");
        assert!(service.contains("pub struct SysPostService {"));
        assert!(service.contains("impl SysPostUseCase for SysPostService {"));
        assert!(service.contains("todo!(\"按 repositories/sys_post.rs 的 SQL 实现新增\")"));
    }

    #[test]
    fn vue_template_keeps_mustache_placeholders() {
        let code = view_code(
            "sys_order",
            "SysOrder",
            &[column_row_of("order_no", "String", None)],
        );

        assert!(code.contains("const primaryField = 'order_no'"));
        assert!(code.contains("<h1>{{ className }}</h1>"));
        assert!(code.contains("<p>{{ tableName }} · {{ primaryField }}</p>"));
    }

    #[test]
    fn api_controller_and_menu_templates_match_project_layout() {
        let api = api_code("system", "post");
        assert!(api.contains("import type { HttpClient, QueryParams } from './http'"));
        assert!(api.contains("export function createPostApi(client: HttpClient) {"));
        assert!(
            api.contains("client.get<PageResult<Record<string, unknown>>>('/system/post', query)")
        );
        assert!(api.contains("client.post<IdResponse>('/system/post', body)"));

        let controller = controller_code("system", "SysPost", "sys_post", "post");
        assert!(controller.contains("pub fn router() -> Router<Arc<Container>> {"));
        assert!(controller.contains(".route(\"/system/post\", get(list).post(create))"));
        assert!(controller.contains("user.require(\"system:post:list\")?;"));
        assert!(controller.contains("ApiJson(command): ApiJson<SysPostWriteCommand>,"));

        assert_eq!(
            menu_sql("SysPost", "system", "post"),
            "-- SysPost 菜单权限\n\
             INSERT INTO sys_menu (id, parent_id, menu_name, menu_type, path, component, permission, sort, visible, status, create_by, update_by)\n\
             SELECT COALESCE(MAX(id), 0) + 1, 0, 'SysPost管理', 'menu', '/system/post', 'system/post/index', 'system:post:list', 1, 1, 1, 'system', 'system'\n\
             FROM sys_menu;\n"
        );
    }

    #[test]
    fn readme_lists_registration_points() {
        let code = readme_code("system", "SysPost", "sys_post", "post");

        assert!(code.contains("# SysPost 接入步骤"));
        assert!(code.contains("pub mod sys_post;"));
        assert!(code.contains(".merge(controllers::sys_post::router())"));
        assert!(code.contains("`backend/src/middleware/audit.rs` 的 `AUDIT_ROUTES`"));
    }

    #[test]
    fn keyword_query_defaults_to_first_page() {
        let page = super::Page::from_query(Some(&KeywordQuery::default().page)).expect("默认分页");
        assert_eq!(page.page_num(), 1);
        assert_eq!(page.limit(), 10);
    }
}
