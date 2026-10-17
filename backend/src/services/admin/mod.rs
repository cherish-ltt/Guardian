use std::collections::HashMap;

use sea_orm::{DatabaseConnection, DatabaseTransaction};

use crate::{
    domain::models::system_admin::{DeptFlatRow, DeptNode, MenuFlatRow, MenuNode},
    error::AppError,
};

mod configs;
mod depts;
mod dicts;
mod graph;
mod logs;
mod menus;
mod posts;
mod roles;
mod users;

/// 系统管理的用例实现。
///
/// 事务边界只在这一层出现：需要多次写入的用例（新增用户、保存角色授权、删除字典类型）
/// 用 [`AdminService::in_transaction`] 包起来，失败时整体回滚。
pub struct AdminService {
    database: DatabaseConnection,
    /// 新增用户未传口令时的默认口令，来自 `.env` 的 `SEED_ADMIN_PASSWORD`。
    default_password: String,
    cache: std::sync::Arc<super::hot_cache::HotCache>,
}

impl AdminService {
    pub fn new(
        database: DatabaseConnection,
        default_password: String,
        cache: std::sync::Arc<super::hot_cache::HotCache>,
    ) -> Self {
        Self {
            database,
            default_password,
            cache,
        }
    }

    /// 在 IMMEDIATE 写事务里执行，细节见 [`super::support::in_immediate_transaction`]。
    async fn in_transaction<T, F>(&self, run: F) -> Result<T, AppError>
    where
        T: Send,
        F: for<'a> FnOnce(&'a DatabaseTransaction) -> super::support::TransactionFuture<'a, T>,
    {
        super::support::in_immediate_transaction(&self.database, run).await
    }
}

/// 必填字段：去掉首尾空白后仍为空即报错，文案与 Spring 版一致。
pub(crate) fn required(value: Option<String>, message: &str) -> Result<String, AppError> {
    trim_to_null(value).ok_or_else(|| AppError::business(message))
}

/// 空白字符串统一收敛成 `None`，让局部更新保持「未传即不改」的语义。
pub(crate) fn trim_to_null(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn default_string(value: Option<String>, default: &str) -> String {
    trim_to_null(value).unwrap_or_else(|| default.to_string())
}

pub(crate) fn default_int(value: Option<i64>, default: i64) -> i64 {
    value.unwrap_or(default)
}

/// 菜单树：先按 `(parentId, sort, id)` 排好序的行拼成树，父节点缺失时提升为根。
pub(crate) fn menu_tree(rows: Vec<MenuFlatRow>) -> Vec<MenuNode> {
    let ids: std::collections::HashSet<i64> = rows.iter().map(|row| row.id).collect();
    let mut children: HashMap<i64, Vec<MenuNode>> = HashMap::new();
    let mut roots: Vec<MenuNode> = Vec::new();

    for row in rows {
        let (id, parent_id) = (row.id, row.parent_id);
        let node = row.into_node();
        if parent_id != id && ids.contains(&parent_id) {
            children.entry(parent_id).or_default().push(node);
        } else {
            roots.push(node);
        }
    }
    for root in &mut roots {
        attach_menu_children(root, &mut children);
    }

    roots
}

fn attach_menu_children(node: &mut MenuNode, children: &mut HashMap<i64, Vec<MenuNode>>) {
    let Some(mut nested) = children.remove(&node.id) else {
        return;
    };
    for child in &mut nested {
        attach_menu_children(child, children);
    }
    node.children = nested;
}

/// 部门树，规则与菜单树一致。
pub(crate) fn dept_tree(rows: Vec<DeptFlatRow>) -> Vec<DeptNode> {
    let ids: std::collections::HashSet<i64> = rows.iter().map(|row| row.id).collect();
    let mut children: HashMap<i64, Vec<DeptNode>> = HashMap::new();
    let mut roots: Vec<DeptNode> = Vec::new();

    for row in rows {
        let (id, parent_id) = (row.id, row.parent_id);
        let node = row.into_node();
        if parent_id != id && ids.contains(&parent_id) {
            children.entry(parent_id).or_default().push(node);
        } else {
            roots.push(node);
        }
    }
    for root in &mut roots {
        attach_dept_children(root, &mut children);
    }

    roots
}

fn attach_dept_children(node: &mut DeptNode, children: &mut HashMap<i64, Vec<DeptNode>>) {
    let Some(mut nested) = children.remove(&node.id) else {
        return;
    };
    for child in &mut nested {
        attach_dept_children(child, children);
    }
    node.children = nested;
}

/// 部门下拉的树形结构（只带 id / parentId / name / status）。
pub(crate) fn dept_option_tree(
    rows: Vec<crate::domain::models::system_admin::DeptOptionRow>,
) -> Vec<crate::domain::models::system_admin::DeptOption> {
    use crate::domain::models::system_admin::DeptOption;

    let ids: std::collections::HashSet<i64> = rows.iter().map(|row| row.id).collect();
    let mut children: HashMap<i64, Vec<DeptOption>> = HashMap::new();
    let mut roots: Vec<DeptOption> = Vec::new();

    for row in rows {
        let (id, parent_id) = (row.id, row.parent_id);
        let node = DeptOption {
            id,
            parent_id,
            name: row.name,
            status: row.status,
            children: Vec::new(),
        };
        if parent_id != id && ids.contains(&parent_id) {
            children.entry(parent_id).or_default().push(node);
        } else {
            roots.push(node);
        }
    }
    for root in &mut roots {
        attach_option_children(root, &mut children);
    }

    roots
}

fn attach_option_children(
    node: &mut crate::domain::models::system_admin::DeptOption,
    children: &mut HashMap<i64, Vec<crate::domain::models::system_admin::DeptOption>>,
) {
    let Some(mut nested) = children.remove(&node.id) else {
        return;
    };
    for child in &mut nested {
        attach_option_children(child, children);
    }
    node.children = nested;
}

/// 部门 `ancestors` 链：上级为空串时返回「上级 ID」，否则返回「上级链 + 上级 ID」。
pub(crate) fn ancestors_of(
    parent_ancestors: Option<String>,
    parent_id: i64,
) -> Result<String, AppError> {
    let Some(parent_ancestors) = parent_ancestors else {
        return Err(AppError::business("上级部门不存在或已删除"));
    };

    if parent_ancestors.is_empty() {
        return Ok(parent_id.to_string());
    }

    Ok(format!("{parent_ancestors},{parent_id}"))
}

impl crate::domain::services::admin::AdminUseCase for AdminService {}
