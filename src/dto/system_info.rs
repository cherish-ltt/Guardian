use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SystemInfoQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SystemInfoResponse {
    pub id: uuid::Uuid,
    pub cpu_count: i32,
    pub cpu_total_load: rust_decimal::Decimal,
    pub memory_used: i64,
    pub memory_total: i64,
    pub disk_used: i64,
    pub disk_total: i64,
    pub network_upload: i64,
    pub network_download: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<crate::entities::guardian_systeminfo::Model> for SystemInfoResponse {
    fn from(model: crate::entities::guardian_systeminfo::Model) -> Self {
        Self {
            id: model.id,
            cpu_count: model.cpu_count,
            cpu_total_load: model.cpu_total_load,
            memory_used: model.memory_used,
            memory_total: model.memory_total,
            disk_used: model.disk_used,
            disk_total: model.disk_total,
            network_upload: model.network_upload,
            network_download: model.network_download,
            created_at: model.created_at.with_timezone(&chrono::Utc),
        }
    }
}
