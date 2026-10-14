use async_trait::async_trait;

use crate::domain::{
    error::AppError,
    models::{identity::DataScope, system_admin::PermissionGraph},
};

/// 权限关系图用例：一次返回角色、菜单与「用户-角色」关联。
#[async_trait]
pub trait GraphAdminUseCase: Send + Sync {
    async fn permission_graph(
        &self,
        scope: &DataScope,
        can_see_users: bool,
    ) -> Result<PermissionGraph, AppError>;
}
