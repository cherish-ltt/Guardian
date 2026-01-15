pub mod admin_controller;
pub mod auth_controller;
pub mod permission_controller;
pub mod role_controller;
pub mod system_info_controller;

use crate::response::Response;
use axum::{Json, http::StatusCode};

pub(crate) use admin_controller::*;
pub(crate) use auth_controller::*;
pub(crate) use permission_controller::*;
pub(crate) use role_controller::*;
pub(crate) use system_info_controller::*;

pub(crate) async fn root() -> (StatusCode, Json<Response<()>>) {
    (
        StatusCode::OK,
        Json(Response::ok_msg(Some(
            "Hello, i am 'Guardian Auth System'".to_string(),
        ))),
    )
}
