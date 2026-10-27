use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use crate::{
    domain::{
        error::AppError,
        models::{
            audit_log::LoginLogEntry,
            identity::{
                AuthUserRow, CaptchaChallenge, ChangePasswordCommand, ClientSource,
                CurrentUserInfo, DataScope, LoginCommand, LoginOutcome, LoginResult, MenuRow,
                RouteNode, VerifyLoginCommand,
            },
        },
        services::admin_auth::AdminAuthUseCase,
    },
    infrastructure::{
        repositories::{
            account::AccountRepository,
            admin_auth::{AdminAuthRepository, SeaOrmAdminAuthRepository},
            config::{ConfigRepository, SeaOrmConfigRepository},
        },
        services::{auth_store::AuthStore, jwt::JwtService},
    },
    util::{captcha, password},
};

mod password_guard;

/// 超管角色编码：不受角色菜单分配限制，拥有全部权限。
pub const SUPER_ADMIN: &str = "super_admin";

/// 登录验证码开关参数键；缺省视为开启。
const CAPTCHA_ENABLED_KEY: &str = "sys.account.captchaEnabled";

/// 后台会话的用例实现：账号来自数据库，会话放在 Redis。
pub struct AdminAuthService {
    database: DatabaseConnection,
    jwt_service: Arc<JwtService>,
    auth_store: Arc<dyn AuthStore>,
    account_service: Arc<super::account::AccountService>,
}

impl AdminAuthService {
    pub fn new(
        database: DatabaseConnection,
        jwt_service: Arc<JwtService>,
        auth_store: Arc<dyn AuthStore>,
        account_service: Arc<super::account::AccountService>,
    ) -> Self {
        Self {
            database,
            jwt_service,
            auth_store,
            account_service,
        }
    }

    async fn finish_login(
        &self,
        user: AuthUserRow,
        version: i64,
        source: &ClientSource,
    ) -> Result<LoginResult, AppError> {
        let roles = self.role_codes(user.id).await?;
        let token = self
            .jwt_service
            .issue_versioned_token(&user.id.to_string(), &user.username, &roles, version)
            .map_err(AppError::from)?;
        self.auth_store
            .save_token(
                &token.access_token,
                &user.username,
                token.expires_in_seconds.unsigned_abs(),
            )
            .await?;

        self.record_login(&user.username, 1, "登录成功", source)
            .await?;
        SeaOrmAdminAuthRepository
            .touch_last_login(&self.database, user.id)
            .await?;

        Ok(LoginResult {
            token: token.access_token,
            expires_in: token.expires_in_seconds,
        })
    }

