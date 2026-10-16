use std::net::{IpAddr, SocketAddr};

use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::Response,
};

use crate::infrastructure::config::MiddlewareConfig;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ClientIp(pub IpAddr);

#[derive(Clone, Copy)]
pub(crate) enum IdentityError {
    MissingPeer,
    InvalidForwardedFor,
}

/// 只保留 1–64 字节的字母、数字、`-_.` 请求 ID，其余（含重复头）整体丢弃后重新生成。
pub(crate) async fn sanitize_request_id(mut request: Request, next: Next) -> Response {
    let mut values = request.headers().get_all("x-request-id").iter();
    let valid = values.next().is_some_and(|value| {
        !value.is_empty()
            && value.len() <= 64
            && value
                .as_bytes()
                .iter()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(b))
    }) && values.next().is_none();
    if !valid {
        request.headers_mut().remove("x-request-id");
    }
    next.run(request).await
}

pub(crate) async fn identify(
    State(config): State<MiddlewareConfig>,
    mut request: Request,
    next: Next,
) -> Response {
    let identity = resolve(&request, &config);
    request.extensions_mut().insert(identity);
    next.run(request).await
}

fn resolve(request: &Request, config: &MiddlewareConfig) -> Result<ClientIp, IdentityError> {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .ok_or(IdentityError::MissingPeer)?
        .0
        .ip();
    let mut current = canonical(peer);
    if !trusted(current, config) {
        return Ok(ClientIp(current));
    }

    let mut values = request.headers().get_all("x-forwarded-for").iter();
    let Some(value) = values.next() else {
        return Ok(ClientIp(current));
    };
    if values.next().is_some() {
        return Err(IdentityError::InvalidForwardedFor);
    }
    let chain = value
        .to_str()
        .map_err(|_| IdentityError::InvalidForwardedFor)?;
    for value in chain.split(',').rev() {
        if !trusted(current, config) {
            break;
        }
        current = canonical(
            value
                .trim()
                .parse()
                .map_err(|_| IdentityError::InvalidForwardedFor)?,
        );
    }
    Ok(ClientIp(current))
}

fn trusted(ip: IpAddr, config: &MiddlewareConfig) -> bool {
    config.trusted_proxies.iter().any(|net| net.contains(&ip))
}

/// IPv4 映射 IPv6（`::ffff:192.0.2.1`）与纯 IPv4 归一成同一个键，共用同一份额度。
fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(ip) => ip
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(ip)),
        ip => ip,
    }
}
