use axum::Router;
use serde::Serialize;
#[allow(unused_imports)]
use serde_json::json;
use utoipa::{
    Modify, OpenApi, ToSchema,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};
use utoipa_swagger_ui::SwaggerUi;

use crate::api::dto::{
    auth::{AccessTokenResponse, CurrentUserResponse, DevLoginRequest},
    example::{
        ExampleDetailResponse, ExampleEchoRequest, ExampleEchoResponse, ExampleListItem,
        ExampleListResponse, ExampleQuery,
    },
    system::{HealthResponse, WelcomeResponse},
    transaction::{
        TransferAuditItem, TransferDetailResponse, TransferListResponse, TransferQuery,
        TransferReceiptResponse, TransferRecordItem, TransferRequest,
    },
};

const SYSTEM_TAG: &str = "System";
const AUTH_TAG: &str = "Auth";
const EXAMPLE_TAG: &str = "Example";
const TRANSACTION_TAG: &str = "Transaction";

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct DocMessageResponse {
    #[schema(example = 200)]
    pub code: u16,
    #[schema(example = "Success")]
    pub message: String,
    #[schema(value_type = Option<Object>, example = json!(null))]
    pub data: Option<serde_json::Value>,
    #[schema(example = 1713179523000i64)]
    pub timestamp: i64,
}

#[derive(Serialize, ToSchema)]
pub struct DocErrorResponse {
    #[schema(example = 400)]
    pub code: u16,
    #[schema(example = "Bad request")]
    pub message: String,
    #[schema(value_type = Option<Object>, example = json!(null))]
    pub data: Option<serde_json::Value>,
    #[schema(example = 1713179523000i64)]
    pub timestamp: i64,
}

#[derive(Serialize, ToSchema)]
pub struct DocWelcomeResponse {
    #[schema(example = 200)]
    pub code: u16,
    #[schema(example = "Success")]
    pub message: String,
    pub data: Option<WelcomeResponse>,
    #[schema(example = 1713179523000i64)]
    pub timestamp: i64,
}

#[derive(Serialize, ToSchema)]
pub struct DocHealthResponse {
    #[schema(example = 200)]
    pub code: u16,
    #[schema(example = "Health check completed")]
    pub message: String,
    pub data: Option<HealthResponse>,
    #[schema(example = 1713179523000i64)]
    pub timestamp: i64,
}

#[derive(Serialize, ToSchema)]
pub struct DocAccessTokenResponse {
    #[schema(example = 200)]
    pub code: u16,
    #[schema(example = "Debug token issued")]
    pub message: String,
    pub data: Option<AccessTokenResponse>,
    #[schema(example = 1713179523000i64)]
    pub timestamp: i64,
}

#[derive(Serialize, ToSchema)]
pub struct DocCurrentUserResponse {
    #[schema(example = 200)]
    pub code: u16,
    #[schema(example = "Success")]
    pub message: String,
    pub data: Option<CurrentUserResponse>,
    #[schema(example = 1713179523000i64)]
    pub timestamp: i64,
}

#[derive(Serialize, ToSchema)]
pub struct DocExampleEchoResponse {
    #[schema(example = 200)]
    pub code: u16,
    #[schema(example = "Example object created")]
    pub message: String,
    pub data: Option<ExampleEchoResponse>,
    #[schema(example = 1713179523000i64)]
    pub timestamp: i64,
}

#[derive(Serialize, ToSchema)]
pub struct DocExampleListResponse {
    #[schema(example = 200)]
    pub code: u16,
    #[schema(example = "Success")]
    pub message: String,
    pub data: Option<ExampleListResponse>,
    #[schema(example = 1713179523000i64)]
    pub timestamp: i64,
}

#[derive(Serialize, ToSchema)]
pub struct DocExampleDetailResponse {
    #[schema(example = 200)]
    pub code: u16,
    #[schema(example = "Success")]
    pub message: String,
    pub data: Option<ExampleDetailResponse>,
    #[schema(example = 1713179523000i64)]
    pub timestamp: i64,
}

#[derive(Serialize, ToSchema)]
pub struct DocTransferReceiptResponse {
    #[schema(example = 200)]
    pub code: u16,
    #[schema(example = "Transfer transaction committed")]
    pub message: String,
    pub data: Option<TransferReceiptResponse>,
    #[schema(example = 1713179523000i64)]
    pub timestamp: i64,
}

#[derive(Serialize, ToSchema)]
pub struct DocTransferDetailResponse {
    #[schema(example = 200)]
    pub code: u16,
    #[schema(example = "Success")]
    pub message: String,
    pub data: Option<TransferDetailResponse>,
    #[schema(example = 1713179523000i64)]
    pub timestamp: i64,
}

#[derive(Serialize, ToSchema)]
pub struct DocTransferListResponse {
    #[schema(example = 200)]
    pub code: u16,
    #[schema(example = "Success")]
    pub message: String,
    pub data: Option<TransferListResponse>,
    #[schema(example = 1713179523000i64)]
    pub timestamp: i64,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::controllers::system_controller::root,
        crate::api::controllers::system_controller::health,
        crate::api::controllers::system_controller::ready,
        crate::api::controllers::auth_controller::me,
        crate::api::controllers::example_controller::create_echo,
        crate::api::controllers::example_controller::list_examples,
        crate::api::controllers::example_controller::get_example,
        crate::api::controllers::transaction_controller::get_transfer,
        crate::api::controllers::transaction_controller::list_transfers
    ),
    components(
        schemas(
            WelcomeResponse,
            HealthResponse,
            DevLoginRequest,
            AccessTokenResponse,
            CurrentUserResponse,
            ExampleEchoRequest,
            ExampleEchoResponse,
            ExampleQuery,
            ExampleListItem,
            ExampleListResponse,
            ExampleDetailResponse,
            DocMessageResponse,
            DocErrorResponse,
            DocWelcomeResponse,
            DocHealthResponse,
            DocAccessTokenResponse,
            DocCurrentUserResponse,
            DocExampleEchoResponse,
            DocExampleListResponse,
            DocExampleDetailResponse,
            TransferRequest,
            TransferReceiptResponse,
            TransferQuery,
            TransferRecordItem,
            TransferAuditItem,
            TransferDetailResponse,
            TransferListResponse,
            DocTransferReceiptResponse,
            DocTransferDetailResponse,
            DocTransferListResponse
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = SYSTEM_TAG, description = "System base APIs"),
        (name = AUTH_TAG, description = "JWT authentication APIs"),
        (name = EXAMPLE_TAG, description = "Example business APIs"),
        (name = TRANSACTION_TAG, description = "Transaction and persistence example APIs")
    )
)]
struct ApiDoc;

/// 调试专用接口（dev-login / dev-transfer）的文档。
///
/// 这两个 handler 只在 `debug_assertions` 下注册，因此文档也必须按同一开关裁剪：
/// release + docs 组合下 `utoipa::path` 不会为它们生成 `__path_*`，写进 `ApiDoc` 会直接编译失败。
#[cfg(debug_assertions)]
#[derive(OpenApi)]
#[openapi(paths(
    crate::api::controllers::auth_controller::dev_login,
    crate::api::controllers::transaction_controller::dev_transfer
))]
struct DevApiDoc;

fn openapi_document() -> utoipa::openapi::OpenApi {
    let document = ApiDoc::openapi();

    #[cfg(debug_assertions)]
    let document = document.merge_from(DevApiDoc::openapi());

    document
}

pub fn mount(router: Router) -> Router {
    router.merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", openapi_document()))
}
