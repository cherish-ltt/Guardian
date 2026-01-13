use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub permission_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub permission_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
pub struct RoleListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub keyword: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RoleResponse {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: DateTime<Local>,
}

#[derive(Debug, Serialize)]
pub struct RoleDetailResponse {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub permissions: Vec<PermissionSimple>,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

#[derive(Debug, Serialize)]
pub struct PermissionSimple {
    pub id: i64,
    pub code: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct RoleListResponse {
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub list: Vec<RoleResponse>,
}
