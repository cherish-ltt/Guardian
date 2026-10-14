use serde::{Deserialize, Serialize};

use crate::domain::models::transaction::{
    TransferAuditView, TransferCommand, TransferDetail, TransferReceipt, TransferRecordPage,
    TransferRecordView,
};

#[allow(unused_imports)]
#[cfg(feature = "docs")]
use serde_json::json;
#[cfg(feature = "docs")]
use utoipa::{IntoParams, ToSchema};

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Deserialize)]
pub struct TransferRequest {
    #[cfg_attr(feature = "docs", schema(example = "acc_alice"))]
    pub from_account_id: String,
    #[cfg_attr(feature = "docs", schema(example = "acc_bob"))]
    pub to_account_id: String,
    #[cfg_attr(feature = "docs", schema(example = 25000))]
    pub amount_cents: i64,
    #[cfg_attr(feature = "docs", schema(example = "sample transfer"))]
    pub remark: Option<String>,
    /// 幂等键：同一个键重复提交会触发唯一约束冲突并回滚整个事务。
    #[cfg_attr(feature = "docs", schema(example = "req-20260927-0001"))]
    pub request_id: Option<String>,
    /// 调试构建专用：在事务内写入全部完成后主动失败，用于验证回滚。
    ///
    /// 接口本身只在调试构建注册，release 构建下该字段与路由都不存在。
    #[cfg(debug_assertions)]
    #[cfg_attr(feature = "docs", schema(example = false, write_only))]
    pub force_fail: Option<bool>,
}

impl From<TransferRequest> for TransferCommand {
    fn from(value: TransferRequest) -> Self {
        Self {
            from_account_id: value.from_account_id,
            to_account_id: value.to_account_id,
            amount_cents: value.amount_cents,
            remark: value.remark,
            request_id: value.request_id,
            #[cfg(debug_assertions)]
            force_fail: value.force_fail.unwrap_or(false),
            #[cfg(not(debug_assertions))]
            force_fail: false,
        }
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Serialize)]
pub struct TransferReceiptResponse {
    #[cfg_attr(
        feature = "docs",
        schema(example = "019680cc-7e1c-7ec0-b7b8-4b4f8e9dff10")
    )]
    pub record_id: String,
    #[cfg_attr(feature = "docs", schema(example = "req-20260927-0001"))]
    pub request_id: Option<String>,
    #[cfg_attr(feature = "docs", schema(example = "acc_alice"))]
    pub from_account_id: String,
    #[cfg_attr(feature = "docs", schema(example = "acc_bob"))]
    pub to_account_id: String,
    #[cfg_attr(feature = "docs", schema(example = 25000))]
    pub amount_cents: i64,
    #[cfg_attr(feature = "docs", schema(example = 75000))]
    pub from_balance_after_cents: i64,
    #[cfg_attr(feature = "docs", schema(example = 125000))]
    pub to_balance_after_cents: i64,
    #[cfg_attr(feature = "docs", schema(example = "sample transfer"))]
    pub remark: Option<String>,
    #[cfg_attr(feature = "docs", schema(example = 1790000000000i64))]
    pub created_at: i64,
    #[cfg_attr(feature = "docs", schema(example = true))]
    pub committed: bool,
    #[cfg_attr(feature = "docs", schema(example = false))]
    pub rolled_back: bool,
}

impl From<TransferReceipt> for TransferReceiptResponse {
    fn from(value: TransferReceipt) -> Self {
        Self {
            record_id: value.record_id,
            request_id: value.request_id,
            from_account_id: value.from_account_id,
            to_account_id: value.to_account_id,
            amount_cents: value.amount_cents,
            from_balance_after_cents: value.from_balance_after_cents,
            to_balance_after_cents: value.to_balance_after_cents,
            remark: value.remark,
            created_at: value.created_at,
            committed: value.committed,
            rolled_back: value.rolled_back,
        }
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Serialize)]
pub struct TransferRecordItem {
    #[cfg_attr(
        feature = "docs",
        schema(example = "019680cc-7e1c-7ec0-b7b8-4b4f8e9dff10")
    )]
    pub record_id: String,
    #[cfg_attr(feature = "docs", schema(example = "req-20260927-0001"))]
    pub request_id: Option<String>,
    #[cfg_attr(feature = "docs", schema(example = "acc_alice"))]
    pub from_account_id: String,
    #[cfg_attr(feature = "docs", schema(example = "acc_bob"))]
    pub to_account_id: String,
    #[cfg_attr(feature = "docs", schema(example = 25000))]
    pub amount_cents: i64,
    #[cfg_attr(feature = "docs", schema(example = 75000))]
    pub from_balance_after_cents: i64,
    #[cfg_attr(feature = "docs", schema(example = 125000))]
    pub to_balance_after_cents: i64,
    #[cfg_attr(feature = "docs", schema(example = "sample transfer"))]
    pub remark: Option<String>,
    #[cfg_attr(feature = "docs", schema(example = 1790000000000i64))]
    pub created_at: i64,
}

impl From<TransferRecordView> for TransferRecordItem {
    fn from(value: TransferRecordView) -> Self {
        Self {
            record_id: value.record_id,
            request_id: value.request_id,
            from_account_id: value.from_account_id,
            to_account_id: value.to_account_id,
            amount_cents: value.amount_cents,
            from_balance_after_cents: value.from_balance_after_cents,
            to_balance_after_cents: value.to_balance_after_cents,
            remark: value.remark,
            created_at: value.created_at,
        }
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Serialize)]
pub struct TransferAuditItem {
    #[cfg_attr(feature = "docs", schema(example = "BALANCE_UPDATED"))]
    pub action: String,
    #[cfg_attr(
        feature = "docs",
        schema(example = "acc_alice balance 100000 -> 75000, acc_bob balance 100000 -> 125000")
    )]
    pub detail: String,
    #[cfg_attr(feature = "docs", schema(example = 1790000000000i64))]
    pub created_at: i64,
}

impl From<TransferAuditView> for TransferAuditItem {
    fn from(value: TransferAuditView) -> Self {
        Self {
            action: value.action,
            detail: value.detail,
            created_at: value.created_at,
        }
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Serialize)]
pub struct TransferDetailResponse {
    pub record: TransferRecordItem,
    pub audits: Vec<TransferAuditItem>,
}

impl From<TransferDetail> for TransferDetailResponse {
    fn from(value: TransferDetail) -> Self {
        Self {
            record: value.record.into(),
            audits: value.audits.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg_attr(feature = "docs", derive(IntoParams, ToSchema))]
#[derive(Debug, Deserialize)]
pub struct TransferQuery {
    #[cfg_attr(feature = "docs", param(example = 1))]
    pub page: Option<u64>,
    #[cfg_attr(feature = "docs", param(example = 10))]
    pub size: Option<u64>,
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Serialize)]
pub struct TransferListResponse {
    #[cfg_attr(feature = "docs", schema(example = 1))]
    pub page: u64,
    #[cfg_attr(feature = "docs", schema(example = 10))]
    pub size: u64,
    #[cfg_attr(feature = "docs", schema(example = 3))]
    pub total: u64,
    pub items: Vec<TransferRecordItem>,
}

impl From<TransferRecordPage> for TransferListResponse {
    fn from(value: TransferRecordPage) -> Self {
        Self {
            page: value.page,
            size: value.size,
            total: value.total,
            items: value.items.into_iter().map(Into::into).collect(),
        }
    }
}
