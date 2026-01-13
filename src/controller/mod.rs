pub mod auth_controller;

use crate::response::Response;
use axum::{Json, http::StatusCode};

pub(crate) use auth_controller::*;

pub(crate) async fn root() -> (StatusCode, Json<Response<()>>) {
    (
        StatusCode::OK,
        Json(Response::ok_msg(Some(
            "Hello, i am 'Guardian Auth System'".to_string(),
        ))),
    )
}
