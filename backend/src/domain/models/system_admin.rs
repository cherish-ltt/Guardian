use serde::{Deserialize, Serialize};

use crate::domain::models::pagination::PageQuery;

/// 用户列表查询条件。
#[derive(Debug, Clone, Default)]
pub struct UserQuery {
    pub page: PageQuery,
    pub keyword: Option<String>,
    pub status: Option<i64>,
    pub dept_id: Option<i64>,
    pub post_id: Option<i64>,
}

/// 只有关键词与状态两个条件的列表查询（角色 / 菜单 / 部门 / 岗位 / 字典类型）。
#[derive(Debug, Clone, Default)]
pub struct StatusQuery {
    pub page: PageQuery,
    pub keyword: Option<String>,
    pub status: Option<i64>,
}

/// 参数配置列表查询：没有状态条件。
#[derive(Debug, Clone, Default)]
pub struct KeywordQuery {
    pub page: PageQuery,
    pub keyword: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRow {
    pub id: i64,
    pub username: String,
    pub nickname: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub dept_name: Option<String>,
    pub post_name: String,
    pub role_name: String,
    pub status: i64,
    pub create_time: Option<String>,
    pub last_login: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleRow {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub data_scope: String,
    pub user_count: i64,
    pub sort: i64,
    pub status: i64,
    pub builtin: bool,
    pub create_time: Option<String>,
    pub menu_ids: Vec<i64>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuNode {
    pub id: i64,
    pub parent_id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub icon: Option<String>,
    pub path: Option<String>,
    pub component: Option<String>,
    pub permission: Option<String>,
    pub sort: i64,
    pub visible: i64,
    pub status: i64,
    pub children: Vec<MenuNode>,
}

/// 菜单扁平行：先查出扁平行再在服务层拼树。
#[derive(Debug, Clone)]
pub struct MenuFlatRow {
    pub id: i64,
    pub parent_id: i64,
    pub name: String,
    pub type_: String,
    pub icon: Option<String>,
    pub path: Option<String>,
    pub component: Option<String>,
    pub permission: Option<String>,
    pub sort: i64,
    pub visible: i64,
    pub status: i64,
}

impl MenuFlatRow {
    pub fn into_node(self) -> MenuNode {
        MenuNode {
            id: self.id,
            parent_id: self.parent_id,
            name: self.name,
            type_: self.type_,
            icon: self.icon,
            path: self.path,
            component: self.component,
            permission: self.permission,
            sort: self.sort,
            visible: self.visible,
            status: self.status,
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeptNode {
    pub id: i64,
    pub parent_id: i64,
    pub name: String,
    pub leader: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub sort: i64,
    pub status: i64,
    pub member_count: i64,
    pub children: Vec<DeptNode>,
}

#[derive(Debug, Clone)]
pub struct DeptFlatRow {
    pub id: i64,
    pub parent_id: i64,
    pub name: String,
    pub leader: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub sort: i64,
    pub status: i64,
    pub member_count: i64,
}

impl DeptFlatRow {
    pub fn into_node(self) -> DeptNode {
        DeptNode {
            id: self.id,
            parent_id: self.parent_id,
            name: self.name,
            leader: self.leader,
            phone: self.phone,
            email: self.email,
            sort: self.sort,
            status: self.status,
            member_count: self.member_count,
            children: Vec::new(),
        }
    }
}

/// 用户页下拉里只带这四项的部门节点（不含负责人联系方式）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeptOption {
    pub id: i64,
    pub parent_id: i64,
    pub name: String,
    pub status: i64,
    pub children: Vec<DeptOption>,
}

/// 部门下拉的扁平行。
#[derive(Debug, Clone)]
pub struct DeptOptionRow {
    pub id: i64,
    pub parent_id: i64,
    pub name: String,
    pub status: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct IdNameOption {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOptions {
    pub depts: Vec<DeptOption>,
    pub roles: Vec<IdNameOption>,
    pub posts: Vec<IdNameOption>,
    pub init_password: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostRow {
    pub id: i64,
    pub post_code: String,
    pub post_name: String,
    pub sort: i64,
    pub status: i64,
    pub remark: Option<String>,
    pub create_time: Option<String>,
    pub user_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictTypeRow {
    pub id: i64,
    pub dict_name: String,
    pub dict_type: String,
    pub status: i64,
    pub create_time: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictDataRow {
    pub id: i64,
    pub label: String,
    pub value: String,
    pub sort: i64,
    pub status: i64,
    pub tone: Option<String>,
    pub def: bool,
}

/// 业务字典下拉项：`def` 与 Spring 版一致地保持 0/1 数值。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictOption {
    pub label: String,
    pub value: String,
    pub tone: Option<String>,
    pub def: i64,
    pub status: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigRow {
    pub id: i64,
    pub name: String,
    pub key: String,
    pub value: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub builtin: bool,
    pub create_time: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IdResponse {
    pub id: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphUser {
    pub id: i64,
    pub username: String,
    pub nickname: String,
    pub dept_name: Option<String>,
    pub status: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRole {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub data_scope: String,
    pub status: i64,
    pub builtin: bool,
    pub user_count: i64,
    pub menu_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphMenu {
    pub id: i64,
    pub parent_id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub icon: Option<String>,
    pub path: Option<String>,
    pub permission: Option<String>,
    pub sort: i64,
    pub visible: i64,
    pub status: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionGraph {
    pub users: Vec<GraphUser>,
    pub roles: Vec<GraphRole>,
    pub menus: Vec<GraphMenu>,
    pub user_roles: Vec<(i64, i64)>,
    pub users_visible: bool,
}

/// 用户角色关联对。
#[derive(Debug, Clone, Copy)]
pub struct UserRoleLink {
    pub user_id: i64,
    pub role_id: i64,
}

/* ── 写入命令 ─────────────────────────────────────────── */

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserWriteCommand {
    pub username: Option<String>,
    pub password: Option<String>,
    pub nickname: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub dept_id: Option<i64>,
    pub status: Option<i64>,
    pub role_ids: Option<Vec<i64>>,
    pub post_ids: Option<Vec<i64>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserStatusCommand {
    pub status: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPasswordCommand {
    pub password: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleWriteCommand {
    pub name: Option<String>,
    pub code: Option<String>,
    pub data_scope: Option<String>,
    pub sort: Option<i64>,
    pub status: Option<i64>,
    pub remark: Option<String>,
    pub menu_ids: Option<Vec<i64>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuWriteCommand {
    pub parent_id: Option<i64>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub icon: Option<String>,
    pub path: Option<String>,
    pub component: Option<String>,
    pub permission: Option<String>,
    pub sort: Option<i64>,
    pub visible: Option<i64>,
    pub status: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeptWriteCommand {
    pub parent_id: Option<i64>,
    pub name: Option<String>,
    pub leader: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub sort: Option<i64>,
    pub status: Option<i64>,
}

/// 部门更新：写入字段 + 按需重算的上级链（`None` 表示请求未带 parentId）。
#[derive(Debug, Clone, Copy)]
pub struct DeptUpdate<'a> {
    pub command: &'a DeptWriteCommand,
    pub ancestors: Option<&'a str>,
    pub operator: &'a str,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostWriteCommand {
    pub post_code: Option<String>,
    pub post_name: Option<String>,
    pub sort: Option<i64>,
    pub status: Option<i64>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictTypeWriteCommand {
    pub dict_name: Option<String>,
    pub dict_type: Option<String>,
    pub status: Option<i64>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictDataWriteCommand {
    pub dict_type: Option<String>,
    pub label: Option<String>,
    pub value: Option<String>,
    pub sort: Option<i64>,
    pub status: Option<i64>,
    pub tone: Option<String>,
    pub def: Option<bool>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigWriteCommand {
    pub name: Option<String>,
    pub key: Option<String>,
    pub value: Option<String>,
    #[serde(rename = "type")]
    #[allow(clippy::struct_field_names)]
    pub config_type: Option<String>,
    pub builtin: Option<bool>,
    pub remark: Option<String>,
}
