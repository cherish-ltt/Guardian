use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::{domain::models::identity::ClientSource as DomainClientSource, middleware::client_ip};

/// 登录 / 退出来源：保护层解析出的真实对端 IP，加上 `User-Agent` 解析出的浏览器与系统。
pub struct ClientSource(pub DomainClientSource);

impl<S> FromRequestParts<S> for ClientSource
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(client_ip::source_of(
            &parts.extensions,
            &parts.headers,
        )))
    }
}
