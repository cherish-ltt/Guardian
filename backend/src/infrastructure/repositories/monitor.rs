use async_trait::async_trait;
use sea_orm::ConnectionTrait;

use crate::{
    domain::models::monitor::OperPoint,
    error::AppError,
    infrastructure::repositories::sql::{int_or_zero, query_all, query_opt, query_text, text},
};

/// 操作日志概览读数。
#[derive(Debug, Clone, Copy, Default)]
pub struct OperOverview {
    pub total: i64,
    pub failed: i64,
    pub avg_cost: i64,
    pub cost_count: i64,
    pub active_users: i64,
}

/// 登录日志概览读数。
#[derive(Debug, Clone, Copy, Default)]
pub struct LoginOverview {
    pub total: i64,
    pub failed: i64,
    pub users: i64,
    pub ips: i64,
}

/// 趋势分桶（`ago` 为距当前时刻的桶数，0 表示当前桶）。
#[derive(Debug, Clone, Copy)]
pub struct BucketCount {
    pub ago: i64,
    pub total: i64,
    pub failed: i64,
}

/// 登录热力的一格：`dow` 0 为周一、6 为周日。
#[derive(Debug, Clone, Copy)]
pub struct HeatmapPoint {
    pub dow: i64,
    pub hour: i64,
    pub total: i64,
    pub failed: i64,
}

/// 失败聚合告警的原始行。
#[derive(Debug, Clone)]
pub struct AnomalyRow {
    pub key: String,
    pub failed: i64,
    pub total: i64,
    pub spread: i64,
    pub first_time: Option<String>,
    pub last_time: Option<String>,
}

/// 监控日志的只读聚合查询。
///
/// 时间边界统一由数据库时钟给出（`datetime('now','localtime')`），
/// 避免应用与会话时区不一致，与 Spring 版的做法一致。
#[async_trait]
pub trait MonitorRepository: Send + Sync {
    async fn db_now<C>(&self, db: &C) -> Result<String, AppError>
    where
        C: ConnectionTrait;

    async fn oper_overview<C>(&self, db: &C, since: &str) -> Result<OperOverview, AppError>
    where
        C: ConnectionTrait;

    /// 升序耗时里第 `offset` 条（从 0 开始），用于算 P95。
    async fn oper_cost_at<C>(
        &self,
        db: &C,
        since: &str,
        offset: i64,
    ) -> Result<Option<i64>, AppError>
    where
        C: ConnectionTrait;

    async fn oper_hourly<C>(
        &self,
        db: &C,
        since: &str,
        now: &str,
    ) -> Result<Vec<BucketCount>, AppError>
    where
        C: ConnectionTrait;

    async fn oper_daily<C>(
        &self,
        db: &C,
        since: &str,
        now: &str,
    ) -> Result<Vec<BucketCount>, AppError>
    where
        C: ConnectionTrait;

    async fn oper_points_of_day<C>(&self, db: &C, start: &str) -> Result<Vec<OperPoint>, AppError>
    where
        C: ConnectionTrait;

    async fn login_overview<C>(&self, db: &C, since: &str) -> Result<LoginOverview, AppError>
    where
        C: ConnectionTrait;

    async fn login_hourly<C>(
        &self,
        db: &C,
        since: &str,
        now: &str,
    ) -> Result<Vec<BucketCount>, AppError>
    where
        C: ConnectionTrait;

    async fn login_daily<C>(
        &self,
        db: &C,
        since: &str,
        now: &str,
    ) -> Result<Vec<BucketCount>, AppError>
    where
        C: ConnectionTrait;

    async fn login_heatmap<C>(&self, db: &C, since: &str) -> Result<Vec<HeatmapPoint>, AppError>
    where
        C: ConnectionTrait;

    async fn login_ip_anomalies<C>(
        &self,
        db: &C,
        since: &str,
        threshold: i64,
    ) -> Result<Vec<AnomalyRow>, AppError>
    where
        C: ConnectionTrait;

    async fn login_user_anomalies<C>(
        &self,
        db: &C,
        since: &str,
        threshold: i64,
    ) -> Result<Vec<AnomalyRow>, AppError>
    where
        C: ConnectionTrait;

    /// 登录失败告警阈值参数（`sys.login.alertThreshold`）。
    async fn login_alert_threshold<C>(&self, db: &C) -> Result<Option<String>, AppError>
    where
        C: ConnectionTrait;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SeaOrmMonitorRepository;

/// 小时桶：与 MySQL `TIMESTAMPDIFF(HOUR, time, now)` 等价（整数秒差除以 3600）。
const HOUR_AGO: &str = "CAST((strftime('%s', ?) - strftime('%s', {column})) AS INTEGER) / 3600";

/// 天桶：与 MySQL `DATEDIFF(DATE(now), DATE(time))` 等价。
const DAY_AGO: &str = "CAST(julianday(date(?)) - julianday(date({column})) AS INTEGER)";

#[async_trait]
impl MonitorRepository for SeaOrmMonitorRepository {
    async fn db_now<C>(&self, db: &C) -> Result<String, AppError>
    where
        C: ConnectionTrait,
    {
        query_text(db, "SELECT datetime('now', 'localtime') AS value", vec![])
            .await?
            .ok_or_else(|| AppError::internal("database clock returned no value"))
    }

