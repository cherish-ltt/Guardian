use sea_orm::entity::prelude::*;

/// 转账审计日志表。
///
/// 一条流水对应多条审计动作，用于演示「一次事务写入多张表」。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "transfer_audits")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub record_id: String,
    pub action: String,
    pub detail: String,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
