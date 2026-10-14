use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_role")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub role_name: String,
    pub role_code: String,
    pub data_scope: String,
    pub sort: i64,
    pub status: i64,
    pub builtin: i64,
    pub remark: Option<String>,
    pub deleted: i64,
    pub create_by: Option<String>,
    pub update_by: Option<String>,
    pub create_time: Option<String>,
    pub update_time: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
