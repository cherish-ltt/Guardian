use anyhow::{Result, anyhow};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, NotSet, QueryFilter, Set,
};

use crate::dto::{
    LoginRequest, LoginResponse, RefreshTokenResponse, TwoFaSetupResponse, TwoFaVerifyResponse,
};
use crate::entities::{admins, token_blacklist};
use crate::response::Response;
use crate::router::AppState;
use crate::utils::{create_token_pair, get_jti, refresh_access_token, verify_password};

pub async fn login_service(
    state: AppState,
    payload: LoginRequest,
) -> Result<Response<LoginResponse>> {
    let now = chrono::Utc::now();
    let admin = admins::Entity::find()
        .filter(admins::Column::Username.eq(&payload.username))
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("用户名或密码错误"))?;

    if let Some(status) = admin.status {
        if status != 1 {
            return Ok(Response::failed("账户已被禁用".to_string()));
        }
    }

    if let Some(locked_until) = admin.locked_until {
        if locked_until > chrono::Utc::now() {
            return Ok(Response::failed("账户已被锁定，请稍后重试".to_string()));
        }
    }

    if !verify_password(&payload.password, &admin.password_hash) {
        let login_attempts = admin.login_attempts.unwrap_or(0) + 1;
        let mut admin_model: admins::ActiveModel = admin.into_active_model();

        if login_attempts >= 5 {
            let locked_until = chrono::Utc::now() + chrono::Duration::minutes(15);
            admin_model.locked_until = Set(Some(locked_until.into()));
            admin_model.login_attempts = Set(Some(login_attempts));
            admin_model.update(&state.conn).await?;
            return Ok(Response::failed(
                "密码错误次数过多，账户已被锁定15分钟".to_string(),
            ));
        }

        admin_model.login_attempts = Set(Some(login_attempts));
        admin_model.update(&state.conn).await?;
        return Ok(Response::failed("用户名或密码错误".to_string()));
    }

    if admin.two_fa_secret.is_some() {
        if payload.two_fa_code.is_none() {
            return Ok(Response::failed("请输入2FA验证码".to_string()));
        }
    }

    let token_pair = create_token_pair(
        admin.id,
        admin.username.clone(),
        admin.is_super_admin.unwrap_or(false),
    )?;

    let mut admin_model: admins::ActiveModel = admin.into_active_model();
    admin_model.last_login_at = Set(Some(now.into()));
    admin_model.login_attempts = Set(Some(0));
    admin_model.locked_until = Set(None);
    admin_model.update(&state.conn).await?;

    Ok(Response::ok_data(LoginResponse {
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        expires_in: token_pair.expires_in,
    }))
}

pub async fn logout_service(state: AppState, refresh_token: String) -> Result<Response<()>> {
    let jti = get_jti(&refresh_token)?;

    let claims = crate::utils::verify_token(&refresh_token)?;

    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);

    let blacklist = token_blacklist::ActiveModel {
        id: NotSet,
        token_id: Set(jti),
        expires_at: Set(expires_at.into()),
        created_at: NotSet,
    };

    blacklist.insert(&state.conn).await?;

    Ok(Response::quick_ok())
}

pub async fn refresh_token_service(
    state: AppState,
    refresh_token: String,
) -> Result<Response<RefreshTokenResponse>> {
    let jti = get_jti(&refresh_token)?;

    let is_blacklisted = token_blacklist::Entity::find()
        .filter(token_blacklist::Column::TokenId.eq(&jti))
        .filter(token_blacklist::Column::ExpiresAt.gt(chrono::Utc::now()))
        .one(&state.conn)
        .await?;

    if is_blacklisted.is_some() {
        return Ok(Response::failed("Refresh token已失效".to_string()));
    }

    let access_token = refresh_access_token(&refresh_token)?;

    Ok(Response::ok_data(RefreshTokenResponse {
        access_token: access_token.access_token,
        expires_in: access_token.expires_in,
    }))
}

pub async fn setup_2fa_service(_state: AppState) -> Result<Response<TwoFaSetupResponse>> {
    Ok(Response::ok_data(TwoFaSetupResponse {
        secret: "placeholder_secret".to_string(),
        qr_code_url: "https://example.com/qr".to_string(),
        backup_codes: vec!["code1".to_string(), "code2".to_string()],
    }))
}

pub async fn verify_2fa_service(
    _state: AppState,
    _code: String,
) -> Result<Response<TwoFaVerifyResponse>> {
    Ok(Response::ok_data(TwoFaVerifyResponse { verified: true }))
}
