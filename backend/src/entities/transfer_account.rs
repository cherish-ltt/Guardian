use sea_orm::entity::prelude::*;

/// 转账演练账户表。
///
/// `version` 每次余额变更都自增，并作为乐观锁条件参与 `UPDATE ... WHERE version = <读取时的值>`，
/// 防止并发下的丢失更新（见 `infrastructure/repositories/transaction.rs`）。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "transfer_accounts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub owner: String,
    pub balance_cents: i64,
    pub version: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
