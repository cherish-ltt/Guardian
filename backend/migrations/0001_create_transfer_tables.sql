-- 事务示例的三张表（列名与 src/entities 中的实体一一对应）。
-- 由 sqlx migrate 管理，老库已有同名表时自动跳过（IF NOT EXISTS）。

CREATE TABLE IF NOT EXISTS transfer_accounts (
    id TEXT PRIMARY KEY NOT NULL,
    owner TEXT NOT NULL,
    balance_cents BIGINT NOT NULL,
    version BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS transfer_records (
    id TEXT PRIMARY KEY NOT NULL,
    request_id TEXT,
    from_account_id TEXT NOT NULL,
    to_account_id TEXT NOT NULL,
    amount_cents BIGINT NOT NULL,
    from_balance_after_cents BIGINT NOT NULL,
    to_balance_after_cents BIGINT NOT NULL,
    remark TEXT,
    created_at BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS transfer_audits (
    id TEXT PRIMARY KEY NOT NULL,
    record_id TEXT NOT NULL,
    action TEXT NOT NULL,
    detail TEXT NOT NULL,
    created_at BIGINT NOT NULL
);