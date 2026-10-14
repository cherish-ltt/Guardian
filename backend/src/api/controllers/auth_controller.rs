use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    routing::{get, post, put},
};

use crate::{
    api::{
        done,
        dto::auth::{AccessTokenResponse, CurrentUserResponse, DevLoginRequest},
        extractors::{admin_user::AdminUser, api_json::ApiJson, current_user::CurrentUser},
        ok,
    },
    container::Container,
    domain::{
        error::AppError,
        models::identity::{
            CaptchaChallenge, ChangePasswordCommand, CurrentUserInfo, LoginCommand, LoginResult,
            RouteNode,
        },
    },
    response::ApiResponse,
};

#[cfg(feature = "docs")]
use crate::docs::{DocAccessTokenResponse, DocCurrentUserResponse, DocErrorResponse};

pub fn router() -> Router<Arc<Container>> {
    let router = Router::new()
        .route("/auth/captcha", get(captcha))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route("/auth/routes", get(routes))
        .route("/auth/password", put(change_password))
        // 脚手架自带的演示接口：`/auth/session` 返回开发令牌对应的会话，
        // 与后台会话（`/auth/me`）区分开，避免同一路径存在两套语义。
        .route("/auth/session", get(session));

    #[cfg(debug_assertions)]
    let router = router.route("/auth/dev-login", post(dev_login));

    router
}

/// 登录验证码：`code` 明文回显给登录页展示（与 Spring 版一致）。
#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/api/v1/auth/captcha",
    tag = "Auth",
    responses((status = 200, description = "Captcha issued"))
))]
pub async fn captcha(
    State(container): State<Arc<Container>>,
) -> Result<ApiResponse<CaptchaChallenge>, AppError> {
    let challenge = container.admin_auth_service.captcha().await?;

    Ok(ok(challenge))
}

#[cfg_attr(feature = "docs", utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Auth",
    request_body = LoginCommand,
    responses(
        (status = 200, description = "Login succeeded"),
        (status = 400, description = "Bad credentials or captcha", body = DocErrorResponse)
    )
))]
pub async fn login(
    State(container): State<Arc<Container>>,
    ApiJson(command): ApiJson<LoginCommand>,
) -> Result<ApiResponse<LoginResult>, AppError> {
    let result = container.admin_auth_service.login(command).await?;

    Ok(ok(result))
}

#[cfg_attr(feature = "docs", utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "Auth",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Session revoked"),
        (status = 401, description = "Unauthorized", body = DocErrorResponse)
    )
))]
pub async fn logout(
    State(container): State<Arc<Container>>,
    user: AdminUser,
) -> Result<ApiResponse<()>, AppError> {
    container
        .admin_auth_service
        .logout(user.token(), user.username())
        .await?;

    Ok(done())
}

/// 当前登录账号：角色与权限码供前端做按钮级控制。
#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "Auth",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current user info fetched"),
        (status = 401, description = "Unauthorized or invalid token", body = DocErrorResponse)
    )
))]
pub async fn me(user: AdminUser) -> Result<ApiResponse<CurrentUserInfo>, AppError> {
    Ok(ok(user.info().clone()))
}

/// 动态路由：菜单树按登录账号的角色授权返回。
#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/api/v1/auth/routes",
    tag = "Auth",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Route tree returned"))
))]
pub async fn routes(
    State(container): State<Arc<Container>>,
    user: AdminUser,
) -> Result<ApiResponse<RoutesResponse>, AppError> {
    let routes = container.admin_auth_service.routes(user.username()).await?;

    Ok(ok(RoutesResponse { routes }))
}

#[cfg_attr(feature = "docs", utoipa::path(
    put,
    path = "/api/v1/auth/password",
    tag = "Auth",
    security(("bearer_auth" = [])),
    request_body = ChangePasswordCommand,
    responses(
        (status = 200, description = "Password changed"),
        (status = 400, description = "Bad request", body = DocErrorResponse)
    )
))]
pub async fn change_password(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    ApiJson(command): ApiJson<ChangePasswordCommand>,
) -> Result<ApiResponse<()>, AppError> {
    container
        .admin_auth_service
        .change_password(user.username(), command)
        .await?;

    Ok(done())
}

/// `/auth/routes` 的业务数据。
#[derive(Debug, serde::Serialize)]
pub struct RoutesResponse {
    pub routes: Vec<RouteNode>,
}

#[cfg_attr(feature = "docs", utoipa::path(
    post,
    path = "/api/v1/auth/dev-login",
    tag = "Auth",
    request_body = DevLoginRequest,
    responses(
        (status = 200, description = "Debug token issued", body = DocAccessTokenResponse),
        (status = 400, description = "Bad request", body = DocErrorResponse)
    )
))]
pub async fn dev_login(
    State(container): State<Arc<Container>>,
    ApiJson(payload): ApiJson<DevLoginRequest>,
) -> Result<ApiResponse<AccessTokenResponse>, AppError> {
    let token = container
        .auth_service
        .issue_dev_token(payload.into())
        .await?;

    Ok(ApiResponse::ok_with_message(
        "Debug token issued",
        token.into(),
    ))
}

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/api/v1/auth/session",
    tag = "Auth",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Current user info fetched", body = DocCurrentUserResponse),
        (status = 401, description = "Unauthorized or invalid token", body = DocErrorResponse)
    )
))]
pub async fn session(
    current_user: CurrentUser,
) -> Result<ApiResponse<CurrentUserResponse>, AppError> {
    Ok(ApiResponse::ok(current_user.into_inner().into()))
}
