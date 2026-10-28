//! 客户端 IP 的地点：本地网段直接判定，其余查离线地名库。
//!
//! 离线库是 ip2region 的 xdb（`vendor/ip2region`），IPv4 与 IPv6 各一个库文件。库文件不随仓库
//! 分发，部署时下载并分别通过 `IP_LOCATION_DB_PATH`、`IP_LOCATION_V6_DB_PATH` 指定；未配置的
//! 协议不查地名，只区分本机 / 内网 / 外网。

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    path::Path,
};

use anyhow::{Context, Result};
use ip2region::{CachePolicy, Searcher};
use tracing::info;

const LOOPBACK: &str = "本机";
const LAN: &str = "内网";
const WAN: &str = "外网";

pub struct IpLocation {
    v4: Option<Searcher>,
    v6: Option<Searcher>,
}

impl IpLocation {
    /// 配置了库文件却加载不了时直接失败：静默降级会让所有日志悄悄丢失地名。
    pub fn load(v4_path: Option<&Path>, v6_path: Option<&Path>) -> Result<Self> {
        let v4 = v4_path
            .map(|path| load_searcher(path, Ipv4Addr::LOCALHOST.into()))
            .transpose()?;
        let v6 = v6_path
            .map(|path| load_searcher(path, Ipv6Addr::LOCALHOST.into()))
            .transpose()?;
        if v4.is_none() && v6.is_none() {
            info!("IP 地名库未配置，日志地点只区分本机 / 内网 / 外网");
        }

        Ok(Self { v4, v6 })
    }

    /// 回环「本机」、私网与链路本地「内网」，其余查对应协议的库；库未配置或未命中时「外网」。
    ///
    /// 地名保留库里的原始串（如 `中国|广东省|茂名市|电信|CN`），不裁剪字段。
    /// `label_local` 为 false 时回环与私网也查库；调用方要保证本地网段不落成「外网」，
    /// 用 [`local_label`] 先判定（属地现查即如此）。
    pub fn resolve(&self, ip: Option<&str>, label_local: bool) -> Option<String> {
        let ip = ip.filter(|value| !value.is_empty())?;
        let addr = ip.parse::<IpAddr>().ok()?.to_canonical();

        if label_local {
            if addr.is_loopback() {
                return Some(LOOPBACK.to_string());
            }
            if is_lan(addr) {
                return Some(LAN.to_string());
            }
        }

        let region = match addr {
            IpAddr::V4(addr) => self
                .v4
                .as_ref()
                .and_then(|searcher| searcher.search(addr).ok()),
            IpAddr::V6(addr) => self
                .v6
                .as_ref()
                .and_then(|searcher| searcher.search(addr).ok()),
        };

        match region {
            Some(region) if is_region(&region) => Some(region),
            _ => Some(WAN.to_string()),
        }
    }
}

/// 库头记录的是 v4 还是 v6 只影响查询结果：库版本与配置的协议不一致时查询一律报错，
/// 上层会把它当成未命中退回「外网」，因此这里用一条本机地址探测一次，让配置错误在启动时暴露。
fn load_searcher(path: &Path, probe: IpAddr) -> Result<Searcher> {
    let searcher = Searcher::new(
        path.to_string_lossy().into_owned(),
        CachePolicy::VectorIndex,
    )
    .with_context(|| format!("加载 IP 地名库失败：{}", path.display()))?;
    match probe {
        IpAddr::V4(probe) => searcher.search(probe),
        IpAddr::V6(probe) => searcher.search(probe),
    }
    .with_context(|| format!("IP 地名库版本与配置的协议不符：{}", path.display()))?;
    info!(path = %path.display(), "IP 地名库已加载");

    Ok(searcher)
}

/// RFC1918 私网、链路本地与 IPv6 ULA 视为内网；CGNAT（100.64/10）不是私网，按外网处理。
fn is_lan(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(addr) => addr.is_private() || addr.is_link_local(),
        IpAddr::V6(addr) => addr.is_unique_local() || addr.is_unicast_link_local(),
    }
}

/// 本地网段无需查库：没有库文件时它们同样要落回「本机」与「内网」，
/// 否则日志与属地现查会把这些地址一律算成外网。
pub fn local_label(addr: IpAddr) -> Option<&'static str> {
    if addr.is_loopback() {
        return Some(LOOPBACK);
    }

    is_lan(addr).then_some(LAN)
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
        IpLocation::load(None, None).expect("不配置库文件时应构造成功")
    }

    /// 日志与属地现查共用同一个解析器，只有「本地网段是否单列」的开关不同。
    fn resolve(location: &IpLocation, ip: Option<&str>) -> Option<String> {
        location.resolve(ip, true)
    }

    #[test]
    fn local_segments_are_recognized_without_the_database() {
        let location = without_database();

        for ip in ["127.0.0.1", "127.0.0.2", "::1", "0:0:0:0:0:0:0:1"] {
            assert_eq!(
                resolve(&location, Some(ip)).as_deref(),
                Some("本机"),
                "{ip}"
            );
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
            assert_eq!(
                resolve(&location, Some(ip)).as_deref(),
                Some("内网"),
                "{ip}"
            );
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
            assert_eq!(
                resolve(&location, Some(ip)).as_deref(),
                Some("外网"),
                "{ip}"
            );
        }

        assert_eq!(resolve(&location, None), None);
        assert_eq!(resolve(&location, Some("")), None);
        assert_eq!(resolve(&location, Some("not-an-ip")), None);
    }

    #[test]
    fn reserved_segments_are_not_treated_as_regions() {
        assert!(super::is_region("中国|广东省|茂名市|电信|CN"));
        assert!(!super::is_region(""));
        assert!(!super::is_region("Reserved|Reserved|Reserved|0|0"));
    }

    /// 属地现查一律查库，但本地网段由调用方先按 [`super::local_label`] 给出三档结论；
    /// 没有库文件时它们不能落成「外网」。
    #[test]
    fn location_lookup_labels_local_segments_before_the_database() {
        let location = without_database();

        for (ip, expected) in [
            ("127.0.0.1", "本机"),
            ("192.168.1.1", "内网"),
            ("fe80::1", "内网"),
        ] {
            let addr = ip.parse().expect("合法地址");
            assert_eq!(super::local_label(addr), Some(expected), "{ip}");
        }
        assert_eq!(
            location.resolve(Some("8.8.8.8"), false).as_deref(),
            Some("外网")
        );
    }

    #[test]
    fn a_configured_but_unusable_database_fails_loudly() {
        let path = Path::new("/nonexistent/ip2region_v4.xdb");
        let v6_path = Path::new("/nonexistent/ip2region_v6.xdb");

        assert!(IpLocation::load(Some(path), None).is_err());
        assert!(IpLocation::load(None, Some(v6_path)).is_err());
    }
}
