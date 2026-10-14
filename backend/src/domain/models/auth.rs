use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct DevLoginCommand {
    pub user_id: Option<String>,
    pub username: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AccessToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: String,
    pub username: String,
    pub roles: Vec<String>,
    pub iss: String,
    pub aud: String,
    pub iat: usize,
    pub exp: usize,
}

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub user_id: String,
    pub username: String,
    pub roles: Vec<String>,
}
