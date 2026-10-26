use std::{ops::Deref, sync::Arc};

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};

use crate::{
    container::Container,
    domain::{
        error::AppError,
        models::identity::{CurrentUserInfo, DataScope},
    },
    services::admin_auth::SUPER_ADMIN,
};

/// 已认证的后台账号：携带会话令牌、角色与权限码，供处理器做权限判定与登出。
#[derive(Debug, Clone)]
pub struct AdminUser {
    info: CurrentUserInfo,
    token: String,
}

impl AdminUser {
    pub fn username(&self) -> &str {
        &self.info.username
    }

    /// 本次请求使用的令牌，登出时用它撤销会话。
    pub fn token(&self) -> &str {
        &self.token
    }

    /// 当前账号信息的副本，直接作为 `/auth/me` 的业务数据。
    pub fn info(&self) -> &CurrentUserInfo {
        &self.info
    }

    /// 是否内置超级管理员：敏感操作（授予超管角色）只允许超管本人执行。
    pub fn is_super_admin(&self) -> bool {
        self.info.roles.iter().any(|role| role == SUPER_ADMIN)
    }

    /// 超管直接放行，其余按授权码判定。
    pub fn has_permission(&self, permission: &str) -> bool {
        self.is_super_admin() || self.info.permissions.iter().any(|code| code == permission)
    }

    pub fn require(&self, permission: &str) -> Result<(), AppError> {
        if self.has_permission(permission) {
            return Ok(());
        }

        Err(AppError::forbidden("没有访问权限"))
    }

    /// 当前账号的数据权限范围。
    pub async fn data_scope(&self, container: &Container) -> Result<DataScope, AppError> {
        container
            .admin_auth_service
            .data_scope(self.username())
            .await
    }
}

impl Deref for AdminUser {
    type Target = CurrentUserInfo;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl FromRequestParts<Arc<Container>> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<Container>,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts).ok_or_else(session_expired)?;
        let info = state.admin_auth_service.authenticate(token).await?;

        Ok(Self {
            info,
            token: token.to_string(),
        })
    }
}

/// 取出 `Authorization: Bearer <token>` 里的令牌；缺失时按会话失效处理。
pub fn bearer_token(parts: &Parts) -> Option<&str> {
    let value = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())?;
    let (scheme, token) = value.split_once(' ')?;

    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }

    let token = token.trim();

    (!token.is_empty()).then_some(token)
}

fn session_expired() -> AppError {
    AppError::session_expired("未认证或登录已过期")
}

#[cfg(test)]
mod tests {
    use axum::http::{Request, header::AUTHORIZATION};

    use super::bearer_token;

    fn parts(value: &str) -> axum::http::request::Parts {
        let request = Request::builder()
            .header(AUTHORIZATION, value)
            .body(())
            .expect("构造请求");

        request.into_parts().0
    }

    #[test]
    fn bearer_token_accepts_any_case_and_padding() {
        assert_eq!(bearer_token(&parts("Bearer abc")), Some("abc"));
        assert_eq!(bearer_token(&parts("bearer  abc  ")), Some("abc"));
        assert_eq!(bearer_token(&parts("BEARER abc")), Some("abc"));
    }

    #[test]
    fn missing_or_malformed_authorization_is_rejected() {
        assert_eq!(bearer_token(&parts("Basic abc")), None);
        assert_eq!(bearer_token(&parts("Bearer")), None);
        assert_eq!(bearer_token(&parts("Bearer   ")), None);

        let request = Request::builder().body(()).expect("构造请求");
        assert_eq!(bearer_token(&request.into_parts().0), None);
    }
}
