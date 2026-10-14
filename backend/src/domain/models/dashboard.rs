use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStat {
    pub key: String,
    pub label: String,
    pub value: i64,
    pub icon: String,
    pub tone: String,
    pub delta: f64,
    pub up: bool,
    /// 上一周期为 0 时标记「新增」，前端不再显示误导的 +100%。
    pub fresh: bool,
    pub sub: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyOps {
    pub date: String,
    pub total: i64,
    pub failed: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardActivity {
    pub who: Option<String>,
    pub action: String,
    pub target: String,
    pub time: Option<String>,
    pub tint: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoleDistribution {
    pub label: String,
    pub value: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    pub stats: Vec<DashboardStat>,
    pub trend: Vec<i64>,
    pub trend_prev: i64,
    pub daily_ops: Vec<DailyOps>,
    pub activities: Vec<DashboardActivity>,
    pub role_dist: Vec<RoleDistribution>,
    pub trend_total: i64,
    pub success_rate: f64,
}

/// 看板统计的原始读数。
#[derive(Debug, Clone, Copy, Default)]
pub struct DashboardCounters {
    pub users: i64,
    pub online: i64,
    pub roles: i64,
    pub menus: i64,
    pub depts: i64,
    pub api_calls: i64,
    pub users_new: i64,
    pub online_2h: i64,
    pub roles_new: i64,
    pub api_prev: i64,
}
