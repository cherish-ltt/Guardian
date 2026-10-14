use async_trait::async_trait;

use crate::{
    domain::{
        error::AppError,
        models::{
            pagination::{Page, PageResult},
            system_admin::{IdResponse, PostRow, PostWriteCommand, StatusQuery},
        },
        services::admin::posts::PostAdminUseCase,
    },
    infrastructure::repositories::post::{PostRepository, SeaOrmPostRepository},
    services::admin::{AdminService, default_int, required, trim_to_null},
    util::csv,
};

use super::users::EXPORT_LIMIT;

#[async_trait]
impl PostAdminUseCase for AdminService {
    async fn list_posts(&self, query: &StatusQuery) -> Result<PageResult<PostRow>, AppError> {
        let page = Page::from_query(Some(&query.page));
        let keyword = query.keyword.as_deref();
        let total = SeaOrmPostRepository
            .count_posts(&self.database, keyword, query.status)
            .await?;
        let rows = SeaOrmPostRepository
            .list_posts(&self.database, keyword, query.status, page)
            .await?;

        Ok(PageResult::new(rows, total, page))
    }

    async fn export_posts(
        &self,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<String, AppError> {
        let rows = SeaOrmPostRepository
            .list_posts(&self.database, keyword, status, Page::export(EXPORT_LIMIT))
            .await?;
        let data = rows
            .iter()
            .map(|row| {
                vec![
                    Some(row.post_code.clone()),
                    Some(row.post_name.clone()),
                    Some(row.sort.to_string()),
                    csv::status_text(Some(row.status)),
                    row.create_time.clone(),
                    row.remark.clone(),
                ]
            })
            .collect::<Vec<_>>();

        Ok(csv::to_csv(
            &["岗位编码", "岗位名称", "排序", "状态", "创建时间", "备注"],
            &data,
        ))
    }

    async fn create_post(
        &self,
        command: PostWriteCommand,
        operator: &str,
    ) -> Result<IdResponse, AppError> {
        let post_code = required(command.post_code.clone(), "岗位编码不能为空")?;
        let post_name = required(command.post_name.clone(), "岗位名称不能为空")?;
        if SeaOrmPostRepository
            .count_post_code(&self.database, &post_code)
            .await?
            > 0
        {
            return Err(AppError::business("岗位编码已存在"));
        }

        let command = PostWriteCommand {
            post_code: Some(post_code),
            post_name: Some(post_name),
            sort: Some(default_int(command.sort, 0)),
            status: Some(default_int(command.status, 1)),
            ..command
        };
        let id = SeaOrmPostRepository
            .insert_post(&self.database, &command, operator)
            .await?;

        Ok(IdResponse { id })
    }

    async fn update_post(
        &self,
        id: i64,
        command: PostWriteCommand,
        operator: &str,
    ) -> Result<(), AppError> {
        let command = PostWriteCommand {
            post_name: trim_to_null(command.post_name.clone()),
            ..command
        };
        if SeaOrmPostRepository
            .update_post(&self.database, id, &command, operator)
            .await?
            == 0
        {
            return Err(AppError::business("岗位不存在或已删除"));
        }

        Ok(())
    }

    async fn delete_post(&self, id: i64, operator: &str) -> Result<(), AppError> {
        if SeaOrmPostRepository
            .soft_delete_post(&self.database, id, operator)
            .await?
            == 0
        {
            return Err(AppError::business("岗位不存在或已删除"));
        }

        Ok(())
    }
}
