use async_trait::async_trait;

use crate::{
    domain::{
        error::AppError,
        models::system_admin::{IdResponse, MenuNode, MenuWriteCommand, StatusQuery},
        services::admin::menus::MenuAdminUseCase,
    },
    infrastructure::repositories::menu::{MenuRepository, SeaOrmMenuRepository},
    services::admin::{
        AdminService, default_int, default_string, menu_tree, required, trim_to_null,
    },
    util::csv,
};

#[async_trait]
impl MenuAdminUseCase for AdminService {
    async fn list_menus(&self, query: &StatusQuery) -> Result<Vec<MenuNode>, AppError> {
        self.cache
            .get_or_load(&format!("list_menus:{query:?}"), || async {
                let rows = SeaOrmMenuRepository
                    .list_menus(&self.database, query.keyword.as_deref(), query.status)
                    .await?;

                Ok(menu_tree(rows))
            })
            .await
    }

    async fn export_menus(
        &self,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<String, AppError> {
        let rows = SeaOrmMenuRepository
            .list_menus(&self.database, keyword, status)
            .await?;
        let data = rows
            .iter()
            .map(|row| {
                vec![
                    Some(row.name.clone()),
                    Some(row.type_.clone()),
                    row.icon.clone(),
                    row.path.clone(),
                    row.component.clone(),
                    row.permission.clone(),
                    Some(row.sort.to_string()),
                    csv::status_text(Some(row.status)),
                ]
            })
            .collect::<Vec<_>>();

        Ok(csv::to_csv(
            &[
                "菜单名称",
                "类型",
                "图标",
                "路由地址",
                "组件路径",
                "权限标识",
                "排序",
                "状态",
            ],
            &data,
        ))
    }

    async fn create_menu(
        &self,
        command: MenuWriteCommand,
        operator: &str,
    ) -> Result<IdResponse, AppError> {
        let _cache_write = self.cache.begin_write().await;
        let name = required(command.name.clone(), "菜单名称不能为空")?;
        let command = MenuWriteCommand {
            parent_id: Some(command.parent_id.unwrap_or(0)),
            name: Some(name),
            type_: Some(default_string(command.type_.clone(), "menu")),
            sort: Some(default_int(command.sort, 0)),
            visible: Some(default_int(command.visible, 1)),
            status: Some(default_int(command.status, 1)),
            ..command
        };

        let operator = operator.to_string();
        let id = self
            .in_transaction(|transaction| {
                Box::pin(async move {
                    let id = SeaOrmMenuRepository.next_menu_id(transaction).await?;
                    ensure_menu_parent(transaction, id, command.parent_id.unwrap_or(0)).await?;
                    SeaOrmMenuRepository
                        .insert_menu(transaction, id, &command, &operator)
                        .await?;
                    Ok(id)
                })
            })
            .await?;

        Ok(IdResponse { id })
    }

    async fn update_menu(
        &self,
        id: i64,
        command: MenuWriteCommand,
        operator: &str,
    ) -> Result<(), AppError> {
        let _cache_write = self.cache.begin_write().await;
        let command = MenuWriteCommand {
            name: trim_to_null(command.name.clone()),
            type_: trim_to_null(command.type_.clone()),
            ..command
        };
        let operator = operator.to_string();
        self.in_transaction(|transaction| {
            Box::pin(async move {
                if let Some(parent_id) = command.parent_id {
                    ensure_menu_parent(transaction, id, parent_id).await?;
                }
                if SeaOrmMenuRepository
                    .update_menu(transaction, id, &command, &operator)
                    .await?
                    == 0
                {
                    return Err(AppError::business("菜单不存在"));
                }
                Ok(())
            })
        })
        .await
    }

    async fn delete_menu(&self, id: i64, _operator: &str) -> Result<(), AppError> {
        let _cache_write = self.cache.begin_write().await;
        if SeaOrmMenuRepository
            .count_menu_children(&self.database, id)
            .await?
            > 0
        {
            return Err(AppError::business("存在子菜单，不能删除"));
        }
        if SeaOrmMenuRepository
            .count_menu_roles(&self.database, id)
            .await?
            > 0
        {
            return Err(AppError::business("菜单已分配角色，不能删除"));
        }
        if SeaOrmMenuRepository.delete_menu(&self.database, id).await? == 0 {
            return Err(AppError::business("菜单不存在"));
        }

        Ok(())
    }
}

async fn ensure_menu_parent(
    db: &sea_orm::DatabaseTransaction,
    id: i64,
    parent_id: i64,
) -> Result<(), AppError> {
    let menus = SeaOrmMenuRepository.list_menus(db, None, None).await?;
    let parents: std::collections::HashMap<_, _> = menus
        .into_iter()
        .map(|menu| (menu.id, menu.parent_id))
        .collect();
    let mut cursor = parent_id;
    let mut seen = std::collections::HashSet::new();
    while cursor != 0 {
        if cursor == id || !seen.insert(cursor) {
            return Err(AppError::business("上级菜单不能是自身或其下级"));
        }
        cursor = *parents
            .get(&cursor)
            .ok_or_else(|| AppError::business("上级菜单不存在"))?;
    }
    Ok(())
}
