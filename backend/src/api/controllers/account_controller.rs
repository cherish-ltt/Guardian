use crate::{
    api::{
        done,
        extractors::{admin_user::AdminUser, api_json::ApiJson},
        ok,
    },
    container::Container,
    domain::{
        error::AppError,
        models::account::{
            Enrollment, Profile, RecoveryCodes, SecurityProof, SessionPage, SessionRevocation,
            UpdateProfile,
        },
    },
    response::ApiResponse,
};
use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use std::sync::Arc;

pub fn router() -> Router<Arc<Container>> {
    Router::new()
        .route("/auth/profile", get(profile).put(update_profile))
        .route("/auth/sessions", get(sessions))
        .route("/auth/sessions/{id}/revoke", post(revoke_session))
        .route("/auth/2fa/setup", post(setup))
        .route("/auth/2fa/enable", post(enable))
        .route("/auth/2fa/disable", post(disable))
        .route("/auth/2fa/recovery-codes", post(recovery_codes))
        .route("/system/users/{id}/2fa/reset", post(reset))
}

#[derive(serde::Deserialize)]
pub struct SessionQuery {
    cursor: Option<String>,
}

#[cfg_attr(feature = "docs", utoipa::path(get, path = "/api/v1/auth/sessions", tag = "Auth", params(("cursor" = Option<String>, Query, description = "上页返回的游标，首次省略")), security(("bearer_auth" = [])), responses((status = 200, description = "有效的本人登录会话，不返回凭据"))))]
pub async fn sessions(
    State(c): State<Arc<Container>>,
    user: AdminUser,
    Query(query): Query<SessionQuery>,
) -> Result<ApiResponse<SessionPage>, AppError> {
    let cursor = match query.cursor {
        None => 0,
        Some(value) => {
            if value.is_empty() || !value.bytes().all(|ch| ch.is_ascii_digit()) {
                return Err(AppError::business("cursor 必须是非负整数"));
            }
            value
                .parse::<u64>()
                .map_err(|_| AppError::business("cursor 超出范围"))?
        },
    };
    Ok(ok(c
        .account_service
        .sessions(user.username(), user.token(), cursor)
        .await?))
}

#[cfg_attr(feature = "docs", utoipa::path(post, path = "/api/v1/auth/sessions/{id}/revoke", tag = "Auth", request_body = SecurityProof, params(("id" = String, Path, description = "本人会话标识")), security(("bearer_auth" = [])), responses((status = 200, description = "仅撤销选中的其他会话；revoked=false 表示已失效"))))]
pub async fn revoke_session(
    State(c): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<String>,
    ApiJson(proof): ApiJson<SecurityProof>,
) -> Result<ApiResponse<SessionRevocation>, AppError> {
    Ok(ok(c
        .account_service
        .revoke_session(user.username(), user.token(), &id, proof)
        .await?))
}

#[cfg_attr(feature = "docs", utoipa::path(get, path = "/api/v1/auth/profile", tag = "Auth", security(("bearer_auth" = [])), responses((status = 200, description = "Personal profile"))))]
pub async fn profile(
    State(c): State<Arc<Container>>,
    user: AdminUser,
) -> Result<ApiResponse<Profile>, AppError> {
    Ok(ok(c.account_service.profile(user.username()).await?))
}
#[cfg_attr(feature = "docs", utoipa::path(put, path = "/api/v1/auth/profile", tag = "Auth", request_body = UpdateProfile, security(("bearer_auth" = [])), responses((status = 200, description = "Profile updated"))))]
pub async fn update_profile(
    State(c): State<Arc<Container>>,
    user: AdminUser,
    ApiJson(command): ApiJson<UpdateProfile>,
) -> Result<ApiResponse<()>, AppError> {
    c.account_service
        .update_profile(user.username(), command)
        .await?;
    Ok(done())
}
#[cfg_attr(feature = "docs", utoipa::path(post, path = "/api/v1/auth/2fa/setup", tag = "Auth", request_body = SecurityProof, security(("bearer_auth" = [])), responses((status = 200, description = "Pending enrollment"))))]
pub async fn setup(
    State(c): State<Arc<Container>>,
    user: AdminUser,
    ApiJson(proof): ApiJson<SecurityProof>,
) -> Result<ApiResponse<Enrollment>, AppError> {
    Ok(ok(c.account_service.setup(user.username(), proof).await?))
}
#[cfg_attr(feature = "docs", utoipa::path(post, path = "/api/v1/auth/2fa/enable", tag = "Auth", request_body = SecurityProof, security(("bearer_auth" = [])), responses((status = 200, description = "Enabled; recovery codes shown once; sessions revoked"))))]
pub async fn enable(
    State(c): State<Arc<Container>>,
    user: AdminUser,
    ApiJson(proof): ApiJson<SecurityProof>,
) -> Result<ApiResponse<RecoveryCodes>, AppError> {
    Ok(ok(c.account_service.enable(user.username(), proof).await?))
}
#[cfg_attr(feature = "docs", utoipa::path(post, path = "/api/v1/auth/2fa/disable", tag = "Auth", request_body = SecurityProof, security(("bearer_auth" = [])), responses((status = 200, description = "Disabled; sessions revoked"))))]
pub async fn disable(
    State(c): State<Arc<Container>>,
    user: AdminUser,
    ApiJson(proof): ApiJson<SecurityProof>,
) -> Result<ApiResponse<()>, AppError> {
    c.account_service.disable(user.username(), proof).await?;
    Ok(done())
}
#[cfg_attr(feature = "docs", utoipa::path(post, path = "/api/v1/auth/2fa/recovery-codes", tag = "Auth", request_body = SecurityProof, security(("bearer_auth" = [])), responses((status = 200, description = "Recovery codes replaced; sessions revoked"))))]
pub async fn recovery_codes(
    State(c): State<Arc<Container>>,
    user: AdminUser,
    ApiJson(proof): ApiJson<SecurityProof>,
) -> Result<ApiResponse<RecoveryCodes>, AppError> {
    Ok(ok(c
        .account_service
        .recovery_codes(user.username(), proof)
        .await?))
}
#[cfg_attr(feature = "docs", utoipa::path(post, path = "/api/v1/system/users/{id}/2fa/reset", tag = "Auth", request_body = SecurityProof, security(("bearer_auth" = [])), responses((status = 200, description = "Target 2FA reset; sessions revoked"))))]
pub async fn reset(
    State(c): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
    ApiJson(proof): ApiJson<SecurityProof>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("system:user:reset2fa")?;
    c.account_service.reset(user.username(), id, proof).await?;
    Ok(done())
}
