use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_role_dept")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub role_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub dept_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
