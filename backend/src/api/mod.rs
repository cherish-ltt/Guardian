pub mod controllers;
pub mod dto;
pub mod extractors;

use std::sync::Arc;

use axum::Router;

use crate::{container::Container, response::ApiResponse};

pub fn router() -> Router<Arc<Container>> {
    Router::new()
        .merge(controllers::auth_controller::router())
        .merge(controllers::admin::router())
        .merge(controllers::monitor_controller::router())
        .merge(controllers::dashboard_controller::router())
        .merge(controllers::generator_controller::router())
        .merge(controllers::system_controller::router())
        .merge(controllers::example_controller::router())
        .merge(controllers::transaction_controller::router())
}

/// 后台管理接口的成功响应：与 Spring 版 `R.SUCCESS_MESSAGE` 一致。
pub(crate) fn ok<T: serde::Serialize>(data: T) -> ApiResponse<T> {
    ApiResponse::ok_with_message(SUCCESS_MESSAGE, data)
}

/// 无数据的成功响应，序列化成 `data: null`。
pub(crate) fn done() -> ApiResponse<()> {
    ApiResponse::ok_with_message(SUCCESS_MESSAGE, ())
}

pub(crate) const SUCCESS_MESSAGE: &str = "操作成功";
