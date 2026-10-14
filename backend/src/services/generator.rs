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
const ACTIVE_FILE_SUFFIX: &str = "Controller.java";
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
}

#[async_trait]
impl GeneratorUseCase for GeneratorService {
    async fn tables(&self, query: &KeywordQuery) -> Result<PageResult<GenTableRow>, AppError> {
        let page = Page::from_query(Some(&query.page));
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
        let active_file = format!("{}{ACTIVE_FILE_SUFFIX}", table.class_name);
        let files = self.rendered_files(&table).await?;
        let code = files
            .iter()
            .find(|file| file.name == active_file)
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
        let active_file = format!("{}{ACTIVE_FILE_SUFFIX}", table.class_name);
        // 文件名与 Spring 版一致：由默认展开的文件名去掉 Controller 后缀得到。
        let file_name = active_file
            .strip_suffix(ACTIVE_FILE_SUFFIX)
            .map_or_else(|| "code".to_string(), |name| format!("{name}.zip"));

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
        java_field: lower_camel(&column.column_name),
        java_type: java_type(&column.data_type).to_string(),
        jdbc_type: column.data_type.clone(),
        column_comment: column.column_comment.clone(),
        insert: i64::from(!primary_like),
        edit: i64::from(!primary_like),
        list: i64::from(!primary_like),
        query: i64::from(query_field),
        query_type: if query_field { "LIKE" } else { "=" }.to_string(),
        form_type: form_type(&column.data_type).to_string(),
        required: i64::from(column.nullable == "NO" && !primary_like),
        sort: column.sort,
    }
}

