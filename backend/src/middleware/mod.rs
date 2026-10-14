//! HTTP 中间件装配。
//!
//! 所有中间件由 [`apply`] 用 `tower::ServiceBuilder` 统一组织，容量参数来自
//! [`crate::infrastructure::config::MiddlewareConfig`]，这里不出现任何业务逻辑。

pub mod audit;
mod error_response;
mod stack;
mod trace;

pub use stack::apply;
