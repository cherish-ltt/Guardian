use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use crate::{
    domain::{
        error::AppError,
        models::identity::{
            AuthUserRow, CaptchaChallenge, ChangePasswordCommand, CurrentUserInfo, DataScope,
            LoginCommand, LoginResult, MenuRow, RouteNode,
        },
        services::admin_auth::AdminAuthUseCase,
    },
    infrastructure::{
        repositories::admin_auth::{AdminAuthRepository, SeaOrmAdminAuthRepository},
        services::{auth_store::AuthStore, jwt::JwtService},
    },
    util::password,
};

/// 超管角色编码：不受角色菜单分配限制，拥有全部权限。
pub const SUPER_ADMIN: &str = "super_admin";

/// 验证码字符集：去掉了容易混淆的 I / O / 0 / 1，与 Spring 版一致。
const CAPTCHA_CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const CAPTCHA_LENGTH: usize = 4;

/// 后台会话的用例实现：账号来自数据库，会话放在 Redis。
pub struct AdminAuthService {
    database: DatabaseConnection,
    jwt_service: Arc<JwtService>,
    auth_store: Arc<dyn AuthStore>,
}

impl AdminAuthService {
    pub fn new(
        database: DatabaseConnection,
        jwt_service: Arc<JwtService>,
        auth_store: Arc<dyn AuthStore>,
    ) -> Self {
        Self {
            database,
            jwt_service,
            auth_store,
        }
    }

    /// 账号快照；不存在或已停用一律按会话失效处理。
    async fn active_user(&self, username: &str) -> Result<AuthUserRow, AppError> {
        let user = SeaOrmAdminAuthRepository
            .find_user_by_username(&self.database, username)
            .await?
            .ok_or_else(session_expired)?;

        if user.status != 1 {
            return Err(session_expired());
        }

        Ok(user)
    }

    async fn role_codes(&self, user_id: i64) -> Result<Vec<String>, AppError> {
        SeaOrmAdminAuthRepository
            .role_codes(&self.database, user_id)
            .await
    }

    /// 超管直接给全部权限码，其余按角色授权取。
    async fn permission_codes(&self, user: &AuthUserRow) -> Result<Vec<String>, AppError> {
        let roles = self.role_codes(user.id).await?;
        if roles.iter().any(|code| code == SUPER_ADMIN) {
            return SeaOrmAdminAuthRepository
                .all_permissions(&self.database)
                .await;
        }

        SeaOrmAdminAuthRepository
            .permissions(&self.database, user.id)
            .await
    }

    async fn route_menus(&self, user: &AuthUserRow) -> Result<Vec<MenuRow>, AppError> {
        let roles = self.role_codes(user.id).await?;
        if roles.iter().any(|code| code == SUPER_ADMIN) {
            return SeaOrmAdminAuthRepository
                .all_route_menus(&self.database)
                .await;
        }

        SeaOrmAdminAuthRepository
            .route_menus(&self.database, user.id)
            .await
    }
}

#[async_trait]
impl AdminAuthUseCase for AdminAuthService {
    async fn authenticate(&self, token: &str) -> Result<CurrentUserInfo, AppError> {
        let claims = self
            .jwt_service
            .verify_access_token(token)
            .map_err(|_| session_expired())?;
        // JWT 只是凭据的一半：退出登录会把缓存里的令牌删掉，旧 JWT 必须立即失效。
        match self.auth_store.token_owner(token).await? {
            Some(owner) if owner == claims.username => {},
            _ => return Err(session_expired()),
        }

        self.current_user(&claims.username).await
    }

    async fn captcha(&self) -> Result<CaptchaChallenge, AppError> {
        let uuid = uuid::Uuid::now_v7().simple().to_string();
        let code = random_captcha_code();
        self.auth_store.save_captcha(&uuid, &code).await?;

        Ok(CaptchaChallenge { uuid, code })
    }

    async fn login(&self, command: LoginCommand) -> Result<LoginResult, AppError> {
        verify_captcha(self.auth_store.as_ref(), &command).await?;

        let Some(user) = SeaOrmAdminAuthRepository
            .find_user_by_username(&self.database, &command.username)
            .await?
        else {
            SeaOrmAdminAuthRepository
                .insert_login_log(&self.database, &command.username, 0, "账号或密码错误")
                .await?;
            return Err(AppError::business("账号或密码错误"));
        };

        if !password::verify_password(&command.password, &user.password)? {
            SeaOrmAdminAuthRepository
                .insert_login_log(&self.database, &command.username, 0, "账号或密码错误")
                .await?;
            return Err(AppError::business("账号或密码错误"));
        }
        if user.status != 1 {
            SeaOrmAdminAuthRepository
                .insert_login_log(&self.database, &command.username, 0, "账号已停用")
                .await?;
            return Err(AppError::business("账号已停用"));
        }

        let roles = self.role_codes(user.id).await?;
        let token = self
            .jwt_service
            .issue_access_token(&user.id.to_string(), &user.username, &roles)
            .map_err(AppError::from)?;
        self.auth_store
            .save_token(
                &token.access_token,
                &user.username,
                token.expires_in_seconds.unsigned_abs(),
            )
            .await?;

        SeaOrmAdminAuthRepository
            .insert_login_log(&self.database, &user.username, 1, "登录成功")
            .await?;
        SeaOrmAdminAuthRepository
            .touch_last_login(&self.database, user.id)
            .await?;

        Ok(LoginResult {
            token: token.access_token,
            expires_in: token.expires_in_seconds,
        })
    }

