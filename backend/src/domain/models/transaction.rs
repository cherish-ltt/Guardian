//! 转账事务示例的领域模型。
//!
//! 金额统一用「最小货币单位」的整数表示（人民币场景即分），
//! 领域内不出现浮点数，避免精度导致的余额对不平。

/// 执行一次转账的入参。
#[derive(Debug, Clone)]
pub struct TransferCommand {
    pub from_account_id: String,
    pub to_account_id: String,
    pub amount_cents: i64,
    pub remark: Option<String>,
    /// 幂等键：同一键重复提交会命中 `transfer_records.request_id` 的唯一约束，
    /// 事务整体回滚，用于演示「跨表写入中约束冲突 → 全部撤销」。
    pub request_id: Option<String>,
    /// 事务内写入全部完成后主动失败，用于验证回滚。
    ///
    /// 仅调试构建可以从 HTTP 传入（见 `api/dto/transaction.rs`），
    /// 服务端始终按此字段决定是否触发回滚。
    pub force_fail: bool,
}

/// 账户当前状态，用于事务内的「读」。
#[derive(Debug, Clone)]
pub struct TransferAccountState {
    pub id: String,
    pub owner: String,
    pub balance_cents: i64,
    pub version: i64,
}

/// 一次已提交的转账事务结果。
#[derive(Debug, Clone)]
pub struct TransferReceipt {
    pub record_id: String,
    pub request_id: Option<String>,
    pub from_account_id: String,
    pub to_account_id: String,
    pub amount_cents: i64,
    pub from_balance_after_cents: i64,
    pub to_balance_after_cents: i64,
    pub remark: Option<String>,
    pub created_at: i64,
    /// 事务是否已提交。
    pub committed: bool,
    /// 是否因 `force_fail` 走了回滚分支。
    pub rolled_back: bool,
}

/// 交易流水清单项（分页查询用）。
#[derive(Debug, Clone)]
pub struct TransferRecordView {
    pub record_id: String,
    pub request_id: Option<String>,
    pub from_account_id: String,
    pub to_account_id: String,
    pub amount_cents: i64,
    pub from_balance_after_cents: i64,
    pub to_balance_after_cents: i64,
    pub remark: Option<String>,
    pub created_at: i64,
}

/// 与某条流水同一事务写入的审计日志。
#[derive(Debug, Clone)]
pub struct TransferAuditView {
    pub action: String,
    pub detail: String,
    pub created_at: i64,
}

/// 单条流水的完整视图，包含审计日志。
#[derive(Debug, Clone)]
pub struct TransferDetail {
    pub record: TransferRecordView,
    pub audits: Vec<TransferAuditView>,
}

/// 流水分页结果。
#[derive(Debug, Clone)]
pub struct TransferRecordPage {
    pub page: u64,
    pub size: u64,
    pub total: u64,
    pub items: Vec<TransferRecordView>,
}

/// 待写入的流水数据。
#[derive(Debug, Clone)]
pub struct NewTransferRecord {
    pub id: String,
    pub request_id: Option<String>,
    pub from_account_id: String,
    pub to_account_id: String,
    pub amount_cents: i64,
    pub from_balance_after_cents: i64,
    pub to_balance_after_cents: i64,
    pub remark: Option<String>,
    pub created_at: i64,
}

/// 待写入的审计日志数据。
#[derive(Debug, Clone)]
pub struct NewTransferAudit {
    pub id: String,
    pub record_id: String,
    pub action: String,
    pub detail: String,
    pub created_at: i64,
}

/// 单条流水的审计动作，集中定义避免散落的魔法字符串。
pub mod audit_action {
    pub const BALANCE_UPDATED: &str = "BALANCE_UPDATED";
    pub const RECORD_CREATED: &str = "RECORD_CREATED";
    pub const ROLLBACK_VERIFIED: &str = "ROLLBACK_VERIFIED";
}
