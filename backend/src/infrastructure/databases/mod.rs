use std::time::Duration;

use anyhow::{Context, Result};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tokio::time::{Duration as TokioDuration, timeout};

use crate::infrastructure::config::DatabaseConfig;

pub mod schema;
pub mod seed;

pub async fn connect_database(config: &DatabaseConfig) -> Result<DatabaseConnection> {
    let mut options = ConnectOptions::new(config.url.clone());
    options
        .min_connections(config.min_connections)
        .max_connections(config.max_connections)
        .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
        .idle_timeout(Duration::from_secs(config.idle_secs))
        .sqlx_logging(config.sqlx_logging);

    let database = Database::connect(options)
        .await
        .with_context(|| format!("database connection failed: {}", config.url))?;

    ping_database(&database, config.connect_timeout_secs).await?;

    Ok(database)
}

pub async fn ping_database(database: &DatabaseConnection, timeout_secs: u64) -> Result<()> {
    timeout(TokioDuration::from_secs(timeout_secs), database.ping())
        .await
        .context("database health check timed out")?
        .context("database health check failed")?;

    Ok(())
}

pub async fn run_migrations(
    database: &DatabaseConnection,
    url: &str,
    seed_admin_password: &str,
) -> Result<()> {
    // 表结构由 sqlx migrate 管理（见 migrations/ 目录），迁移记录写入 _sqlx_migrations 表。
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(url)
        .await
        .context("failed to connect to the migration database")?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run database migrations")?;
    pool.close().await;

    // 老库兼容与索引、种子留在 Rust：
    // - 老库缺列的补列逻辑依赖 SQLite pragma（sqlx 迁移文件无法表达 conditional 补列）；
    // - 幂等键唯一索引必须等补列完成后才能建；
    // - 种子数据用 NOT EXISTS 保持幂等。
    schema::create_transfer_indexes(database).await?;
    schema::seed_transfer_accounts(database).await?;
    seed::seed_admin_data(database, seed_admin_password).await?;

    Ok(())
}
