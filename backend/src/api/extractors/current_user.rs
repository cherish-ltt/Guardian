use std::{ops::Deref, sync::Arc};

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};

use crate::{
    container::Container, domain::models::auth::CurrentUser as DomainCurrentUser, error::AppError,
};

#[derive(Debug, Clone)]
pub struct CurrentUser(DomainCurrentUser);

impl CurrentUser {
    pub fn into_inner(self) -> DomainCurrentUser {
        self.0
    }
}

impl Deref for CurrentUser {
    type Target = DomainCurrentUser;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequestParts<Arc<Container>> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<Container>,
    ) -> Result<Self, Self::Rejection> {
        let authorization = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AppError::unauthorized("missing Authorization header"))?;

        let current_user = state
            .auth_service
            .current_user_from_authorization(authorization)
            .await?;

        // 后台 JWT 的 subject 是数据库整数 ID，不能借脚手架入口绕过账号状态校验。
        if current_user.user_id.parse::<i64>().is_ok() {
            super::admin_user::AdminUser::from_request_parts(parts, state).await?;
        }

        Ok(Self(current_user))
    }
}
