use std::collections::HashMap;

use async_trait::async_trait;
use sea_orm::{ConnectionTrait, Value};

use crate::{
    domain::models::{
        identity::DataScope,
        pagination::Page,
        system_admin::{UserQuery, UserRow, UserWriteCommand},
    },
    error::AppError,
    infrastructure::repositories::sql::{
        execute, insert, int_or_zero, query_all, query_count, query_opt, text, text_or_none,
    },
    util::time,
};

/// 用户与其角色 / 岗位关联的读写。
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn count_users<C>(
        &self,
        db: &C,
        query: &UserQuery,
        scope: &DataScope,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    async fn list_users<C>(
        &self,
        db: &C,
        query: &UserQuery,
        scope: &DataScope,
        page: Page,
    ) -> Result<Vec<UserRow>, AppError>
    where
        C: ConnectionTrait;

    async fn count_username<C>(&self, db: &C, username: &str) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    /// 账号名：重置口令后要按它撤销会话。账号不存在时返回 `None`。
    async fn find_username<C>(&self, db: &C, id: i64) -> Result<Option<String>, AppError>
    where
        C: ConnectionTrait;

    async fn insert_user<C>(
        &self,
        db: &C,
        command: &UserWriteCommand,
        operator: &str,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    /// 局部更新：只覆盖显式传入的字段（`None` 保持原值）。
    async fn update_user<C>(
        &self,
        db: &C,
        id: i64,
        command: &UserWriteCommand,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn update_user_status<C>(
        &self,
        db: &C,
        id: i64,
        status: i64,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn reset_user_password<C>(
        &self,
        db: &C,
        id: i64,
        password_hash: &str,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn soft_delete_user<C>(&self, db: &C, id: i64, operator: &str) -> Result<u64, AppError>
    where
        C: ConnectionTrait;

    async fn replace_user_roles<C>(
        &self,
        db: &C,
        user_id: i64,
        role_ids: &[i64],
    ) -> Result<(), AppError>
    where
        C: ConnectionTrait;

    async fn replace_user_posts<C>(
        &self,
        db: &C,
        user_id: i64,
        post_ids: &[i64],
    ) -> Result<(), AppError>
    where
        C: ConnectionTrait;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SeaOrmUserRepository;

#[async_trait]
impl UserRepository for SeaOrmUserRepository {
    async fn count_users<C>(
        &self,
        db: &C,
        query: &UserQuery,
        scope: &DataScope,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, values) = filter_of(query, scope);
        let sql = format!("SELECT COUNT(*) AS value FROM sys_user u WHERE {filter}");

        query_count(db, &sql, values).await
    }

    async fn list_users<C>(
        &self,
        db: &C,
        query: &UserQuery,
        scope: &DataScope,
        page: Page,
    ) -> Result<Vec<UserRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let (filter, mut values) = filter_of(query, scope);
        let sql = format!(
            "SELECT u.id, u.username, u.nickname, u.phone, u.email, d.dept_name AS dept_name, \
             u.status, substr(u.create_time, 1, 16) AS create_time, \
             substr(u.last_login_time, 1, 16) AS last_login \
             FROM sys_user u LEFT JOIN sys_dept d ON d.id = u.dept_id \
             WHERE {filter} ORDER BY u.id LIMIT ? OFFSET ?"
        );
        values.push(page.limit().into());
        values.push(page.offset().into());
        let rows = query_all(db, &sql, values).await?;
        let ids: Vec<i64> = rows.iter().map(|row| int_or_zero(row, "id")).collect();
        let post_names = names_of(db, &ids, POST_NAMES).await?;
        let role_names = names_of(db, &ids, ROLE_NAMES).await?;

        Ok(rows
            .iter()
            .map(|row| {
                let id = int_or_zero(row, "id");

                UserRow {
                    id,
                    username: text(row, "username").unwrap_or_default(),
                    nickname: text(row, "nickname").unwrap_or_default(),
                    phone: text_or_none(row, "phone"),
                    email: text_or_none(row, "email"),
                    dept_name: text_or_none(row, "dept_name"),
                    post_name: joined(post_names.get(&id)),
                    role_name: joined(role_names.get(&id)),
                    status: int_or_zero(row, "status"),
                    create_time: text_or_none(row, "create_time"),
                    last_login: text_or_none(row, "last_login"),
                }
            })
            .collect())
    }

    async fn count_username<C>(&self, db: &C, username: &str) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_user WHERE username = ? AND deleted = 0",
            vec![username.into()],
        )
        .await
    }

    async fn find_username<C>(&self, db: &C, id: i64) -> Result<Option<String>, AppError>
    where
        C: ConnectionTrait,
    {
        let row = query_opt(
            db,
            "SELECT username FROM sys_user WHERE id = ? AND deleted = 0",
            vec![id.into()],
        )
        .await?;

        row.map(|row| row.try_get("", "username").map_err(AppError::from))
            .transpose()
    }

    async fn insert_user<C>(
        &self,
        db: &C,
        command: &UserWriteCommand,
        operator: &str,
    ) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        let now = time::now();

        insert(
            db,
            "INSERT INTO sys_user (username, password, nickname, phone, email, dept_id, status, \
             create_by, update_by, create_time, update_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            vec![
                command.username.clone().unwrap_or_default().into(),
                command.password.clone().unwrap_or_default().into(),
                command.nickname.clone().unwrap_or_default().into(),
                command.phone.clone().into(),
                command.email.clone().into(),
                command.dept_id.into(),
                command.status.unwrap_or(1).into(),
                operator.into(),
                operator.into(),
                now.clone().into(),
                now.into(),
            ],
        )
        .await
    }

    async fn update_user<C>(
        &self,
        db: &C,
        id: i64,
        command: &UserWriteCommand,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE sys_user SET nickname = COALESCE(?, nickname), phone = COALESCE(?, phone), \
             email = COALESCE(?, email), dept_id = COALESCE(?, dept_id), \
             status = COALESCE(?, status), update_by = ?, update_time = ? \
             WHERE id = ? AND deleted = 0",
            vec![
                command.nickname.clone().into(),
                command.phone.clone().into(),
                command.email.clone().into(),
                command.dept_id.into(),
                command.status.into(),
                operator.into(),
                time::now().into(),
                id.into(),
            ],
        )
        .await
    }

    async fn update_user_status<C>(
        &self,
        db: &C,
        id: i64,
        status: i64,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE sys_user SET status = ?, update_by = ?, update_time = ? \
             WHERE id = ? AND deleted = 0",
            vec![
                status.into(),
                operator.into(),
                time::now().into(),
                id.into(),
            ],
        )
        .await
    }

    async fn reset_user_password<C>(
        &self,
        db: &C,
        id: i64,
        password_hash: &str,
        operator: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE sys_user SET password = ?, update_by = ?, update_time = ? \
             WHERE id = ? AND deleted = 0",
            vec![
                password_hash.into(),
                operator.into(),
                time::now().into(),
                id.into(),
            ],
        )
        .await
    }

    async fn soft_delete_user<C>(&self, db: &C, id: i64, operator: &str) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "UPDATE sys_user SET deleted = 1, update_by = ?, update_time = ? \
             WHERE id = ? AND deleted = 0",
            vec![operator.into(), time::now().into(), id.into()],
        )
        .await
    }

    async fn replace_user_roles<C>(
        &self,
        db: &C,
        user_id: i64,
        role_ids: &[i64],
    ) -> Result<(), AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "DELETE FROM sys_user_role WHERE user_id = ?",
            vec![user_id.into()],
        )
        .await?;
        for role_id in role_ids {
            execute(
                db,
                "INSERT OR IGNORE INTO sys_user_role (user_id, role_id) VALUES (?, ?)",
                vec![user_id.into(), (*role_id).into()],
            )
            .await?;
        }

        Ok(())
    }

    async fn replace_user_posts<C>(
        &self,
        db: &C,
        user_id: i64,
        post_ids: &[i64],
    ) -> Result<(), AppError>
    where
        C: ConnectionTrait,
    {
        execute(
            db,
            "DELETE FROM sys_user_post WHERE user_id = ?",
            vec![user_id.into()],
        )
        .await?;
        for post_id in post_ids {
            execute(
                db,
                "INSERT OR IGNORE INTO sys_user_post (user_id, post_id) VALUES (?, ?)",
                vec![user_id.into(), (*post_id).into()],
            )
            .await?;
        }

        Ok(())
    }
}

