use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_file")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub original_name: String,
    pub storage_name: String,
    pub url: String,
    pub content_type: Option<String>,
    pub size: i64,
    pub create_by: Option<String>,
    pub update_by: Option<String>,
    pub create_time: Option<String>,
    pub update_time: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
