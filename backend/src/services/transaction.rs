use std::sync::Arc;

use async_trait::async_trait;
use chrono::Local;
use sea_orm::{
    DatabaseConnection, DatabaseTransaction, SqliteTransactionMode, TransactionOptions,
    TransactionTrait,
};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    domain::{
        error::AppError,
        models::transaction::{
            NewTransferAudit, NewTransferRecord, TransferAccountState, TransferCommand,
            TransferDetail, TransferReceipt, TransferRecordPage, audit_action,
        },
        services::transaction::TransferUseCase,
    },
    infrastructure::repositories::transaction::TransferRepository,
};

const DEFAULT_PAGE: u64 = 1;
const DEFAULT_SIZE: u64 = 10;
const MAX_SIZE: u64 = 100;
const MAX_PAGE: u64 = 10_000;

/// 转账事务示例的用例实现。
///
/// 事务边界只在这里出现一次：成功提交，任一环节失败则回滚，
/// 余额更新、流水、审计日志要么一起生效、要么一起消失。
pub struct TransferService<R: TransferRepository> {
    database: DatabaseConnection,
    repository: Arc<R>,
}

impl<R: TransferRepository> TransferService<R> {
    pub fn new(database: DatabaseConnection, repository: Arc<R>) -> Self {
        Self {
            database,
            repository,
        }
    }
}

#[async_trait]
impl<R: TransferRepository> TransferUseCase for TransferService<R> {
    async fn transfer(&self, command: TransferCommand) -> Result<TransferReceipt, AppError> {
        validate(&command)?;

        // 参数校验放在开启事务之前，避免为必然失败的请求占用连接。
        // SQLite 不支持在事务内设置隔离级别（`IsolationLevel` 会被忽略）；改用 IMMEDIATE
        // 让 BEGIN 时就拿到写锁，避免 DEFERRED 下并发读改写互相覆盖。
        let transaction = self
            .database
            .begin_with_options(TransactionOptions {
                sqlite_transaction_mode: Some(SqliteTransactionMode::Immediate),
                ..Default::default()
            })
            .await?;

        match transfer_in_transaction(&transaction, self.repository.as_ref(), &command).await {
            Ok(receipt) => {
                transaction.commit().await?;
                info!(record_id = %receipt.record_id, "transfer transaction committed");
                Ok(receipt)
            },
            Err(error) => {
                // 回滚失败不覆盖业务错误：该连接会在归还连接池时被丢弃重建。
                if let Err(rollback_error) = transaction.rollback().await {
                    warn!(%rollback_error, "transfer transaction rollback failed");
                }
                info!(%error, "transfer transaction rolled back");
                Err(error)
            },
        }
    }

    async fn get_transfer(&self, record_id: String) -> Result<TransferDetail, AppError> {
        if record_id.trim().is_empty() {
            return Err(AppError::not_found("transfer record ID must not be empty"));
        }

        let record = self
            .repository
            .find_record(&self.database, &record_id)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("transfer record {record_id} does not exist"))
            })?;
        let audits = self
            .repository
            .list_audits(&self.database, &record_id)
            .await?;

        Ok(TransferDetail { record, audits })
    }

    async fn list_transfers(
        &self,
        page: Option<u64>,
        size: Option<u64>,
    ) -> Result<TransferRecordPage, AppError> {
        let page = page.unwrap_or(DEFAULT_PAGE);
        let size = size.unwrap_or(DEFAULT_SIZE);
        if page == 0 || page > MAX_PAGE {
            return Err(AppError::bad_request(format!(
                "page must be between 1 and {MAX_PAGE}"
            )));
        }
        if size == 0 || size > MAX_SIZE {
            return Err(AppError::bad_request(format!(
                "size must be between 1 and {MAX_SIZE}"
            )));
        }

        let total = self.repository.count_records(&self.database).await?;
        let items = self
            .repository
            .list_records(&self.database, page, size)
            .await?;

        Ok(TransferRecordPage {
            page,
            size,
            total,
            items,
        })
    }
}

