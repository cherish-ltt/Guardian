use anyhow::{Context, Result};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};

/// 补齐历史库缺列，再建索引。
///
/// 顺序不能颠倒：`request_id` 列必须先存在，才能建幂等键唯一索引。
pub async fn create_transfer_indexes(database: &DatabaseConnection) -> Result<()> {
    apply_pending_migrations(database).await?;

    for statement in [RECORD_INDEX, RECORD_REQUEST_ID_INDEX, AUDIT_INDEX] {
        database
            .execute_unprepared(statement)
            .await
            .context("failed to create transaction example indexes")?;
    }

    Ok(())
}

/// 首次启动时写入两个演示账户，重复启动不会重复插入。
///
/// 使用 `NOT EXISTS` 而不是先查后写，让播种本身也是幂等且原子的。
/// 每个语句单独执行，避免依赖驱动的多语句支持。
pub async fn seed_transfer_accounts(database: &DatabaseConnection) -> Result<()> {
    for statement in [SEED_ALICE, SEED_BOB] {
        database
            .execute_unprepared(statement)
            .await
            .context("failed to seed transaction example accounts")?;
    }

    Ok(())
}

/// 为老版本数据库补 `transfer_records.request_id` 列。
///
/// 列已存在时返回空语句，因此本函数可重复执行。
pub async fn apply_pending_migrations(database: &DatabaseConnection) -> Result<()> {
    let statement = Statement::from_string(DbBackend::Sqlite, CHECK_RECORD_REQUEST_ID.to_string());
    let row = database
        .query_one_raw(statement)
        .await
        .context("failed to check transfer_records.request_id column")?
        .ok_or_else(|| {
            anyhow::anyhow!("checking transfer_records.request_id column returned no result")
        })?;
    let migration: String = row
        .try_get("", "migration")
        .context("failed to read migration statement")?;

    if migration.is_empty() {
        return Ok(());
    }

    database
        .execute_unprepared(&migration)
        .await
        .context("failed to add transfer_records.request_id column")?;

    Ok(())
}

const RECORD_INDEX: &str =
    "CREATE INDEX IF NOT EXISTS idx_transfer_records_created_at ON transfer_records (created_at)";

/// 幂等键唯一索引。
///
/// SQLite / MySQL / PostgreSQL 的唯一索引都允许多个 NULL，因此历史流水的空 `request_id`
/// 不会互相冲突，补列后可以直接建索引。
const RECORD_REQUEST_ID_INDEX: &str = "CREATE UNIQUE INDEX IF NOT EXISTS \
     idx_transfer_records_request_id ON transfer_records (request_id)";

const AUDIT_INDEX: &str =
    "CREATE INDEX IF NOT EXISTS idx_transfer_audits_record_id ON transfer_audits (record_id)";

/// 判断老库是否需要补列，需要时直接返回 ALTER 语句。
///
/// SQLite 不支持 `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`，先用 `pragma_table_info`
/// 判断列是否存在；表不存在时返回空语句，因为随后执行的建表语句会带上该列。
/// 换 MySQL / PostgreSQL 部署时，这里需要替换为对应的迁移工具。
const CHECK_RECORD_REQUEST_ID: &str = "SELECT CASE \
     WHEN EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'transfer_records') \
       AND NOT EXISTS (SELECT 1 FROM pragma_table_info('transfer_records') WHERE name = 'request_id') \
     THEN 'ALTER TABLE transfer_records ADD COLUMN request_id TEXT' \
     ELSE '' END AS migration";

const SEED_ALICE: &str = "INSERT INTO transfer_accounts (id, owner, balance_cents, version, updated_at) \
     SELECT 'acc_alice', 'Alice', 100000, 0, 0 \
     WHERE NOT EXISTS (SELECT 1 FROM transfer_accounts WHERE id = 'acc_alice')";

