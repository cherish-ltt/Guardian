use anyhow::Result;
use dotenv::dotenv;
use guardian_backend::{
    container::Container, create_app::create_app, infrastructure::config::AppConfig, logging,
};
use mimalloc::MiMalloc;
use std::sync::Arc;
use tracing::info;

/// 全局内存分配器。
///
/// 对于长期运行的 Web 服务，统一分配器有助于保持行为稳定。
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

/// 程序启动入口。
///
/// 启动流程固定为：
///
/// 1. 读取环境变量
/// 2. 初始化日志
/// 3. 打印生效配置
/// 4. 构建全局状态
/// 5. 构建 Axum 路由
/// 6. 启动 HTTP 服务
#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let config = AppConfig::from_env()?;
    config.validate_runtime()?;
    let _log_guard = logging::init(&config)?;
    logging::log_startup_config(&config);

    let container = Arc::new(Container::bootstrap(config).await?);
    let app = create_app(container.clone());
    let address = container.config.server.socket_addr()?;

    let listener = tokio::net::TcpListener::bind(address).await?;
    info!(address = %address, "HTTP server started successfully");

    axum::serve(
        listener,
        // 请求保护层按真实 TCP 对端分配额度，缺少对端信息时拒绝请求，不静默绕过。
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    info!("HTTP server shut down gracefully");
    if let Err(error) = container.database.close_by_ref().await {
        tracing::warn!(%error, "failed to close the database connection pool");
    }
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for Ctrl-C signal");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to listen for termination signal")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("received Ctrl-C, starting graceful shutdown"),
        _ = terminate => info!("received termination signal, starting graceful shutdown"),
    }
}