/// 事务内的全部读写：读账户 → 校验余额 → 更新双方余额 → 写流水与审计。
///
/// 运行在调用方开启的事务中；返回 `Err` 由调用方负责回滚。
async fn transfer_in_transaction<R: TransferRepository>(
    transaction: &DatabaseTransaction,
    repository: &R,
    command: &TransferCommand,
) -> Result<TransferReceipt, AppError> {
    let from = require_account(transaction, repository, &command.from_account_id).await?;
    let to = require_account(transaction, repository, &command.to_account_id).await?;

    let from_after = debit(&from, command.amount_cents)?;
    let to_after = credit(&to, command.amount_cents)?;
    repository
        .update_account_balance(transaction, &from_after, from.version)
        .await?;
    repository
        .update_account_balance(transaction, &to_after, to.version)
        .await?;

    let identity = RecordIdentity {
        record_id: Uuid::now_v7().to_string(),
        request_id: command.request_id.clone(),
        created_at: Local::now().timestamp_millis(),
    };
    let record = build_record(&identity, command, &from_after, &to_after);
    // 幂等键冲突会在这里返回唯一约束错误，由本函数的上层回滚整个事务。
    repository.insert_record(transaction, &record).await?;

    let changes = BalanceChanges {
        from_before: &from,
        from_after: &from_after,
        to_before: &to,
        to_after: &to_after,
    };
    let mut audits = balance_audits(&identity, &changes);
    if command.force_fail {
        // 写完全部数据后再失败，用来证明回滚确实撤销了已写入的内容。
        audits.push(build_audit(
            &identity.record_id,
            audit_action::ROLLBACK_VERIFIED,
            "debug switch triggered: all writes in the transaction will be rolled back",
            identity.created_at,
        ));
    }

    for audit in &audits {
        repository.insert_audit(transaction, audit).await?;
    }

    if command.force_fail {
        return Err(AppError::internal(
            "force_fail enabled: failing after all writes within the transaction",
        ));
    }

    Ok(TransferReceipt {
        record_id: identity.record_id,
        request_id: identity.request_id,
        from_account_id: command.from_account_id.clone(),
        to_account_id: command.to_account_id.clone(),
        amount_cents: command.amount_cents,
        from_balance_after_cents: from_after.balance_cents,
        to_balance_after_cents: to_after.balance_cents,
        remark: command.remark.clone(),
        created_at: identity.created_at,
        committed: true,
        rolled_back: false,
    })
}

/// 事务内读取账户，不存在时直接失败以触发回滚。
async fn require_account<R: TransferRepository>(
    transaction: &DatabaseTransaction,
    repository: &R,
    account_id: &str,
) -> Result<TransferAccountState, AppError> {
    repository
        .find_account(transaction, account_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("account {account_id} does not exist")))
}

fn validate(command: &TransferCommand) -> Result<(), AppError> {
    if command.from_account_id.trim().is_empty() || command.to_account_id.trim().is_empty() {
        return Err(AppError::bad_request("account ID must not be empty"));
    }
    if command.from_account_id == command.to_account_id {
        return Err(AppError::bad_request(
            "from account and to account must not be the same",
        ));
    }
    if command.amount_cents <= 0 {
        return Err(AppError::bad_request(
            "transfer amount must be greater than 0",
        ));
    }

    Ok(())
}

/// 扣减余额并自增版本号；余额不足时返回业务错误，由调用方触发回滚。
fn debit(
    account: &TransferAccountState,
    amount_cents: i64,
) -> Result<TransferAccountState, AppError> {
    if account.balance_cents < amount_cents {
        return Err(AppError::bad_request(format!(
            "account {} has insufficient balance: available {} cents, need {amount_cents} cents",
            account.id, account.balance_cents
        )));
    }

    Ok(TransferAccountState {
        balance_cents: account.balance_cents - amount_cents,
        version: account.version + 1,
        ..account.clone()
    })
}

/// 增加余额并自增版本号；溢出时显式失败，不做静默截断。
fn credit(
    account: &TransferAccountState,
    amount_cents: i64,
) -> Result<TransferAccountState, AppError> {
    let balance_cents = account
        .balance_cents
        .checked_add(amount_cents)
        .ok_or_else(|| {
            AppError::bad_request("balance after transfer exceeds the representable range")
        })?;

    Ok(TransferAccountState {
        balance_cents,
        version: account.version + 1,
        ..account.clone()
    })
}

/// 一次事务写入的身份信息：主键、幂等键与时间戳。
struct RecordIdentity {
    record_id: String,
    request_id: Option<String>,
    created_at: i64,
}

fn build_record(
    identity: &RecordIdentity,
    command: &TransferCommand,
    from_after: &TransferAccountState,
    to_after: &TransferAccountState,
) -> NewTransferRecord {
    NewTransferRecord {
        id: identity.record_id.clone(),
        request_id: identity.request_id.clone(),
        from_account_id: command.from_account_id.clone(),
        to_account_id: command.to_account_id.clone(),
        amount_cents: command.amount_cents,
        from_balance_after_cents: from_after.balance_cents,
        to_balance_after_cents: to_after.balance_cents,
        remark: command.remark.clone(),
        created_at: identity.created_at,
    }
}

