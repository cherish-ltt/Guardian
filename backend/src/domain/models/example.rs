use crate::domain::models::auth::CurrentUser;

#[derive(Debug, Clone)]
pub struct CreateExampleCommand {
    pub title: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExampleEcho {
    pub id: String,
    pub title: String,
    pub note: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct ExampleFilters {
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub keyword: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExampleItem {
    pub id: String,
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct ExampleList {
    pub page: u64,
    pub size: u64,
    pub keyword: Option<String>,
    pub items: Vec<ExampleItem>,
}

#[derive(Debug, Clone)]
pub struct ExampleDetail {
    pub id: String,
    pub title: String,
    pub description: String,
    pub requested_by: String,
    pub roles: Vec<String>,
}

impl ExampleDetail {
    pub fn new(id: String, current_user: CurrentUser) -> Self {
        Self {
            id,
            title: "Service template".to_string(),
            description: "A detail API template for extending new modules.".to_string(),
            requested_by: current_user.username,
            roles: current_user.roles,
        }
    }
}
