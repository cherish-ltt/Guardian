use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub username: String,
    pub nickname: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub dept_name: Option<String>,
    pub roles: Vec<String>,
    pub two_factor_enabled: bool,
    pub recovery_codes_remaining: usize,
}

#[cfg_attr(feature = "docs", derive(utoipa::ToSchema))]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateProfile {
    pub nickname: String,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[cfg_attr(feature = "docs", derive(utoipa::ToSchema))]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecurityProof {
    pub password: String,
    pub second_factor: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Enrollment {
    pub secret: String,
    pub otpauth_url: String,
    pub expires_in: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryCodes {
    pub recovery_codes: Vec<String>,
}
