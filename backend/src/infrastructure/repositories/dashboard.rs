use async_trait::async_trait;
use sea_orm::ConnectionTrait;

use crate::{
    domain::models::dashboard::{DashboardActivity, DashboardCounters, RoleDistribution},
    error::AppError,
    infrastructure::repositories::sql::{int_or_zero, query_all, query_count, query_text, text},
};

/// 工作台汇总所需的只读查询。
#[async_trait]
pub trait DashboardRepository: Send + Sync {
    /// 统计卡读数与环比基数，一次取齐。
    async fn counters<C>(&self, db: &C) -> Result<DashboardCounters, AppError>
    where
        C: ConnectionTrait;

    /// 最近 `hours` 小时的操作次数与成功次数。
    async fn api_calls_in_hours<C>(&self, db: &C, hours: i64) -> Result<(i64, i64), AppError>
    where
        C: ConnectionTrait;

    /// 上一等长周期的登录次数。
    async fn previous_logins<C>(&self, db: &C, hours: i64) -> Result<i64, AppError>
    where
        C: ConnectionTrait;

    /// 近 24 小时的登录量，按小时分组（`(hour, total)`）。
    async fn login_trend_hours<C>(&self, db: &C) -> Result<Vec<(i64, i64)>, AppError>
    where
        C: ConnectionTrait;

    /// 近 `days` 天的登录量（`(day_index, total)`，0 为最早一天）。
    async fn login_trend_days<C>(&self, db: &C, days: i64) -> Result<Vec<(i64, i64)>, AppError>
    where
        C: ConnectionTrait;

    /// 近 7 天按日操作量（`(day_index, total, failed)`，6 为今天）。
    async fn daily_ops<C>(&self, db: &C) -> Result<Vec<(i64, i64, i64)>, AppError>
    where
        C: ConnectionTrait;

    async fn activities<C>(&self, db: &C) -> Result<Vec<DashboardActivity>, AppError>
    where
        C: ConnectionTrait;

    async fn role_distribution<C>(&self, db: &C) -> Result<Vec<RoleDistribution>, AppError>
    where
        C: ConnectionTrait;

