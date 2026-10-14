use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_config")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub config_name: String,
    pub config_key: String,
    pub config_value: String,
    pub config_type: String,
    pub builtin: i64,
    pub remark: Option<String>,
    pub create_by: Option<String>,
    pub update_by: Option<String>,
    pub create_time: Option<String>,
    pub update_time: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