    async fn logout(&self, token: &str, username: &str) -> Result<(), AppError> {
        self.auth_store.revoke_token(token).await?;
        SeaOrmAdminAuthRepository
            .insert_login_log(&self.database, username, 1, "退出登录")
            .await?;

        Ok(())
    }

    async fn current_user(&self, username: &str) -> Result<CurrentUserInfo, AppError> {
        let user = self.active_user(username).await?;
        let roles = self.role_codes(user.id).await?;
        let permissions = self.permission_codes(&user).await?;

        Ok(CurrentUserInfo {
            user_id: user.id,
            username: user.username,
            nickname: user.nickname,
            dept_id: user.dept_id,
            dept_name: user.dept_name,
            roles,
            permissions,
        })
    }

    async fn routes(&self, username: &str) -> Result<Vec<RouteNode>, AppError> {
        let user = self.active_user(username).await?;
        let menus = self.route_menus(&user).await?;

        Ok(build_route_tree(menus))
    }

    async fn change_password(
        &self,
        username: &str,
        command: ChangePasswordCommand,
    ) -> Result<(), AppError> {
        let length = command.new_password.chars().count();
        if !(6..=64).contains(&length) {
            return Err(AppError::business("newPassword 长度必须在6到64位之间"));
        }

        let user = self.active_user(username).await?;
        if !password::verify_password(&command.old_password, &user.password)? {
            return Err(AppError::business("旧密码不正确"));
        }
        if password::verify_password(&command.new_password, &user.password)? {
            return Err(AppError::business("新密码不能与旧密码一致"));
        }

        let hashed = password::hash_password(&command.new_password)?;
        let updated = SeaOrmAdminAuthRepository
            .update_password(&self.database, user.id, &user.username, &hashed)
            .await?;
        if updated == 0 {
            return Err(AppError::business("密码修改失败"));
        }

        // 改密码后旧凭证必须失效：撤销该账号已签发的全部会话。
        self.auth_store.revoke_user_tokens(&user.username).await?;

        Ok(())
    }

    async fn data_scope(&self, username: &str) -> Result<DataScope, AppError> {
        let repository = SeaOrmAdminAuthRepository;
        let Some((user_id, dept_id)) = repository
            .active_user_profile(&self.database, username)
            .await?
        else {
            return Ok(DataScope::self_only(None));
        };

        let roles = repository
            .active_role_scopes(&self.database, user_id)
            .await?;
        if roles.iter().any(|(_, scope)| scope == "ALL") {
            return Ok(DataScope::all());
        }

        let mut dept_ids: Vec<i64> = Vec::new();
        for (role_id, scope) in roles {
            match scope.as_str() {
                "DEPT" => dept_ids.extend(dept_id),
                "DEPT_AND_CHILD" => {
                    if let Some(dept_id) = dept_id {
                        dept_ids.extend(
                            repository
                                .dept_and_child_ids(&self.database, dept_id)
                                .await?,
                        );
                    }
                },
                "CUSTOM" => {
                    dept_ids.extend(repository.role_dept_ids(&self.database, role_id).await?)
                },
                _ => {},
            }
        }

        Ok(DataScope {
            all: false,
            user_id: Some(user_id),
            dept_ids: dedup(dept_ids),
        })
    }
}

fn session_expired() -> AppError {
    AppError::session_expired("未认证或登录已过期")
}

async fn verify_captcha(store: &dyn AuthStore, command: &LoginCommand) -> Result<(), AppError> {
    let (Some(uuid), Some(code)) = (command.captcha_uuid.as_deref(), command.captcha.as_deref())
    else {
        return Err(AppError::business("验证码错误或已过期"));
    };

    let stored = store.take_captcha(uuid).await?;

    match stored {
        Some(stored) if stored.eq_ignore_ascii_case(code) => Ok(()),
        _ => Err(AppError::business("验证码错误或已过期")),
    }
}

/// 验证码取 UUID v7 的随机段字节映射到字符集，不额外引入随机数依赖。
fn random_captcha_code() -> String {
    let bytes = uuid::Uuid::now_v7().into_bytes();

    (0..CAPTCHA_LENGTH)
        .map(|index| CAPTCHA_CHARS[usize::from(bytes[6 + index]) % CAPTCHA_CHARS.len()] as char)
        .collect()
}

fn dedup(values: Vec<i64>) -> Vec<i64> {
    let mut seen = std::collections::HashSet::new();

    values
        .into_iter()
        .filter(|value| seen.insert(*value))
        .collect()
}

/// 菜单按 `(parentId, sort, id)` 排序后拼成树；父节点不在结果集里的作为根。
pub fn build_route_tree(mut menus: Vec<MenuRow>) -> Vec<RouteNode> {
    menus.sort_by_key(|menu| (menu.parent_id, menu.sort, menu.id));
    let ids: std::collections::HashSet<i64> = menus.iter().map(|menu| menu.id).collect();

    let mut children: std::collections::HashMap<i64, Vec<RouteNode>> =
        std::collections::HashMap::new();
    let mut roots: Vec<RouteNode> = Vec::new();
    for menu in menus {
        let parent_id = menu.parent_id;
        let node = RouteNode::leaf(menu);
        if parent_id != node.id && ids.contains(&parent_id) {
            children.entry(parent_id).or_default().push(node);
        } else {
            roots.push(node);
        }
    }

    for root in &mut roots {
        attach_children(root, &mut children);
    }

    roots
}

fn attach_children(
    node: &mut RouteNode,
    children: &mut std::collections::HashMap<i64, Vec<RouteNode>>,
) {
    let Some(mut nested) = children.remove(&node.id) else {
        return;
    };
    for child in &mut nested {
        attach_children(child, children);
    }
    node.children = nested;
}
