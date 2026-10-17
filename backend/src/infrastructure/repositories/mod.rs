//! 基础设施层的仓储适配器目录。
//!
//! 仓储接口与 SeaORM 实现都放在这里：接口签名需要引用 `sea_orm::ConnectionTrait`，
//! 放在 `domain` 会破坏「领域层不依赖 sea-orm」的边界。
//!
//! 后台管理的仓储按聚合拆成多个小接口（用户 / 角色 / 菜单 / 部门 / 岗位 / 字典 /
//! 参数 / 审计日志 / 权限图 / 监控 / 工作台 / 代码生成），每个接口一个文件。
//! 适配器都是无状态的单元结构体，由 `services` 在调用点直接使用；
//! 需要替换实现时替换该结构体即可，`container.rs` 只负责装配数据库连接与用例实现。

pub mod account;
pub mod admin_auth;
pub mod audit_log;
pub mod config;
pub mod dashboard;
pub mod dept;
pub mod dict;
pub mod generator;
pub mod menu;
pub mod monitor;
pub mod permission_graph;
pub mod post;
pub mod role;
pub mod sql;
pub mod transaction;
pub mod user;
