use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreatePermissionRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub resource_type: String,
    pub http_method: Option<String>,
    pub resource_path: Option<String>,
    pub parent_id: Option<i64>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePermissionRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub resource_type: Option<String>,
    pub http_method: Option<String>,
    pub resource_path: Option<String>,
    pub parent_id: Option<i64>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct PermissionListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub resource_type: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PermissionResponse {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub resource_type: String,
    pub http_method: Option<String>,
    pub resource_path: Option<String>,
    pub parent_id: Option<i64>,
    pub sort_order: i32,
    pub is_system: bool,
    pub created_at: DateTime<Local>,
}

#[derive(Debug, Serialize)]
pub struct PermissionTreeResponse {
    pub id: i64,
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

#[derive(Debug, Serialize)]
pub struct PermissionListResponse {
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub list: Vec<PermissionResponse>,
}
