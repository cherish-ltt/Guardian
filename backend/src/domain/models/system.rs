#[derive(Debug, Clone)]
pub struct WelcomeInfo {
    pub service_name: String,
    pub environment: String,
    pub version: String,
    pub docs_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct HealthReport {
    pub service_name: String,
    pub environment: String,
    pub version: String,
    pub status: String,
    pub database_status: String,
    pub timestamp: i64,
}
