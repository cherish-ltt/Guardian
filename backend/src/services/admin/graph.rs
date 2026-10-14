use async_trait::async_trait;

use crate::{
    domain::{
        error::AppError,
        models::{
            identity::DataScope,
            pagination::Page,
            system_admin::{GraphMenu, GraphRole, PermissionGraph},
        },
        services::admin::graph::GraphAdminUseCase,
    },
    infrastructure::repositories::{
        menu::{MenuRepository, SeaOrmMenuRepository},
        permission_graph::{PermissionGraphRepository, SeaOrmPermissionGraphRepository},
        role::{RoleRepository, SeaOrmRoleRepository},
    },
    services::admin::AdminService,
};

use super::users::EXPORT_LIMIT;

#[async_trait]
impl GraphAdminUseCase for AdminService {
    /// 权限关系图：角色、菜单与「用户-角色」关联一次返回。
    /// 用户节点只对有用户查询权限的账号返回，并遵守数据权限。
    async fn permission_graph(
        &self,
        scope: &DataScope,
        can_see_users: bool,
    ) -> Result<PermissionGraph, AppError> {
        let roles = SeaOrmRoleRepository
            .list_roles(&self.database, None, None, Page::export(EXPORT_LIMIT))
            .await?
            .into_iter()
            .map(|row| GraphRole {
                id: row.id,
                name: row.name,
                code: row.code,
                data_scope: row.data_scope,
                status: row.status,
                builtin: row.builtin,
                user_count: row.user_count,
                menu_ids: row.menu_ids,
            })
            .collect::<Vec<_>>();
        let menus = SeaOrmMenuRepository
            .list_menus(&self.database, None, None)
            .await?
            .into_iter()
            .map(|row| GraphMenu {
                id: row.id,
                parent_id: row.parent_id,
                name: row.name,
                type_: row.type_,
                icon: row.icon,
                path: row.path,
                permission: row.permission,
                sort: row.sort,
                visible: row.visible,
                status: row.status,
            })
            .collect::<Vec<_>>();

        let users = if can_see_users {
            SeaOrmPermissionGraphRepository
                .graph_users(&self.database, scope)
                .await?
        } else {
            Vec::new()
        };
        let user_roles = if can_see_users {
            let visible = users
                .iter()
                .map(|user| user.id)
                .collect::<std::collections::HashSet<_>>();
            SeaOrmPermissionGraphRepository
                .user_role_links(&self.database)
                .await?
                .into_iter()
                .filter(|link| visible.contains(&link.user_id))
                .map(|link| (link.user_id, link.role_id))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        Ok(PermissionGraph {
            users,
            roles,
            menus,
            user_roles,
            users_visible: can_see_users,
        })
    }
}
