use async_trait::async_trait;
use chrono::Local;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, sea_query::Expr,
};

use crate::{
    domain::models::transaction::{
        NewTransferAudit, NewTransferRecord, TransferAccountState, TransferAuditView,
        TransferRecordView,
    },
    entities::{transfer_account, transfer_audit, transfer_record},
    error::AppError,
};

/// 转账示例的仓储接口。
///
/// 每个方法都接收 `C: ConnectionTrait`：传入 `&DatabaseConnection` 表示不进事务，
/// 传入 `&DatabaseTransaction` 表示在调用方开启的事务内执行。方法本身不开启事务，
/// 事务边界由 `services/transaction.rs` 统一掌控。
#[async_trait]
pub trait TransferRepository: Send + Sync {
    async fn find_account<C>(
        &self,
        db: &C,
        id: &str,
    ) -> Result<Option<TransferAccountState>, AppError>
    where
        C: ConnectionTrait;

    /// 以乐观锁更新账户余额：只有当库中 `version` 仍等于 `expected_version` 时才写入，
    /// 并将 `version` 自增到 `account.version`。影响行数为 0 表示期间被其他事务改过，
    /// 返回 409 而不是静默覆盖。
    async fn update_account_balance<C>(
        &self,
        db: &C,
        account: &TransferAccountState,
        expected_version: i64,
    ) -> Result<(), AppError>
    where
        C: ConnectionTrait;

    async fn insert_record<C>(&self, db: &C, record: &NewTransferRecord) -> Result<(), AppError>
    where
        C: ConnectionTrait;

    async fn insert_audit<C>(&self, db: &C, audit: &NewTransferAudit) -> Result<(), AppError>
    where
        C: ConnectionTrait;

    async fn find_record<C>(
        &self,
        db: &C,
        record_id: &str,
    ) -> Result<Option<TransferRecordView>, AppError>
    where
        C: ConnectionTrait;

    async fn list_audits<C>(
        &self,
        db: &C,
        record_id: &str,
    ) -> Result<Vec<TransferAuditView>, AppError>
    where
        C: ConnectionTrait;

    async fn count_records<C>(&self, db: &C) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn list_records<C>(
        &self,
        db: &C,
        page: u64,
        size: u64,
    ) -> Result<Vec<TransferRecordView>, AppError>
    where
        C: ConnectionTrait;
}

/// 基于 SeaORM 的仓储实现。
#[derive(Debug, Default, Clone, Copy)]
pub struct SeaOrmTransferRepository;

#[async_trait]
impl TransferRepository for SeaOrmTransferRepository {
    async fn find_account<C>(
        &self,
        db: &C,
        id: &str,
    ) -> Result<Option<TransferAccountState>, AppError>
    where
        C: ConnectionTrait,
    {
        let account = transfer_account::Entity::find_by_id(id.to_string())
            .one(db)
            .await?;

        Ok(account.map(TransferAccountState::from))
    }

    async fn update_account_balance<C>(
        &self,
        db: &C,
        account: &TransferAccountState,
        expected_version: i64,
    ) -> Result<(), AppError>
    where
        C: ConnectionTrait,
    {
        let updated = transfer_account::Entity::update_many()
            .col_expr(
                transfer_account::Column::BalanceCents,
                Expr::value(account.balance_cents),
            )
            .col_expr(
                transfer_account::Column::Version,
                Expr::value(account.version),
            )
            .col_expr(
                transfer_account::Column::UpdatedAt,
                Expr::value(Local::now().timestamp_millis()),
            )
            .filter(transfer_account::Column::Id.eq(account.id.clone()))
            .filter(transfer_account::Column::Version.eq(expected_version))
            .exec(db)
            .await?;

        if updated.rows_affected == 0 {
            return Err(AppError::conflict(format!(
                "account {} was modified concurrently or no longer exists",
                account.id
            )));
        }

        Ok(())
    }

    async fn insert_record<C>(&self, db: &C, record: &NewTransferRecord) -> Result<(), AppError>
    where
        C: ConnectionTrait,
    {
        let active = transfer_record::ActiveModel {
            id: Set(record.id.clone()),
            request_id: Set(record.request_id.clone()),
            from_account_id: Set(record.from_account_id.clone()),
            to_account_id: Set(record.to_account_id.clone()),
            amount_cents: Set(record.amount_cents),
            from_balance_after_cents: Set(record.from_balance_after_cents),
            to_balance_after_cents: Set(record.to_balance_after_cents),
            remark: Set(record.remark.clone()),
            created_at: Set(record.created_at),
        };
        active.insert(db).await?;

        Ok(())
    }

    async fn insert_audit<C>(&self, db: &C, audit: &NewTransferAudit) -> Result<(), AppError>
    where
        C: ConnectionTrait,
    {
        let active = transfer_audit::ActiveModel {
            id: Set(audit.id.clone()),
            record_id: Set(audit.record_id.clone()),
            action: Set(audit.action.clone()),
            detail: Set(audit.detail.clone()),
            created_at: Set(audit.created_at),
        };
        active.insert(db).await?;

        Ok(())
    }

    async fn find_record<C>(
        &self,
        db: &C,
        record_id: &str,
    ) -> Result<Option<TransferRecordView>, AppError>
    where
        C: ConnectionTrait,
    {
        let record = transfer_record::Entity::find_by_id(record_id.to_string())
            .one(db)
            .await?;

        Ok(record.map(TransferRecordView::from))
    }

