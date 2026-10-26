use async_trait::async_trait;

use crate::{
    domain::{
        error::AppError,
        models::{
            identity::DataScope,
            pagination::{Page, PageResult},
            system_admin::{IdResponse, UserOptions, UserQuery, UserRow, UserWriteCommand},
        },
        services::admin::users::UserAdminUseCase,
    },
    infrastructure::repositories::{
        account::AccountRepository,
        config::{ConfigRepository, SeaOrmConfigRepository},
        dept::{DeptRepository, SeaOrmDeptRepository},
        post::{PostRepository, SeaOrmPostRepository},
        role::{RoleRepository, SeaOrmRoleRepository},
        user::{SeaOrmUserRepository, UserRepository},
    },
    services::admin::{
        AdminService, default_int, default_string, dept_option_tree, required, trim_to_null,
    },
    util::{csv, password},
};

/// 账号初始密码参数键；用户页只读取、不展示。
const INIT_PASSWORD_KEY: &str = "sys.user.initPassword";
/// 导出上限与 Spring 版一致。
pub(crate) const EXPORT_LIMIT: u32 = 5000;

impl AdminService {
    /// 新增用户未传口令时的回退口令：优先 `sys.user.initPassword`，
    /// 参数未配置（或为空）时才用 `.env` 的 `SEED_ADMIN_PASSWORD`。
    async fn initial_password(&self) -> Result<String, AppError> {
        let configured = SeaOrmConfigRepository
            .config_value(&self.database, INIT_PASSWORD_KEY)
            .await?;

        Ok(trim_to_null(configured).unwrap_or_else(|| self.default_password.clone()))
    }
}

#[async_trait]
impl UserAdminUseCase for AdminService {
    async fn user_options(
        &self,
        can_edit: bool,
        can_set_password: bool,
    ) -> Result<UserOptions, AppError> {
        let depts = self
            .cache
            .get_or_load("dept_options", || async {
                Ok(dept_option_tree(
                    SeaOrmDeptRepository
                        .list_dept_options(&self.database)
                        .await?,
                ))
            })
            .await?;
        let roles = if can_edit {
            SeaOrmRoleRepository
                .list_role_options(&self.database)
                .await?
        } else {
            Vec::new()
        };
        let posts = if can_edit {
            SeaOrmPostRepository
                .list_post_options(&self.database)
                .await?
        } else {
            Vec::new()
        };
        let init_password = if can_set_password {
            SeaOrmConfigRepository
                .config_value(&self.database, INIT_PASSWORD_KEY)
                .await?
                .unwrap_or_default()
        } else {
            String::new()
        };

        Ok(UserOptions {
            depts,
            roles,
            posts,
            init_password,
        })
    }

    async fn list_users(
        &self,
        query: &UserQuery,
        scope: &DataScope,
    ) -> Result<PageResult<UserRow>, AppError> {
        let page = Page::from_query(Some(&query.page))?;
        let total = SeaOrmUserRepository
            .count_users(&self.database, query, scope)
            .await?;
        let rows = SeaOrmUserRepository
            .list_users(&self.database, query, scope, page)
            .await?;

        Ok(PageResult::new(rows, total, page))
    }

    async fn export_users(&self, query: &UserQuery, scope: &DataScope) -> Result<String, AppError> {
        let rows = SeaOrmUserRepository
            .list_users(&self.database, query, scope, Page::export(EXPORT_LIMIT))
            .await?;
        let data = rows
            .iter()
            .map(|row| {
                vec![
                    Some(row.username.clone()),
                    Some(row.nickname.clone()),
                    row.phone.clone(),
                    row.email.clone(),
                    row.dept_name.clone(),
                    Some(row.post_name.clone()),
                    Some(row.role_name.clone()),
                    csv::status_text(Some(row.status)),
                    row.create_time.clone(),
                    row.last_login.clone(),
                ]
            })
            .collect::<Vec<_>>();

        Ok(csv::to_csv(
            &[
                "登录账号",
                "用户昵称",
                "手机号",
                "邮箱",
                "部门",
                "岗位",
                "角色",
                "状态",
                "创建时间",
                "最近登录",
            ],
            &data,
        ))
    }

