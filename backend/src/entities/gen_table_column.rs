use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "gen_table_column")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub table_id: i64,
    pub column_name: String,
    pub java_field: String,
    pub java_type: String,
    pub jdbc_type: String,
    pub column_comment: Option<String>,
    pub is_insert: i64,
    pub is_edit: i64,
    pub is_list: i64,
    pub is_query: i64,
    pub query_type: String,
    pub form_type: String,
    pub required: i64,
    pub sort: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