const SEED_BOB: &str = "INSERT INTO transfer_accounts (id, owner, balance_cents, version, updated_at) \
     SELECT 'acc_bob', 'Bob', 100000, 0, 0 \
     WHERE NOT EXISTS (SELECT 1 FROM transfer_accounts WHERE id = 'acc_bob')";

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, Database, DatabaseConnection, EntityTrait};
    use uuid::Uuid;

    use super::super::run_migrations;
    use crate::entities::{transfer_account, transfer_audit, transfer_record};
    use crate::infrastructure::config::SeedConfig;

    /// 测试里的播种配置：口令固定，账号用内置演示账号；生产取自 `.env`。
    fn test_seed() -> SeedConfig {
        SeedConfig {
            admin_password: "seed-test-password".to_string(),
            ..SeedConfig::default()
        }
    }

    async fn setup_database() -> (DatabaseConnection, String) {
        let url = format!(
            "sqlite://{}?mode=rwc",
            std::env::temp_dir()
                .join(format!("guardian-schema-test-{}.db", Uuid::now_v7()))
                .display()
        );

        let database = Database::connect(url.clone())
            .await
            .expect("连接临时数据库");
        (database, url)
    }

    /// 执行 sqlx 迁移 + 索引 + 播种，可重复执行。
    async fn bootstrap(database: &DatabaseConnection, url: &str) {
        run_migrations(database, url, &test_seed())
            .await
            .expect("执行迁移成功");
    }

    #[tokio::test]
    async fn tables_and_seed_are_created_once() {
        let (database, url) = setup_database().await;

        bootstrap(&database, &url).await;
        // 重复执行验证幂等性：再次启动服务不应产生重复账户或建表错误。
        bootstrap(&database, &url).await;

        let accounts = transfer_account::Entity::find()
            .all(&database)
            .await
            .expect("查询账户");
        assert_eq!(accounts.len(), 2);
        assert!(
            accounts
                .iter()
                .all(|account| account.balance_cents == 100000)
        );
        assert!(
            transfer_account::Entity::find_by_id("acc_alice".to_string())
                .one(&database)
                .await
                .expect("按主键查询")
                .is_some()
        );

        let records = transfer_record::Entity::find()
            .all(&database)
            .await
            .expect("查询流水");
        assert!(records.is_empty());

        let audits = transfer_audit::Entity::find()
            .all(&database)
            .await
            .expect("查询审计日志");
        assert!(audits.is_empty());
    }

    /// 老库（无 request_id 列）启动后应自动补列并建好唯一索引。
    #[tokio::test]
    async fn legacy_database_gets_request_id_column_and_unique_index() {
        let (database, url) = setup_database().await;

        database
            .execute_unprepared(
                "CREATE TABLE transfer_records (\
                 id TEXT PRIMARY KEY NOT NULL, \
                 from_account_id TEXT NOT NULL, \
                 to_account_id TEXT NOT NULL, \
                 amount_cents BIGINT NOT NULL, \
                 from_balance_after_cents BIGINT NOT NULL, \
                 to_balance_after_cents BIGINT NOT NULL, \
                 remark TEXT, \
                 created_at BIGINT NOT NULL)",
            )
            .await
            .expect("创建老版本表结构");
        database
            .execute_unprepared(
                "INSERT INTO transfer_records (id, from_account_id, to_account_id, amount_cents, \
                 from_balance_after_cents, to_balance_after_cents, remark, created_at) \
                 VALUES ('legacy', 'acc_alice', 'acc_bob', 1, 1, 1, NULL, 0)",
            )
            .await
            .expect("写入历史流水");

        // 真实启动流程：sqlx 迁移建表（老库同名表自动跳过）→ 补列建索引 → 播种；重复执行应幂等。
        run_migrations(&database, &url, &test_seed())
            .await
            .expect("执行迁移成功");
        run_migrations(&database, &url, &test_seed())
            .await
            .expect("重复执行迁移成功");

        let legacy = transfer_record::Entity::find_by_id("legacy".to_string())
            .one(&database)
            .await
            .expect("查询历史流水")
            .expect("历史流水存在");
        assert!(legacy.request_id.is_none());

        // 唯一索引生效：同一 request_id 再次写入必然失败。
        database
            .execute_unprepared(
                "UPDATE transfer_records SET request_id = 'dup' WHERE id = 'legacy'",
            )
            .await
            .expect("写入幂等键");
        let conflict = database
            .execute_unprepared(
                "INSERT INTO transfer_records (id, request_id, from_account_id, to_account_id, \
                 amount_cents, from_balance_after_cents, to_balance_after_cents, remark, created_at) \
                 VALUES ('other', 'dup', 'acc_alice', 'acc_bob', 1, 1, 1, NULL, 0)",
            )
            .await;
        assert!(conflict.is_err(), "重复幂等键应当被唯一索引拒绝");
    }

    /// 老库里同一字典类型存在多个默认项时，迁移后只保留排序最靠前的一项。
    #[tokio::test]
    async fn legacy_dict_defaults_are_collapsed_to_one_per_type() {
        let (database, url) = setup_database().await;

        // 先建到 0008 的状态，再写入脏数据，最后让 0009 在脏数据上执行。
        database
            .execute_unprepared(
                "CREATE TABLE sys_dict_data (\
                 id INTEGER PRIMARY KEY AUTOINCREMENT, \
                 dict_type TEXT NOT NULL, dict_label TEXT NOT NULL, dict_value TEXT NOT NULL, \
                 sort INTEGER NOT NULL DEFAULT 0, status INTEGER NOT NULL DEFAULT 1, \
                 tone TEXT, is_default INTEGER NOT NULL DEFAULT 0, remark TEXT, \
                 create_by TEXT, update_by TEXT, create_time TEXT, update_time TEXT)",
            )
            .await
            .expect("创建老版本字典数据表");
        database
            .execute_unprepared(
                "INSERT INTO sys_dict_data (id, dict_type, dict_label, dict_value, sort, is_default) \
                 VALUES (1, 'task_status', '待处理', '0', 1, 1), \
                        (2, 'task_status', '处理中', '1', 2, 1), \
                        (3, 'task_status', '已完成', '2', 3, 0), \
                        (4, 'task_status', '已关闭', '3', 4, 1), \
                        (5, 'sys_yes_no', '是', 'Y', 1, 1), \
                        (6, 'sys_yes_no', '否', 'N', 2, 0)",
            )
            .await
            .expect("写入多个默认项");

        bootstrap(&database, &url).await;

        let rows = database
            .query_all_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Sqlite,
                "SELECT id, dict_type, is_default FROM sys_dict_data ORDER BY id".to_string(),
            ))
            .await
            .expect("查询字典数据");
        let defaults = rows
            .iter()
            .filter(|row| row.try_get::<i64>("", "is_default").unwrap_or_default() == 1)
            .map(|row| row.try_get::<i64>("", "id").unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(defaults, vec![1, 5], "每个字典类型只留排序最靠前的一项");
    }
}
