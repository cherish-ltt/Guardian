use async_trait::async_trait;

use crate::{
    domain::{
        error::AppError,
        models::system_admin::{DeptNode, DeptUpdate, DeptWriteCommand, IdResponse, StatusQuery},
        services::admin::depts::DeptAdminUseCase,
    },
    infrastructure::repositories::dept::{DeptRepository, SeaOrmDeptRepository},
    services::admin::{AdminService, ancestors_of, default_int, dept_tree, required, trim_to_null},
    util::csv,
};

#[async_trait]
impl DeptAdminUseCase for AdminService {
    async fn list_depts(&self, query: &StatusQuery) -> Result<Vec<DeptNode>, AppError> {
        let rows = SeaOrmDeptRepository
            .list_depts(&self.database, query.keyword.as_deref(), query.status)
            .await?;

        Ok(dept_tree(rows))
    }

    async fn export_depts(
        &self,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<String, AppError> {
        let rows = SeaOrmDeptRepository
            .list_depts(&self.database, keyword, status)
            .await?;
        let data = rows
            .iter()
            .map(|row| {
                vec![
                    Some(row.name.clone()),
                    row.leader.clone(),
                    row.phone.clone(),
                    row.email.clone(),
                    Some(row.sort.to_string()),
                    csv::status_text(Some(row.status)),
                ]
            })
            .collect::<Vec<_>>();

        Ok(csv::to_csv(
            &["部门名称", "负责人", "联系电话", "邮箱", "排序", "状态"],
            &data,
        ))
    }

    async fn create_dept(
        &self,
        command: DeptWriteCommand,
        operator: &str,
    ) -> Result<IdResponse, AppError> {
        let name = required(command.name.clone(), "部门名称不能为空")?;
        let parent_id = command.parent_id.unwrap_or(0);
        let ancestors = self.ancestors_for(parent_id).await?;
        let command = DeptWriteCommand {
            parent_id: Some(parent_id),
            name: Some(name),
            sort: Some(default_int(command.sort, 0)),
            status: Some(default_int(command.status, 1)),
            ..command
        };

        let id = SeaOrmDeptRepository
            .insert_dept(&self.database, &command, &ancestors, operator)
            .await?;

        Ok(IdResponse { id })
    }

    async fn update_dept(
        &self,
        id: i64,
        command: DeptWriteCommand,
        operator: &str,
    ) -> Result<(), AppError> {
        // 请求未带 parentId 时不调整上级链，带了才重新计算。
        let ancestors = match command.parent_id {
            Some(parent_id) => Some(self.ancestors_for(parent_id).await?),
            None => None,
        };
        let command = DeptWriteCommand {
            name: trim_to_null(command.name.clone()),
            ..command
        };

        if SeaOrmDeptRepository
            .update_dept(
                &self.database,
                id,
                &DeptUpdate {
                    command: &command,
                    ancestors: ancestors.as_deref(),
                    operator,
                },
            )
            .await?
            == 0
        {
            return Err(AppError::business("部门不存在或已删除"));
        }

        Ok(())
    }

    async fn delete_dept(&self, id: i64, operator: &str) -> Result<(), AppError> {
        if SeaOrmDeptRepository
            .count_dept_children(&self.database, id)
            .await?
            > 0
        {
            return Err(AppError::business("存在下级部门，不能删除"));
        }
        if SeaOrmDeptRepository
            .count_dept_users(&self.database, id)
            .await?
            > 0
        {
            return Err(AppError::business("部门已关联用户，不能删除"));
        }
        if SeaOrmDeptRepository
            .soft_delete_dept(&self.database, id, operator)
            .await?
            == 0
        {
            return Err(AppError::business("部门不存在或已删除"));
        }

        Ok(())
    }
}

impl AdminService {
    /// 上级部门链：顶层部门为空串；上级不存在时报错，避免写入悬空的 ancestors。
    async fn ancestors_for(&self, parent_id: i64) -> Result<String, AppError> {
        if parent_id == 0 {
            return Ok(String::new());
        }

        let parent_ancestors = SeaOrmDeptRepository
            .dept_ancestors(&self.database, parent_id)
            .await?;

        ancestors_of(parent_ancestors, parent_id)
    }
}
