use crate::response::ResponseCode;
use crate::utils::verify_token;
use axum::{Json, extract::Request, http::StatusCode, middleware::Next, response::Response};
use uuid::Uuid;

#[derive(Clone)]
pub struct AuthContext {
    pub admin_id: Uuid,
    pub username: String,
    pub is_super_admin: bool,
}

pub(crate) async fn auth_middleware(
    mut request: Request,
    next: Next,
) -> std::result::Result<Response, (StatusCode, Json<crate::response::Response<()>>)> {
    let headers = request.headers();

    let token = headers.get("Authorization").and_then(|h| h.to_str().ok());

    if token.is_none() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ResponseCode::AuthError.to_response(Some("缺少认证令牌".to_string()))),
        ));
    }

    let token = token.unwrap();
    let (_, token) = token.split_at(7);

    let claims = match verify_token(token) {
        Ok(claims) => claims,
        Err(_) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ResponseCode::AuthError.to_response(Some("无效的token".to_string()))),
            ));
        }
    };

    let admin_id = Uuid::parse_str(&claims.sub).unwrap_or_default();

    let auth_context = AuthContext {
        admin_id,
        username: claims.username,
        is_super_admin: claims.is_super_admin,
    };

    request.extensions_mut().insert(auth_context);

    Result::Ok(next.run(request).await)
}