    async fn create_user(
        &self,
        command: UserWriteCommand,
        operator: &str,
        operator_is_super: bool,
    ) -> Result<IdResponse, AppError> {
        let _cache_write = self.cache.begin_write().await;
        let username = required(command.username.clone(), "登录账号不能为空")?;
        let nickname = required(command.nickname.clone(), "用户昵称不能为空")?;
        if SeaOrmUserRepository
            .count_username(&self.database, &username)
            .await?
            > 0
        {
            return Err(AppError::business("登录账号已存在"));
        }

        // 未传口令时取系统参数 `sys.user.initPassword`；参数未配置才回退到
        // `.env` 的 SEED_ADMIN_PASSWORD，保持与文档、前端提示一致。
        let fallback = self.initial_password().await?;
        let hashed = password::hash_password(&default_string(command.password.clone(), &fallback))?;
        let command = UserWriteCommand {
            username: Some(username),
            nickname: Some(nickname),
            password: Some(hashed),
            status: Some(default_int(command.status, 1)),
            ..command
        };

        let operator = operator.to_string();
        let id = self
            .in_transaction(|transaction| {
                let command = command.clone();
                Box::pin(async move {
                    ensure_relations(transaction, &command).await?;
                    ensure_grant_allowed(transaction, &command, operator_is_super).await?;
                    let id = SeaOrmUserRepository
                        .insert_user(transaction, &command, &operator)
                        .await?;
                    SeaOrmUserRepository
                        .replace_user_roles(
                            transaction,
                            id,
                            command.role_ids.as_deref().unwrap_or_default(),
                        )
                        .await?;
                    SeaOrmUserRepository
                        .replace_user_posts(
                            transaction,
                            id,
                            command.post_ids.as_deref().unwrap_or_default(),
                        )
                        .await?;

                    Ok(id)
                })
            })
            .await?;

        Ok(IdResponse { id })
    }

    async fn update_user(
        &self,
        id: i64,
        command: UserWriteCommand,
        operator: &str,
        operator_is_super: bool,
    ) -> Result<(), AppError> {
        let _cache_write = self.cache.begin_write().await;
        // 昵称为空字符串时按「不修改」处理，与 Spring 的 trimToNull 一致。
        let command = UserWriteCommand {
            nickname: trim_to_null(command.nickname.clone()),
            ..command
        };
        // 编辑接口也能改 `status`，取值校验与 `/status` 端点保持一致：
        // 任意整数都能写进库会让账号进入既非正常也非停用的状态。
        if let Some(status) = command.status
            && status != 0
            && status != 1
        {
            return Err(AppError::business("用户状态只能为正常或停用"));
        }
        if id == 1 && command.status == Some(0) {
            return Err(AppError::business("内置管理员不允许停用"));
        }
        let role_ids = command.role_ids.clone();
        let post_ids = command.post_ids.clone();

        let operator = operator.to_string();
        self.in_transaction(|transaction| {
            let command = command.clone();
            let role_ids = role_ids.clone();
            let post_ids = post_ids.clone();
            Box::pin(async move {
                ensure_relations(transaction, &command).await?;
                ensure_grant_allowed(transaction, &command, operator_is_super).await?;
                if SeaOrmUserRepository
                    .update_user(transaction, id, &command, &operator)
                    .await?
                    == 0
                {
                    return Err(AppError::business("用户不存在或已删除"));
                }
                if let Some(role_ids) = role_ids {
                    SeaOrmUserRepository
                        .replace_user_roles(transaction, id, &role_ids)
                        .await?;
                }
                if let Some(post_ids) = post_ids {
                    SeaOrmUserRepository
                        .replace_user_posts(transaction, id, &post_ids)
                        .await?;
                }

                Ok(())
            })
        })
        .await
    }

    async fn update_user_status(
        &self,
        id: i64,
        status: Option<i64>,
        operator: &str,
    ) -> Result<(), AppError> {
        let _cache_write = self.cache.begin_write().await;
        let Some(status) = status.filter(|status| *status == 0 || *status == 1) else {
            return Err(AppError::business("用户状态只能为正常或停用"));
        };
        if id == 1 && status == 0 {
            return Err(AppError::business("内置管理员不允许停用"));
        }
        if SeaOrmUserRepository
            .update_user_status(&self.database, id, status, operator)
            .await?
            == 0
        {
            return Err(AppError::business("用户不存在或已删除"));
        }

        Ok(())
    }

