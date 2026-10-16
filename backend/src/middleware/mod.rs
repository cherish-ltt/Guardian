//! HTTP 中间件装配。
//!
//! 所有中间件由 [`apply`] 用 `tower::ServiceBuilder` 统一组织，容量参数来自
//! [`crate::infrastructure::config::MiddlewareConfig`]，这里不出现任何业务逻辑。

pub mod audit;
mod client_ip;
mod client_limits;
pub(crate) mod diagnostics;
mod error_response;
mod stack;
mod trace;

pub(crate) use diagnostics::measure;
pub use stack::apply;
