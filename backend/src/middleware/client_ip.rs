use std::net::{IpAddr, SocketAddr};

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{Extensions, HeaderMap, header::USER_AGENT},
    middleware::Next,
    response::Response,
};

use crate::{
    domain::models::identity::ClientSource, infrastructure::config::MiddlewareConfig,
    util::user_agent,
};

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

/// 客户端来源：`identify` 放进扩展里的真实对端 IP，加上 `User-Agent` 里的浏览器与系统。
///
/// IP 只用保护层的解析结果、不看请求头原值，否则日志里的来源可以被调用方伪造。
pub(crate) fn source_of(extensions: &Extensions, headers: &HeaderMap) -> ClientSource {
    let ip = extensions
        .get::<Result<ClientIp, IdentityError>>()
        .and_then(|resolved| resolved.as_ref().ok())
        .map(|ip| ip.0.to_string());
    let (browser, os) = user_agent::describe(
        headers
            .get(USER_AGENT)
            .and_then(|value| value.to_str().ok()),
    );

    ClientSource { ip, browser, os }
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

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::http::Request;

    use super::{ClientIp, IdentityError, source_of};
    use crate::domain::models::identity::ClientSource;

    fn source(peer: Option<&str>, user_agent: Option<&str>) -> ClientSource {
        let mut builder = Request::builder();
        if let Some(user_agent) = user_agent {
            builder = builder.header("user-agent", user_agent);
        }
        let (mut parts, _) = builder.body(()).expect("构造请求").into_parts();
        let identity: Result<ClientIp, IdentityError> =
            peer.map_or(Err(IdentityError::MissingPeer), |peer| {
                Ok(ClientIp(
                    peer.parse::<SocketAddr>().expect("测试对端地址应合法").ip(),
                ))
            });
        parts.extensions.insert(identity);

        source_of(&parts.extensions, &parts.headers)
    }

    #[test]
    fn source_combines_peer_ip_with_parsed_user_agent() {
        let chrome = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
                      Chrome/141.0.0.0 Safari/537.36";
        let resolved = source(Some("192.0.2.7:1000"), Some(chrome));

        assert_eq!(resolved.ip.as_deref(), Some("192.0.2.7"));
        assert_eq!(resolved.browser.as_deref(), Some("Chrome"));
        assert_eq!(resolved.os.as_deref(), Some("Windows 10"));
    }

    #[test]
    fn source_is_empty_without_identity_or_user_agent() {
        let resolved = source(None, None);

        assert_eq!(resolved.ip, None);
        assert_eq!(resolved.browser, None);
        assert_eq!(resolved.os, None);
    }
}