/// 一次转账前后的双方账户，用于生成审计明细。
struct BalanceChanges<'a> {
    from_before: &'a TransferAccountState,
    from_after: &'a TransferAccountState,
    to_before: &'a TransferAccountState,
    to_after: &'a TransferAccountState,
}

/// 一次事务内的两条审计记录：余额变更与流水创建。
fn balance_audits(
    identity: &RecordIdentity,
    changes: &BalanceChanges<'_>,
) -> Vec<NewTransferAudit> {
    vec![
        build_audit(
            &identity.record_id,
            audit_action::BALANCE_UPDATED,
            balance_detail(changes),
            identity.created_at,
        ),
        build_audit(
            &identity.record_id,
            audit_action::RECORD_CREATED,
            "transfer record written",
            identity.created_at,
        ),
    ]
}

fn balance_detail(changes: &BalanceChanges<'_>) -> String {
    format!(
        "{} balance {} -> {}, {} balance {} -> {}",
        changes.from_before.id,
        changes.from_before.balance_cents,
        changes.from_after.balance_cents,
        changes.to_before.id,
        changes.to_before.balance_cents,
        changes.to_after.balance_cents
    )
}

fn build_audit(
    record_id: &str,
    action: &str,
    detail: impl Into<String>,
    created_at: i64,
) -> NewTransferAudit {
    NewTransferAudit {
        id: Uuid::now_v7().to_string(),
        record_id: record_id.to_string(),
        action: action.to_string(),
        detail: detail.into(),
        created_at,
    }
}

#[cfg(test)]
mod tests {
    use sea_orm::{Database, DatabaseConnection};

    use super::TransferService;
    use crate::infrastructure::repositories::transaction::SeaOrmTransferRepository as SeaOrmRepo;
    use crate::{
        domain::{
            error::AppError,
            models::transaction::{TransferCommand, audit_action},
            services::transaction::TransferUseCase,
        },
        entities::{transfer_account, transfer_record},
        infrastructure::{
            databases::run_migrations, repositories::transaction::SeaOrmTransferRepository,
        },
    };
    use sea_orm::EntityTrait;
    use std::sync::Arc;
    use uuid::Uuid;

    const ALICE: &str = "acc_alice";
    const BOB: &str = "acc_bob";
    const SEED_BALANCE: i64 = 100000;

    /// 每个用例独立建库并跑一遍建表与播种，互不共享状态。
    async fn setup() -> (TransferService<SeaOrmRepo>, DatabaseConnection) {
        let url = format!(
            "sqlite://{}?mode=rwc",
            std::env::temp_dir()
                .join(format!("guardian-transfer-test-{}.db", Uuid::now_v7()))
                .display()
        );
        let database = Database::connect(url.clone())
            .await
            .expect("连接临时数据库");
        run_migrations(&database, &url, "seed-test-password")
            .await
            .expect("执行迁移与播种成功");

        let repository = Arc::new(SeaOrmTransferRepository);
        let service = TransferService::new(database.clone(), repository);

        (service, database)
    }

    fn command(from: &str, to: &str, amount_cents: i64) -> TransferCommand {
        TransferCommand {
            from_account_id: from.to_string(),
            to_account_id: to.to_string(),
            amount_cents,
            remark: Some("单测转账".to_string()),
            request_id: None,
            force_fail: false,
        }
    }

    async fn balance(database: &DatabaseConnection, account_id: &str) -> i64 {
        transfer_account::Entity::find_by_id(account_id.to_string())
            .one(database)
            .await
            .expect("查询账户")
            .expect("账户存在")
            .balance_cents
    }

    async fn account_version(database: &DatabaseConnection, account_id: &str) -> i64 {
        transfer_account::Entity::find_by_id(account_id.to_string())
            .one(database)
            .await
            .expect("查询账户")
            .expect("账户存在")
            .version
    }

    async fn record_count(database: &DatabaseConnection) -> usize {
        transfer_record::Entity::find()
            .all(database)
            .await
            .expect("查询流水")
            .len()
    }

