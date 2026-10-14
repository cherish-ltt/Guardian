use sea_orm::entity::prelude::*;

/// 转账流水表。
///
/// 与账户余额同属一个事务，任一步失败都会一起回滚。
/// `request_id` 上有唯一索引，作为幂等键使用。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "transfer_records")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 幂等键，可空以兼容历史数据；有值时唯一。
    pub request_id: Option<String>,
    pub from_account_id: String,
    pub to_account_id: String,
    pub amount_cents: i64,
    pub from_balance_after_cents: i64,
    pub to_balance_after_cents: i64,
    pub remark: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
