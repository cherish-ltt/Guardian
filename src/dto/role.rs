use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub permission_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub permission_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize)]
pub struct RoleListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub keyword: Option<String>,
}

#[derive(Debug, Serialize, Default)]
pub struct RoleResponse {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

#[derive(Debug, Serialize, Default)]
pub struct RoleDetailResponse {
    pub id: Uuid,
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
    pub id: Uuid,
    pub code: String,
    pub name: String,
}

#[derive(Debug, Serialize, Default)]
pub struct RoleListResponse {
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub list: Vec<RoleResponse>,
}
