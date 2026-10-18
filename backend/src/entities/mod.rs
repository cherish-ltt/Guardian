//! SeaORM 实体目录。
//!
//! 洋葱架构里，实体集中放在这里，由基础设施层的仓储适配器使用。
//! 每个实体单独一个文件，也可以通过 `sea-orm-cli generate entity` 生成到该目录下。

pub mod gen_table;
pub mod gen_table_column;
pub mod sys_config;
pub mod sys_dept;
pub mod sys_dict_data;
pub mod sys_dict_type;
pub mod sys_login_log;
pub mod sys_menu;
pub mod sys_oper_log;
pub mod sys_post;
pub mod sys_role;
pub mod sys_role_dept;
pub mod sys_role_menu;
pub mod sys_user;
pub mod sys_user_post;
pub mod sys_user_role;
pub mod sys_user_security;
pub mod transfer_account;
pub mod transfer_audit;
pub mod transfer_record;
