use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreatePermissionRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub resource_type: String,
    pub http_method: Option<String>,
    pub resource_path: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePermissionRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub resource_type: Option<String>,
    pub http_method: Option<String>,
    pub resource_path: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct PermissionListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub resource_type: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Debug, Serialize, Default)]
pub struct PermissionResponse {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub resource_type: String,
    pub http_method: Option<String>,
    pub resource_path: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: i32,
    pub is_system: bool,
    pub created_at: DateTime<Local>,
}

#[derive(Debug, Serialize, Default)]
pub struct PermissionTreeResponse {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub resource_type: String,
    pub http_method: Option<String>,
    pub resource_path: Option<String>,
    pub sort_order: i32,
    pub is_system: bool,
    pub children: Vec<PermissionTreeResponse>,
}

#[derive(Debug, Serialize, Default)]
pub struct PermissionListResponse {
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub list: Vec<PermissionResponse>,
}
