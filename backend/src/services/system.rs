use std::sync::Arc;

use async_trait::async_trait;
use chrono::Local;
use sea_orm::DatabaseConnection;

use crate::{
    domain::{
        error::AppError,
        models::system::{HealthReport, WelcomeInfo},
        services::system::SystemUseCase,
    },
    infrastructure::{config::AppConfig, databases::ping_database},
};

pub struct SystemService {
    config: Arc<AppConfig>,
    database: DatabaseConnection,
}

impl SystemService {
    pub fn new(config: Arc<AppConfig>, database: DatabaseConnection) -> Self {
        Self { config, database }
    }
}

#[async_trait]
impl SystemUseCase for SystemService {
    async fn welcome(&self) -> Result<WelcomeInfo, AppError> {
        Ok(WelcomeInfo {
            service_name: self.config.app_name.clone(),
            environment: self.config.app_env.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            docs_enabled: cfg!(feature = "docs"),
        })
    }

    async fn health(&self) -> Result<HealthReport, AppError> {
        let database_status = "not_checked".to_string();
        let status = "ok".to_string();

        Ok(HealthReport {
            service_name: self.config.app_name.clone(),
            environment: self.config.app_env.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            status,
            database_status,
            timestamp: Local::now().timestamp_millis(),
        })
    }

    async fn ready(&self) -> Result<(), AppError> {
        if ping_database(&self.database, 2).await.is_err() {
            return Err(AppError::unavailable("database is not ready yet"));
        }

        Ok(())
    }
}
