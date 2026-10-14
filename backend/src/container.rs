use std::sync::Arc;

use sea_orm::DatabaseConnection;

use anyhow::Result;

use crate::{
    domain::services::{
        admin::AdminUseCase, admin_auth::AdminAuthUseCase, audit::OperationLogUseCase,
        auth::AuthUseCase, dashboard::DashboardUseCase, example::ExampleUseCase,
        generator::GeneratorUseCase, monitor::MonitorUseCase, system::SystemUseCase,
        transaction::TransferUseCase,
    },
    infrastructure::{
        config::AppConfig,
        databases::{connect_database, run_migrations},
        repositories::transaction::SeaOrmTransferRepository,
        services::{
            auth_store::{AuthStore, MemoryAuthStore},
            jwt::JwtService,
            redis_store::RedisAuthStore,
        },
    },
    services::{
        admin::AdminService, admin_auth::AdminAuthService, audit::AuditService, auth::AuthService,
        dashboard::DashboardService, example::ExampleService, generator::GeneratorService,
        monitor::MonitorService, system::SystemService, transaction::TransferService,
    },
};

pub struct Container {
    pub config: Arc<AppConfig>,
    pub database: DatabaseConnection,
    /// 令牌签发与校验：审计中间件用它解析操作人，避免每写一条日志都查库。
    pub jwt_service: Arc<JwtService>,
    pub auth_service: Arc<dyn AuthUseCase>,
    pub example_service: Arc<dyn ExampleUseCase>,
    pub system_service: Arc<dyn SystemUseCase>,
    pub transfer_service: Arc<dyn TransferUseCase>,
    /// 后台会话：验证码、登录、动态路由与数据权限。
    pub admin_auth_service: Arc<dyn AdminAuthUseCase>,
    /// 后台系统管理：用户 / 角色 / 菜单 / 部门 / 岗位 / 字典 / 参数 / 日志。
    pub admin_service: Arc<dyn AdminUseCase>,
    pub monitor_service: Arc<dyn MonitorUseCase>,
    pub dashboard_service: Arc<dyn DashboardUseCase>,
    pub generator_service: Arc<dyn GeneratorUseCase>,
    /// 操作留痕：由审计中间件在写操作后调用。
    pub audit_service: Arc<dyn OperationLogUseCase>,
}

impl Container {
    /// 生产装配：会话落在 Redis。
    pub async fn bootstrap(config: AppConfig) -> Result<Self> {
        let store: Arc<dyn AuthStore> = Arc::new(RedisAuthStore::connect(&config.redis).await?);

        Self::bootstrap_with_auth_store(config, store).await
    }

    /// 指定会话存储的装配：集成测试用内存实现，避免依赖外部 Redis。
    pub async fn bootstrap_with_auth_store(
        config: AppConfig,
        auth_store: Arc<dyn AuthStore>,
    ) -> Result<Self> {
        let config = Arc::new(config);
        let database = connect_database(&config.database).await?;
        run_migrations(&database, &config.database.url).await?;
        let jwt_service = Arc::new(JwtService::new(config.jwt.clone())?);

        let auth_service: Arc<dyn AuthUseCase> = Arc::new(AuthService::new(jwt_service.clone()));
        let example_service: Arc<dyn ExampleUseCase> = Arc::new(ExampleService::new());
        let system_service: Arc<dyn SystemUseCase> =
            Arc::new(SystemService::new(config.clone(), database.clone()));
        let transfer_service: Arc<dyn TransferUseCase> = Arc::new(TransferService::new(
            database.clone(),
            Arc::new(SeaOrmTransferRepository),
        ));
        let admin_auth_service: Arc<dyn AdminAuthUseCase> = Arc::new(AdminAuthService::new(
            database.clone(),
            jwt_service.clone(),
            auth_store,
        ));
        let admin_service: Arc<dyn AdminUseCase> = Arc::new(AdminService::new(database.clone()));
        let monitor_service: Arc<dyn MonitorUseCase> =
            Arc::new(MonitorService::new(database.clone()));
        let dashboard_service: Arc<dyn DashboardUseCase> =
            Arc::new(DashboardService::new(database.clone()));
        let generator_service: Arc<dyn GeneratorUseCase> =
            Arc::new(GeneratorService::new(database.clone()));
        let audit_service: Arc<dyn OperationLogUseCase> =
            Arc::new(AuditService::new(database.clone()));

        Ok(Self {
            config,
            database,
            jwt_service,
            auth_service,
            example_service,
            system_service,
            transfer_service,
            admin_auth_service,
            admin_service,
            monitor_service,
            dashboard_service,
            generator_service,
            audit_service,
        })
    }

    /// 集成分身用的内存会话存储，测试里显式构造。
    pub fn memory_auth_store() -> Arc<dyn AuthStore> {
        Arc::new(MemoryAuthStore::new())
    }
}