    /// 数据库当天日期，缺省日期补零时用它对齐。
    async fn today<C>(&self, db: &C) -> Result<String, AppError>
    where
        C: ConnectionTrait;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SeaOrmDashboardRepository;

#[async_trait]
impl DashboardRepository for SeaOrmDashboardRepository {
    async fn counters<C>(&self, db: &C) -> Result<DashboardCounters, AppError>
    where
        C: ConnectionTrait,
    {
        let users = query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_user WHERE deleted = 0",
            vec![],
        )
        .await?;
        let online = query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_login_log WHERE status = 1 \
             AND login_time >= datetime('now', 'localtime', '-1 hour')",
            vec![],
        )
        .await?;
        let roles = query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_role WHERE deleted = 0",
            vec![],
        )
        .await?;
        let menus = query_count(db, "SELECT COUNT(*) AS value FROM sys_menu", vec![]).await?;
        let depts = query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_dept WHERE deleted = 0",
            vec![],
        )
        .await?;
        let api_calls = query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_oper_log \
             WHERE oper_time >= datetime('now', 'localtime', '-24 hours')",
            vec![],
        )
        .await?;
        let users_new = query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_user WHERE deleted = 0 \
             AND create_time >= datetime('now', 'localtime', '-1 day')",
            vec![],
        )
        .await?;
        let online_2h = query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_login_log WHERE status = 1 \
             AND login_time >= datetime('now', 'localtime', '-2 hours')",
            vec![],
        )
        .await?;
        let roles_new = query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_role WHERE deleted = 0 \
             AND create_time >= datetime('now', 'localtime', '-1 day')",
            vec![],
        )
        .await?;
        let api_prev = query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_oper_log \
             WHERE oper_time >= datetime('now', 'localtime', '-48 hours') \
               AND oper_time < datetime('now', 'localtime', '-24 hours')",
            vec![],
        )
        .await?;

        Ok(DashboardCounters {
            users,
            online,
            roles,
            menus,
            depts,
            api_calls,
            users_new,
            online_2h,
            roles_new,
            api_prev,
        })
    }

    async fn api_calls_in_hours<C>(&self, db: &C, hours: i64) -> Result<(i64, i64), AppError>
    where
        C: ConnectionTrait,
    {
        let modifier = format!("-{hours} hours");
        let total = query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_oper_log WHERE oper_time >= datetime('now', 'localtime', ?)",
            vec![modifier.clone().into()],
        )
        .await?;
        let success = query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_oper_log WHERE oper_time >= datetime('now', 'localtime', ?) \
             AND result = 1",
            vec![modifier.into()],
        )
        .await?;

        Ok((total, success))
    }

    async fn previous_logins<C>(&self, db: &C, hours: i64) -> Result<i64, AppError>
    where
        C: ConnectionTrait,
    {
        if hours <= 24 {
            return query_count(
                db,
                "SELECT COUNT(*) AS value FROM sys_login_log \
                 WHERE login_time >= datetime('now', 'localtime', '-48 hours') \
                   AND login_time < datetime('now', 'localtime', '-24 hours')",
                vec![],
            )
            .await;
        }

        let days = hours / 24;
        let start = format!("-{} days", days * 2);
        let end = format!("-{days} days");

        query_count(
            db,
            "SELECT COUNT(*) AS value FROM sys_login_log \
             WHERE date(login_time) >= date('now', 'localtime', ?) \
               AND date(login_time) < date('now', 'localtime', ?)",
            vec![start.into(), end.into()],
        )
        .await
    }

    async fn login_trend_hours<C>(&self, db: &C) -> Result<Vec<(i64, i64)>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT CAST(strftime('%H', login_time) AS INTEGER) AS hour_value, COUNT(*) AS total \
             FROM sys_login_log WHERE login_time >= datetime('now', 'localtime', '-24 hours') \
             GROUP BY hour_value",
            vec![],
        )
        .await?;

        Ok(rows
            .iter()
            .map(|row| (int_or_zero(row, "hour_value"), int_or_zero(row, "total")))
            .collect())
    }

    async fn login_trend_days<C>(&self, db: &C, days: i64) -> Result<Vec<(i64, i64)>, AppError>
    where
        C: ConnectionTrait,
    {
        let offset = format!("-{} days", days - 1);
        let rows = query_all(
            db,
            "SELECT CAST(julianday(date(login_time)) - julianday(date('now', 'localtime', ?)) \
             AS INTEGER) AS day_index, COUNT(*) AS total FROM sys_login_log \
             WHERE date(login_time) >= date('now', 'localtime', ?) \
             GROUP BY day_index ORDER BY day_index",
            vec![offset.clone().into(), offset.into()],
        )
        .await?;

        Ok(rows
            .iter()
            .map(|row| (int_or_zero(row, "day_index"), int_or_zero(row, "total")))
            .collect())
    }

    async fn daily_ops<C>(&self, db: &C) -> Result<Vec<(i64, i64, i64)>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT CAST(julianday(date(oper_time)) - julianday(date('now', 'localtime', '-6 days')) \
             AS INTEGER) AS day_index, COUNT(*) AS total, COALESCE(SUM(result = 0), 0) AS failed \
             FROM sys_oper_log WHERE date(oper_time) >= date('now', 'localtime', '-6 days') \
             GROUP BY day_index",
            vec![],
        )
        .await?;

        Ok(rows
            .iter()
            .map(|row| {
                (
                    int_or_zero(row, "day_index"),
                    int_or_zero(row, "total"),
                    int_or_zero(row, "failed"),
                )
            })
            .collect())
    }

    async fn activities<C>(&self, db: &C) -> Result<Vec<DashboardActivity>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT username AS who, CASE status WHEN 1 THEN '登录系统' ELSE '登录失败' END AS action, \
             COALESCE(msg, '登录') AS target, substr(login_time, 1, 16) AS occurred_at, \
             CASE status WHEN 1 THEN 'ok' ELSE 'danger' END AS tint FROM sys_login_log \
             ORDER BY login_time DESC, id DESC LIMIT 8",
            vec![],
        )
        .await?;

        Ok(rows
            .iter()
            .map(|row| DashboardActivity {
                who: text(row, "who"),
                action: text(row, "action").unwrap_or_default(),
                target: text(row, "target").unwrap_or_default(),
                time: text(row, "occurred_at"),
                tint: text(row, "tint").unwrap_or_default(),
            })
            .collect())
    }

    async fn role_distribution<C>(&self, db: &C) -> Result<Vec<RoleDistribution>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT r.data_scope AS label, COUNT(u.id) AS value FROM sys_role r \
             LEFT JOIN sys_user_role ur ON r.id = ur.role_id \
             LEFT JOIN sys_user u ON u.id = ur.user_id AND u.deleted = 0 \
             WHERE r.deleted = 0 GROUP BY r.data_scope ORDER BY value DESC",
            vec![],
        )
        .await?;

        Ok(rows
            .iter()
            .map(|row| RoleDistribution {
                label: text(row, "label").unwrap_or_default(),
                value: int_or_zero(row, "value"),
            })
            .collect())
    }

    async fn today<C>(&self, db: &C) -> Result<String, AppError>
    where
        C: ConnectionTrait,
    {
        query_text(db, "SELECT date('now', 'localtime') AS value", vec![])
            .await?
            .ok_or_else(|| AppError::internal("database clock returned no value"))
    }
}
