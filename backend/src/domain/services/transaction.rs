use async_trait::async_trait;

use crate::domain::{
    error::AppError,
    models::transaction::{TransferCommand, TransferDetail, TransferReceipt, TransferRecordPage},
};

/// 转账事务示例的用例入口。
///
/// 实现方（`services/transaction.rs`）负责事务边界：开启事务、读写、提交或回滚。
#[async_trait]
pub trait TransferUseCase: Send + Sync {
    /// 在一个事务内完成：读账户 → 校验 → 扣减/增加余额 → 写流水与审计 → 提交。
    async fn transfer(&self, command: TransferCommand) -> Result<TransferReceipt, AppError>;

    /// 查询单条流水及其同一事务写入的审计日志。
    async fn get_transfer(&self, record_id: String) -> Result<TransferDetail, AppError>;

    /// 分页查询流水。
    async fn list_transfers(
        &self,
        page: Option<u64>,
        size: Option<u64>,
    ) -> Result<TransferRecordPage, AppError>;
}
