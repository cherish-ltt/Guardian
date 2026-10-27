use async_trait::async_trait;

use crate::{
    domain::{
        error::AppError,
        models::ip_location::{IpLocationBatch, IpLocationItem, IpLocationQuery},
    },
    services::ip_location_cache::IpLocationService,
};

/// IP 属地用例：把地址交给离线库现查，不读写日志表。
#[async_trait]
pub trait IpLocationUseCase: Send + Sync {
    /// 单条查询；地址为空或非法时 `location` 为空串。
    async fn locate_ip(&self, query: &IpLocationQuery) -> Result<IpLocationItem, AppError>;

    /// 批量查询（整页）：顺序与入参一致。
    async fn locate_ips(&self, batch: &IpLocationBatch) -> Result<Vec<IpLocationItem>, AppError>;
}

#[async_trait]
impl IpLocationUseCase for IpLocationService {
    async fn locate_ip(&self, query: &IpLocationQuery) -> Result<IpLocationItem, AppError> {
        let ip = query.ip.clone().unwrap_or_default();
        let mut items = self.lookup(&[ip]).await;

        Ok(items.remove(0))
    }

    async fn locate_ips(&self, batch: &IpLocationBatch) -> Result<Vec<IpLocationItem>, AppError> {
        Ok(self.lookup(&batch.ips).await)
    }
}
