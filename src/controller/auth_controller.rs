use axum::{Json, extract::State, http::StatusCode};

use crate::dto::{
    LoginRequest, LoginResponse, RefreshTokenRequest, RefreshTokenResponse, TwoFaSetupResponse,
    TwoFaVerifyRequest, TwoFaVerifyResponse,
};
use crate::middleware::auth::AuthContext;
use crate::response::{Response, ResponseCode};
use crate::router::AppState;

pub async fn login(
    state: State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> (StatusCode, Json<Response<LoginResponse>>) {
    match crate::service::login_service(state.0, payload).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (
            StatusCode::OK,
            Json(ResponseCode::AuthError.to_response(Some(e.to_string()))),
        ),
    }
}

pub async fn logout(
    state: State<AppState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> (StatusCode, Json<Response<()>>) {
    match crate::service::logout_service(state.0, payload.refresh_token).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (
            StatusCode::OK,
            Json(ResponseCode::AuthError.to_response(Some(e.to_string()))),
        ),
    }
}

pub async fn refresh_token(
    state: State<AppState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> (StatusCode, Json<Response<RefreshTokenResponse>>) {
    match crate::service::refresh_token_service(state.0, payload.refresh_token).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (
            StatusCode::OK,
            Json(ResponseCode::AuthError.to_response(Some(e.to_string()))),
        ),
    }
}

pub async fn setup_2fa(
    State(state): State<AppState>,
    auth_context: axum::Extension<AuthContext>,
) -> (StatusCode, Json<Response<TwoFaSetupResponse>>) {
    match crate::service::setup_2fa_service(state, auth_context.0).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (
            StatusCode::OK,
            Json(ResponseCode::InternalError.to_response(Some(e.to_string()))),
        ),
    }
}

pub async fn verify_2fa(
    State(state): State<AppState>,
    auth_context: axum::Extension<AuthContext>,
    Json(payload): Json<TwoFaVerifyRequest>,
) -> (StatusCode, Json<Response<TwoFaVerifyResponse>>) {
    match crate::service::verify_2fa_service(state, auth_context.0, payload.code).await {
        Ok(res) => (StatusCode::OK, Json(res)),
        Err(e) => (
            StatusCode::OK,
            Json(ResponseCode::InternalError.to_response(Some(e.to_string()))),
        ),
    }
}
