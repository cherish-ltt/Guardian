use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_dict_data")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub dict_type: String,
    pub dict_label: String,
    pub dict_value: String,
    pub sort: i64,
    pub status: i64,
    pub tone: Option<String>,
    pub is_default: i64,
    pub remark: Option<String>,
    pub create_by: Option<String>,
    pub update_by: Option<String>,
    pub create_time: Option<String>,
    pub update_time: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
