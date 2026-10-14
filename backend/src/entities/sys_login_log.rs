use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_login_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub username: Option<String>,
    pub ip: Option<String>,
    pub location: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub status: i64,
    pub msg: Option<String>,
    pub login_time: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
