use serde::{Deserialize, Serialize};

use crate::domain::models::auth::{AccessToken, CurrentUser, DevLoginCommand};

#[allow(unused_imports)]
#[cfg(feature = "docs")]
use serde_json::json;
#[cfg(feature = "docs")]
use utoipa::ToSchema;

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Deserialize)]
pub struct DevLoginRequest {
    #[cfg_attr(
        feature = "docs",
        schema(example = "019680cc-7e1c-7ec0-b7b8-4b4f8e9dff10")
    )]
    pub user_id: Option<String>,
    #[cfg_attr(feature = "docs", schema(example = "demo-admin"))]
    pub username: String,
    #[cfg_attr(feature = "docs", schema(example = json!(["developer", "admin"])))]
    #[serde(default)]
    pub roles: Vec<String>,
}

impl From<DevLoginRequest> for DevLoginCommand {
    fn from(value: DevLoginRequest) -> Self {
        Self {
            user_id: value.user_id,
            username: value.username,
            roles: value.roles,
        }
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Serialize)]
pub struct AccessTokenResponse {
    #[cfg_attr(feature = "docs", schema(example = "eyJhbGciOiJIUzI1NiJ9.demo.token"))]
    pub access_token: String,
    #[cfg_attr(feature = "docs", schema(example = "Bearer"))]
    pub token_type: String,
    #[cfg_attr(feature = "docs", schema(example = 7200))]
    pub expires_in_seconds: i64,
}

impl From<AccessToken> for AccessTokenResponse {
    fn from(value: AccessToken) -> Self {
        Self {
            access_token: value.access_token,
            token_type: value.token_type,
            expires_in_seconds: value.expires_in_seconds,
        }
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Serialize)]
pub struct CurrentUserResponse {
    #[cfg_attr(
        feature = "docs",
        schema(example = "019680cc-7e1c-7ec0-b7b8-4b4f8e9dff10")
    )]
    pub user_id: String,
    #[cfg_attr(feature = "docs", schema(example = "demo-admin"))]
    pub username: String,
    #[cfg_attr(feature = "docs", schema(example = json!(["developer", "admin"])))]
    pub roles: Vec<String>,
}

impl From<CurrentUser> for CurrentUserResponse {
    fn from(value: CurrentUser) -> Self {
        Self {
            user_id: value.user_id,
            username: value.username,
            roles: value.roles,
        }
    }
}