    async fn oper_overview<C>(&self, db: &C, since: &str) -> Result<OperOverview, AppError>
    where
        C: ConnectionTrait,
    {
        let sql = "SELECT COUNT(*) AS total, COALESCE(SUM(result = 0), 0) AS failed, \
             COALESCE(CAST(ROUND(AVG(cost)) AS INTEGER), 0) AS avg_cost, COUNT(cost) AS cost_count, \
             COUNT(DISTINCT oper_name) AS active_users FROM sys_oper_log WHERE oper_time >= ?";
        let row = query_opt(db, sql, vec![since.into()])
            .await?
            .ok_or_else(|| AppError::internal("oper log overview returned no row"))?;

        Ok(OperOverview {
            total: int_or_zero(&row, "total"),
            failed: int_or_zero(&row, "failed"),
            avg_cost: int_or_zero(&row, "avg_cost"),
            cost_count: int_or_zero(&row, "cost_count"),
            active_users: int_or_zero(&row, "active_users"),
        })
    }

    async fn oper_cost_at<C>(
        &self,
        db: &C,
        since: &str,
        offset: i64,
    ) -> Result<Option<i64>, AppError>
    where
        C: ConnectionTrait,
    {
        let row = query_opt(
            db,
            "SELECT cost AS value FROM sys_oper_log \
             WHERE oper_time >= ? AND cost IS NOT NULL ORDER BY cost LIMIT 1 OFFSET ?",
            vec![since.into(), offset.into()],
        )
        .await?;

        Ok(row.as_ref().and_then(|row| super::sql::int(row, "value")))
    }

    async fn oper_hourly<C>(
        &self,
        db: &C,
        since: &str,
        now: &str,
    ) -> Result<Vec<BucketCount>, AppError>
    where
        C: ConnectionTrait,
    {
        let sql = format!(
            "SELECT {} AS ago, COUNT(*) AS total, COALESCE(SUM(result = 0), 0) AS failed \
             FROM sys_oper_log WHERE oper_time >= ? GROUP BY ago",
            HOUR_AGO.replace("{column}", "oper_time")
        );

        bucket_counts(db, &sql, since, now).await
    }

    async fn oper_daily<C>(
        &self,
        db: &C,
        since: &str,
        now: &str,
    ) -> Result<Vec<BucketCount>, AppError>
    where
        C: ConnectionTrait,
    {
        let sql = format!(
            "SELECT {} AS ago, COUNT(*) AS total, COALESCE(SUM(result = 0), 0) AS failed \
             FROM sys_oper_log WHERE oper_time >= ? GROUP BY ago",
            DAY_AGO.replace("{column}", "oper_time")
        );

        bucket_counts(db, &sql, since, now).await
    }

    async fn oper_points_of_day<C>(&self, db: &C, start: &str) -> Result<Vec<OperPoint>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT id, module, \
             (CAST(strftime('%H', oper_time) AS INTEGER) * 3600 \
              + CAST(strftime('%M', oper_time) AS INTEGER) * 60 \
              + CAST(strftime('%S', oper_time) AS INTEGER)) AS sec, \
             result, COALESCE(cost, 0) AS cost FROM sys_oper_log \
             WHERE oper_time >= ? AND oper_time < datetime(?, '+1 day') \
             ORDER BY module, oper_time, id LIMIT 5000",
            vec![start.into(), start.into()],
        )
        .await?;