    #[tokio::test]
    async fn transfer_commits_balances_record_and_audits() {
        let (service, database) = setup().await;

        let receipt = service
            .transfer(command(ALICE, BOB, 25000))
            .await
            .expect("转账成功");

        assert!(receipt.committed);
        assert!(!receipt.rolled_back);
        assert_eq!(receipt.from_balance_after_cents, SEED_BALANCE - 25000);
        assert_eq!(receipt.to_balance_after_cents, SEED_BALANCE + 25000);
        assert_eq!(balance(&database, ALICE).await, 75000);
        assert_eq!(balance(&database, BOB).await, 125000);
        assert_eq!(account_version(&database, ALICE).await, 1);
        assert_eq!(account_version(&database, BOB).await, 1);

        let detail = service
            .get_transfer(receipt.record_id.clone())
            .await
            .expect("查询流水详情");
        assert_eq!(detail.record.amount_cents, 25000);
        assert_eq!(detail.audits.len(), 2);
        assert_eq!(detail.audits[0].action, audit_action::BALANCE_UPDATED);
        assert_eq!(detail.audits[1].action, audit_action::RECORD_CREATED);
    }

    #[tokio::test]
    async fn rollback_keeps_database_unchanged_when_balance_is_insufficient() {
        let (service, database) = setup().await;

        let error = service
            .transfer(command(ALICE, BOB, SEED_BALANCE + 1))
            .await
            .expect_err("余额不足应当失败");

        assert!(matches!(error, AppError::BadRequest(_)));
        assert_eq!(balance(&database, ALICE).await, SEED_BALANCE);
        assert_eq!(balance(&database, BOB).await, SEED_BALANCE);
        assert_eq!(account_version(&database, ALICE).await, 0);
        assert_eq!(record_count(&database).await, 0);
    }

    #[tokio::test]
    async fn rollback_discards_all_writes_when_force_fail_is_on() {
        let (service, database) = setup().await;

        let mut failing = command(ALICE, BOB, 10000);
        failing.force_fail = true;
        let error = service.transfer(failing).await.expect_err("强制失败");

        assert!(matches!(error, AppError::Internal(_)));
        // 四次写操作（两个账户、一条流水、两条审计）全部被回滚。
        assert_eq!(balance(&database, ALICE).await, SEED_BALANCE);
        assert_eq!(balance(&database, BOB).await, SEED_BALANCE);
        assert_eq!(account_version(&database, ALICE).await, 0);
        assert_eq!(record_count(&database).await, 0);
    }

    /// 跨表写入过程中命中唯一约束：账户余额更新、流水、审计全部回滚。
    #[tokio::test]
    async fn duplicate_request_id_rolls_back_every_table() {
        let (service, database) = setup().await;

        let mut first = command(ALICE, BOB, 20000);
        first.request_id = Some("req-dup-001".to_string());
        let committed = service.transfer(first).await.expect("首次提交成功");
        assert_eq!(committed.request_id.as_deref(), Some("req-dup-001"));
        assert_eq!(balance(&database, ALICE).await, 80000);
        assert_eq!(balance(&database, BOB).await, 120000);
        assert_eq!(record_count(&database).await, 1);

        let mut duplicate = command(ALICE, BOB, 5000);
        duplicate.request_id = Some("req-dup-001".to_string());
        let error = service
            .transfer(duplicate)
            .await
            .expect_err("重复幂等键必须失败");

        assert!(
            matches!(error, AppError::Conflict(_)),
            "唯一约束冲突应映射为 409 冲突，实际: {error:?}"
        );
        // 第二次的余额更新与写入全部被回滚，只剩首次提交的结果。
        assert_eq!(balance(&database, ALICE).await, 80000);
        assert_eq!(balance(&database, BOB).await, 120000);
        assert_eq!(record_count(&database).await, 1);
    }

