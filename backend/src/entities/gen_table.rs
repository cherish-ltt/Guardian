use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "gen_table")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub table_name: String,
    pub table_comment: Option<String>,
    pub class_name: String,
    pub module_name: String,
    pub business_name: String,
    pub synced: i64,
    pub create_by: Option<String>,
    pub update_by: Option<String>,
    pub create_time: Option<String>,
    pub update_time: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
