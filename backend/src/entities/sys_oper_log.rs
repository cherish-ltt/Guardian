use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_oper_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub module: String,
    pub action: String,
    pub request_method: Option<String>,
    pub request_url: Option<String>,
    pub request_params: Option<String>,
    pub oper_name: Option<String>,
    pub dept_name: Option<String>,
    pub ip: Option<String>,
    pub location: Option<String>,
    pub cost: Option<i64>,
    pub result: i64,
    pub error_msg: Option<String>,
    pub oper_time: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