/// 七个模板产物：后端四层、前端 api/页面、菜单权限 SQL。
fn render_files(table: &GenTableDetail, columns: &[GenColumnRow]) -> Vec<GenFile> {
    let module = table.module.as_str();
    let class_name = table.class_name.as_str();
    let resource = decapitalize(class_name);
    let table_name = table.table_name.as_str();
    let entries = [
        (
            format!("backend/src/main/java/com/scaffold/{module}/domain/{class_name}.java"),
            entity_code(module, class_name, columns),
        ),
        (
            format!("backend/src/main/java/com/scaffold/{module}/mapper/{class_name}Mapper.java"),
            mapper_code(module, class_name),
        ),
        (
            format!("backend/src/main/java/com/scaffold/{module}/service/{class_name}Service.java"),
            service_code(module, class_name),
        ),
        (
            format!(
                "backend/src/main/java/com/scaffold/{module}/controller/{class_name}Controller.java"
            ),
            controller_code(module, class_name, &resource),
        ),
        (
            format!("frontend/src/api/{module}/{resource}.ts"),
            api_code(module, &resource),
        ),
        (
            format!("frontend/src/views/{module}/{resource}/index.vue"),
            view_code(table_name, class_name, columns),
        ),
        (
            format!("sql/{table_name}_menu.sql"),
            menu_sql(table_name, class_name, module, &resource),
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

/// 实体类：字段注释来自列注释，只在需要时引入 `BigDecimal` / `LocalDateTime`。
fn entity_code(module: &str, class_name: &str, columns: &[GenColumnRow]) -> String {
    let mut fields = String::new();
    let mut imports = String::new();
    let mut needs_big_decimal = false;
    let mut needs_local_date_time = false;
    for column in columns {
        needs_big_decimal |= column.java_type == "BigDecimal";
        needs_local_date_time |= column.java_type == "LocalDateTime";
        fields.push_str(&format!(
            "    /** {} */\n    private {} {};\n\n",
            column.comment.as_deref().unwrap_or_default(),
            column.java_type,
            column.java_field
        ));
    }
    if needs_big_decimal {
        imports.push_str("import java.math.BigDecimal;\n");
    }
    if needs_local_date_time {
        imports.push_str("import java.time.LocalDateTime;\n");
    }
    if !imports.trim().is_empty() {
        imports.push('\n');
    }

    format!(
        "package com.scaffold.{module}.domain;\n\n{imports}\npublic class {class_name} {{\n{fields}}}\n"
    )
}

fn mapper_code(module: &str, class_name: &str) -> String {
    format!(
        "package com.scaffold.{module}.mapper;\n\nimport org.apache.ibatis.annotations.Mapper;\n\n\
         @Mapper\npublic interface {class_name}Mapper {{\n}}\n"
    )
}

fn service_code(module: &str, class_name: &str) -> String {
    format!(
        "package com.scaffold.{module}.service;\n\nimport org.springframework.stereotype.Service;\n\n\
         @Service\npublic class {class_name}Service {{\n}}\n"
    )
}

fn controller_code(module: &str, class_name: &str, resource: &str) -> String {
    format!(
        "package com.scaffold.{module}.controller;\n\n\
         import org.springframework.web.bind.annotation.RequestMapping;\n\
         import org.springframework.web.bind.annotation.RestController;\n\n\
         @RestController\n@RequestMapping(\"/api/{module}/{resource}\")\n\
         public class {class_name}Controller {{\n}}\n"
    )
}

fn api_code(module: &str, resource: &str) -> String {
    format!(
        "import {{ httpClient }} from '@/api/http'\n\n\
         export const {resource}Api = {{\n  \
         list: (query?: Record<string, unknown>) => httpClient.get('/{module}/{resource}', query as any),\n\
         }}\n"
    )
}

fn view_code(table_name: &str, class_name: &str, columns: &[GenColumnRow]) -> String {
    let primary_field = columns
        .iter()
        .find(|column| column.list)
        .map(|column| column.java_field.as_str())
        .unwrap_or(PRIMARY_COLUMN);

    format!(
        "<script setup lang=\"ts\">\nconst tableName = '{table_name}'\nconst className = '{class_name}'\n\
         const primaryField = '{primary_field}'\n</script>\n\n\
         <template>\n  <div>\n    <h1>{{{{ className }}}}</h1>\n    <p>{{{{ tableName }}}} · {{{{ primaryField }}}}</p>\n  </div>\n</template>\n"
    )
}

fn menu_sql(table_name: &str, class_name: &str, module: &str, resource: &str) -> String {
    format!(
        "-- {table_name} 菜单权限\n\
         INSERT INTO sys_menu (id, parent_id, menu_name, menu_type, path, component, permission, sort, visible, status, create_by, update_by)\n\
         SELECT COALESCE(MAX(id), 0) + 1, 0, '{class_name}管理', 'menu', '/{module}/{resource}', '{module}/{resource}/index', '{module}:{resource}:list', 1, 1, 1, 'system', 'system'\n\
         FROM sys_menu;\n"
    )
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

    lower_camel(raw)
}

fn upper_camel(value: &str) -> String {
    let lower = lower_camel(value);

    match lower.chars().next() {
        Some(first) => format!("{}{}", first.to_uppercase(), &lower[first.len_utf8()..]),
        None => lower,
    }
}

fn lower_camel(value: &str) -> String {
    let lowered = value.to_lowercase();
    let mut builder = String::new();
    for part in lowered.split('_').filter(|part| !part.is_empty()) {
        if builder.is_empty() {
            builder.push_str(part);
        } else if let Some(first) = part.chars().next() {
            builder.push_str(&first.to_uppercase().to_string());
            builder.push_str(&part[first.len_utf8()..]);
        }
    }

    builder
}

fn decapitalize(value: &str) -> String {
    match value.chars().next() {
        Some(first) => format!("{}{}", first.to_lowercase(), &value[first.len_utf8()..]),
        None => value.to_string(),
    }
}

fn java_type(data_type: &str) -> &'static str {
    match data_type {
        "bigint" => "Long",
        "int" | "integer" | "smallint" | "tinyint" => "Integer",
        "decimal" | "numeric" => "BigDecimal",
        "datetime" | "timestamp" | "date" | "time" => "LocalDateTime",
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
        api_code, business_name, column_row, controller_code, decapitalize, entity_code, form_type,
        java_type, lower_camel, menu_sql, module_name, query_field, upper_camel, view_code,
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

    #[test]
    fn naming_helpers_follow_spring_rules() {
        assert_eq!(module_name("sys_user"), "system");
        assert_eq!(module_name("biz_order"), "biz");
        assert_eq!(module_name("account"), "system");
        assert_eq!(business_name("sys_user_role"), "userRole");
        assert_eq!(upper_camel("biz_order"), "BizOrder");
        assert_eq!(lower_camel("SYS_USER"), "sysUser");
        assert_eq!(decapitalize("BizOrder"), "bizOrder");
        assert_eq!(upper_camel(""), "");
    }

    #[test]
    fn type_mapping_matches_spring_switches() {
        assert_eq!(java_type("bigint"), "Long");
        assert_eq!(java_type("integer"), "Integer");
        assert_eq!(java_type("decimal"), "BigDecimal");
        assert_eq!(java_type("datetime"), "LocalDateTime");
        assert_eq!(java_type("varchar"), "String");
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
        assert_eq!(id.java_field, "id");
        assert_eq!(id.java_type, "Integer");
        assert_eq!((id.insert, id.edit, id.list), (0, 0, 0));
        assert_eq!(id.required, 0);

        let name = column_row(7, &column("user_name", "varchar", "NO", 2));
        assert_eq!(name.java_field, "userName");
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
    fn entity_template_keeps_spring_layout() {
        let columns = vec![
            GenColumnRow {
                id: 1,
                col: "id".to_string(),
                java_field: "id".to_string(),
                java_type: "Long".to_string(),
                jdbc_type: "bigint".to_string(),
                comment: Some("主键".to_string()),
                insert: false,
                edit: false,
                list: false,
                query: false,
                query_type: "=".to_string(),
                form_type: "数字框".to_string(),
                required: false,
            },
            GenColumnRow {
                id: 2,
                col: "amount".to_string(),
                java_field: "amount".to_string(),
                java_type: "BigDecimal".to_string(),
                jdbc_type: "decimal".to_string(),
                comment: None,
                insert: true,
                edit: true,
                list: true,
                query: false,
                query_type: "=".to_string(),
                form_type: "数字框".to_string(),
                required: false,
            },
        ];

        let code = entity_code("system", "SysOrder", &columns);
        assert_eq!(
            code,
            "package com.scaffold.system.domain;\n\nimport java.math.BigDecimal;\n\n\n\
             public class SysOrder {\n    /** 主键 */\n    private Long id;\n\n    /**  */\n    private BigDecimal amount;\n\n}\n"
        );
    }

    #[test]
    fn vue_template_keeps_mustache_placeholders() {
        let code = view_code(
            "sys_order",
            "SysOrder",
            &[GenColumnRow {
                id: 1,
                col: "order_no".to_string(),
                java_field: "orderNo".to_string(),
                java_type: "String".to_string(),
                jdbc_type: "varchar".to_string(),
                comment: None,
                insert: true,
                edit: true,
                list: true,
                query: true,
                query_type: "LIKE".to_string(),
                form_type: "文本框".to_string(),
                required: true,
            }],
        );

        assert!(code.contains("const primaryField = 'orderNo'"));
        assert!(code.contains("<h1>{{ className }}</h1>"));
        assert!(code.contains("<p>{{ tableName }} · {{ primaryField }}</p>"));
    }

    #[test]
    fn api_controller_and_menu_templates_match_spring_output() {
        assert_eq!(
            api_code("system", "sysOrder"),
            "import { httpClient } from '@/api/http'\n\n\
             export const sysOrderApi = {\n  \
             list: (query?: Record<string, unknown>) => httpClient.get('/system/sysOrder', query as any),\n\
             }\n"
        );
        assert_eq!(
            controller_code("system", "SysOrder", "sysOrder"),
            "package com.scaffold.system.controller;\n\n\
             import org.springframework.web.bind.annotation.RequestMapping;\n\
             import org.springframework.web.bind.annotation.RestController;\n\n\
             @RestController\n@RequestMapping(\"/api/system/sysOrder\")\n\
             public class SysOrderController {\n}\n"
        );
        assert_eq!(
            menu_sql("sys_order", "SysOrder", "system", "sysOrder"),
            "-- sys_order 菜单权限\n\
             INSERT INTO sys_menu (id, parent_id, menu_name, menu_type, path, component, permission, sort, visible, status, create_by, update_by)\n\
             SELECT COALESCE(MAX(id), 0) + 1, 0, 'SysOrder管理', 'menu', '/system/sysOrder', 'system/sysOrder/index', 'system:sysOrder:list', 1, 1, 1, 'system', 'system'\n\
             FROM sys_menu;\n"
        );
    }

    #[test]
    fn keyword_query_defaults_to_first_page() {
        let page = super::Page::from_query(Some(&KeywordQuery::default().page));
        assert_eq!(page.page_num(), 1);
        assert_eq!(page.limit(), 10);
    }
}
