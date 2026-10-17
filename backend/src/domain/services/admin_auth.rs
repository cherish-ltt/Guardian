use async_trait::async_trait;

use crate::domain::{
    error::AppError,
    models::identity::{
        CaptchaChallenge, ChangePasswordCommand, ClientSource, CurrentUserInfo, DataScope,
        LoginCommand, LoginOutcome, LoginResult, RouteNode, VerifyLoginCommand,
    },
};

/// 后台会话用例：验证码、登录、退出、当前用户、动态路由与数据权限。
///
/// 实现方（`services/admin_auth.rs`）负责把账号状态、Redis 会话与数据权限串起来。
#[async_trait]
pub trait AdminAuthUseCase: Send + Sync {
    /// 校验会话：JWT 与缓存里的令牌必须同时有效（与 Spring 版一致），
    /// 且账号存在、未停用。
    async fn authenticate(&self, token: &str) -> Result<CurrentUserInfo, AppError>;

    async fn captcha(&self) -> Result<CaptchaChallenge, AppError>;

    async fn login(
        &self,
        command: LoginCommand,
        source: &ClientSource,
    ) -> Result<LoginOutcome, AppError>;

    async fn verify_login(
        &self,
        command: VerifyLoginCommand,
        source: &ClientSource,
    ) -> Result<LoginResult, AppError>;

    async fn logout(
        &self,
        token: &str,
        username: &str,
        source: &ClientSource,
    ) -> Result<(), AppError>;

    async fn current_user(&self, username: &str) -> Result<CurrentUserInfo, AppError>;

    async fn routes(&self, username: &str) -> Result<Vec<RouteNode>, AppError>;

    async fn change_password(
        &self,
        username: &str,
        command: ChangePasswordCommand,
    ) -> Result<(), AppError>;

    /// 按当前账号的角色数据范围计算查询边界。
    async fn data_scope(&self, username: &str) -> Result<DataScope, AppError>;
}
