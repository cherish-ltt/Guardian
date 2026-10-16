//! ip2region 官方 Rust 查询客户端（vendor 副本，见 ../README.md）。

mod error;
mod header;
mod ip_value;
mod searcher;

pub use ip_value::IpValueExt;
pub use searcher::{CachePolicy, Searcher};
