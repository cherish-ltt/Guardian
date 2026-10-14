use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_menu")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub parent_id: i64,
    pub menu_name: String,
    pub menu_type: String,
    pub icon: Option<String>,
    pub path: Option<String>,
    pub component: Option<String>,
    pub permission: Option<String>,
    pub sort: i64,
    pub visible: i64,
    pub status: i64,
    pub create_by: Option<String>,
    pub update_by: Option<String>,
    pub create_time: Option<String>,
    pub update_time: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