    async fn list_audits<C>(
        &self,
        db: &C,
        record_id: &str,
    ) -> Result<Vec<TransferAuditView>, AppError>
    where
        C: ConnectionTrait,
    {
        let audits = transfer_audit::Entity::find()
            .filter(transfer_audit::Column::RecordId.eq(record_id))
            .order_by_asc(transfer_audit::Column::CreatedAt)
            .order_by_asc(transfer_audit::Column::Id)
            .all(db)
            .await?;

        Ok(audits.into_iter().map(TransferAuditView::from).collect())
    }

    async fn count_records<C>(&self, db: &C) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        let total = transfer_record::Entity::find().count(db).await?;

        Ok(total)
    }

    async fn list_records<C>(
        &self,
        db: &C,
        page: u64,
        size: u64,
    ) -> Result<Vec<TransferRecordView>, AppError>
    where
        C: ConnectionTrait,
    {
        // page 与 size 由 service 层校验（1..=MAX_PAGE、1..=MAX_SIZE），
        // 此处仍用 checked_sub / checked_mul 兜底，防御未来被其他调用方直接使用。
        let offset = page
            .checked_sub(1)
            .and_then(|page| page.checked_mul(size))
            .ok_or_else(|| {
                AppError::bad_request("pagination offset exceeds the representable range")
            })?;
        let records = transfer_record::Entity::find()
            .order_by_desc(transfer_record::Column::CreatedAt)
            .offset(offset)
            .limit(size)
            .all(db)
            .await?;

        Ok(records.into_iter().map(TransferRecordView::from).collect())
    }
}

impl From<transfer_account::Model> for TransferAccountState {
    fn from(model: transfer_account::Model) -> Self {
        Self {
            id: model.id,
            owner: model.owner,
            balance_cents: model.balance_cents,
            version: model.version,
        }
    }
}

impl From<transfer_record::Model> for TransferRecordView {
    fn from(model: transfer_record::Model) -> Self {
        Self {
            record_id: model.id,
            request_id: model.request_id,
            from_account_id: model.from_account_id,
            to_account_id: model.to_account_id,
            amount_cents: model.amount_cents,
            from_balance_after_cents: model.from_balance_after_cents,
            to_balance_after_cents: model.to_balance_after_cents,
            remark: model.remark,
            created_at: model.created_at,
        }
    }
}

impl From<transfer_audit::Model> for TransferAuditView {
    fn from(model: transfer_audit::Model) -> Self {
        Self {
            action: model.action,
            detail: model.detail,
            created_at: model.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use sea_orm::{Database, DatabaseConnection};
    use uuid::Uuid;

    use super::{SeaOrmTransferRepository, TransferRepository};
    use crate::{
        domain::{error::AppError, models::transaction::TransferAccountState},
        infrastructure::databases::run_migrations,
    };

    const ALICE: &str = "acc_alice";

    async fn setup() -> DatabaseConnection {
        let url = format!(
            "sqlite://{}?mode=rwc",
            std::env::temp_dir()
                .join(format!("guardian-repository-test-{}.db", Uuid::now_v7()))
                .display()
        );
        let database = Database::connect(url.clone())
            .await
            .expect("连接临时数据库");
        run_migrations(&database, &url)
            .await
            .expect("执行迁移与播种成功");

        database
    }

    /// 读改写期间版本被他人推进后，旧版本写入必须被拒绝而不是静默覆盖。
    #[tokio::test]
    async fn update_with_stale_version_is_rejected_as_conflict() {
        let database = setup().await;
        let repository = SeaOrmTransferRepository;

        let account = repository
            .find_account(&database, ALICE)
            .await
            .expect("查询账户")
            .expect("账户存在");
        assert_eq!(account.version, 0);

        let first = TransferAccountState {
            balance_cents: account.balance_cents - 1000,
            version: account.version + 1,
            ..account.clone()
        };
        repository
            .update_account_balance(&database, &first, account.version)
            .await
            .expect("首次更新应成功");

        let stale = TransferAccountState {
            balance_cents: account.balance_cents - 2000,
            version: account.version + 1,
            ..account.clone()
        };
        let error = repository
            .update_account_balance(&database, &stale, account.version)
            .await
            .expect_err("过期版本必须冲突");
        assert!(
            matches!(error, AppError::Conflict(_)),
            "过期版本应映射为 409 冲突，实际: {error:?}"
        );

        let reloaded = repository
            .find_account(&database, ALICE)
            .await
            .expect("重新查询")
            .expect("账户存在");
        assert_eq!(reloaded.balance_cents, first.balance_cents);
        assert_eq!(reloaded.version, 1);
    }

    /// 乐观锁条件不满足（账户不存在或版本已被推进）时返回 409，不得留下中间状态。
    #[tokio::test]
    async fn update_with_unknown_account_is_rejected_as_conflict() {
        let database = setup().await;
        let repository = SeaOrmTransferRepository;

        let missing = TransferAccountState {
            id: "acc_missing".to_string(),
            owner: "nobody".to_string(),
            balance_cents: 1,
            version: 1,
        };
        let error = repository
            .update_account_balance(&database, &missing, 0)
            .await
            .expect_err("账户不存在时也应冲突");
        assert!(matches!(error, AppError::Conflict(_)));
    }
}
