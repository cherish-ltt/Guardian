//! 监控与工作台接口的契约测试，以及写操作留痕（审计中间件）的行为验证。

mod support;

use axum::http::StatusCode;
use serde_json::{Value, json};

use support::{
    admin_token, delete_with_bearer, get, get_with_bearer, login, post_json, post_json_with_bearer,
    send, send_raw, setup_app, setup_app_with_trusted_proxies,
};

fn with_forwarded_for(
    mut request: axum::http::Request<axum::body::Body>,
) -> axum::http::Request<axum::body::Body> {
    request
        .headers_mut()
        .insert("x-forwarded-for", "127.0.0.1".parse().expect("头值"));

    request
}

#[tokio::test]
async fn write_operations_are_recorded_with_module_action_and_operator() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, _) = send(
        app.clone(),
        with_forwarded_for(post_json_with_bearer(
            "/api/v1/system/posts",
            &token,
            json!({ "postCode": "audit_probe", "postName": "留痕岗位" }),
        )),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/oper-logs?pageNum=1&pageSize=10", &token),
    )
    .await;
    assert_eq!(logs["data"]["total"], 1);
    let row = &logs["data"]["rows"][0];
    assert_eq!(row["module"], "岗位管理");
    assert_eq!(row["action"], "新增");
    assert_eq!(row["method"], "POST");
    assert_eq!(row["url"], "/system/posts");
    assert_eq!(row["operName"], "admin");
    assert_eq!(row["dept"], Value::Null, "与 Spring 版一致地不写部门");
    assert_eq!(row["ip"], "127.0.0.1");
    assert_eq!(row["location"], "本机");
    assert_eq!(row["result"], 1);
    assert_eq!(row["errorMsg"], Value::Null);
    assert!(row["cost"].as_i64().unwrap_or(-1) >= 0);
    assert!(row["operTime"].as_str().expect("操作时间").len() >= 16);
    let params = row["params"].as_str().expect("请求参数");
    assert!(params.contains("audit_probe"));

    // 失败的业务操作也留痕，并带上错误文案。
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/posts",
            &token,
            json!({ "postCode": "audit_probe", "postName": "重复岗位" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "岗位编码已存在");

    let (_, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/oper-logs?result=0", &token),
    )
    .await;
    assert_eq!(logs["data"]["total"], 1);
    assert_eq!(logs["data"]["rows"][0]["action"], "新增");
    assert_eq!(logs["data"]["rows"][0]["errorMsg"], "岗位编码已存在");

    // 权限被拒的请求不进业务方法，因此不留痕（与 Spring 版切面一致）。
    let outsider = login(&app, "linjh", support::DEMO_PASSWORD).await;
    let (status, _) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/posts",
            &outsider,
            json!({ "postCode": "denied", "postName": "被拒" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (_, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/oper-logs?pageNum=1&pageSize=10", &token),
    )
    .await;
    assert_eq!(logs["data"]["total"], 2);

    // 敏感字段脱敏：重置密码把口令写成 `******`。
    let (status, _) = send(
        app.clone(),
        support::put_json_with_bearer(
            "/api/v1/system/users/3/password",
            &token,
            json!({ "password": "Ss@123456" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/oper-logs?keyword=重置密码", &token),
    )
    .await;
    let params = logs["data"]["rows"][0]["params"]
        .as_str()
        .expect("请求参数");
    assert!(params.contains("\"password\":\"******\""));
    assert!(!params.contains("Ss@123456"));

    // 删除与清空。
    let (status, _) = send(
        app.clone(),
        delete_with_bearer(
            &format!(
                "/api/v1/monitor/oper-logs/{}",
                logs["data"]["rows"][0]["id"]
            ),
            &token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/monitor/oper-logs/999999", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "操作日志不存在");

    let (status, _) = send(
        app.clone(),
        delete_with_bearer("/api/v1/monitor/oper-logs", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/oper-logs", &token),
    )
    .await;
    assert_eq!(logs["data"]["total"], 0);
}

#[tokio::test]
async fn oper_stats_and_timeline_aggregate_recorded_operations() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    for (code, name) in [("stat_a", "统计甲"), ("stat_b", "统计乙")] {
        let (status, _) = send(
            app.clone(),
            post_json_with_bearer(
                "/api/v1/system/posts",
                &token,
                json!({ "postCode": code, "postName": name }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }
    let (_, _) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/posts",
            &token,
            json!({ "postCode": "stat_a", "postName": "重复" }),
        ),
    )
    .await;

    let (status, stats) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/oper-logs/stats?range=24h", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let data = &stats["data"];
    assert_eq!(data["range"], "24h");
    assert_eq!(data["total"], 3);
    assert_eq!(data["failed"], 1);
    assert_eq!(data["successRate"], 66.7);
    assert_eq!(data["activeUsers"], 1);
    assert!(data["avgCost"].as_i64().unwrap_or(-1) >= 0);
    assert!(data["p95Cost"].as_i64().unwrap_or(-1) >= 0);
    let series = data["series"].as_array().expect("趋势系列");
    assert_eq!(series.len(), 24);
    assert_eq!(
        series
            .iter()
            .map(|bucket| bucket["total"].as_i64().unwrap_or(0))
            .sum::<i64>(),
        3
    );
    let bucket = series[23]["bucket"].as_str().expect("桶标签");
    assert!(bucket.ends_with(":00"), "24h 档按小时分桶: {bucket}");

    let (_, weekly) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/oper-logs/stats?range=7d", &token),
    )
    .await;
    assert_eq!(weekly["data"]["range"], "7d");
    assert_eq!(weekly["data"]["series"].as_array().expect("7 天").len(), 7);

    let (status, timeline) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/oper-logs/timeline", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let lanes = timeline["data"]["lanes"].as_array().expect("泳道");
    assert_eq!(lanes.len(), 1);
    assert_eq!(lanes[0]["module"], "岗位管理");
    assert_eq!(lanes[0]["count"], 3);
    let blocks = lanes[0]["blocks"].as_array().expect("时间块");
    assert_eq!(blocks.len(), 1, "同一模块的连续操作合为一个块");
    assert_eq!(blocks[0]["count"], 3);
    assert_eq!(blocks[0]["failed"], 1);
    assert_eq!(blocks[0]["ids"].as_array().expect("日志 ID").len(), 3);
    assert!(timeline["data"]["nowSec"].as_i64().is_some());

    // 指定日期（今天）与数据库当天一致；过去日期没有 nowSec。
    let today = timeline["data"]["date"].as_str().expect("日期");
    let (_, dated) = send(
        app.clone(),
        get_with_bearer(
            &format!("/api/v1/monitor/oper-logs/timeline?date={today}"),
            &token,
        ),
    )
    .await;
    assert_eq!(dated["data"]["lanes"].as_array().expect("泳道").len(), 1);

    let (status, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/oper-logs/timeline?date=2020-01-01", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["lanes"].as_array().expect("泳道").len(), 0);
    assert_eq!(body["data"]["nowSec"], Value::Null);
}

#[tokio::test]
async fn login_stats_heatmap_and_exports() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, stats) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/login-logs/stats", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stats["data"]["range"], "24h");
    assert_eq!(stats["data"]["total"], 1);
    assert_eq!(stats["data"]["failed"], 0);
    assert_eq!(stats["data"]["successRate"], 100.0);
    assert_eq!(stats["data"]["users"], 1);
    assert_eq!(stats["data"]["series"].as_array().expect("系列").len(), 24);

    let (status, heatmap) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/login-logs/heatmap", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(heatmap["data"]["days"], 7);
    assert_eq!(
        heatmap["data"]["threshold"], 5,
        "默认阈值来自参数缺失时的默认值"
    );
    assert_eq!(
        heatmap["data"]["cells"].as_array().expect("热力格").len(),
        1
    );
    assert_eq!(heatmap["data"]["cells"][0]["total"], 1);
    assert_eq!(
        heatmap["data"]["anomalies"].as_array().expect("告警").len(),
        0
    );

    let (_, monthly) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/login-logs/heatmap?days=30", &token),
    )
    .await;
    assert_eq!(monthly["data"]["days"], 30);

    let (_, _, login_csv) = send_raw(
        app.clone(),
        get_with_bearer("/api/v1/monitor/login-logs/export", &token),
    )
    .await;
    let login_csv = String::from_utf8(login_csv).expect("CSV");
    assert!(login_csv.starts_with("登录账号,IP,地点,浏览器,系统,状态,描述,登录时间\n"));
    assert!(
        login_csv.contains("admin,127.0.0.1,本机,,,成功,登录成功,"),
        "来源 IP 与地点已采集，测试请求不带 User-Agent 时浏览器与系统留空: {login_csv}"
    );

    let (_, _, oper_csv) = send_raw(
        app.clone(),
        get_with_bearer("/api/v1/monitor/oper-logs/export", &token),
    )
    .await;
    let oper_csv = String::from_utf8(oper_csv).expect("CSV");
    assert!(oper_csv.starts_with(
        "模块,类型,请求方式,请求地址,请求参数,操作人,部门,IP,地点,耗时,结果,操作时间,异常信息\n"
    ));

    let (_, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/login-logs?keyword=admin", &token),
    )
    .await;
    let log_id = logs["data"]["rows"][0]["id"].as_i64().expect("日志 ID");
    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/monitor/login-logs/{log_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/monitor/login-logs/{log_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "登录日志不存在");

    let (status, _) = send(
        app.clone(),
        delete_with_bearer("/api/v1/monitor/login-logs", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/login-logs", &token),
    )
    .await;
    assert_eq!(logs["data"]["total"], 0);
}

#[tokio::test]
async fn dashboard_summary_reports_cards_trends_and_activities() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, summary) = send(
        app.clone(),
        get_with_bearer("/api/v1/dashboard/summary", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let data = &summary["data"];
    let stats = data["stats"].as_array().expect("统计卡");
    assert_eq!(stats.len(), 6);
    let by_key = |key: &str| {
        stats
            .iter()
            .find(|stat| stat["key"] == key)
            .expect("统计卡存在")
            .clone()
    };
    assert_eq!(by_key("users")["value"], 3);
    assert_eq!(by_key("users")["label"], "用户总数");
    assert_eq!(by_key("users")["sub"], "当前启用账号");
    assert_eq!(by_key("users")["tone"], "ok");
    assert_eq!(by_key("users")["up"], true);
    assert_eq!(by_key("menus")["value"], 38);
    assert_eq!(by_key("menus")["sub"], "目录 / 菜单 / 按钮");
    assert_eq!(by_key("depts")["value"], 11);
    assert_eq!(by_key("online")["value"], 1, "刚刚登录了一个账号");
    // 本次用例没有写操作，也没有上一周期的数据，因此不显示环比。
    assert_eq!(by_key("api")["value"], 0);
    assert_eq!(by_key("api")["delta"], 0.0);
    assert_eq!(by_key("api")["up"], false);
    assert_eq!(by_key("api")["fresh"], false);
    assert_eq!(by_key("api")["sub"], "最近 24 小时操作");

    assert_eq!(data["dailyOps"].as_array().expect("近 7 天").len(), 7);
    assert_eq!(data["dailyOps"][6]["total"], 0);
    assert!(data["dailyOps"][6]["date"].as_str().expect("日期").len() == 10);
    assert_eq!(data["trend"].as_array().expect("趋势").len(), 24);
    assert_eq!(data["trendPrev"], 0);
    assert_eq!(data["trendTotal"], 0);
    assert_eq!(data["successRate"], 100.0);
    let activities = data["activities"].as_array().expect("近期动态");
    assert_eq!(activities.len(), 1);
    assert_eq!(activities[0]["who"], "admin");
    assert_eq!(activities[0]["action"], "登录系统");
    assert_eq!(activities[0]["target"], "登录成功");
    assert_eq!(activities[0]["tint"], "ok");
    // 角色分布按数据范围聚合，值是该范围下已分配用户的账号数。
    let role_dist = data["roleDist"].as_array().expect("角色分布");
    assert_eq!(role_dist.len(), 5);
    let mut pairs = role_dist
        .iter()
        .map(|row| {
            (
                row["label"].as_str().unwrap_or_default().to_string(),
                row["value"].as_i64().unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    pairs.sort();
    assert_eq!(
        pairs,
        vec![
            ("ALL".to_string(), 1),
            ("CUSTOM".to_string(), 0),
            ("DEPT".to_string(), 1),
            ("DEPT_AND_CHILD".to_string(), 1),
            ("SELF".to_string(), 0),
        ]
    );

    let (_, weekly) = send(
        app.clone(),
        get_with_bearer("/api/v1/dashboard/summary?range=7d", &token),
    )
    .await;
    assert_eq!(weekly["data"]["trend"].as_array().expect("趋势").len(), 7);
}

#[tokio::test]
async fn health_probe_is_public_and_reports_up() {
    let app = setup_app().await;

    let (status, body) = send(app.clone(), get("/api/v1/health")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "UP");
    assert_eq!(body["data"]["serviceName"], "guardian-backend");
    assert!(body["data"]["time"].as_str().expect("服务端时间").len() >= 19);

    // 既有探针保持不变。
    let (status, _) = send(app.clone(), get("/api/v1/system/health")).await;
    assert_eq!(status, StatusCode::OK);
}

/// 操作日志导出：有记录时逐列输出，列顺序与 Spring 版一致。
#[tokio::test]
async fn oper_log_export_contains_recorded_rows() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, _) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/posts",
            &token,
            json!({ "postCode": "export_probe", "postName": "导出探针", "sort": 20 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, headers, bytes) = send_raw(
        app.clone(),
        get_with_bearer("/api/v1/monitor/oper-logs/export", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/csv; charset=UTF-8")
    );

    let csv = String::from_utf8(bytes).expect("CSV 为合法 UTF-8");
    assert_eq!(
        csv.lines().next().expect("表头"),
        "模块,类型,请求方式,请求地址,请求参数,操作人,部门,IP,地点,耗时,结果,操作时间,异常信息"
    );

    let row = csv
        .lines()
        .find(|line| line.contains("export_probe"))
        .expect("导出内容包含刚写入的岗位");
    assert!(
        row.starts_with("岗位管理,新增,POST,/system/posts,"),
        "前四列是模块 / 动作 / 方法 / 相对路径（已 nest 掉 /api/v1）: {row}"
    );
    assert!(
        row.contains(r#""{""postCode"#),
        "请求参数按 CSV 规则转义后原样落盘: {row}"
    );
    assert!(
        row.contains(",admin,,127.0.0.1,本机,"),
        "操作人已记录；部门留空，IP 与地点取保护层解析出的真实对端: {row}"
    );
    assert!(row.contains(",成功,"), "结果列为成功: {row}");
}

/// 登录失败阈值取自参数配置；达到阈值的账号出现在热力图告警里。
#[tokio::test]
async fn login_heatmap_uses_configured_threshold_and_reports_anomalies() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, _) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/configs",
            &token,
            json!({
                "name": "登录失败告警阈值",
                "key": "sys.login.alertThreshold",
                "value": "3",
                "type": "N",
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 每次失败登录都要带一个刚签发的验证码，否则会提前因验证码错误返回。
    for _ in 0..5 {
        let (_, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
        let (status, body) = send(
            app.clone(),
            post_json(
                "/api/v1/auth/login",
                json!({
                    "username": "admin",
                    "password": "wrong-password",
                    "captcha": captcha["data"]["code"],
                    "captchaUuid": captcha["data"]["uuid"],
                }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["message"], "账号或密码错误");
    }

    let (status, heatmap) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/login-logs/heatmap", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        heatmap["data"]["threshold"], 3,
        "阈值取自 sys.login.alertThreshold 而不是代码默认值"
    );

    let anomalies = heatmap["data"]["anomalies"].as_array().expect("告警列表");
    let by_user = anomalies
        .iter()
        .find(|row| row["kind"] == "user")
        .expect("按账号聚合的失败登录告警");
    assert_eq!(by_user["key"], "admin");
    assert_eq!(by_user["failed"], 5);
    assert_eq!(by_user["total"], 6, "5 次失败 + 1 次成功登录");
}

/// 时间线的 date 参数只接受 `yyyy-MM-dd`，其余一律 400。
#[tokio::test]
async fn oper_timeline_rejects_a_malformed_date() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    for value in ["2026-13-40", "yesterday", "20260101"] {
        let (status, body) = send(
            app.clone(),
            get_with_bearer(
                &format!("/api/v1/monitor/oper-logs/timeline?date={value}"),
                &token,
            ),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "{value}");
        assert_eq!(body["message"], "date 格式应为 yyyy-MM-dd", "{value}");
    }
}

/// 留痕里的客户端地址取保护层的可信解析结果：伪造的转发头不被采信，
/// 只有显式配置为可信代理的对端送来的 `X-Forwarded-For` 才会按真实来源记录。
#[tokio::test]
async fn recorded_client_ip_follows_the_trusted_proxy_chain() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let mut forged = post_json_with_bearer(
        "/api/v1/system/posts",
        &token,
        json!({ "postCode": "forged_probe", "postName": "伪造探针" }),
    );
    forged
        .headers_mut()
        .insert("x-forwarded-for", "10.10.10.10".parse().expect("头值"));
    forged
        .headers_mut()
        .insert("x-real-ip", "10.10.10.11".parse().expect("头值"));
    let (status, _) = send(app.clone(), forged).await;
    assert_eq!(status, StatusCode::OK);

    let (status, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/oper-logs?keyword=forged_probe", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let row = &logs["data"]["rows"][0];
    assert_eq!(row["ip"], "127.0.0.1", "不可信对端的转发头不能改写来源");
    assert_eq!(row["location"], "本机");

    // 同一条请求在配置了可信代理后按转发头里的真实来源记录。
    let trusted =
        setup_app_with_trusted_proxies(vec!["127.0.0.1/32".parse().expect("构造网段")]).await;
    let trusted_token = admin_token(&trusted).await;

    let mut proxied = post_json_with_bearer(
        "/api/v1/system/posts",
        &trusted_token,
        json!({ "postCode": "proxied_probe", "postName": "代理探针" }),
    );
    proxied
        .headers_mut()
        .insert("x-forwarded-for", "10.10.10.10".parse().expect("头值"));
    let (status, _) = send(trusted.clone(), proxied).await;
    assert_eq!(status, StatusCode::OK);

    let (_, logs) = send(
        trusted.clone(),
        get_with_bearer(
            "/api/v1/monitor/oper-logs?keyword=proxied_probe",
            &trusted_token,
        ),
    )
    .await;
    let row = &logs["data"]["rows"][0];
    assert_eq!(row["ip"], "10.10.10.10");
    assert_eq!(row["location"], "内网");
}

/// 工作台汇总统计的是已落库的操作量：写一次后当天与近 24 小时都能看到。
#[tokio::test]
async fn dashboard_summary_counts_persisted_operations() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, _) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/posts",
            &token,
            json!({ "postCode": "dash_probe", "postName": "工作台探针" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, summary) = send(
        app.clone(),
        get_with_bearer("/api/v1/dashboard/summary", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let data = &summary["data"];

    let daily = data["dailyOps"].as_array().expect("近 7 天");
    assert_eq!(daily.len(), 7);
    assert_eq!(daily[6]["total"], 1, "今天有 1 次写操作");
    assert_eq!(data["trendTotal"], 1, "近 24 小时操作量");
    assert_eq!(data["successRate"], 100.0);

    let api_card = data["stats"]
        .as_array()
        .expect("统计卡")
        .iter()
        .find(|stat| stat["key"] == "api")
        .expect("24h 接口调用卡片");
    assert_eq!(api_card["value"], 1);
    assert_eq!(api_card["up"], true);
}