/// `post_name` / `role_name` 两条 JOIN 的后缀一致：按用户聚名字。
const POST_NAMES: &str = "SELECT up.user_id AS user_id, p.post_name AS name FROM sys_user_post up \
     JOIN sys_post p ON p.id = up.post_id AND p.deleted = 0 \
     WHERE up.user_id IN ({ids}) ORDER BY p.sort, p.id";

const ROLE_NAMES: &str = "SELECT ur.user_id AS user_id, r.role_name AS name FROM sys_user_role ur \
     JOIN sys_role r ON r.id = ur.role_id AND r.deleted = 0 \
     WHERE ur.user_id IN ({ids}) ORDER BY r.sort, r.id";

/// 列表过滤条件：关键词 / 状态 / 部门（含下级）/ 岗位 / 数据权限。
///
/// 返回的占位符顺序与 `values` 一一对应，供 count 与 list 复用。
fn filter_of(query: &UserQuery, scope: &DataScope) -> (String, Vec<Value>) {
    let mut clauses = vec!["u.deleted = 0".to_string()];
    let mut values = Vec::new();

    if let Some(keyword) = query.keyword.as_deref().filter(|value| !value.is_empty()) {
        clauses.push("(u.username LIKE ? OR u.nickname LIKE ? OR u.phone LIKE ?)".to_string());
        let pattern = format!("%{keyword}%");
        for _ in 0..3 {
            values.push(pattern.clone().into());
        }
    }
    if let Some(status) = query.status {
        clauses.push("u.status = ?".to_string());
        values.push(status.into());
    }
    if let Some(dept_id) = query.dept_id {
        clauses.push(
            "u.dept_id IN (SELECT d.id FROM sys_dept d \
             WHERE d.id = ? OR (',' || d.ancestors || ',') LIKE ('%,' || ? || ',%'))"
                .to_string(),
        );
        values.push(dept_id.into());
        values.push(dept_id.into());
    }
    if let Some(post_id) = query.post_id {
        clauses.push(
            "EXISTS (SELECT 1 FROM sys_user_post fup WHERE fup.user_id = u.id AND fup.post_id = ?)"
                .to_string(),
        );
        values.push(post_id.into());
    }
    if !scope.all {
        let mut condition = "u.id = ?".to_string();
        values.push(scope.user_id_or_default().into());
        if !scope.dept_ids.is_empty() {
            let placeholders = vec!["?"; scope.dept_ids.len()].join(", ");
            condition.push_str(&format!(" OR u.dept_id IN ({placeholders})"));
            for dept_id in &scope.dept_ids {
                values.push((*dept_id).into());
            }
        }
        clauses.push(format!("({condition})"));
    }

    (clauses.join(" AND "), values)
}

/// 按用户聚名字，顺序由 SQL 的 ORDER BY 决定。
async fn names_of<C>(
    db: &C,
    user_ids: &[i64],
    template: &str,
) -> Result<HashMap<i64, Vec<String>>, AppError>
where
    C: ConnectionTrait,
{
    let mut grouped: HashMap<i64, Vec<String>> = HashMap::new();
    if user_ids.is_empty() {
        return Ok(grouped);
    }

    let placeholders = vec!["?"; user_ids.len()].join(", ");
    let sql = template.replace("{ids}", &placeholders);
    let values: Vec<Value> = user_ids.iter().map(|id| (*id).into()).collect();
    for row in query_all(db, &sql, values).await? {
        if let Some(name) = text(&row, "name") {
            grouped
                .entry(int_or_zero(&row, "user_id"))
                .or_default()
                .push(name);
        }
    }

    Ok(grouped)
}

fn joined(names: Option<&Vec<String>>) -> String {
    names.map(|names| names.join(",")).unwrap_or_default()
}
