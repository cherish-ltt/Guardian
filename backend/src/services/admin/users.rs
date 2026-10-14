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

/// 新增用户未传密码时的初始密码。
const DEFAULT_USER_PASSWORD: &str = "admin123456";
/// 重置密码未传新密码时的默认值。
const DEFAULT_RESET_PASSWORD: &str = "Yy@123456";
/// 账号初始密码参数键；用户页只读取、不展示。
const INIT_PASSWORD_KEY: &str = "sys.user.initPassword";
/// 导出上限与 Spring 版一致。
pub(crate) const EXPORT_LIMIT: u32 = 5000;

#[async_trait]
impl UserAdminUseCase for AdminService {
    async fn user_options(
        &self,
        can_edit: bool,
        can_set_password: bool,
    ) -> Result<UserOptions, AppError> {
        let depts = dept_option_tree(
            SeaOrmDeptRepository
                .list_dept_options(&self.database)
                .await?,
        );
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
        let page = Page::from_query(Some(&query.page));
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
    ) -> Result<IdResponse, AppError> {
        let username = required(command.username.clone(), "登录账号不能为空")?;
        let nickname = required(command.nickname.clone(), "用户昵称不能为空")?;
        if SeaOrmUserRepository
            .count_username(&self.database, &username)
            .await?
            > 0
        {
            return Err(AppError::business("登录账号已存在"));
        }

        let hashed = password::hash_password(&default_string(
            command.password.clone(),
            DEFAULT_USER_PASSWORD,
        ))?;
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
    ) -> Result<(), AppError> {
        // 昵称为空字符串时按「不修改」处理，与 Spring 的 trimToNull 一致。
        let command = UserWriteCommand {
            nickname: trim_to_null(command.nickname.clone()),
            ..command
        };
        let role_ids = command.role_ids.clone();
        let post_ids = command.post_ids.clone();

        let operator = operator.to_string();
        self.in_transaction(|transaction| {
            let command = command.clone();
            let role_ids = role_ids.clone();
            let post_ids = post_ids.clone();
            Box::pin(async move {
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
        let plain = default_string(password, DEFAULT_RESET_PASSWORD);
        let length = plain.chars().count();
        if !(6..=64).contains(&length) {
            return Err(AppError::business("密码长度必须在6到64位之间"));
        }

        let hashed = password::hash_password(&plain)?;
        if SeaOrmUserRepository
            .reset_user_password(&self.database, id, &hashed, operator)
            .await?
            == 0
        {
            return Err(AppError::business("用户不存在或已删除"));
        }

        Ok(())
    }

    async fn delete_user(&self, id: i64, operator: &str) -> Result<(), AppError> {
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
