use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, RawQuery, State},
    routing::get,
};

use crate::{
    api::{
        done,
        dto::admin::QueryString,
        extractors::{admin_user::AdminUser, api_json::ApiJson},
        ok,
    },
    container::Container,
    domain::{
        error::AppError,
        models::{
            audit_log::{LoginLogQuery, LoginLogRow, OperLogQuery, OperLogRow},
            ip_location::{IpLocationBatch, IpLocationItem, IpLocationQuery},
            monitor::{LoginHeatmap, LoginStats, OperStats, OperTimeline},
            pagination::PageResult,
        },
    },
    response::{ApiResponse, csv_attachment},
    util::ip_mask::{IpVisibility, PERMISSION_FULL},
};

pub fn router() -> Router<Arc<Container>> {
    Router::new()
        .route(
            "/monitor/oper-logs",
            get(list_oper_logs).delete(clear_oper_logs),
        )
        .route("/monitor/oper-logs/stats", get(oper_stats))
        .route("/monitor/oper-logs/timeline", get(oper_timeline))
        .route("/monitor/oper-logs/export", get(export_oper_logs))
        .route(
            "/monitor/oper-logs/{id}",
            axum::routing::delete(delete_oper_log),
        )
        .route(
            "/monitor/login-logs",
            get(list_login_logs).delete(clear_login_logs),
        )
        .route("/monitor/login-logs/stats", get(login_stats))
        .route("/monitor/login-logs/heatmap", get(login_heatmap))
        .route("/monitor/login-logs/export", get(export_login_logs))
        .route(
            "/monitor/login-logs/{id}",
            axum::routing::delete(delete_login_log),
        )
        .route("/monitor/ip-locations", get(locate_ip).post(locate_ips))
}

async fn list_oper_logs(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<ApiResponse<PageResult<OperLogRow>>, AppError> {
    user.require("monitor:operlog:list")?;
    let query = QueryString::parse(raw.as_deref());
    let visibility = IpVisibility::of(|code| user.has_permission(code));
    let rows = container
        .admin_service
        .list_oper_logs(
            &OperLogQuery {
                page: query.page()?,
                keyword: query.text("keyword"),
                result: query.int("result")?,
            },
            visibility,
        )
        .await?;

    Ok(ok(rows))
}

async fn oper_stats(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<ApiResponse<OperStats>, AppError> {
    user.require("monitor:operlog:list")?;
    let query = QueryString::parse(raw.as_deref());
    let stats = container
        .monitor_service
        .oper_stats(query.text("range").as_deref())
        .await?;

    Ok(ok(stats))
}

async fn oper_timeline(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<ApiResponse<OperTimeline>, AppError> {
    user.require("monitor:operlog:list")?;
    let query = QueryString::parse(raw.as_deref());
    let timeline = container
        .monitor_service
        .oper_timeline(query.text("date").as_deref())
        .await?;

    Ok(ok(timeline))
}

async fn export_oper_logs(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<axum::response::Response, AppError> {
    user.require("monitor:operlog:list")?;
    let query = QueryString::parse(raw.as_deref());
    let csv = container
        .admin_service
        .export_oper_logs(query.text("keyword").as_deref(), query.int("result")?)
        .await?;

    Ok(csv_attachment("oper-logs.csv", csv))
}

async fn delete_oper_log(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("monitor:operlog:remove")?;
    container.admin_service.delete_oper_log(id).await?;

    Ok(done())
}

async fn clear_oper_logs(
    State(container): State<Arc<Container>>,
    user: AdminUser,
) -> Result<ApiResponse<()>, AppError> {
    user.require("monitor:operlog:remove")?;
    container.admin_service.clear_oper_logs().await?;

    Ok(done())
}

async fn list_login_logs(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<ApiResponse<PageResult<LoginLogRow>>, AppError> {
    user.require("monitor:loginlog:list")?;
    let query = QueryString::parse(raw.as_deref());
    let visibility = IpVisibility::of(|code| user.has_permission(code));
    let rows = container
        .admin_service
        .list_login_logs(
            &LoginLogQuery {
                page: query.page()?,
                keyword: query.text("keyword"),
                status: query.int("status")?,
            },
            visibility,
        )
        .await?;

    Ok(ok(rows))
}

async fn login_stats(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<ApiResponse<LoginStats>, AppError> {
    user.require("monitor:loginlog:list")?;
    let query = QueryString::parse(raw.as_deref());
    let stats = container
        .monitor_service
        .login_stats(query.text("range").as_deref())
        .await?;

    Ok(ok(stats))
}

async fn login_heatmap(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<ApiResponse<LoginHeatmap>, AppError> {
    user.require("monitor:loginlog:list")?;
    let query = QueryString::parse(raw.as_deref());
    let heatmap = container
        .monitor_service
        .login_heatmap(query.int("days")?)
        .await?;

    Ok(ok(heatmap))
}

async fn export_login_logs(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<axum::response::Response, AppError> {
    user.require("monitor:loginlog:list")?;
    let query = QueryString::parse(raw.as_deref());
    let csv = container
        .admin_service
        .export_login_logs(query.text("keyword").as_deref(), query.int("status")?)
        .await?;

    Ok(csv_attachment("login-logs.csv", csv))
}

async fn delete_login_log(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    user.require("monitor:loginlog:remove")?;
    container.admin_service.delete_login_log(id).await?;

    Ok(done())
}

async fn clear_login_logs(
    State(container): State<Arc<Container>>,
    user: AdminUser,
) -> Result<ApiResponse<()>, AppError> {
    user.require("monitor:loginlog:remove")?;
    container.admin_service.clear_login_logs().await?;

    Ok(done())
}

/// 单条属地查询：只有完全查看档可以使用，其余档位一律 403。
#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/api/v1/monitor/ip-locations",
    tag = "Monitor",
    params(("ip" = String, Query, description = "待查询的地址")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Location of the address"),
        (status = 403, description = "Only the full visibility level may query")
    )
))]
pub async fn locate_ip(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    RawQuery(raw): RawQuery,
) -> Result<ApiResponse<IpLocationItem>, AppError> {
    user.require(PERMISSION_FULL)?;
    let query = QueryString::parse(raw.as_deref());
    let item = container
        .ip_location_service
        .locate_ip(&IpLocationQuery {
            ip: query.text("ip"),
        })
        .await?;

    Ok(ok(item))
}

/// 整页批量查询：请求体为 `{ "ips": ["1.2.3.4", ...] }`，返回顺序与请求一致。
#[cfg_attr(feature = "docs", utoipa::path(
    post,
    path = "/api/v1/monitor/ip-locations",
    tag = "Monitor",
    request_body = IpLocationBatch,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Locations of the given addresses"),
        (status = 403, description = "Only the full visibility level may query")
    )
))]
pub async fn locate_ips(
    State(container): State<Arc<Container>>,
    user: AdminUser,
    ApiJson(batch): ApiJson<IpLocationBatch>,
) -> Result<ApiResponse<Vec<IpLocationItem>>, AppError> {
    user.require(PERMISSION_FULL)?;
    let items = container
        .ip_location_service
        .locate_ips(&IpLocationBatch { ips: batch.ips })
        .await?;

    Ok(ok(items))
}
