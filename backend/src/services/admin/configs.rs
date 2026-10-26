use async_trait::async_trait;

use crate::{
    domain::{
        error::AppError,
        models::{
            pagination::{Page, PageResult},
            system_admin::{ConfigRow, ConfigWriteCommand, IdResponse, KeywordQuery},
        },
        services::admin::configs::ConfigAdminUseCase,
    },
    infrastructure::repositories::config::{ConfigRepository, SeaOrmConfigRepository},
    services::admin::{AdminService, default_string, required, trim_to_null},
    util::csv,
};

use super::users::EXPORT_LIMIT;

#[async_trait]
impl ConfigAdminUseCase for AdminService {
    async fn list_configs(&self, query: &KeywordQuery) -> Result<PageResult<ConfigRow>, AppError> {
        self.cache
            .get_or_load(&format!("list_configs:{query:?}"), || async {
                let page = Page::from_query(Some(&query.page))?;
                let keyword = query.keyword.as_deref();
                let total = SeaOrmConfigRepository
                    .count_configs(&self.database, keyword)
                    .await?;
                let rows = SeaOrmConfigRepository
                    .list_configs(&self.database, keyword, page)
                    .await?;

                Ok(PageResult::new(rows, total, page))
            })
            .await
    }

    async fn export_configs(&self, keyword: Option<&str>) -> Result<String, AppError> {
        let rows = SeaOrmConfigRepository
            .list_configs(&self.database, keyword, Page::export(EXPORT_LIMIT))
            .await?;
        let data = rows
            .iter()
            .map(|row| {
                vec![
                    Some(row.name.clone()),
                    Some(row.key.clone()),
                    Some(row.value.clone()),
                    Some(row.type_.clone()),
                    row.create_time.clone(),
                    row.remark.clone(),
                ]
            })
            .collect::<Vec<_>>();

        Ok(csv::to_csv(
            &[
                "参数名称",
                "参数键名",
                "参数键值",
                "系统内置",
                "创建时间",
                "备注",
            ],
            &data,
        ))
    }

    async fn create_config(
        &self,
        command: ConfigWriteCommand,
        operator: &str,
    ) -> Result<IdResponse, AppError> {
        let _cache_write = self.cache.begin_write().await;
        let name = required(command.name.clone(), "参数名称不能为空")?;
        let key = required(command.key.clone(), "参数键名不能为空")?;
        let value = required(command.value.clone(), "参数键值不能为空")?;
        if SeaOrmConfigRepository
            .count_config_key(&self.database, &key)
            .await?
            > 0
        {
            return Err(AppError::business("参数键名已存在"));
        }

        let command = ConfigWriteCommand {
            name: Some(name),
            key: Some(key),
            value: Some(value),
            config_type: Some(default_string(command.config_type.clone(), "N")),
            builtin: Some(command.builtin.unwrap_or(false)),
            ..command
        };
        let id = SeaOrmConfigRepository
            .insert_config(&self.database, &command, operator)
            .await?;

        Ok(IdResponse { id })
    }

    async fn update_config(
        &self,
        id: i64,
        command: ConfigWriteCommand,
        operator: &str,
    ) -> Result<(), AppError> {
        let _cache_write = self.cache.begin_write().await;
        let command = ConfigWriteCommand {
            name: trim_to_null(command.name.clone()),
            value: trim_to_null(command.value.clone()),
            config_type: trim_to_null(command.config_type.clone()),
            ..command
        };
        if SeaOrmConfigRepository
            .update_config(&self.database, id, &command, operator)
            .await?
            == 0
        {
            return Err(AppError::business("参数配置不存在或为内置参数"));
        }

        Ok(())
    }

    async fn delete_config(&self, id: i64) -> Result<(), AppError> {
        let _cache_write = self.cache.begin_write().await;
        if SeaOrmConfigRepository
            .delete_config(&self.database, id)
            .await?
            == 0
        {
            return Err(AppError::business("参数配置不存在或为内置参数"));
        }

        Ok(())
    }
}