    async fn reset_user_password(
        &self,
        id: i64,
        password: Option<String>,
        operator: &str,
    ) -> Result<(), AppError> {
        // 不再有默认口令：重置必须显式指定新口令，否则一律拒绝。
        let plain = required(password, "新密码不能为空")?;
        let length = plain.chars().count();
        if !(6..=64).contains(&length) {
            return Err(AppError::business("密码长度必须在6到64位之间"));
        }

        let hashed = password::hash_password(&plain)?;
        let target = SeaOrmUserRepository
            .find_username(&self.database, id)
            .await?;
        let operator = operator.to_string();
        self.in_transaction(|transaction| {
            let hashed = hashed.clone();
            let operator = operator.clone();
            Box::pin(async move {
                if SeaOrmUserRepository
                    .reset_user_password(transaction, id, &hashed, &operator)
                    .await?
                    == 0
                {
                    return Err(AppError::business("用户不存在或已删除"));
                }
                // 重置口令通常发生在「怀疑账号被盗」之后：旧会话必须立即失效。
                // 光删令牌不够——Redis 清理失败或令牌索引缺失时，旧 JWT 仍能验签通过；
                // 递增 auth_version 让 authenticate 的版本校验兜住这种残留。
                AccountRepository::bump_auth_version(transaction, id).await
            })
        })
        .await?;

        // 缓存清理在事务外：数据库版本已生效，Redis 故障不该回滚口令重置。
        if let Some(username) = target
            && let Err(error) = self.auth_store.revoke_user_tokens(&username).await
        {
            tracing::warn!(%error, "auth version bumped; Redis session cleanup deferred to TTL");
        }

        Ok(())
    }

    async fn delete_user(&self, id: i64, operator: &str) -> Result<(), AppError> {
        let _cache_write = self.cache.begin_write().await;
        if id == 1 {
            return Err(AppError::business("内置管理员不允许删除"));
        }
        if SeaOrmUserRepository
            .soft_delete_user(&self.database, id, operator)
            .await?
            == 0
        {
            return Err(AppError::business("用户不存在或已删除"));
        }

        Ok(())
    }
}

/// 角色 / 岗位关联必须先确认目标存在：`replace_*` 是「先删后插」，
/// 传错 ID 不会报错，只会留下指向空记录的关联行。
async fn ensure_relations(
    db: &sea_orm::DatabaseTransaction,
    command: &UserWriteCommand,
) -> Result<(), AppError> {
    if let Some(role_ids) = command.role_ids.as_deref() {
        let found = SeaOrmRoleRepository.count_role_ids(db, role_ids).await?;
        if found as usize != dedup_ids(role_ids).len() {
            return Err(AppError::business("角色不存在或已停用"));
        }
    }
    if let Some(post_ids) = command.post_ids.as_deref() {
        let found = SeaOrmPostRepository.count_post_ids(db, post_ids).await?;
        if found as usize != dedup_ids(post_ids).len() {
            return Err(AppError::business("岗位不存在或已删除"));
        }
    }

    Ok(())
}

/// 授予内置超级管理员角色属于提权：只有超管本人可以，避免「用户编辑」权限
/// 通过 `roleIds` 把自己或新建账号变成超管。
async fn ensure_grant_allowed(
    db: &sea_orm::DatabaseTransaction,
    command: &UserWriteCommand,
    operator_is_super: bool,
) -> Result<(), AppError> {
    if operator_is_super {
        return Ok(());
    }
    let Some(role_ids) = command.role_ids.as_deref() else {
        return Ok(());
    };
    if role_ids.is_empty() {
        return Ok(());
    }
    let Some(super_role_id) = SeaOrmRoleRepository
        .find_role_id_by_code(db, crate::services::admin_auth::SUPER_ADMIN)
        .await?
    else {
        return Ok(());
    };
    if role_ids.contains(&super_role_id) {
        return Err(AppError::forbidden("只有超级管理员可以授予超级管理员角色"));
    }

    Ok(())
}

fn dedup_ids(ids: &[i64]) -> Vec<i64> {
    let mut seen = std::collections::HashSet::new();

    ids.iter().copied().filter(|id| seen.insert(*id)).collect()
}
