use async_trait::async_trait;

use crate::{
    domain::{
        error::AppError,
        models::{
            audit_log::{LoginLogQuery, LoginLogRow, OperLogQuery, OperLogRow},
            pagination::{Page, PageResult},
        },
        services::admin::logs::LogAdminUseCase,
    },
    infrastructure::repositories::audit_log::{AuditLogRepository, SeaOrmAuditLogRepository},
    services::admin::AdminService,
    util::{csv, ip_mask::IpVisibility},
};

use super::users::EXPORT_LIMIT;

#[async_trait]
impl LogAdminUseCase for AdminService {
    async fn list_oper_logs(
        &self,
        query: &OperLogQuery,
        visibility: IpVisibility,
    ) -> Result<PageResult<OperLogRow>, AppError> {
        let page = Page::from_query(Some(&query.page))?;
        let keyword = query.keyword.as_deref();
        let total = SeaOrmAuditLogRepository
            .count_oper_logs(&self.database, keyword, query.result)
            .await?;
        let mut rows = SeaOrmAuditLogRepository
            .list_oper_logs(&self.database, keyword, query.result, page)
            .await?;
        for row in &mut rows {
            row.ip = visibility.display(Some(row.ip.as_str()));
        }

        Ok(PageResult::new(rows, total, page))
    }

    async fn export_oper_logs(
        &self,
        keyword: Option<&str>,
        result: Option<i64>,
    ) -> Result<String, AppError> {
        let rows = SeaOrmAuditLogRepository
            .list_oper_logs(&self.database, keyword, result, Page::export(EXPORT_LIMIT))
            .await?;
        let data = rows
            .iter()
            .map(|row| {
                vec![
                    Some(row.module.clone()),
                    Some(row.action.clone()),
                    row.method.clone(),
                    row.url.clone(),
                    row.params.clone(),
                    row.oper_name.clone(),
                    row.dept.clone(),
                    Some(row.ip.clone()),
                    csv::number_cell(row.cost),
                    csv::result_text(Some(row.result)),
                    row.oper_time.clone(),
                    row.error_msg.clone(),
                ]
            })
            .collect::<Vec<_>>();

        Ok(csv::to_csv(
            &[
                "模块",
                "类型",
                "请求方式",
                "请求地址",
                "请求参数",
                "操作人",
                "部门",
                "IP",
                "耗时",
                "结果",
                "操作时间",
                "异常信息",
            ],
            &data,
        ))
    }

    async fn delete_oper_log(&self, id: i64) -> Result<(), AppError> {
        if SeaOrmAuditLogRepository
            .delete_oper_log(&self.database, id)
            .await?
            == 0
        {
            return Err(AppError::business("操作日志不存在"));
        }

        Ok(())
    }

    async fn clear_oper_logs(&self) -> Result<(), AppError> {
        SeaOrmAuditLogRepository
            .clear_oper_logs(&self.database)
            .await?;

        Ok(())
    }

    async fn list_login_logs(
        &self,
        query: &LoginLogQuery,
        visibility: IpVisibility,
    ) -> Result<PageResult<LoginLogRow>, AppError> {
        let page = Page::from_query(Some(&query.page))?;
        let keyword = query.keyword.as_deref();
        let total = SeaOrmAuditLogRepository
            .count_login_logs(&self.database, keyword, query.status)
            .await?;
        let mut rows = SeaOrmAuditLogRepository
            .list_login_logs(&self.database, keyword, query.status, page)
            .await?;
        for row in &mut rows {
            row.ip = visibility.display(Some(row.ip.as_str()));
        }

        Ok(PageResult::new(rows, total, page))
    }

    async fn export_login_logs(
        &self,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> Result<String, AppError> {
        let rows = SeaOrmAuditLogRepository
            .list_login_logs(&self.database, keyword, status, Page::export(EXPORT_LIMIT))
            .await?;
        let data = rows
            .iter()
            .map(|row| {
                vec![
                    row.username.clone(),
                    Some(row.ip.clone()),
                    row.browser.clone(),
                    row.os.clone(),
                    csv::result_text(Some(row.status)),
                    row.msg.clone(),
                    row.login_time.clone(),
                ]
            })
            .collect::<Vec<_>>();

        Ok(csv::to_csv(
            &[
                "登录账号",
                "IP",
                "浏览器",
                "系统",
                "状态",
                "描述",
                "登录时间",
            ],
            &data,
        ))
    }

    async fn delete_login_log(&self, id: i64) -> Result<(), AppError> {
        if SeaOrmAuditLogRepository
            .delete_login_log(&self.database, id)
            .await?
            == 0
        {
            return Err(AppError::business("登录日志不存在"));
        }

        Ok(())
    }

    async fn clear_login_logs(&self) -> Result<(), AppError> {
        SeaOrmAuditLogRepository
            .clear_login_logs(&self.database)
            .await?;

        Ok(())
    }
}
