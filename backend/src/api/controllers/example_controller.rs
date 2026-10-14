use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};

use crate::{
    api::{
        dto::example::{
            ExampleDetailResponse, ExampleEchoRequest, ExampleEchoResponse, ExampleListResponse,
            ExampleQuery,
        },
        extractors::current_user::CurrentUser,
    },
    container::Container,
    error::AppError,
    response::ApiResponse,
};

#[cfg(feature = "docs")]
use crate::docs::{
    DocErrorResponse, DocExampleDetailResponse, DocExampleEchoResponse, DocExampleListResponse,
};

pub fn router() -> Router<Arc<Container>> {
    Router::new()
        .route("/examples/echo", post(create_echo))
        .route("/examples", get(list_examples))
        .route("/examples/{id}", get(get_example))
}

#[cfg_attr(feature = "docs", utoipa::path(
    post,
    path = "/api/v1/examples/echo",
    tag = "Example",
    request_body = ExampleEchoRequest,
    responses(
        (status = 200, description = "Example object created", body = DocExampleEchoResponse),
        (status = 400, description = "Bad request", body = DocErrorResponse)
    )
))]
pub async fn create_echo(
    State(container): State<Arc<Container>>,
    Json(payload): Json<ExampleEchoRequest>,
) -> Result<ApiResponse<ExampleEchoResponse>, AppError> {
    let echo = container
        .example_service
        .create_echo(payload.into())
        .await?;

    Ok(ApiResponse::ok_with_message(
        "Example object created",
        echo.into(),
    ))
}

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/api/v1/examples",
    tag = "Example",
    params(ExampleQuery),
    responses(
        (status = 200, description = "Example list fetched", body = DocExampleListResponse)
    )
))]
pub async fn list_examples(
    State(container): State<Arc<Container>>,
    Query(query): Query<ExampleQuery>,
) -> Result<ApiResponse<ExampleListResponse>, AppError> {
    let list = container
        .example_service
        .list_examples(query.into())
        .await?;
    Ok(ApiResponse::ok(list.into()))
}

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/api/v1/examples/{id}",
    tag = "Example",
    params(
        ("id" = String, Path, description = "Example ID")
    ),
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Example detail fetched", body = DocExampleDetailResponse),
        (status = 401, description = "Unauthorized or invalid token", body = DocErrorResponse),
        (status = 404, description = "Not found", body = DocErrorResponse)
    )
))]
pub async fn get_example(
    State(container): State<Arc<Container>>,
    Path(id): Path<String>,
    current_user: CurrentUser,
) -> Result<ApiResponse<ExampleDetailResponse>, AppError> {
    let detail = container
        .example_service
        .get_example_detail(id, current_user.into_inner())
        .await?;

    Ok(ApiResponse::ok(detail.into()))
}
