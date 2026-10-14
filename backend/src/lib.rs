//! Axum 洋葱架构脚手架库入口。
//!
//! 生产代码（`src/`）额外开启两条 Clippy lint，阈值/开关在 `.clippy.toml`：
//! 阈值只有在 lint 被启用时才会生效，所以启用位置必须与此处保持一致。
//! 集成测试（`tests/`）不适用：测试用例的 setup / 断言天然长而分支多。
#![warn(clippy::cognitive_complexity, clippy::unwrap_used)]

pub mod api;
pub mod container;
pub mod create_app;
#[cfg(feature = "docs")]
pub mod docs;
pub mod domain;
pub mod entities;
pub mod error;
pub mod infrastructure;
pub mod logging;
pub mod middleware;
pub mod response;
pub mod services;
pub mod util;
