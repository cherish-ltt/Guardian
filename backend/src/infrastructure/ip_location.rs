//! 客户端 IP 的地点：本地网段直接判定，其余查离线地名库。
//!
//! 离线库是 ip2region 的 xdb（`vendor/ip2region`）。库文件不随仓库分发，部署时下载并通过
//! `IP_LOCATION_DB_PATH` 指定；未配置时不查地名，只区分本机 / 内网 / 外网。

use std::{net::IpAddr, path::Path};

use anyhow::{Context, Result};
use ip2region::{CachePolicy, Searcher};
use tracing::info;

const LOOPBACK: &str = "本机";
const LAN: &str = "内网";
const WAN: &str = "外网";

pub struct IpLocation {
    searcher: Option<Searcher>,
}

impl IpLocation {
    /// 配置了库文件却加载不了时直接失败：静默降级会让所有日志悄悄丢失地名。
    pub fn load(db_path: Option<&Path>) -> Result<Self> {
        let Some(db_path) = db_path else {
            info!("IP 地名库未配置，日志地点只区分本机 / 内网 / 外网");
            return Ok(Self { searcher: None });
        };

        let searcher = Searcher::new(
            db_path.to_string_lossy().into_owned(),
            CachePolicy::VectorIndex,
        )
        .with_context(|| format!("加载 IP 地名库失败：{}", db_path.display()))?;
        info!(path = %db_path.display(), "IP 地名库已加载");

        Ok(Self {
            searcher: Some(searcher),
        })
    }

    /// 回环「本机」、私网与链路本地「内网」，其余查地名库；库未配置或未命中时「外网」。
    ///
    /// 地名保留库里的原始串（如 `中国|广东省|茂名市|电信|CN`），不裁剪字段。
    pub fn resolve(&self, ip: Option<&str>) -> Option<String> {
        let ip = ip.filter(|value| !value.is_empty())?;
        let addr = ip.parse::<IpAddr>().ok()?.to_canonical();

        if addr.is_loopback() {
            return Some(LOOPBACK.to_string());
        }
        if is_lan(addr) {
            return Some(LAN.to_string());
        }

        // 只加载 IPv4 库，IPv6 来源不查地名。
        let IpAddr::V4(addr) = addr else {
            return Some(WAN.to_string());
        };

        match self
            .searcher
            .as_ref()
            .and_then(|searcher| searcher.search(addr).ok())
        {
            Some(region) if is_region(&region) => Some(region),
            _ => Some(WAN.to_string()),
        }
    }
}

/// RFC1918 私网、链路本地与 IPv6 ULA 视为内网；CGNAT（100.64/10）不是私网，按外网处理。
fn is_lan(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(addr) => addr.is_private() || addr.is_link_local(),
        IpAddr::V6(addr) => addr.is_unique_local() || addr.is_unicast_link_local(),
    }
}

/// 库对未收录的地址返回空串，对保留地址段返回 `Reserved|...`，两者都不是地名。
fn is_region(region: &str) -> bool {
    !region.is_empty() && !region.starts_with("Reserved")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::IpLocation;

    fn without_database() -> IpLocation {
        IpLocation::load(None).expect("不配置库文件时应构造成功")
    }

    #[test]
    fn local_segments_are_recognized_without_the_database() {
        let location = without_database();

        for ip in ["127.0.0.1", "127.0.0.2", "::1", "0:0:0:0:0:0:0:1"] {
            assert_eq!(location.resolve(Some(ip)).as_deref(), Some("本机"), "{ip}");
        }

        for ip in [
            "10.0.0.1",
            "172.16.0.1",
            "172.31.255.254",
            "192.168.1.1",
            "169.254.10.1",
            "fc00::1",
            "fe80::1",
            "::ffff:192.168.1.1",
        ] {
            assert_eq!(location.resolve(Some(ip)).as_deref(), Some("内网"), "{ip}");
        }
    }

    #[test]
    fn public_addresses_fall_back_to_wan_without_the_database() {
        let location = without_database();

        for ip in [
            "8.8.8.8",
            "172.32.0.1",
            "100.64.0.1",
            "203.0.113.7",
            "2001:db8::1",
        ] {
            assert_eq!(location.resolve(Some(ip)).as_deref(), Some("外网"), "{ip}");
        }

        assert_eq!(location.resolve(None), None);
        assert_eq!(location.resolve(Some("")), None);
        assert_eq!(location.resolve(Some("not-an-ip")), None);
    }

    #[test]
    fn reserved_segments_are_not_treated_as_regions() {
        assert!(super::is_region("中国|广东省|茂名市|电信|CN"));
        assert!(!super::is_region(""));
        assert!(!super::is_region("Reserved|Reserved|Reserved|0|0"));
    }

    #[test]
    fn a_configured_but_unusable_database_fails_loudly() {
        let path = Path::new("/nonexistent/ip2region_v4.xdb");

        assert!(IpLocation::load(Some(path)).is_err());
    }
}
