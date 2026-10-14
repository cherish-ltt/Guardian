use serde::{Deserialize, Serialize};

use crate::domain::models::example::{
    CreateExampleCommand, ExampleDetail, ExampleEcho, ExampleFilters, ExampleItem, ExampleList,
};

#[allow(unused_imports)]
#[cfg(feature = "docs")]
use serde_json::json;
#[cfg(feature = "docs")]
use utoipa::{IntoParams, ToSchema};

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Deserialize)]
pub struct ExampleEchoRequest {
    #[cfg_attr(feature = "docs", schema(example = "Set up a new service"))]
    pub title: String,
    #[cfg_attr(feature = "docs", schema(example = "Start with logging and JWT"))]
    pub note: Option<String>,
}

impl From<ExampleEchoRequest> for CreateExampleCommand {
    fn from(value: ExampleEchoRequest) -> Self {
        Self {
            title: value.title,
            note: value.note,
        }
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Serialize)]
pub struct ExampleEchoResponse {
    #[cfg_attr(
        feature = "docs",
        schema(example = "019680cc-7e1c-7ec0-b7b8-4b4f8e9dff10")
    )]
    pub id: String,
    #[cfg_attr(feature = "docs", schema(example = "Set up a new service"))]
    pub title: String,
    #[cfg_attr(feature = "docs", schema(example = "Start with logging and JWT"))]
    pub note: Option<String>,
    #[cfg_attr(feature = "docs", schema(example = "example-service"))]
    pub source: String,
}

impl From<ExampleEcho> for ExampleEchoResponse {
    fn from(value: ExampleEcho) -> Self {
        Self {
            id: value.id,
            title: value.title,
            note: value.note,
            source: value.source,
        }
    }
}

#[cfg_attr(feature = "docs", derive(IntoParams, ToSchema))]
#[derive(Debug, Deserialize)]
pub struct ExampleQuery {
    #[cfg_attr(feature = "docs", param(example = 1))]
    pub page: Option<u64>,
    #[cfg_attr(feature = "docs", param(example = 10))]
    pub size: Option<u64>,
    #[cfg_attr(feature = "docs", param(example = "service"))]
    pub keyword: Option<String>,
}

impl From<ExampleQuery> for ExampleFilters {
    fn from(value: ExampleQuery) -> Self {
        Self {
            page: value.page,
            size: value.size,
            keyword: value.keyword,
        }
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Serialize)]
pub struct ExampleListItem {
    #[cfg_attr(feature = "docs", schema(example = "example_001"))]
    pub id: String,
    #[cfg_attr(feature = "docs", schema(example = "Service template"))]
    pub title: String,
    #[cfg_attr(
        feature = "docs",
        schema(example = "Demo for pagination and query parameters")
    )]
    pub summary: String,
}

impl From<ExampleItem> for ExampleListItem {
    fn from(value: ExampleItem) -> Self {
        Self {
            id: value.id,
            title: value.title,
            summary: value.summary,
        }
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Serialize)]
pub struct ExampleListResponse {
    #[cfg_attr(feature = "docs", schema(example = 1))]
    pub page: u64,
    #[cfg_attr(feature = "docs", schema(example = 10))]
    pub size: u64,
    #[cfg_attr(feature = "docs", schema(example = "service"))]
    pub keyword: Option<String>,
    pub items: Vec<ExampleListItem>,
}

impl From<ExampleList> for ExampleListResponse {
    fn from(value: ExampleList) -> Self {
        Self {
            page: value.page,
            size: value.size,
            keyword: value.keyword,
            items: value.items.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Serialize)]
pub struct ExampleDetailResponse {
    #[cfg_attr(feature = "docs", schema(example = "example_001"))]
    pub id: String,
    #[cfg_attr(feature = "docs", schema(example = "Service template"))]
    pub title: String,
    #[cfg_attr(
        feature = "docs",
        schema(example = "A detail API template for extending new modules.")
    )]
    pub description: String,
    #[cfg_attr(feature = "docs", schema(example = "demo-admin"))]
    pub requested_by: String,
    #[cfg_attr(feature = "docs", schema(example = json!(["developer", "admin"])))]
    pub roles: Vec<String>,
}

impl From<ExampleDetail> for ExampleDetailResponse {
    fn from(value: ExampleDetail) -> Self {
        Self {
            id: value.id,
            title: value.title,
            description: value.description,
            requested_by: value.requested_by,
            roles: value.roles,
        }
    }
}
