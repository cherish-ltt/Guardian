use std::sync::Arc;

use axum::{
    Router,
    extract::{RawQuery, State},
    routing::get,
};

use crate::{
    api::{dto::admin::QueryString, extractors::admin_user::AdminUser, ok},
    container::Container,
    domain::{error::AppError, models::dashboard::DashboardSummary},
    response::ApiResponse,
};

pub fn router() -> Router<Arc<Container>> {
    Router::new().route("/dashboard/summary", get(summary))
}

/// 工作台汇总：统计卡、趋势、近 7 天操作量与近期动态。
async fn summary(
    State(container): State<Arc<Container>>,
    _user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<ApiResponse<DashboardSummary>, AppError> {
    let query = QueryString::parse(raw.as_deref());
    let summary = container
        .dashboard_service
        .summary(query.text("range").as_deref())
        .await?;

    Ok(ok(summary))
}
