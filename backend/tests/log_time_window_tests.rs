mod support;

use axum::http::StatusCode;
use sea_orm::ConnectionTrait;
use serde_json::Value;

use support::{admin_token, get, get_with_bearer, login, send, send_raw, setup_app_with_container};

const START: &str = "2026-01-02 10:00:00";
const END: &str = "2026-01-02 11:00:00";

fn window(start: Option<&str>, end: Option<&str>) -> String {
    let mut query = String::new();
    for (key, value) in [("startTime", start), ("endTime", end)] {
        if let Some(value) = value {
            query.push_str(&format!("&{key}={}", value.replace(' ', "%20")));
        }
    }
    query
}

#[tokio::test]
async fn log_windows_filter_counts_pages_and_csv_with_the_same_boundaries() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, store.as_ref()).await;
    for (resource, table, time_column, keyword_column, status_column, extra) in [
        (
            "oper-logs",
            "sys_oper_log",
            "oper_time",
            "module",
            "result",
            ",action",
        ),
        (
            "login-logs",
            "sys_login_log",
            "login_time",
            "username",
            "status",
            ",msg",
        ),
    ] {
        for (index, time) in [
            "2026-01-02 09:59:59",
            START,
            "2026-01-02 10:30:00",
            END,
            "2026-01-02 11:00:01",
        ]
        .iter()
        .enumerate()
        {
            container.database.execute_unprepared(&format!(
                "INSERT INTO {table} ({keyword_column},{status_column},{time_column}{extra}) VALUES ('=window-{index}',0,'{time}','fixture')"
            )).await.unwrap();
        }
        for (keyword, status) in [("=window-other-status", 1), ("unrelated", 0)] {
            container.database.execute_unprepared(&format!(
                "INSERT INTO {table} ({keyword_column},{status_column},{time_column}{extra}) VALUES ('{keyword}',{status},'{START}','fixture')"
            )).await.unwrap();
        }
        for (start, end, expected) in [
            (Some(START), Some(END), vec![2, 1]),
            (Some(START), None, vec![4, 3, 2, 1]),
            (None, Some(END), vec![2, 1, 0]),
            (None, None, vec![4, 3, 2, 1, 0]),
            (Some("2027-01-01 00:00:00"), None, vec![]),
        ] {
            let query = format!("keyword=window&{status_column}=0{}", window(start, end));
            let mut names = Vec::new();
            for page in 1..=expected.len().max(1) {
                let (status, body) = send(
                    app.clone(),
                    get_with_bearer(
                        &format!("/api/v1/monitor/{resource}?{query}&pageNum={page}&pageSize=1"),
                        &token,
                    ),
                )
                .await;
                assert_eq!(status, StatusCode::OK, "{body}");
                assert_eq!(body["data"]["total"], expected.len(), "{resource}: {query}");
                for row in body["data"]["rows"].as_array().unwrap() {
                    names.push(row[keyword_column].as_str().unwrap().to_owned());
                }
            }
            assert_eq!(
                names,
                expected
                    .iter()
                    .map(|index| format!("=window-{index}"))
                    .collect::<Vec<_>>()
            );
            let (status, _, bytes) = send_raw(
                app.clone(),
                get_with_bearer(
                    &format!("/api/v1/monitor/{resource}/export?{query}"),
                    &token,
                ),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
            let csv = String::from_utf8(bytes).unwrap();
            let exported = csv
                .lines()
                .skip(1)
                .map(|line| line.split(',').next().unwrap().to_owned())
                .collect::<Vec<_>>();
            assert_eq!(
                exported,
                names
                    .iter()
                    .map(|name| format!("'{name}"))
                    .collect::<Vec<_>>(),
                "CSV keeps formula neutralization: {csv}"
            );
        }
    }
}

#[tokio::test]
async fn invalid_log_windows_return_400_and_do_not_bypass_permissions() {
    let (app, store, _) = setup_app_with_container().await;
    let token = admin_token(&app, store.as_ref()).await;
    let restricted = login(&app, store.as_ref(), "heyj", support::DEMO_PASSWORD).await;
    for resource in ["oper-logs", "login-logs"] {
        for suffix in ["", "/export"] {
            let path = format!("/api/v1/monitor/{resource}{suffix}");
            for query in [
                "startTime=",
                "endTime=%20%20",
                "startTime=2026-02-30%2010:00:00",
                "startTime=2026-01-02",
                "endTime=not-a-time",
                "startTime=2026-1-2%2010:00:00",
                "startTime=2026-01-02T10:00:00Z",
                "startTime=2026-01-02%2011:00:00&endTime=2026-01-02%2010:00:00",
                "startTime=2026-01-02%2010:00:00&endTime=2026-01-02%2010:00:00",
            ] {
                let (status, body) = send(
                    app.clone(),
                    get_with_bearer(&format!("{path}?{query}"), &token),
                )
                .await;
                assert_eq!(status, StatusCode::BAD_REQUEST, "{path}?{query}: {body}");
                assert_eq!(body["code"], 400);
                assert!(body["message"].as_str().unwrap().contains("Time"));
                assert_eq!(body["data"], Value::Null);
            }
            let invalid = format!("{path}?startTime=bad");
            assert_eq!(
                send(app.clone(), get(&invalid)).await.0,
                StatusCode::UNAUTHORIZED
            );
            assert_eq!(
                send(app.clone(), get_with_bearer(&invalid, &restricted))
                    .await
                    .0,
                StatusCode::FORBIDDEN
            );
        }
    }
}

#[tokio::test]
async fn filtered_exports_keep_the_existing_5000_row_limit() {
    let (app, store, container) = setup_app_with_container().await;
    let token = admin_token(&app, store.as_ref()).await;
    for (resource, table, time_column, keyword_column, extra) in [
        ("oper-logs", "sys_oper_log", "oper_time", "module", "action"),
        (
            "login-logs",
            "sys_login_log",
            "login_time",
            "username",
            "msg",
        ),
    ] {
        container.database.execute_unprepared(&format!(
            "WITH RECURSIVE numbers(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM numbers WHERE n<5001) INSERT INTO {table} ({keyword_column},{time_column},{extra}) SELECT 'limit-fixture','{START}','fixture' FROM numbers"
        )).await.unwrap();
        let query = format!("keyword=limit-fixture{}", window(Some(START), Some(END)));
        let (_, body) = send(
            app.clone(),
            get_with_bearer(&format!("/api/v1/monitor/{resource}?{query}"), &token),
        )
        .await;
        assert_eq!(body["data"]["total"], 5001);
        let (status, _, bytes) = send_raw(
            app.clone(),
            get_with_bearer(
                &format!("/api/v1/monitor/{resource}/export?{query}"),
                &token,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(String::from_utf8(bytes).unwrap().lines().count(), 5001);
    }
}
