use std::pin::Pin;

use sea_orm::{
    DatabaseConnection, DatabaseTransaction, SqliteTransactionMode, TransactionOptions,
    TransactionTrait,
};
use tracing::warn;

use crate::domain::error::AppError;

/// 事务闭包返回的 future 类型：只能借用事务本身，外部数据需移入闭包。
pub(crate) type TransactionFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, AppError>> + Send + 'a>>;

/// 在 IMMEDIATE 写事务里执行：SQLite 不支持事务内切换隔离级别，
/// IMMEDIATE 让 BEGIN 时就拿到写锁，避免并发读改写互相覆盖。
pub(crate) async fn in_immediate_transaction<T, F>(
    database: &DatabaseConnection,
    run: F,
) -> Result<T, AppError>
where
    T: Send,
    F: for<'a> FnOnce(&'a DatabaseTransaction) -> TransactionFuture<'a, T>,
{
    let transaction = database
        .begin_with_options(TransactionOptions {
            sqlite_transaction_mode: Some(SqliteTransactionMode::Immediate),
            ..Default::default()
        })
        .await?;

    match run(&transaction).await {
        Ok(value) => {
            transaction.commit().await?;
            Ok(value)
        },
        Err(error) => {
            // 回滚失败不覆盖业务错误：该连接会在归还连接池时被丢弃重建。
            if let Err(rollback_error) = transaction.rollback().await {
                warn!(%rollback_error, "transaction rollback failed");
            }
            Err(error)
        },
    }
}
