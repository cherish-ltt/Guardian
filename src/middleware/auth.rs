use crate::utils::verify_token;
use axum::{Json, extract::Request, http::StatusCode, middleware::Next, response::Response};

pub(crate) async fn auth_middleware(
    request: Request,
    next: Next,
) -> std::result::Result<Response, (StatusCode, Json<crate::response::Response<()>>)> {
    let headers = request.headers();

    let token = headers.get("Authorization").and_then(|h| h.to_str().ok());

    if token.is_none() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(crate::response::Response::failed("no auth".to_string())),
        ));
    }

    let token = token.unwrap();
    let (_, token) = token.split_at(7);

    if verify_token(token).is_err() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(crate::response::Response::failed("无效的token".to_string())),
        ));
    }

    Result::Ok(next.run(request).await)
}
