use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    error::AppError,
    models::{
        auth::CurrentUser,
        example::{
            CreateExampleCommand, ExampleDetail, ExampleEcho, ExampleFilters, ExampleItem,
            ExampleList,
        },
    },
    services::example::ExampleUseCase,
};

#[derive(Default)]
pub struct ExampleService;

impl ExampleService {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExampleUseCase for ExampleService {
    async fn create_echo(&self, command: CreateExampleCommand) -> Result<ExampleEcho, AppError> {
        let title = command.title.trim();
        if title.is_empty() {
            return Err(AppError::bad_request("title must not be empty"));
        }

        Ok(ExampleEcho {
            id: Uuid::now_v7().to_string(),
            title: title.to_string(),
            note: command.note,
            source: "example-service".to_string(),
        })
    }

    async fn list_examples(&self, filters: ExampleFilters) -> Result<ExampleList, AppError> {
        const MAX_SIZE: u64 = 100;
        const MAX_PAGE: u64 = 10_000;
        let page = filters.page.unwrap_or(1);
        let size = filters.size.unwrap_or(10);

        if page == 0 || page > MAX_PAGE {
            return Err(AppError::bad_request(format!(
                "page must be between 1 and {MAX_PAGE}"
            )));
        }

        if size == 0 || size > MAX_SIZE {
            return Err(AppError::bad_request(format!(
                "size must be between 1 and {MAX_SIZE}"
            )));
        }

        let keyword = filters.keyword;
        let mut items = vec![
            ExampleItem {
                id: "example_001".to_string(),
                title: "Service template".to_string(),
                summary: "Demo for pagination and query parameters".to_string(),
            },
            ExampleItem {
                id: "example_002".to_string(),
                title: "Auth sample".to_string(),
                summary: "Demo of JWT-protected endpoints".to_string(),
            },
            ExampleItem {
                id: "example_003".to_string(),
                title: "Swagger sample".to_string(),
                summary: "Demo of OpenAPI annotation organization".to_string(),
            },
        ];

        if let Some(keyword) = &keyword {
            items.retain(|item| item.title.contains(keyword) || item.summary.contains(keyword));
        }

        Ok(ExampleList {
            page,
            size,
            keyword,
            items,
        })
    }

    async fn get_example_detail(
        &self,
        id: String,
        current_user: CurrentUser,
    ) -> Result<ExampleDetail, AppError> {
        if id.trim().is_empty() {
            return Err(AppError::not_found("example ID does not exist"));
        }

        Ok(ExampleDetail::new(id, current_user))
    }
}
