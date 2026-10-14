use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, Query, State},
    routing::get,
};

use crate::{
    api::dto::transaction::{TransferDetailResponse, TransferListResponse, TransferQuery},
    api::extractors::current_user::CurrentUser,
    container::Container,
    error::AppError,
    response::ApiResponse,
};

// 事务示例入口只在 debug 构建注册，相关导入同样按构建类型裁剪。
#[cfg(debug_assertions)]
use axum::{Json, routing::post};

#[cfg(debug_assertions)]
use crate::api::dto::transaction::{TransferReceiptResponse, TransferRequest};

#[cfg(feature = "docs")]
use crate::docs::{DocErrorResponse, DocTransferDetailResponse, DocTransferListResponse};

// 仅 dev_transfer 的文档引用该响应结构，随 handler 一起按构建类型裁剪。
#[cfg(all(feature = "docs", debug_assertions))]
use crate::docs::DocTransferReceiptResponse;

pub fn router() -> Router<Arc<Container>> {
    let router = Router::new()
        .route("/transactions", get(list_transfers))
        .route("/transactions/{id}", get(get_transfer));

    #[cfg(debug_assertions)]
    let router = router.route("/transactions/dev-transfer", post(dev_transfer));

    router
}

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/api/v1/transactions",
    tag = "Transaction",
    params(TransferQuery),
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Transfer records fetched (paginated)", body = DocTransferListResponse),
        (status = 400, description = "Invalid pagination parameters", body = DocErrorResponse),
        (status = 401, description = "Unauthorized or invalid token", body = DocErrorResponse)
    )
))]
pub async fn list_transfers(
    State(container): State<Arc<Container>>,
    Query(query): Query<TransferQuery>,
    _current_user: CurrentUser,
) -> Result<ApiResponse<TransferListResponse>, AppError> {
    let page = container
        .transfer_service
        .list_transfers(query.page, query.size)
        .await?;

    Ok(ApiResponse::ok(page.into()))
}

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/api/v1/transactions/{id}",
    tag = "Transaction",
    params(
        ("id" = String, Path, description = "Transfer record ID")
    ),
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Transfer detail and audit logs fetched", body = DocTransferDetailResponse),
        (status = 401, description = "Unauthorized or invalid token", body = DocErrorResponse),
        (status = 404, description = "Transfer record not found", body = DocErrorResponse)
    )
))]
pub async fn get_transfer(
    State(container): State<Arc<Container>>,
    Path(id): Path<String>,
    _current_user: CurrentUser,
) -> Result<ApiResponse<TransferDetailResponse>, AppError> {
    let detail = container.transfer_service.get_transfer(id).await?;

    Ok(ApiResponse::ok(detail.into()))
}

/// 调试构建专用：事务示例入口，可传 `force_fail` 验证回滚。
///
/// release 构建不会注册该路由，事务示例请参考 `services/transaction.rs`。
#[cfg(debug_assertions)]
#[cfg_attr(feature = "docs", utoipa::path(
    post,
    path = "/api/v1/transactions/dev-transfer",
    tag = "Transaction",
    request_body = TransferRequest,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Transfer transaction committed", body = DocTransferReceiptResponse),
        (status = 400, description = "Bad request or insufficient balance", body = DocErrorResponse),
        (status = 401, description = "Unauthorized or invalid token", body = DocErrorResponse),
        (status = 404, description = "Account not found", body = DocErrorResponse),
        (status = 500, description = "Transaction rolled back (force_fail verification)", body = DocErrorResponse)
    )
))]
pub async fn dev_transfer(
    State(container): State<Arc<Container>>,
    _current_user: CurrentUser,
    Json(payload): Json<TransferRequest>,
) -> Result<ApiResponse<TransferReceiptResponse>, AppError> {
    let receipt = container.transfer_service.transfer(payload.into()).await?;

    Ok(ApiResponse::ok_with_message(
        "Transfer transaction committed",
        receipt.into(),
    ))
}
