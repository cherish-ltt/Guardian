use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_user_security")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: i64,
    pub secret: Option<String>,
    pub pending_secret: Option<String>,
    pub pending_expires: i64,
    pub last_step: i64,
    pub recovery_hashes: String,
    pub auth_version: i64,
    pub failures: i64,
    pub blocked_until: i64,
    pub factor_failures: i64,
    pub factor_blocked_until: i64,
    pub login_challenge_hash: Option<String>,
    pub login_challenge_expires: i64,
    pub login_password_hash: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
