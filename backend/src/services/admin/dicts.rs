use async_trait::async_trait;

use crate::{
    domain::{
        error::AppError,
        models::{
            pagination::{Page, PageResult},
            system_admin::{
                DictDataRow, DictDataWriteCommand, DictOption, DictTypeRow, DictTypeWriteCommand,
                IdResponse, StatusQuery,
            },
        },
        services::admin::dicts::DictAdminUseCase,
    },
    infrastructure::repositories::dict::{DictRepository, SeaOrmDictRepository},
    services::admin::{AdminService, default_int, required, trim_to_null},
    util::csv,
};

use super::users::EXPORT_LIMIT;

#[async_trait]
impl DictAdminUseCase for AdminService {
    async fn list_dict_types(
        &self,
        query: &StatusQuery,
    ) -> Result<PageResult<DictTypeRow>, AppError> {
        self.cache
            .get_or_load(&format!("list_dict_types:{query:?}"), || async {
                let page = Page::from_query(Some(&query.page));
                let keyword = query.keyword.as_deref();
                let total = SeaOrmDictRepository
                    .count_dict_types(&self.database, keyword, query.status)
                    .await?;
                let rows = SeaOrmDictRepository
                    .list_dict_types(&self.database, keyword, query.status, page)
                    .await?;

                Ok(PageResult::new(rows, total, page))
            })
            .await
    }

