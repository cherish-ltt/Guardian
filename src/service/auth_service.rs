use anyhow::{Ok, Result, anyhow};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, NotSet, QueryFilter, Set,
};
use totp_rs::{Secret, TOTP};

use crate::dto::{
    ChangeOwnPasswordRequest, LoginRequest, LoginResponse, RefreshTokenResponse,
    ResetPasswordRequest, TwoFaDisableResponse, TwoFaSetupResponse, TwoFaVerifyResponse,
};
use crate::entities::{admins, token_blacklist};
use crate::middleware::auth::AuthContext;
use crate::response::{Response, ResponseCode};
use crate::router::AppState;
use crate::utils::{
    create_token_pair, get_jti, hash_password, refresh_access_token, verify_password,
};

pub async fn login_service(
    state: AppState,
    payload: LoginRequest,
) -> Result<Response<LoginResponse>> {
    let now = chrono::Local::now();
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
        if locked_until > chrono::Local::now() {
            return Ok(Response::failed("账户已被锁定，请稍后重试".to_string()));
        }
    }

    if !verify_password(&payload.password, &admin.password_hash) {
        let login_attempts = admin.login_attempts.unwrap_or(0) + 1;
        let mut admin_model: admins::ActiveModel = admin.into_active_model();

        if login_attempts >= 5 {
            let locked_until = chrono::Local::now() + chrono::Duration::minutes(15);
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
            return Ok(Response::from_code(ResponseCode::InvalidTwoFaCode, None));
        }
        let two_fa_code = payload.two_fa_code.unwrap();

        let two_fa_secret = admin.two_fa_secret.as_ref().unwrap();
        let secret = Secret::Encoded(two_fa_secret.to_string());

        let totp = TOTP::new(
            totp_rs::Algorithm::SHA1,
            6,
            1,
            30,
            secret.to_bytes()?,
            Some("Guardian".to_string()),
            admin.username.clone(),
        )
        .map_err(|e| anyhow!("生成TOTP失败: {}", e))?;

        let is_valid = totp
            .check_current(&two_fa_code)
            .map_err(|e| anyhow!("验证2FA失败: {}", e))?;

        if !is_valid {
            return Ok(ResponseCode::InvalidTwoFaCode.to_response(None));
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

    let expires_at = chrono::Local::now() + chrono::Duration::days(7);

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
        .filter(token_blacklist::Column::ExpiresAt.gt(chrono::Local::now()))
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

pub async fn setup_2fa_service(
    state: AppState,
    auth_context: AuthContext,
) -> Result<Response<TwoFaSetupResponse>> {
    let admin = admins::Entity::find()
        .filter(admins::Column::Id.eq(auth_context.admin_id))
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("管理员不存在"))?;

    if admin.two_fa_secret.is_some() {
        return Ok(ResponseCode::TwoFaAlreadyEnabled.to_response(None));
    }

    let secret = Secret::generate_secret();

    let totp = TOTP::new(
        totp_rs::Algorithm::SHA1,
        6,
        1,
        30,
        secret.to_bytes()?,
        Some("Guardian".to_string()),
        auth_context.username.clone(),
    )
    .map_err(|e| anyhow!("生成TOTP失败: {}", e))?;

    let qr_code_url = totp
        .get_qr_base64()
        .map_err(|e| anyhow!("生成QR码失败: {}", e))?;

    let backup_codes: Vec<String> = (0..10)
        .map(|_| {
            use rand::Rng;
            let code: u32 = rand::thread_rng().gen_range(100000..999999);
            format!("{:08}", code)
        })
        .collect();

    let mut admin_model: admins::ActiveModel = admin.into_active_model();
    admin_model.two_fa_secret = Set(Some(secret.to_encoded().to_string()));
    admin_model.update(&state.conn).await?;

    Ok(Response::ok_data(TwoFaSetupResponse {
        secret: secret.to_encoded().to_string(),
        qr_code_url,
        backup_codes,
    }))
}

pub async fn verify_2fa_service(
    state: AppState,
    auth_context: AuthContext,
    code: String,
) -> Result<Response<TwoFaVerifyResponse>> {
    let admin = admins::Entity::find()
        .filter(admins::Column::Id.eq(auth_context.admin_id))
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("管理员不存在"))?;

    let two_fa_secret = admin.two_fa_secret.ok_or_else(|| anyhow!("未启用2FA"))?;

    let secret = Secret::Encoded(two_fa_secret.to_string());

    let totp = TOTP::new(
        totp_rs::Algorithm::SHA1,
        6,
        1,
        30,
        secret.to_bytes()?,
        Some("Guardian".to_string()),
        auth_context.username,
    )
    .map_err(|e| anyhow!("生成TOTP失败: {}", e))?;

    let is_valid = totp
        .check_current(&code)
        .map_err(|e| anyhow!("验证2FA失败: {}", e))?;

    if is_valid {
        Ok(Response::ok_data(TwoFaVerifyResponse { verified: true }))
    } else {
        Ok(ResponseCode::InvalidTwoFaCode.to_response(None))
    }
}

