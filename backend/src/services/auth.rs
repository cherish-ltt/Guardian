use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{
        constants::DEFAULT_DEVELOPER_ROLE,
        error::AppError,
        models::auth::{CurrentUser, DevLoginCommand},
        services::auth::AuthUseCase,
    },
    infrastructure::services::jwt::JwtService,
};

pub struct AuthService {
    jwt_service: Arc<JwtService>,
}

impl AuthService {
    pub fn new(jwt_service: Arc<JwtService>) -> Self {
        Self { jwt_service }
    }
}

#[async_trait]
impl AuthUseCase for AuthService {
    async fn issue_dev_token(
        &self,
        command: DevLoginCommand,
    ) -> Result<crate::domain::models::auth::AccessToken, AppError> {
        let username = command.username.trim();
        if username.is_empty() {
            return Err(AppError::bad_request("username must not be empty"));
        }

        let user_id = command
            .user_id
            .unwrap_or_else(|| Uuid::now_v7().to_string());
        let roles = if command.roles.is_empty() {
            vec![DEFAULT_DEVELOPER_ROLE.to_string()]
        } else {
            command.roles
        };

        self.jwt_service
            .issue_access_token(&user_id, username, &roles)
            .map_err(AppError::from)
    }

    async fn current_user_from_authorization(
        &self,
        authorization: &str,
    ) -> Result<CurrentUser, AppError> {
        let (scheme, token) = authorization
            .trim()
            .split_once(char::is_whitespace)
            .ok_or_else(|| {
                AppError::unauthorized("Authorization header must use the format 'Bearer <token>'")
            })?;
        if !scheme.eq_ignore_ascii_case("Bearer") || token.trim().is_empty() {
            return Err(AppError::unauthorized(
                "Authorization header must use the format 'Bearer <token>'",
            ));
        }

        let claims = self.jwt_service.verify_access_token(token.trim())?;

        Ok(CurrentUser {
            user_id: claims.sub,
            username: claims.username,
            roles: claims.roles,
        })
    }
}
