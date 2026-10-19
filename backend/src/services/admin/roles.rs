use async_trait::async_trait;

use crate::{
    domain::{
        error::AppError,
        models::{
            pagination::{Page, PageResult},
            system_admin::{IdResponse, RoleRow, RoleWriteCommand, StatusQuery},
        },
        services::admin::roles::RoleAdminUseCase,
    },
    infrastructure::repositories::role::{RoleRepository, SeaOrmRoleRepository},
    services::admin::{AdminService, default_int, default_string, required, trim_to_null},
    util::csv,
};

use super::users::EXPORT_LIMIT;

#[async_trait]
impl RoleAdminUseCase for AdminService {
    async fn list_roles(&self, query: &StatusQuery) -> Result<PageResult<RoleRow>, AppError> {
        let page = Page::from_query(Some(&query.page));
        let keyword = query.keyword.as_deref();
        let total = SeaOrmRoleRepository
            .count_roles(&self.database, keyword, query.status)
            .await?;
        let rows = SeaOrmRoleRepository
            .list_roles(&self.database, keyword, query.status, page)
            .await?;

        Ok(PageResult::new(rows, total, page))
    }

    async fn export_roles(
        &self,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<String, AppError> {
        let rows = SeaOrmRoleRepository
            .list_roles(&self.database, keyword, status, Page::export(EXPORT_LIMIT))
            .await?;
        let data = rows
            .iter()
            .map(|row| {
                vec![
                    Some(row.name.clone()),
                    Some(row.code.clone()),
                    Some(row.data_scope.clone()),
                    csv::status_text(Some(row.status)),
                    row.create_time.clone(),
                    row.remark.clone(),
                ]
            })
            .collect::<Vec<_>>();

        Ok(csv::to_csv(
            &[
                "角色名称",
                "权限字符",
                "数据权限",
                "状态",
                "创建时间",
                "备注",
            ],
            &data,
        ))
    }

    async fn role_menus(&self, role_id: i64) -> Result<Vec<i64>, AppError> {
        SeaOrmRoleRepository
            .role_menu_ids(&self.database, role_id)
            .await
    }

    async fn role_depts(&self, role_id: i64) -> Result<Vec<i64>, AppError> {
        SeaOrmRoleRepository
            .role_dept_ids(&self.database, role_id)
            .await
    }

    async fn create_role(
        &self,
        command: RoleWriteCommand,
        operator: &str,
    ) -> Result<IdResponse, AppError> {
        let name = required(command.name.clone(), "角色名称不能为空")?;
        let code = required(command.code.clone(), "角色编码不能为空")?;
        if SeaOrmRoleRepository
            .count_role_code(&self.database, &code)
            .await?
            > 0
        {
            return Err(AppError::business("角色编码已存在"));
        }

        let command = RoleWriteCommand {
            name: Some(name),
            code: Some(code),
            data_scope: Some(default_string(command.data_scope.clone(), "SELF")),
            sort: Some(default_int(command.sort, 0)),
            status: Some(default_int(command.status, 1)),
            ..command
        };
        let menu_ids = command.menu_ids.clone().unwrap_or_default();
        let dept_ids = custom_dept_ids(&command)?;

        let operator = operator.to_string();
        let id = self
            .in_transaction(|transaction| {
                let command = command.clone();
                let menu_ids = menu_ids.clone();
                let dept_ids = dept_ids.clone();
                Box::pin(async move {
                    let id = SeaOrmRoleRepository
                        .insert_role(transaction, &command, &operator)
                        .await?;
                    SeaOrmRoleRepository
                        .replace_role_menus(transaction, id, &menu_ids)
                        .await?;
                    if let Some(dept_ids) = dept_ids {
                        SeaOrmRoleRepository
                            .replace_role_depts(transaction, id, &dept_ids)
                            .await?;
                    }

                    Ok(id)
                })
            })
            .await?;

        Ok(IdResponse { id })
    }

    async fn update_role(
        &self,
        id: i64,
        command: RoleWriteCommand,
        operator: &str,
    ) -> Result<(), AppError> {
        let command = RoleWriteCommand {
            name: trim_to_null(command.name.clone()),
            data_scope: trim_to_null(command.data_scope.clone()),
            ..command
        };
        let menu_ids = command.menu_ids.clone();
        let dept_ids = custom_dept_ids(&command)?;

        let operator = operator.to_string();
        self.in_transaction(|transaction| {
            let command = command.clone();
            let menu_ids = menu_ids.clone();
            let dept_ids = dept_ids.clone();
            Box::pin(async move {
                if SeaOrmRoleRepository
                    .update_role(transaction, id, &command, &operator)
                    .await?
                    == 0
                {
                    return Err(AppError::business("角色不存在、已删除或为内置角色"));
                }
                if let Some(menu_ids) = menu_ids {
                    SeaOrmRoleRepository
                        .replace_role_menus(transaction, id, &menu_ids)
                        .await?;
                }
                if let Some(dept_ids) = dept_ids {
                    SeaOrmRoleRepository
                        .replace_role_depts(transaction, id, &dept_ids)
                        .await?;
                }

                Ok(())
            })
        })
        .await
    }

    async fn delete_role(&self, id: i64, operator: &str) -> Result<(), AppError> {
        if SeaOrmRoleRepository
            .count_role_users(&self.database, id)
            .await?
            > 0
        {
            return Err(AppError::business("角色已分配用户，不能删除"));
        }

        let operator = operator.to_string();
        self.in_transaction(|transaction| {
            Box::pin(async move {
                SeaOrmRoleRepository
                    .delete_role_menus(transaction, id)
                    .await?;
                SeaOrmRoleRepository
                    .replace_role_depts(transaction, id, &[])
                    .await?;
                if SeaOrmRoleRepository
                    .soft_delete_role(transaction, id, &operator)
                    .await?
                    == 0
                {
                    return Err(AppError::business("角色不存在、已删除或为内置角色"));
                }

                Ok(())
            })
        })
        .await
    }
}

/// `CUSTOM` 数据范围必须带部门列表，否则静默退化成「仅本人」。
///
/// 非 `CUSTOM` 的角色忽略 `deptIds` 并清空既有范围，避免切换范围后残留旧数据。
fn custom_dept_ids(command: &RoleWriteCommand) -> Result<Option<Vec<i64>>, AppError> {
    match command.data_scope.as_deref() {
        Some("CUSTOM") => {
            let dept_ids = command.dept_ids.clone().unwrap_or_default();
            if dept_ids.is_empty() {
                return Err(AppError::business("自定义数据权限必须选择部门"));
            }

            Ok(Some(dept_ids))
        },
        Some(_) => Ok(Some(Vec::new())),
        None => Ok(None),
    }
}
