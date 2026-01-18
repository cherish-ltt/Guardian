use crate::middleware::auth::AuthContext;
use crate::response::ResponseCode;
use crate::router::AppState;
use crate::service::check_api_permission;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

pub async fn permission_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, axum::Json<crate::response::Response<()>>)> {
    let auth_context = request.extensions().get::<AuthContext>().ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            axum::Json(ResponseCode::AuthError.to_response(Some("缺少认证上下文".to_string()))),
        )
    })?;

    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    let has_permission = check_api_permission(state, auth_context.clone(), method, path).await;

    match has_permission {
        Ok(true) => Ok(next.run(request).await),
        Ok(false) => Err((
            StatusCode::FORBIDDEN,
            axum::Json(ResponseCode::PermissionDenied.to_response(Some("权限不足".to_string()))),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(
                ResponseCode::InternalError.to_response(Some(format!("权限检查失败: {}", e))),
            ),
        )),
    }
}
