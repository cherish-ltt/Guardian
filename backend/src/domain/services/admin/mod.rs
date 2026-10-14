//! 系统管理的用例接口。
//!
//! 按页面拆成多个小 trait（用户 / 角色 / 菜单 / 部门 / 岗位 / 字典 / 参数 / 日志），
//! 每个 trait 在自己的文件里由 `services/admin/*` 实现；Rust 不允许把一个 trait 的实现
//! 拆到多个 `impl` 块，拆 trait 是既能分文件又不重复签名的做法。
//!
//! [`AdminUseCase`] 是它们的合并视图，`container.rs` 与 HTTP 层只依赖它。

pub mod configs;
pub mod depts;
pub mod dicts;
pub mod graph;
pub mod logs;
pub mod menus;
pub mod posts;
pub mod roles;
pub mod users;

/// 系统管理的全部用例。
pub trait AdminUseCase:
    users::UserAdminUseCase
    + roles::RoleAdminUseCase
    + graph::GraphAdminUseCase
    + menus::MenuAdminUseCase
    + depts::DeptAdminUseCase
    + posts::PostAdminUseCase
    + dicts::DictAdminUseCase
    + configs::ConfigAdminUseCase
    + logs::LogAdminUseCase
    + Send
    + Sync
{
}
