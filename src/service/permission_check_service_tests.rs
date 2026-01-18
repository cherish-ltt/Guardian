#[cfg(test)]
mod tests {
    use crate::entities::prelude::*;
    use crate::middleware::auth::AuthContext;
    use crate::router::AppState;
    use crate::service::permission_check_service::*;
    use sea_orm::*;

    async fn get_test_connection() -> DatabaseConnection {
        let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://postgres:123456@127.0.0.1:5432/guardian_auth".to_string()
        });

        Database::connect(db_url)
            .await
            .expect("Failed to connect to test database")
    }

    #[tokio::test]
    async fn test_super_admin_bypasses_permission_check() {
        // 测试：超级管理员应该跳过权限检查

        let auth_context = AuthContext {
            admin_id: uuid::Uuid::new_v4(),
            username: "test_user".to_string(),
            is_super_admin: true, // <--- 关键：这是超级管理员
        };

        let state = AppState {
            conn: get_test_connection().await,
        };

        let result = check_api_permission(
            state,
            auth_context,
            "GET".to_string(),
            "/guardian-auth/v1/admins".to_string(),
        )
        .await;

        // 超级管理员应该总是返回 true
        assert!(result.is_ok(), "超级管理员权限检查应该成功");
        assert_eq!(
            result.unwrap(),
            true,
            "超级管理员应该有所有权限（返回 true）"
        );
    }

    #[tokio::test]
    async fn test_non_super_admin_without_permission() {
        let auth_context = AuthContext {
            admin_id: uuid::Uuid::new_v4(),
            username: "test_user".to_string(),
            is_super_admin: false,
        };

        let state = AppState {
            conn: get_test_connection().await,
        };

        let result = check_api_permission(
            state,
            auth_context,
            "GET".to_string(),
            "/guardian-auth/v1/admins".to_string(),
        )
        .await;

        // 没有权限的非超级管理员应该返回 false
        assert!(result.is_ok(), "权限检查应该成功");
        assert_eq!(
            result.unwrap(),
            false,
            "没有权限的非超级管理员应该被拒绝（返回 false）"
        );
    }
}
