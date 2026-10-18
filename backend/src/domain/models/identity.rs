use serde::{Deserialize, Serialize};
#[cfg(feature = "docs")]
use utoipa::ToSchema;

/// 登录鉴权用的账号快照：包含密码密文与部门名，只在服务层流转。
#[derive(Debug, Clone)]
pub struct AuthUserRow {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub nickname: String,
    pub status: i64,
    pub dept_id: Option<i64>,
    pub dept_name: Option<String>,
}

/// `/auth/me` 的业务数据。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentUserInfo {
    pub user_id: i64,
    pub username: String,
    pub nickname: String,
    pub dept_id: Option<i64>,
    pub dept_name: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

/// `/auth/login` 的业务数据。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    pub token: String,
    pub expires_in: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum LoginOutcome {
    Authenticated(LoginResult),
    Challenge {
        #[serde(rename = "twoFactorRequired")]
        two_factor_required: bool,
        challenge: String,
        #[serde(rename = "expiresIn")]
        expires_in: u64,
    },
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerifyLoginCommand {
    pub challenge: String,
    pub second_factor: String,
}

/// 路由树节点，字段名与 Spring 版 `RouteVO` 一致。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteNode {
    pub id: i64,
    pub parent_id: i64,
    pub name: String,
    pub path: Option<String>,
    pub component: Option<String>,
    pub icon: Option<String>,
    pub permission: Option<String>,
    pub children: Vec<RouteNode>,
}

impl RouteNode {
    pub fn leaf(menu: MenuRow) -> Self {
        Self {
            id: menu.id,
            parent_id: menu.parent_id,
            name: menu.menu_name,
            path: menu.path,
            component: menu.component,
            icon: menu.icon,
            permission: menu.permission,
            children: Vec::new(),
        }
    }
}

/// 菜单行（路由与权限解析共用）。
#[derive(Debug, Clone)]
pub struct MenuRow {
    pub id: i64,
    pub parent_id: i64,
    pub menu_name: String,
    pub menu_type: String,
    pub icon: Option<String>,
    pub path: Option<String>,
    pub component: Option<String>,
    pub permission: Option<String>,
    pub sort: i64,
}

/// 登录请求（验证码在服务层与缓存比对）。
#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginCommand {
    pub username: String,
    pub password: String,
    pub captcha: Option<String>,
    pub captcha_uuid: Option<String>,
}

/// 一次登录 / 退出请求的客户端来源，用于写入 `sys_login_log`。
///
/// `ip` 是保护层按可信代理解析出的真实对端，不是请求头里的原值；
/// `location` 由仓储在写入时按 `ip` 推导，不在这里计算。
#[derive(Debug, Clone, Default)]
pub struct ClientSource {
    pub ip: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
}

/// 登录时生成的验证码：`code` 直接回显给登录页显示（与 Spring 版一致）。
///
/// `enabled` 为 `false` 时（`sys.account.captchaEnabled` 被关闭）不生成验证码，
/// `uuid` 与 `code` 为空串，登录页据此隐藏输入框。
#[derive(Debug, Clone, Serialize)]
pub struct CaptchaChallenge {
    pub enabled: bool,
    pub uuid: String,
    pub code: String,
}

/// 修改密码命令。
#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordCommand {
    pub old_password: String,
    pub new_password: String,
}

/// 数据权限范围：`all` 表示不限，否则按本人与可见部门过滤。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DataScope {
    pub all: bool,
    pub user_id: Option<i64>,
    pub dept_ids: Vec<i64>,
}

impl DataScope {
    pub fn all() -> Self {
        Self {
            all: true,
            user_id: None,
            dept_ids: Vec::new(),
        }
    }

    pub fn self_only(user_id: Option<i64>) -> Self {
        Self {
            all: false,
            user_id,
            dept_ids: Vec::new(),
        }
    }

    /// `user_id` 必填：仓储层要用它拼 SQL。
    pub fn user_id_or_default(&self) -> i64 {
        self.user_id.unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::{DataScope, RouteNode};
    use crate::domain::models::identity::MenuRow;

    #[test]
    fn route_node_copies_menu_fields() {
        let node = RouteNode::leaf(MenuRow {
            id: 101,
            parent_id: 100,
            menu_name: "用户管理".to_string(),
            menu_type: "menu".to_string(),
            icon: Some("users".to_string()),
            path: Some("/system/user".to_string()),
            component: Some("system/user/index".to_string()),
            permission: Some("system:user:list".to_string()),
            sort: 1,
        });

        assert_eq!(node.id, 101);
        assert_eq!(node.name, "用户管理");
        assert_eq!(node.permission.as_deref(), Some("system:user:list"));
        assert!(node.children.is_empty());
    }

    #[test]
    fn data_scope_variants() {
        assert!(DataScope::all().all);
        let limited = DataScope::self_only(Some(7));
        assert!(!limited.all);
        assert_eq!(limited.user_id_or_default(), 7);
        assert_eq!(DataScope::default().user_id_or_default(), 0);
    }
}