    async fn export_dict_types(
        &self,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<String, AppError> {
        let rows = SeaOrmDictRepository
            .list_dict_types(&self.database, keyword, status, Page::export(EXPORT_LIMIT))
            .await?;
        let data = rows
            .iter()
            .map(|row| {
                vec![
                    Some(row.dict_name.clone()),
                    Some(row.dict_type.clone()),
                    csv::status_text(Some(row.status)),
                    row.create_time.clone(),
                    row.remark.clone(),
                ]
            })
            .collect::<Vec<_>>();

        Ok(csv::to_csv(
            &["字典名称", "字典类型", "状态", "创建时间", "备注"],
            &data,
        ))
    }

    async fn create_dict_type(
        &self,
        command: DictTypeWriteCommand,
        operator: &str,
    ) -> Result<IdResponse, AppError> {
        let _cache_write = self.cache.begin_write().await;
        let dict_name = required(command.dict_name.clone(), "字典名称不能为空")?;
        let dict_type = required(command.dict_type.clone(), "字典类型不能为空")?;
        if SeaOrmDictRepository
            .count_dict_type(&self.database, &dict_type)
            .await?
            > 0
        {
            return Err(AppError::business("字典类型已存在"));
        }

        let command = DictTypeWriteCommand {
            dict_name: Some(dict_name),
            dict_type: Some(dict_type),
            status: Some(default_int(command.status, 1)),
            ..command
        };
        let id = SeaOrmDictRepository
            .insert_dict_type(&self.database, &command, operator)
            .await?;

        Ok(IdResponse { id })
    }

    async fn update_dict_type(
        &self,
        id: i64,
        command: DictTypeWriteCommand,
        operator: &str,
    ) -> Result<(), AppError> {
        let _cache_write = self.cache.begin_write().await;
        let command = DictTypeWriteCommand {
            dict_name: trim_to_null(command.dict_name.clone()),
            ..command
        };
        if SeaOrmDictRepository
            .update_dict_type(&self.database, id, &command, operator)
            .await?
            == 0
        {
            return Err(AppError::business("字典类型不存在"));
        }

        Ok(())
    }

    async fn delete_dict_type(&self, id: i64) -> Result<(), AppError> {
        let _cache_write = self.cache.begin_write().await;
        self.in_transaction(|transaction| {
            Box::pin(async move {
                let Some(dict_type) = SeaOrmDictRepository.dict_type_code(transaction, id).await?
                else {
                    return Err(AppError::business("字典类型不存在"));
                };
                if SeaOrmDictRepository
                    .count_dict_data_by_type(transaction, &dict_type)
                    .await?
                    > 0
                {
                    return Err(AppError::business("字典类型存在字典数据，不能删除"));
                }
                SeaOrmDictRepository
                    .delete_dict_type(transaction, id)
                    .await?;

                Ok(())
            })
        })
        .await
    }

    async fn list_dict_data(&self, dict_type: &str) -> Result<Vec<DictDataRow>, AppError> {
        self.cache
            .get_or_load(&format!("list_dict_data:{dict_type}"), || async {
                SeaOrmDictRepository
                    .list_dict_data(&self.database, dict_type)
                    .await
            })
            .await
    }

    async fn dict_options(&self, dict_type: &str) -> Result<Vec<DictOption>, AppError> {
        self.cache
            .get_or_load(&format!("dict_options:{dict_type}"), || async {
                SeaOrmDictRepository
                    .list_dict_options(&self.database, dict_type)
                    .await
            })
            .await
    }

    async fn create_dict_data(
        &self,
        command: DictDataWriteCommand,
        operator: &str,
    ) -> Result<IdResponse, AppError> {
        let _cache_write = self.cache.begin_write().await;
        let dict_type = required(command.dict_type.clone(), "字典类型不能为空")?;
        let label = required(command.label.clone(), "字典标签不能为空")?;
        let value = required(command.value.clone(), "字典值不能为空")?;
        let command = DictDataWriteCommand {
            dict_type: Some(dict_type.clone()),
            label: Some(label),
            value: Some(value),
            sort: Some(default_int(command.sort, 0)),
            status: Some(default_int(command.status, 1)),
            def: Some(command.def.unwrap_or(false)),
            ..command
        };

        let id = self
            .in_transaction(|transaction| {
                let command = command.clone();
                let operator = operator.to_string();
                Box::pin(async move {
                    // 同一字典类型只允许一个默认项：先把旧默认项清掉，再插入新项。
                    if command.def == Some(true) {
                        SeaOrmDictRepository
                            .clear_dict_default(transaction, &dict_type, None)
                            .await?;
                    }
                    SeaOrmDictRepository
                        .insert_dict_data(transaction, &command, &operator)
                        .await
                })
            })
            .await?;

        Ok(IdResponse { id })
    }

    async fn update_dict_data(
        &self,
        id: i64,
        command: DictDataWriteCommand,
        operator: &str,
    ) -> Result<(), AppError> {
        let _cache_write = self.cache.begin_write().await;
        let operator = operator.to_string();
        let command = DictDataWriteCommand {
            dict_type: trim_to_null(command.dict_type.clone()),
            label: trim_to_null(command.label.clone()),
            value: trim_to_null(command.value.clone()),
            ..command
        };

        self.in_transaction(|transaction| {
            let command = command.clone();
            let operator = operator.clone();
            Box::pin(async move {
                let origin = SeaOrmDictRepository
                    .dict_data_origin(transaction, id)
                    .await?
                    .ok_or_else(|| AppError::business("字典数据不存在"))?;

                // 改类型时新类型里可能已经有默认项，按更新后的最终状态收敛：
                // 本行仍是默认项，就清掉最终所属类型里的其它默认项。
                let final_type = command.dict_type.as_deref().unwrap_or(&origin.0);
                let final_default = command.def.unwrap_or(origin.1);
                if final_default {
                    SeaOrmDictRepository
                        .clear_dict_default(transaction, final_type, Some(id))
                        .await?;
                }

                SeaOrmDictRepository
                    .update_dict_data(transaction, id, &command, &operator)
                    .await?;

                Ok(())
            })
        })
        .await
    }

    async fn delete_dict_data(&self, id: i64) -> Result<(), AppError> {
        let _cache_write = self.cache.begin_write().await;
        if SeaOrmDictRepository
            .delete_dict_data(&self.database, id)
            .await?
            == 0
        {
            return Err(AppError::business("字典数据不存在"));
        }

        Ok(())
    }
}