    /// 并发扣减不会丢失更新，也不会把锁竞争暴露成数据库错误。
    ///
    /// alice 初始 100000，5 笔各 30000 同时发起：只可能有 3 笔成功，且失败的 2 笔必须是
    /// 「余额不足」（读到已提交的余额）而非 `AppError::Database`。写事务以 BEGIN IMMEDIATE
    /// 开启，写锁在事务开始时就取得，不会出现 DEFERRED 下「读后升级锁」的冲突窗口。
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_transfers_serialize_without_losing_updates() {
        let (service, database) = setup().await;
        let service = Arc::new(service);

        let mut handles = Vec::new();
        for _ in 0..5 {
            let service = Arc::clone(&service);
            handles.push(tokio::spawn(async move {
                service.transfer(command(ALICE, BOB, 30000)).await
            }));
        }

        let mut outcomes = Vec::new();
        for handle in handles {
            outcomes.push(handle.await.expect("并发任务不应 panic"));
        }

        let committed = outcomes.iter().filter(|outcome| outcome.is_ok()).count();
        let rejected = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Err(AppError::BadRequest(_))))
            .count();
        assert_eq!(committed, 3, "只有 3 笔能扣满 100000");
        assert_eq!(
            rejected, 2,
            "其余请求应是余额不足而非数据库错误: {outcomes:?}"
        );

        assert_eq!(balance(&database, ALICE).await, 10000);
        assert_eq!(balance(&database, BOB).await, 190000);
        assert_eq!(record_count(&database).await, 3);
    }

    #[tokio::test]
    async fn missing_account_is_reported_as_not_found() {
        let (service, database) = setup().await;

        let error = service
            .transfer(command(ALICE, "acc_missing", 100))
            .await
            .expect_err("账户不存在应当失败");

        assert!(matches!(error, AppError::NotFound(_)));
        assert_eq!(balance(&database, ALICE).await, SEED_BALANCE);
        assert_eq!(record_count(&database).await, 0);
    }

    #[tokio::test]
    async fn invalid_commands_are_rejected_before_transaction() {
        let (service, _database) = setup().await;

        let cases = [
            command("", BOB, 100),
            command(ALICE, ALICE, 100),
            command(ALICE, BOB, 0),
            command(ALICE, BOB, -1),
        ];

        for case in cases {
            let error = service.transfer(case).await.expect_err("非法参数应当失败");
            assert!(matches!(error, AppError::BadRequest(_)));
        }
    }

    #[tokio::test]
    async fn credit_overflow_is_rejected() {
        let (service, database) = setup().await;

        // 先把转入账户余额推到接近 i64 上限，再触发溢出分支。
        let account = transfer_account::Entity::find_by_id(BOB.to_string())
            .one(&database)
            .await
            .expect("查询账户")
            .expect("账户存在");
        use sea_orm::{ActiveModelTrait, Set};
        transfer_account::ActiveModel {
            id: Set(account.id),
            balance_cents: Set(i64::MAX),
            ..Default::default()
        }
        .update(&database)
        .await
        .expect("更新余额");

        let error = service
            .transfer(command(ALICE, BOB, 1))
            .await
            .expect_err("溢出应当失败");

        assert!(matches!(error, AppError::BadRequest(_)));
        assert_eq!(balance(&database, ALICE).await, SEED_BALANCE);
    }

    #[tokio::test]
    async fn list_transfers_paginates_newest_first() {
        let (service, _database) = setup().await;

        for amount in [1000, 2000, 3000] {
            service
                .transfer(command(ALICE, BOB, amount))
                .await
                .expect("转账成功");
        }

        let page = service
            .list_transfers(Some(1), Some(2))
            .await
            .expect("分页查询");
        assert_eq!(page.page, 1);
        assert_eq!(page.size, 2);
        assert_eq!(page.total, 3);
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].amount_cents, 3000);

        let remaining = service
            .list_transfers(Some(2), Some(2))
            .await
            .expect("第二页");
        assert_eq!(remaining.items.len(), 1);
        assert_eq!(remaining.items[0].amount_cents, 1000);
    }

    #[tokio::test]
    async fn list_transfers_rejects_invalid_paging() {
        let (service, _database) = setup().await;

        assert!(matches!(
            service.list_transfers(Some(0), None).await,
            Err(AppError::BadRequest(_))
        ));
        // 页码超过上限直接拒绝，避免 (page - 1) * size 在查询层溢出。
        assert!(matches!(
            service
                .list_transfers(Some(super::MAX_PAGE + 1), None)
                .await,
            Err(AppError::BadRequest(_))
        ));
        assert!(matches!(
            service.list_transfers(None, Some(0)).await,
            Err(AppError::BadRequest(_))
        ));
        assert!(matches!(
            service.list_transfers(None, Some(101)).await,
            Err(AppError::BadRequest(_))
        ));
        // 不传分页参数时使用默认值：第 1 页、每页 10 条。
        let page = service.list_transfers(None, None).await.expect("默认分页");
        assert_eq!((page.page, page.size), (1, 10));
    }

    #[tokio::test]
    async fn get_transfer_rejects_empty_and_unknown_id() {
        let (service, _database) = setup().await;

        assert!(matches!(
            service.get_transfer("   ".to_string()).await,
            Err(AppError::NotFound(_))
        ));
        assert!(matches!(
            service.get_transfer("missing-record".to_string()).await,
            Err(AppError::NotFound(_))
        ));
    }
}