    /// 写一条登录日志：来源信息由调用方从请求上下文带入。
    async fn record_login(
        &self,
        username: &str,
        status: i64,
        message: &str,
        source: &ClientSource,
    ) -> Result<(), AppError> {
        SeaOrmAdminAuthRepository
            .insert_login_log(
                &self.database,
                &LoginLogEntry {
                    username: username.to_string(),
                    status,
                    message: message.to_string(),
                    source: source.clone(),
                },
            )
            .await
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

    /// 验证码开关：参数缺失时按开启处理，不能因为读不到参数而放宽登录。
    async fn captcha_enabled(&self) -> Result<bool, AppError> {
        let value = SeaOrmConfigRepository
            .config_value(&self.database, CAPTCHA_ENABLED_KEY)
            .await?;

        Ok(!matches!(value.as_deref(), Some("false")))
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

        let user = self.current_user(&claims.username).await?;
        let state = crate::infrastructure::repositories::account::AccountRepository::security(
            &self.database,
            user.user_id,
        )
        .await?;
        if claims.auth_version != state.auth_version {
            return Err(session_expired());
        }
        Ok(user)
    }

    async fn captcha(&self) -> Result<CaptchaChallenge, AppError> {
        if !self.captcha_enabled().await? {
            return Ok(CaptchaChallenge {
                enabled: false,
                uuid: String::new(),
                image: String::new(),
            });
        }

        let uuid = uuid::Uuid::now_v7().simple().to_string();
        let code = captcha::random_code()?;
        self.auth_store.save_captcha(&uuid, &code).await?;

        Ok(CaptchaChallenge {
            enabled: true,
            uuid,
            image: captcha::render_image(&code)?,
        })
    }

    async fn login(
        &self,
        command: LoginCommand,
        source: &ClientSource,
    ) -> Result<LoginOutcome, AppError> {
        if self.captcha_enabled().await? {
            verify_captcha(self.auth_store.as_ref(), &command).await?;
        }

        let Some(user) = SeaOrmAdminAuthRepository
            .find_user_by_username(&self.database, &command.username)
            .await?
        else {
            // 账号不存在时也消耗一次口令校验成本，避免响应时间泄露账号是否存在。
            password_guard::spend_unknown_account_cost()?;
            self.record_login(&command.username, 0, "账号或密码错误", source)
                .await?;
            return Err(AppError::business("账号或密码错误"));
        };

        // 锁定期内不再校验口令：否则锁定只是限速，不是阻断。
        password_guard::check_lock(&self.database, user.id).await?;

        if !password::verify_password(&command.password, &user.password)? {
            password_guard::record(&self.database, user.id, false).await?;
            self.record_login(&command.username, 0, "账号或密码错误", source)
                .await?;
            return Err(AppError::business("账号或密码错误"));
        }
        password_guard::record(&self.database, user.id, true).await?;
        if user.status != 1 {
            self.record_login(&command.username, 0, "账号已停用", source)
                .await?;
            return Err(AppError::business("账号已停用"));
        }

        match self.account_service.begin_login(&user).await? {
            super::account::login::PasswordStep::Complete(version) => Ok(
                LoginOutcome::Authenticated(self.finish_login(user, version, source).await?),
            ),
            super::account::login::PasswordStep::Challenge(challenge) => {
                Ok(LoginOutcome::Challenge {
                    two_factor_required: true,
                    challenge,
                    expires_in: 300,
                })
            },
        }
    }

    async fn verify_login(
        &self,
        command: VerifyLoginCommand,
        source: &ClientSource,
    ) -> Result<LoginResult, AppError> {
        let hash = crate::infrastructure::services::two_factor::recovery_hash(&command.challenge);
        let owner =
            crate::infrastructure::repositories::account::AccountRepository::challenge_owner(
                &self.database,
                &hash,
            )
            .await?;
        match self.account_service.complete_login(command).await {
            Ok((user, version)) => self.finish_login(user, version, source).await,
            Err(error) => {
                if let Some(username) = owner {
                    self.record_login(&username, 0, "二次验证失败", source)
                        .await?;
                }
                Err(error)
            },
        }
    }

    async fn logout(
        &self,
        token: &str,
        username: &str,
        source: &ClientSource,
    ) -> Result<(), AppError> {
        self.auth_store.revoke_token(token).await?;
        self.record_login(username, 1, "退出登录", source).await?;

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
        let user_id = user.id;
        let username = user.username.clone();
        super::support::in_immediate_transaction(&self.database, |transaction| {
            let hashed = hashed.clone();
            let username = username.clone();
            Box::pin(async move {
                let updated = SeaOrmAdminAuthRepository
                    .update_password(transaction, user_id, &username, &hashed)
                    .await?;
                if updated == 0 {
                    return Err(AppError::business("密码修改失败"));
                }
                // 与管理员重置口令同口径：光删 Redis 会话不够，Redis 清理失败或令牌索引
                // 缺失时旧 JWT 仍能验签通过，递增 auth_version 让版本校验兜住残留。
                AccountRepository::bump_auth_version(transaction, user_id).await
            })
        })
        .await?;

        // 会话撤销在事务外：数据库版本已生效，Redis 故障不该回滚口令修改。
        if let Err(error) = self.auth_store.revoke_user_tokens(&user.username).await {
            tracing::warn!(%error, "auth version bumped; Redis session cleanup deferred to TTL");
        }

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
