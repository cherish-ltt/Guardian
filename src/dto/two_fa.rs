use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Default)]
pub struct TwoFaSetupResponse {
    pub secret: String,
    pub qr_code_url: String,
    pub backup_codes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct TwoFaVerifyRequest {
    pub code: String,
}

#[derive(Debug, Serialize, Default)]
pub struct TwoFaVerifyResponse {
    pub verified: bool,
}
