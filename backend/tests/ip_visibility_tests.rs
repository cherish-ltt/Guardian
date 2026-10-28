//! 真实 IP 查看权限与属地现查：打码在 axum 返回前完成，属地不入库。

mod support;

use axum::{Router, http::StatusCode};
use serde_json::json;

use support::{
    DEMO_PASSWORD, admin_token, get_with_bearer, login, post_json_with_bearer,
    put_json_with_bearer, send, setup_app_with_store, setup_app_with_store_with_trusted_proxies,
};

/// 可信代理透传的公网来源：三档打码都在这条地址上看差别。
const FORWARDED_FOR: &str = "203.0.113.7";

/// 系统管理员角色（种子里的 `linjh`）。
const SYS_ADMIN_ROLE_ID: i64 = 2;

fn with_forwarded_for(
    mut request: axum::http::Request<axum::body::Body>,
) -> axum::http::Request<axum::body::Body> {
    request
        .headers_mut()
        .insert("x-forwarded-for", FORWARDED_FOR.parse().expect("头值"));

    request
}

/// 超级管理员写一次岗位留下公网来源的留痕，并把监控菜单授给系统管理员角色。
///
/// 返回应用、超级管理员令牌与系统管理员令牌；`grant` 为空表示不授予二级查看权限。
async fn app_with_proxied_operation(grant: &[i64]) -> (Router, String, String) {
    let (app, store) =
        setup_app_with_store_with_trusted_proxies(vec!["127.0.0.1/32".parse().expect("网段")])
            .await;
    let admin = admin_token(&app, store.as_ref()).await;

    let (status, body) = send(
        app.clone(),
        with_forwarded_for(post_json_with_bearer(
            "/api/v1/system/posts",
            &admin,
            json!({ "postCode": "ip_probe", "postName": "属地探针" }),
        )),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "写操作应成功: {body}");

    let mut menu_ids = vec![200, 201, 202];
    menu_ids.extend_from_slice(grant);
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/roles/{SYS_ADMIN_ROLE_ID}"),
            &admin,
            json!({ "menuIds": menu_ids }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "保存授权失败: {body}");

    let sys_admin = login(&app, store.as_ref(), "linjh", DEMO_PASSWORD).await;

    (app, admin, sys_admin)
}

/// 只授予日志查看：落到无权限档，只留首尾段。
#[tokio::test]
async fn without_a_visibility_grant_the_address_keeps_only_its_ends() {
    let (app, _, sys_admin) = app_with_proxied_operation(&[]).await;

    let (status, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/oper-logs?keyword=ip_probe", &sys_admin),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{logs}");
    let row = &logs["data"]["rows"][0];
    assert_eq!(row["ip"], "203.x.x.7");
    assert_eq!(row.get("location"), None, "属地不再随日志返回");

    // 本地联调的登录记录同样打码：无权限档不因「不是公网」就放行。
    let (_, login_logs) = send(
        app,
        get_with_bearer("/api/v1/monitor/login-logs?keyword=linjh", &sys_admin),
    )
    .await;
    assert_eq!(login_logs["data"]["rows"][0]["ip"], "127.x.x.1");
}

/// 二级查看：保留前两段；本地网段同样打码。
#[tokio::test]
async fn partial_level_keeps_the_first_two_segments() {
    let (app, admin, sys_admin) = app_with_proxied_operation(&[203, 2032]).await;

    let (status, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/oper-logs?keyword=ip_probe", &sys_admin),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{logs}");
    assert_eq!(logs["data"]["rows"][0]["ip"], "203.0.x.x");

    // 超管自己看列表是完整地址，本地回环也原样返回。
    let (_, own) = send(
        app,
        get_with_bearer("/api/v1/monitor/login-logs?keyword=admin", &admin),
    )
    .await;
    assert_eq!(own["data"]["rows"][0]["ip"], "127.0.0.1");
}

/// 超级管理员默认完全查看，可以按需查属地；二级与无权限档一律 403。
#[tokio::test]
async fn only_the_full_level_may_query_the_location() {
    let (app, store) =
        setup_app_with_store_with_trusted_proxies(vec!["127.0.0.1/32".parse().expect("网段")])
            .await;
    let admin = admin_token(&app, store.as_ref()).await;
    let sys_admin = login(&app, store.as_ref(), "linjh", DEMO_PASSWORD).await;

    let (status, single) = send(
        app.clone(),
        get_with_bearer(
            &format!("/api/v1/monitor/ip-locations?ip={FORWARDED_FOR}"),
            &admin,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{single}");
    assert_eq!(single["data"]["ip"], FORWARDED_FOR);
    assert!(
        !single["data"]["location"]
            .as_str()
            .expect("属地")
            .is_empty()
    );

    let (status, batch) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/monitor/ip-locations",
            &admin,
            json!({ "ips": [FORWARDED_FOR, "127.0.0.1", "192.168.1.1"] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{batch}");
    let items = batch["data"].as_array().expect("批量结果");
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["ip"], FORWARDED_FOR);
    assert_eq!(items[1]["ip"], "127.0.0.1");
    // 未配置地名库时本地网段也要落在三档里的前两档，不能算外网。
    assert_eq!(items[1]["location"], "本机");
    assert_eq!(items[2]["location"], "内网");

    for (method, request) in [
        (
            "GET",
            get_with_bearer(
                &format!("/api/v1/monitor/ip-locations?ip={FORWARDED_FOR}"),
                &sys_admin,
            ),
        ),
        (
            "POST",
            post_json_with_bearer(
                "/api/v1/monitor/ip-locations",
                &sys_admin,
                json!({ "ips": [FORWARDED_FOR] }),
            ),
        ),
    ] {
        let (status, body) = send(app.clone(), request).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} 二级查看档应被拒绝");
        assert_eq!(body["code"], 403);
    }
}

/// 统一响应结构里的地址字段在三档之外不再有 `location`：属地必须由按钮触发现查。
#[tokio::test]
async fn the_list_no_longer_carries_a_stored_location() {
    let (app, store) = setup_app_with_store().await;
    let token = admin_token(&app, store.as_ref()).await;

    let (status, _) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/posts",
            &token,
            json!({ "postCode": "full_probe", "postName": "完整地址" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, logs) = send(
        app,
        get_with_bearer("/api/v1/monitor/oper-logs?keyword=full_probe", &token),
    )
    .await;
    let row = &logs["data"]["rows"][0];
    assert_eq!(row["ip"], "127.0.0.1");
    assert_eq!(row.get("location"), None);
}
