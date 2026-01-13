use chrono::{DateTime, Local};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateAdminRequest {
    pub username: String,
    pub password: String,
    pub is_super_admin: Option<bool>,
    pub role_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAdminRequest {
    pub password: Option<String>,
    pub status: Option<i16>,
    pub role_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<i16>,
    pub keyword: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminResponse {
    pub id: i64,
    pub username: String,
    pub is_super_admin: bool,
    pub status: i16,
    pub last_login_at: Option<DateTime<Local>>,
    pub created_at: DateTime<Local>,
}

#[derive(Debug, Serialize)]
pub struct AdminDetailResponse {
    pub id: i64,
    pub username: String,
    pub is_super_admin: bool,
    pub status: i16,
    pub last_login_at: Option<DateTime<Local>>,
    pub login_attempts: i32,
    pub locked_until: Option<DateTime<Local>>,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

#[derive(Debug, Serialize)]
pub struct AdminListResponse {
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub list: Vec<AdminResponse>,
}
