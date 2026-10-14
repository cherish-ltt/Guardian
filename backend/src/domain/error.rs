use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Service unavailable: {0}")]
    Unavailable(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Internal error: {0}")]
    Internal(String),

    /// 业务校验失败：文案直接展示给前台，不加任何前缀。
    #[error("{0}")]
    Business(String),
    /// 已认证但缺少权限码，对应 HTTP 403。
    #[error("{0}")]
    Forbidden(String),
    /// 会话失效：未携带令牌、令牌过期或账号已停用，对应 HTTP 401。
    #[error("{0}")]
    SessionExpired(String),
}

impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::Unavailable(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    /// 业务错误（HTTP 400），文案原样透出。
    pub fn business(message: impl Into<String>) -> Self {
        Self::Business(message.into())
    }

    /// 缺少权限（HTTP 403）。
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden(message.into())
    }

    /// 会话失效（HTTP 401）。
    pub fn session_expired(message: impl Into<String>) -> Self {
        Self::SessionExpired(message.into())
    }

    pub fn http_code(&self) -> u16 {
        match self {
            Self::Config(_) | Self::Internal(_) | Self::Database(_) => 500,
            Self::BadRequest(_) | Self::Business(_) => 400,
            Self::Unauthorized(_) | Self::SessionExpired(_) => 401,
            Self::Forbidden(_) => 403,
            Self::NotFound(_) => 404,
            Self::Conflict(_) => 409,
            Self::Unavailable(_) => 503,
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error.to_string())
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(error: sea_orm::DbErr) -> Self {
        // 唯一约束冲突属于客户端可控冲突（如幂等键重复提交），映射为 409 而非 500。
        // 数据库原始 detail 含表名/列名，只进日志，不进响应体。
        if let Some(sea_orm::SqlErr::UniqueConstraintViolation(detail)) = error.sql_err() {
            tracing::debug!(detail = %detail, "unique constraint violation mapped to 409 conflict");
            return Self::Conflict("resource already exists".to_string());
        }

        Self::Database(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::AppError;

    #[test]
    fn constructors_match_variants() {
        assert!(matches!(
            AppError::bad_request("x"),
            AppError::BadRequest(_)
        ));
        assert!(matches!(
            AppError::unauthorized("x"),
            AppError::Unauthorized(_)
        ));
        assert!(matches!(AppError::not_found("x"), AppError::NotFound(_)));
        assert!(matches!(AppError::conflict("x"), AppError::Conflict(_)));
        assert!(matches!(
            AppError::unavailable("x"),
            AppError::Unavailable(_)
        ));
        assert!(matches!(AppError::internal("x"), AppError::Internal(_)));
    }

    #[test]
    fn http_codes_match_design() {
        assert_eq!(AppError::bad_request("x").http_code(), 400);
        assert_eq!(AppError::business("x").http_code(), 400);
        assert_eq!(AppError::unauthorized("x").http_code(), 401);
        assert_eq!(AppError::session_expired("x").http_code(), 401);
        assert_eq!(AppError::forbidden("x").http_code(), 403);
        assert_eq!(AppError::not_found("x").http_code(), 404);
        assert_eq!(AppError::conflict("x").http_code(), 409);
        assert_eq!(AppError::unavailable("x").http_code(), 503);
        assert_eq!(AppError::internal("x").http_code(), 500);
        assert_eq!(AppError::Config("x".to_string()).http_code(), 500);
        assert_eq!(AppError::Database("x".to_string()).http_code(), 500);
    }

    /// 业务错误、权限错误与会话失效的文案要原样展示给前台，不能带英文前缀。
    #[test]
    fn localized_errors_keep_message_without_prefix() {
        assert_eq!(
            AppError::business("账号或密码错误").to_string(),
            "账号或密码错误"
        );
        assert_eq!(
            AppError::forbidden("没有访问权限").to_string(),
            "没有访问权限"
        );
        assert_eq!(
            AppError::session_expired("未认证或登录已过期").to_string(),
            "未认证或登录已过期"
        );
    }

    #[test]
    fn display_messages_include_details() {
        assert_eq!(
            AppError::bad_request("title must not be empty").to_string(),
            "Bad request: title must not be empty"
        );
        assert_eq!(
            AppError::unauthorized("token expired").to_string(),
            "Unauthorized: token expired"
        );
        assert_eq!(
            AppError::not_found("example_001").to_string(),
            "Not found: example_001"
        );
        assert_eq!(
            AppError::conflict("duplicate idempotency key").to_string(),
            "Conflict: duplicate idempotency key"
        );
        assert_eq!(
            AppError::unavailable("database not ready").to_string(),
            "Service unavailable: database not ready"
        );
        assert_eq!(
            AppError::Config("missing field".to_string()).to_string(),
            "Configuration error: missing field"
        );
        assert_eq!(
            AppError::Database("connection failed".to_string()).to_string(),
            "Database error: connection failed"
        );
        assert_eq!(
            AppError::Internal("panic".to_string()).to_string(),
            "Internal error: panic"
        );
    }

    #[test]
    fn anyhow_error_converts_to_internal() {
        let error: AppError = anyhow::anyhow!("boom").into();
        assert!(matches!(error, AppError::Internal(message) if message.contains("boom")));
    }

    #[test]
    fn db_error_converts_to_database() {
        let error: AppError = sea_orm::DbErr::Custom("connection failed".to_string()).into();
        assert!(
            matches!(error, AppError::Database(message) if message.contains("connection failed"))
        );
    }

    /// 真实触发数据库唯一约束，确认被识别为 409 冲突而不是 500。
    #[tokio::test]
    async fn unique_constraint_violation_maps_to_conflict() {
        use sea_orm::{ConnectionTrait, Database};

        let url = format!(
            "sqlite://{}?mode=rwc",
            std::env::temp_dir()
                .join(format!("guardian-error-test-{}.db", uuid::Uuid::now_v7()))
                .display()
        );
        let database = Database::connect(url).await.expect("连接临时数据库");

        database
            .execute_unprepared(
                "CREATE TABLE demo (id TEXT PRIMARY KEY NOT NULL, request_id TEXT, \
                 CONSTRAINT demo_request_id_unique UNIQUE (request_id))",
            )
            .await
            .expect("建表成功");
        database
            .execute_unprepared("INSERT INTO demo (id, request_id) VALUES ('a', 'dup')")
            .await
            .expect("首次写入成功");

        let db_error = database
            .execute_unprepared("INSERT INTO demo (id, request_id) VALUES ('b', 'dup')")
            .await
            .expect_err("重复幂等键必须被拒绝");

        let error: AppError = db_error.into();
        assert!(
            matches!(error, AppError::Conflict(_)),
            "唯一约束冲突应映射为 Conflict，实际: {error:?}"
        );
        assert_eq!(error.http_code(), 409);
        // 数据库 detail 含表名/列名，只能进日志，不得出现在客户端可见的文案里。
        assert_eq!(error.to_string(), "Conflict: resource already exists");
        assert!(
            !error.to_string().contains("UNIQUE constraint failed"),
            "冲突信息不得透出数据库内部结构: {error}"
        );
    }
}
