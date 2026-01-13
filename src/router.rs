use anyhow::{Ok, Result};
use axum::{
    Json, Router,
    routing::{get, post},
};
use sea_orm::{Database, DatabaseConnection};
use tower_http::cors::{Any, CorsLayer};

use crate::controller::{auth_controller::*, root};
use crate::middleware::middleware_api::auth_middleware;

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
        .route_layer(axum::middleware::from_fn(auth_middleware));

    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);
    Ok(app)
}
