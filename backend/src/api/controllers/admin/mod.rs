//! 系统管理接口（`/api/v1/system/*`），按前端页面拆成多个路由文件。
//!
//! 每个文件只负责自己页面的路由注册与处理器：权限码、数据范围与响应体形状
//! 都对齐 Spring 版 `SystemManagementController`。

pub mod configs;
pub mod depts;
pub mod dicts;
pub mod menus;
pub mod posts;
pub mod roles;
pub mod users;

use std::sync::Arc;

use axum::Router;

use crate::container::Container;

pub fn router() -> Router<Arc<Container>> {
    Router::new()
        .merge(users::router())
        .merge(roles::router())
        .merge(menus::router())
        .merge(depts::router())
        .merge(posts::router())
        .merge(dicts::router())
        .merge(configs::router())
}
