//! 返回给前端的 IP 打码：按「真实 IP 查看权限」的三档粒度输出。
//!
//! 打码在 axum 返回前完成，数据库里存的仍是原始地址；本地网段、回环与公网地址一视同仁。

use std::net::IpAddr;

/// 真实 IP 的查看粒度，由调用方的权限码决定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpVisibility {
    /// 完全查看：原样返回完整地址。
    Full,
    /// 二级查看：IPv4 保留前两段，IPv6 保留前四段。
    Partial,
    /// 无权限：只留首尾段，中间按段打码。
    Masked,
}

/// 权限码：完全查看（`sys_ip_visibility` 的最高档）。
pub const PERMISSION_FULL: &str = "monitor:ip:view:full";

/// 权限码：二级查看。
pub const PERMISSION_PARTIAL: &str = "monitor:ip:view:partial";

/// 文本提示：无权限时不报错，用固定文案代替地址与地名。
pub const NO_PERMISSION: &str = "无权查看";

impl IpVisibility {
    /// 完全查看优先：同时拥有多档时按最高档输出。
    pub fn of(has_permission: impl Fn(&str) -> bool) -> Self {
        if has_permission(PERMISSION_FULL) {
            Self::Full
        } else if has_permission(PERMISSION_PARTIAL) {
            Self::Partial
        } else {
            Self::Masked
        }
    }

    /// 是否允许查询 IP 属地：只有完全查看档可以发起现查。
    pub fn can_query_location(self) -> bool {
        matches!(self, Self::Full)
    }

    /// 按本档打码；非法地址原样返回、空地址返回 `None`，不因脏数据让整个列表失败。
    pub fn mask(self, ip: Option<&str>) -> Option<String> {
        let ip = ip?.trim();
        if ip.is_empty() {
            return None;
        }
        let Ok(address) = ip.parse::<IpAddr>() else {
            return Some(ip.to_string());
        };

        Some(match (self, address.to_canonical()) {
            (Self::Full, _) => ip.to_string(),
            (Self::Partial, IpAddr::V4(address)) => {
                let octets = address.octets();

                format!("{}.{}.x.x", octets[0], octets[1])
            },
            (Self::Partial, IpAddr::V6(address)) => {
                let groups = address.segments();

                format!(
                    "{:x}:{:x}:{:x}:{:x}:x:x:x:x",
                    groups[0], groups[1], groups[2], groups[3]
                )
            },
            (Self::Masked, IpAddr::V4(address)) => {
                let octets = address.octets();

                format!("{}.x.x.{}", octets[0], octets[3])
            },
            (Self::Masked, IpAddr::V6(address)) => {
                let groups = address.segments();

                format!("{:x}:x:x:x:x:x:x:{:x}", groups[0], groups[7])
            },
        })
    }

    /// 打码后的地址；无权限时用固定文案，调用方不要再拼地名。
    pub fn display(self, ip: Option<&str>) -> String {
        self.mask(ip).unwrap_or_else(|| NO_PERMISSION.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{IpVisibility, NO_PERMISSION, PERMISSION_FULL, PERMISSION_PARTIAL};

    #[test]
    fn the_strongest_granted_level_wins() {
        assert_eq!(IpVisibility::of(|_| false), IpVisibility::Masked);
        assert_eq!(
            IpVisibility::of(|code| code == PERMISSION_PARTIAL),
            IpVisibility::Partial
        );
        assert_eq!(
            IpVisibility::of(|code| code == PERMISSION_FULL || code == PERMISSION_PARTIAL),
            IpVisibility::Full
        );
        assert_eq!(
            IpVisibility::of(|code| code == PERMISSION_FULL),
            IpVisibility::Full
        );
    }

    #[test]
    fn only_the_full_level_may_query_the_location() {
        assert!(IpVisibility::Full.can_query_location());
        assert!(!IpVisibility::Partial.can_query_location());
        assert!(!IpVisibility::Masked.can_query_location());
    }

    #[test]
    fn ipv4_is_masked_by_segment() {
        let ip = Some("61.135.131.21");

        assert_eq!(
            IpVisibility::Full.mask(ip).as_deref(),
            Some("61.135.131.21")
        );
        assert_eq!(
            IpVisibility::Partial.mask(ip).as_deref(),
            Some("61.135.x.x")
        );
        assert_eq!(IpVisibility::Masked.mask(ip).as_deref(), Some("61.x.x.21"));
    }

    #[test]
    fn ipv6_is_masked_by_segment() {
        let ip = Some("2001:0db8:85a3:0000:0000:8a2e:0370:7334");

        assert_eq!(
            IpVisibility::Partial.mask(ip).as_deref(),
            Some("2001:db8:85a3:0:x:x:x:x")
        );
        assert_eq!(
            IpVisibility::Masked.mask(ip).as_deref(),
            Some("2001:x:x:x:x:x:x:7334")
        );
        assert_eq!(
            IpVisibility::Masked.mask(Some("::1")).as_deref(),
            Some("0:x:x:x:x:x:x:1")
        );
    }

    /// 内网与回环同样打码，不因为「不是公网」就放行。
    #[test]
    fn local_addresses_are_masked_too() {
        assert_eq!(
            IpVisibility::Masked.mask(Some("127.0.0.1")).as_deref(),
            Some("127.x.x.1")
        );
        assert_eq!(
            IpVisibility::Partial.mask(Some("192.168.1.10")).as_deref(),
            Some("192.168.x.x")
        );
        assert_eq!(
            IpVisibility::Masked.mask(Some("::1")).as_deref(),
            Some("0:x:x:x:x:x:x:1")
        );
    }

    /// IPv4 映射的 IPv6 先归一成 IPv4，避免打码后露出完整 IPv6 写法。
    #[test]
    fn mapped_ipv4_is_normalized_before_masking() {
        assert_eq!(
            IpVisibility::Masked
                .mask(Some("::ffff:192.168.1.10"))
                .as_deref(),
            Some("192.x.x.10")
        );
    }

    #[test]
    fn missing_and_dirty_values_do_not_break_the_list() {
        assert_eq!(IpVisibility::Full.mask(None), None);
        assert_eq!(IpVisibility::Full.mask(Some("  ")), None);
        assert_eq!(
            IpVisibility::Full.mask(Some("unknown")).as_deref(),
            Some("unknown")
        );
        assert_eq!(IpVisibility::Masked.display(None), NO_PERMISSION);
        assert_eq!(IpVisibility::Partial.display(Some("1.2.3.4")), "1.2.x.x");
    }
}