        Ok(rows
            .iter()
            .map(|row| OperPoint {
                id: int_or_zero(row, "id"),
                module: text(row, "module"),
                sec: int_or_zero(row, "sec"),
                result: int_or_zero(row, "result"),
                cost: int_or_zero(row, "cost"),
            })
            .collect())
    }

    async fn login_overview<C>(&self, db: &C, since: &str) -> Result<LoginOverview, AppError>
    where
        C: ConnectionTrait,
    {
        let sql = "SELECT COUNT(*) AS total, COALESCE(SUM(status = 0), 0) AS failed, \
             COUNT(DISTINCT username) AS users, COUNT(DISTINCT ip) AS ips \
             FROM sys_login_log WHERE login_time >= ?";
        let row = query_opt(db, sql, vec![since.into()])
            .await?
            .ok_or_else(|| AppError::internal("login log overview returned no row"))?;

        Ok(LoginOverview {
            total: int_or_zero(&row, "total"),
            failed: int_or_zero(&row, "failed"),
            users: int_or_zero(&row, "users"),
            ips: int_or_zero(&row, "ips"),
        })
    }

    async fn login_hourly<C>(
        &self,
        db: &C,
        since: &str,
        now: &str,
    ) -> Result<Vec<BucketCount>, AppError>
    where
        C: ConnectionTrait,
    {
        let sql = format!(
            "SELECT {} AS ago, COUNT(*) AS total, COALESCE(SUM(status = 0), 0) AS failed \
             FROM sys_login_log WHERE login_time >= ? GROUP BY ago",
            HOUR_AGO.replace("{column}", "login_time")
        );

        bucket_counts(db, &sql, since, now).await
    }

    async fn login_daily<C>(
        &self,
        db: &C,
        since: &str,
        now: &str,
    ) -> Result<Vec<BucketCount>, AppError>
    where
        C: ConnectionTrait,
    {
        let sql = format!(
            "SELECT {} AS ago, COUNT(*) AS total, COALESCE(SUM(status = 0), 0) AS failed \
             FROM sys_login_log WHERE login_time >= ? GROUP BY ago",
            DAY_AGO.replace("{column}", "login_time")
        );

        bucket_counts(db, &sql, since, now).await
    }

    async fn login_heatmap<C>(&self, db: &C, since: &str) -> Result<Vec<HeatmapPoint>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT ((CAST(strftime('%w', login_time) AS INTEGER) + 6) % 7) AS dow, \
             CAST(strftime('%H', login_time) AS INTEGER) AS hour, COUNT(*) AS total, \
             COALESCE(SUM(status = 0), 0) AS failed FROM sys_login_log \
             WHERE login_time >= ? GROUP BY dow, hour",
            vec![since.into()],
        )
        .await?;

        Ok(rows
            .iter()
            .map(|row| HeatmapPoint {
                dow: int_or_zero(row, "dow"),
                hour: int_or_zero(row, "hour"),
                total: int_or_zero(row, "total"),
                failed: int_or_zero(row, "failed"),
            })
            .collect())
    }

    async fn login_ip_anomalies<C>(
        &self,
        db: &C,
        since: &str,
        threshold: i64,
    ) -> Result<Vec<AnomalyRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT ip AS anomaly_key, COALESCE(SUM(status = 0), 0) AS failed, COUNT(*) AS total, \
             COUNT(DISTINCT username) AS spread, substr(MIN(login_time), 1, 16) AS first_time, \
             substr(MAX(login_time), 1, 16) AS last_time FROM sys_login_log \
             WHERE login_time >= ? AND ip IS NOT NULL AND ip <> '' GROUP BY ip \
             HAVING COALESCE(SUM(status = 0), 0) >= ? \
             ORDER BY failed DESC, last_time DESC LIMIT 12",
            vec![since.into(), threshold.into()],
        )
        .await?;

        Ok(rows.iter().map(map_anomaly).collect())
    }

    async fn login_user_anomalies<C>(
        &self,
        db: &C,
        since: &str,
        threshold: i64,
    ) -> Result<Vec<AnomalyRow>, AppError>
    where
        C: ConnectionTrait,
    {
        let rows = query_all(
            db,
            "SELECT username AS anomaly_key, COALESCE(SUM(status = 0), 0) AS failed, \
             COUNT(*) AS total, COUNT(DISTINCT ip) AS spread, \
             substr(MIN(login_time), 1, 16) AS first_time, substr(MAX(login_time), 1, 16) AS last_time \
             FROM sys_login_log WHERE login_time >= ? AND username IS NOT NULL AND username <> '' \
             GROUP BY username HAVING COALESCE(SUM(status = 0), 0) >= ? \
             ORDER BY failed DESC, last_time DESC LIMIT 12",
            vec![since.into(), threshold.into()],
        )
        .await?;

        Ok(rows.iter().map(map_anomaly).collect())
    }

    async fn login_alert_threshold<C>(&self, db: &C) -> Result<Option<String>, AppError>
    where
        C: ConnectionTrait,
    {
        query_text(
            db,
            "SELECT config_value AS value FROM sys_config \
             WHERE config_key = 'sys.login.alertThreshold' LIMIT 1",
            vec![],
        )
        .await
    }
}

async fn bucket_counts<C>(
    db: &C,
    sql: &str,
    since: &str,
    now: &str,
) -> Result<Vec<BucketCount>, AppError>
where
    C: ConnectionTrait,
{
    let rows = query_all(db, sql, vec![now.into(), since.into()]).await?;

    Ok(rows
        .iter()
        .map(|row| BucketCount {
            ago: int_or_zero(row, "ago"),
            total: int_or_zero(row, "total"),
            failed: int_or_zero(row, "failed"),
        })
        .collect())
}

fn map_anomaly(row: &sea_orm::QueryResult) -> AnomalyRow {
    AnomalyRow {
        key: text(row, "anomaly_key").unwrap_or_default(),
        failed: int_or_zero(row, "failed"),
        total: int_or_zero(row, "total"),
        spread: int_or_zero(row, "spread"),
        first_time: text(row, "first_time"),
        last_time: text(row, "last_time"),
    }
}
