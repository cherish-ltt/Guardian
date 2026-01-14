use anyhow::{Ok, Result};
use axum::{
    Json, Router,
    routing::{delete, get, post, put},
};
use sea_orm::{Database, DatabaseConnection};
use tower_http::cors::{Any, CorsLayer};

use crate::controller::{
    admin_controller::*, auth_controller::*, permission_controller::*, role_controller::*, root,
};
use crate::middleware::middleware_api::{auth_middleware, rate_limit_middleware};

const API_PREFIX: &str = "/guardian-auth/v1";

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) conn: DatabaseConnection,
}

pub(crate) async fn get_router() -> Result<Router> {
    let db_connection_str = std::env::var("DATABASE_URL").unwrap();

    let conn = Database::connect(db_connection_str)
        .await
        .expect("Database connection failed");

    let state = AppState { conn };

    // 公开路由（无需认证）
    let public_routes = Router::new()
        .route("/", get(root))
        .route(&format!("{}/auth/login", API_PREFIX), post(login))
        .route(&format!("{}/auth/refresh", API_PREFIX), post(refresh_token));

    // 受保护路由（需要认证）
    let protected_routes = Router::new()
        .route(&format!("{}/auth/logout", API_PREFIX), post(logout))
        .route(&format!("{}/auth/2fa/setup", API_PREFIX), post(setup_2fa))
        .route(&format!("{}/auth/2fa/verify", API_PREFIX), post(verify_2fa))
        .route(&format!("{}/admins", API_PREFIX), get(list_admin))
        .route(&format!("{}/admins", API_PREFIX), post(create_admin))
        .route(&format!("{}/admins/id", API_PREFIX), get(get_admin))
        .route(&format!("{}/admins/id", API_PREFIX), put(update_admin))
        .route(&format!("{}/admins/id", API_PREFIX), delete(delete_admin))
        .route(&format!("{}/roles", API_PREFIX), get(list_role))
        .route(&format!("{}/roles", API_PREFIX), post(create_role))
        .route(&format!("{}/roles/id", API_PREFIX), get(get_role))
        .route(&format!("{}/roles/id", API_PREFIX), put(update_role))
        .route(&format!("{}/roles/id", API_PREFIX), delete(delete_role))
        .route(
            &format!("{}/roles/id/permissions", API_PREFIX),
            post(assign_permissions),
        )
        .route(
            &format!("{}/permissions/tree", API_PREFIX),
            get(get_permission_tree),
        )
        .route(&format!("{}/permissions", API_PREFIX), get(list_permission))
        .route(
            &format!("{}/permissions", API_PREFIX),
            post(create_permission),
        )
        .route(
            &format!("{}/permissions/id", API_PREFIX),
            get(get_permission),
        )
        .route(
            &format!("{}/permissions/id", API_PREFIX),
            put(update_permission),
        )
        .route(
            &format!("{}/permissions/id", API_PREFIX),
            delete(delete_permission),
        )
        .route_layer(axum::middleware::from_fn(auth_middleware));

    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .route_layer(axum::middleware::from_fn(rate_limit_middleware))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);
    Ok(app)
}
