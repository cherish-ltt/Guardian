//! xdb v3 头部：常量与头部解析。
//!
//! 取自上游 `maker` crate 的 `header.rs`，只保留查询侧需要的部分：`Header` 的其余字段由
//! 生成侧写入、查询侧不读，`IndexPolicy` 与 `IPAddrExt` 也只在生成侧使用。这样查询 crate
//! 不必依赖生成工具（clap / chrono / bytes / itertools）。

use std::fmt::Display;

pub const HEADER_INFO_LENGTH: usize = 256;
pub const VECTOR_INDEX_COLS: usize = 256;
pub const VECTOR_INDEX_ROWS: usize = 256;
pub const VECTOR_INDEX_SIZE: usize = 8;
pub const VECTOR_INDEX_LENGTH: usize = VECTOR_INDEX_COLS * VECTOR_INDEX_ROWS * VECTOR_INDEX_SIZE;

#[derive(Debug, thiserror::Error)]
pub enum MakerError {
    #[error("Header parse error: {0}")]
    HeaderParsed(String),
}

#[derive(Debug)]
pub struct Header {
    ip_version: IpVersion,
}

impl TryFrom<&[u8; 256]> for Header {
    type Error = MakerError;

    fn try_from(value: &[u8; 256]) -> Result<Self, MakerError> {
        let ip_version = match u16::from_le_bytes([value[16], value[17]]) {
            4 => IpVersion::V4,
            6 => IpVersion::V6,
            other => {
                return Err(MakerError::HeaderParsed(format!(
                    "Header ip version invalid: {other}"
                )));
            },
        };

        Ok(Header { ip_version })
    }
}

impl Header {
    pub fn ip_bytes_len(&self) -> usize {
        self.ip_version.ip_bytes_len()
    }

    pub fn segment_index_size(&self) -> usize {
        self.ip_version.segment_index_size()
    }

    pub fn ip_version(&self) -> &IpVersion {
        &self.ip_version
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum IpVersion {
    V4,
    V6,
}

impl IpVersion {
    pub fn ip_bytes_len(&self) -> usize {
        match &self {
            IpVersion::V4 => 4,
            IpVersion::V6 => 16,
        }
    }

    pub fn segment_index_size(&self) -> usize {
        match &self {
            IpVersion::V4 => 14,
            IpVersion::V6 => 38,
        }
    }
}

impl Display for IpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpVersion::V4 => write!(f, "V4"),
            IpVersion::V6 => write!(f, "V6"),
        }
    }
}
