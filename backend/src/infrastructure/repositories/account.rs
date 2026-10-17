use crate::{
    domain::{
        error::AppError,
        models::account::{Profile, UpdateProfile},
    },
    entities::sys_user_security::{self, Model},
    infrastructure::repositories::sql::{execute, query_opt},
    util::time,
};
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};

pub struct AccountRepository;

impl AccountRepository {
    pub async fn security<C: ConnectionTrait>(db: &C, user_id: i64) -> Result<Model, AppError> {
        Ok(sys_user_security::Entity::find_by_id(user_id)
            .one(db)
            .await?
            .unwrap_or(Model {
                user_id,
                secret: None,
                pending_secret: None,
                pending_expires: 0,
                last_step: -1,
                recovery_hashes: "[]".into(),
                auth_version: 0,
                failures: 0,
                blocked_until: 0,
                login_challenge_hash: None,
                login_challenge_expires: 0,
                login_password_hash: None,
            }))
    }

    pub async fn save_security<C: ConnectionTrait>(db: &C, state: Model) -> Result<(), AppError> {
        execute(
            db,
            "INSERT OR IGNORE INTO sys_user_security (user_id) VALUES (?)",
            vec![state.user_id.into()],
        )
        .await?;
        sys_user_security::ActiveModel {
            user_id: Set(state.user_id),
            secret: Set(state.secret),
            pending_secret: Set(state.pending_secret),
            pending_expires: Set(state.pending_expires),
            last_step: Set(state.last_step),
            recovery_hashes: Set(state.recovery_hashes),
            auth_version: Set(state.auth_version),
            failures: Set(state.failures),
            blocked_until: Set(state.blocked_until),
            login_challenge_hash: Set(state.login_challenge_hash),
            login_challenge_expires: Set(state.login_challenge_expires),
            login_password_hash: Set(state.login_password_hash),
        }
        .update(db)
        .await?;
        Ok(())
    }

    pub async fn profile<C: ConnectionTrait>(db: &C, username: &str) -> Result<Profile, AppError> {
        let row = query_opt(db, "SELECT u.nickname, u.phone, u.email, d.dept_name FROM sys_user u LEFT JOIN sys_dept d ON d.id = u.dept_id WHERE u.username = ? AND u.deleted = 0 AND u.status = 1", vec![username.into()]).await?.ok_or_else(|| AppError::session_expired("未认证或登录已过期"))?;
        Ok(Profile {
            username: username.into(),
            nickname: row.try_get("", "nickname")?,
            phone: row.try_get("", "phone")?,
            email: row.try_get("", "email")?,
            dept_name: row.try_get("", "dept_name")?,
            roles: Vec::new(),
            two_factor_enabled: false,
            recovery_codes_remaining: 0,
        })
    }

    pub async fn update_profile<C: ConnectionTrait>(
        db: &C,
        username: &str,
        command: UpdateProfile,
    ) -> Result<(), AppError> {
        let changed = execute(db, "UPDATE sys_user SET nickname = ?, phone = ?, email = ?, update_by = ?, update_time = ? WHERE username = ? AND deleted = 0 AND status = 1",
            vec![command.nickname.into(), command.phone.into(), command.email.into(), username.into(), time::now().into(), username.into()]).await?;
        if changed == 0 {
            return Err(AppError::session_expired("未认证或登录已过期"));
        }
        Ok(())
    }

    pub async fn challenge_owner<C: ConnectionTrait>(
        db: &C,
        hash: &str,
    ) -> Result<Option<String>, AppError> {
        let row = query_opt(db, "SELECT u.username FROM sys_user_security s JOIN sys_user u ON u.id = s.user_id WHERE s.login_challenge_hash = ? AND u.deleted = 0 AND u.status = 1", vec![hash.into()]).await?;
        row.map(|r| r.try_get("", "username").map_err(AppError::from))
            .transpose()
    }

    pub async fn username<C: ConnectionTrait>(db: &C, user_id: i64) -> Result<String, AppError> {
        let row = query_opt(
            db,
            "SELECT username FROM sys_user WHERE id = ? AND deleted = 0",
            vec![user_id.into()],
        )
        .await?
        .ok_or_else(|| AppError::business("用户不存在"))?;
        Ok(row.try_get("", "username")?)
    }
}