pub async fn disable_2fa_service(
    state: AppState,
    auth_context: AuthContext,
) -> Result<Response<TwoFaDisableResponse>> {
    let admin = admins::Entity::find()
        .filter(admins::Column::Id.eq(auth_context.admin_id))
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("管理员不存在"))?;

    if admin.two_fa_secret.is_none() {
        return Ok(ResponseCode::TwoFaNotEnabled.to_response(None));
    }

    let mut admin_model: admins::ActiveModel = admin.into_active_model();
    admin_model.two_fa_secret = Set(None);
    admin_model.update(&state.conn).await?;

    Ok(Response::ok_data(TwoFaDisableResponse { disabled: true }))
}

pub async fn reset_password_service(
    state: AppState,
    payload: ResetPasswordRequest,
) -> Result<Response<()>> {
    let admin = admins::Entity::find()
        .filter(admins::Column::Username.eq(&payload.username))
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("用户不存在"))?;

    let two_fa_secret = admin
        .two_fa_secret
        .as_ref()
        .ok_or_else(|| anyhow!("未启用2FA，无法通过此方式重置密码"))?;

    let secret = Secret::Encoded(two_fa_secret.to_string());

    let totp = TOTP::new(
        totp_rs::Algorithm::SHA1,
        6,
        1,
        30,
        secret.to_bytes()?,
        Some("Guardian".to_string()),
        admin.username.clone(),
    )
    .map_err(|e| anyhow!("生成TOTP失败: {}", e))?;

    let is_valid = totp
        .check_current(&payload.two_fa_code)
        .map_err(|e| anyhow!("验证2FA失败: {}", e))?;

    if !is_valid {
        return Ok(ResponseCode::InvalidTwoFaCode.to_response(None));
    }

    let password_hash = hash_password(&payload.new_password);

    let mut admin_model: admins::ActiveModel = admin.into_active_model();
    admin_model.password_hash = Set(password_hash);
    admin_model.updated_at = Set(Some(chrono::Local::now().into()));
    admin_model.update(&state.conn).await?;

    Ok(Response::ok_msg(Some("密码重置成功".to_string())))
}

pub async fn change_own_password_service(
    state: AppState,
    auth_context: AuthContext,
    payload: ChangeOwnPasswordRequest,
) -> Result<Response<()>> {
    let admin = admins::Entity::find()
        .filter(admins::Column::Id.eq(auth_context.admin_id))
        .one(&state.conn)
        .await?
        .ok_or_else(|| anyhow!("管理员不存在"))?;

    let password_hash = hash_password(&payload.new_password);

    let mut admin_model: admins::ActiveModel = admin.into_active_model();
    admin_model.password_hash = Set(password_hash);
    admin_model.updated_at = Set(Some(chrono::Local::now().into()));
    admin_model.update(&state.conn).await?;

    Ok(Response::ok_msg(Some("密码修改成功".to_string())))
}
