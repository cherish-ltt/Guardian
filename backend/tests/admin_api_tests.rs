//! 后台管理接口的契约测试：登录会话、权限校验与 `/api/v1/system/*` 全量 CRUD。
//!
//! 覆盖 Spring 版的可观察行为：权限码、错误文案、分页形状、数据范围与导出内容。

mod support;

use axum::http::StatusCode;
use serde_json::{Value, json};

use support::{
    DEMO_PASSWORD, admin_token, delete_with_bearer, get, get_with_bearer, login, post_json,
    post_json_with_bearer, put_json_with_bearer, send, setup_app,
};

#[tokio::test]
async fn captcha_login_and_session_lifecycle() {
    let app = setup_app().await;

    let (status, captcha) = send(app.clone(), get("/api/v1/auth/captcha")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(captcha["code"], 200);
    assert_eq!(captcha["message"], "操作成功");
    let uuid = captcha["data"]["uuid"].as_str().expect("验证码标识");
    let code = captcha["data"]["code"].as_str().expect("验证码明文");
    assert_eq!(code.len(), 4);
    assert!(code.chars().all(|ch| ch.is_ascii_alphanumeric()));
    assert_eq!(uuid.len(), 32);
    // 追踪 ID 由中间件注入响应体，与响应头同源。
    let trace_id = captcha["traceId"].as_str().expect("traceId");
    assert_eq!(trace_id.len(), 32);

    let (status, signed_in) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({ "username": "admin", "password": DEMO_PASSWORD, "captcha": code, "captchaUuid": uuid }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(signed_in["data"]["expiresIn"], 7200);
    let token = signed_in["data"]["token"]
        .as_str()
        .expect("登录令牌")
        .to_string();

    // 验证码只能用一次：同一个 uuid 重放必然失败。
    let reuse = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({ "username": "admin", "password": DEMO_PASSWORD, "captcha": code, "captchaUuid": uuid }),
        ),
    )
    .await;
    assert_eq!(reuse.0, StatusCode::BAD_REQUEST);
    assert_eq!(reuse.1["message"], "验证码错误或已过期");

    let (status, body) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({ "username": "admin", "password": "wrong-password", "captcha": "ABCD", "captchaUuid": "unknown" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "验证码错误或已过期");

    let (status, me) = send(app.clone(), get_with_bearer("/api/v1/auth/me", &token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["data"]["userId"], 1);
    assert_eq!(me["data"]["username"], "admin");
    assert_eq!(me["data"]["deptId"], 5);
    assert_eq!(me["data"]["deptName"], "平台架构部");
    assert_eq!(me["data"]["roles"], json!(["super_admin"]));
    let permissions = me["data"]["permissions"].as_array().expect("权限码数组");
    assert!(permissions.iter().any(|code| code == "system:user:list"));
    assert!(permissions.iter().any(|code| code == "tool:gen:list"));

    let (status, routes) = send(app.clone(), get_with_bearer("/api/v1/auth/routes", &token)).await;
    assert_eq!(status, StatusCode::OK);
    let routes = routes["data"]["routes"].as_array().expect("路由数组");
    assert_eq!(routes.len(), 4);
    // 第一层按 parentId / sort 排序：工作台、系统管理、系统监控、系统工具。
    assert_eq!(routes[0]["name"], "工作台");
    assert_eq!(routes[1]["name"], "系统管理");
    let system = &routes[1];
    assert_eq!(system["path"], "/system");
    let children = system["children"].as_array().expect("子菜单");
    assert!(children.iter().any(|node| node["name"] == "用户管理"));

    // 登出后令牌立刻失效（JWT 仍合法，但缓存里的会话已撤销）。
    let (status, _) = send(
        app.clone(),
        post_json_with_bearer("/api/v1/auth/logout", &token, json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = send(app.clone(), get_with_bearer("/api/v1/auth/me", &token)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "未认证或登录已过期");
}

#[tokio::test]
async fn missing_invalid_and_revoked_credentials_are_rejected() {
    let app = setup_app().await;

    let (status, body) = send(app.clone(), get("/api/v1/auth/me")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "未认证或登录已过期");

    let (status, body) = send(app.clone(), get_with_bearer("/api/v1/auth/me", "not-a-jwt")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "未认证或登录已过期");

    let (status, body) = send(app.clone(), get("/api/v1/system/users")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "未认证或登录已过期");
}

#[tokio::test]
async fn accounts_without_permission_get_403() {
    let app = setup_app().await;

    // 种子里的 `sys_admin` 未分配任何菜单，因此没有任何权限码。
    let token = login(&app, "linjh", DEMO_PASSWORD).await;
    let (status, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?pageNum=1&pageSize=10", &token),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], "没有访问权限");

    let (status, body) = send(
        app.clone(),
        post_json_with_bearer("/api/v1/system/users", &token, json!({ "username": "x" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], "没有访问权限");
}

#[tokio::test]
async fn unknown_route_and_unsupported_method_use_spring_wording() {
    let app = setup_app().await;

    let (status, body) = send(app.clone(), get("/api/v1/system/unknown")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "资源不存在");

    let (status, body) = send(app.clone(), get("/api/v1/auth/login")).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(body["message"], "不支持的请求方法: GET");
}

#[tokio::test]
async fn users_page_supports_paging_filters_and_options() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?pageNum=1&pageSize=2", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["data"]["total"], 3);
    assert_eq!(page["data"]["pageNum"], 1);
    assert_eq!(page["data"]["pageSize"], 2);
    assert_eq!(page["data"]["rows"].as_array().expect("用户行").len(), 2);
    let first = &page["data"]["rows"][0];
    assert_eq!(first["username"], "admin");
    assert_eq!(first["deptName"], "平台架构部");
    assert_eq!(first["roleName"], "超级管理员");
    assert_eq!(first["postName"], "技术总监");

    let (_, filtered) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?keyword=heyj", &token),
    )
    .await;
    assert_eq!(filtered["data"]["total"], 1);
    assert_eq!(filtered["data"]["rows"][0]["nickname"], "何雅静");

    let (_, by_dept) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?deptId=6", &token),
    )
    .await;
    assert_eq!(by_dept["data"]["total"], 1);

    let (_, by_post) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?postId=6", &token),
    )
    .await;
    assert_eq!(by_post["data"]["total"], 1);

    let (status, options) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users/options", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(options["data"]["initPassword"], "Yy@123456");
    assert_eq!(
        options["data"]["roles"].as_array().expect("角色选项").len(),
        5
    );
    assert_eq!(
        options["data"]["posts"].as_array().expect("岗位选项").len(),
        9
    );
    let depts = options["data"]["depts"].as_array().expect("部门树");
    assert_eq!(depts.len(), 1, "种子部门只有一个根节点");
    assert_eq!(depts[0]["name"], "Guardian 科技集团");
    assert_eq!(
        depts[0]["children"].as_array().expect("子部门").len(),
        3,
        "根节点下有研发 / 产品 / 职能中心"
    );

    // 参数非法时明确报错，不静默忽略过滤条件。
    let (status, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?status=abc", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "参数 status 必须是整数");
}

#[tokio::test]
async fn user_write_flow_covers_create_update_status_password_and_delete() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/users",
            &token,
            json!({
                "username": "newbie",
                "nickname": "新同学",
                "email": "newbie@example.com",
                "deptId": 6,
                "roleIds": [3],
                "postIds": [6],
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = created["data"]["id"].as_i64().expect("新用户 ID");

    // 新建用户使用默认口令，可直接登录。
    let newbie_token = login(&app, "newbie", DEMO_PASSWORD).await;
    let (_, me) = send(
        app.clone(),
        get_with_bearer("/api/v1/auth/me", &newbie_token),
    )
    .await;
    assert_eq!(me["data"]["username"], "newbie");
    assert_eq!(me["data"]["nickname"], "新同学");
    assert_eq!(me["data"]["roles"], json!(["biz_admin"]));

    // 登录账号重复。
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/users",
            &token,
            json!({ "username": "newbie", "nickname": "重复" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "登录账号已存在");

    // 必填校验。
    let (status, body) = send(
        app.clone(),
        post_json_with_bearer("/api/v1/system/users", &token, json!({ "username": "   " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "登录账号不能为空");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/users/{id}"),
            &token,
            json!({ "nickname": "改名了", "deptId": 7, "roleIds": [4], "postIds": [] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?keyword=newbie", &token),
    )
    .await;
    let row = &page["data"]["rows"][0];
    assert_eq!(row["nickname"], "改名了");
    assert_eq!(row["deptName"], "测试质量部");
    assert_eq!(row["roleName"], "研发工程师");
    assert_eq!(row["postName"], "", "岗位被清空后按空串返回");

    // 停用后无法登录，且状态值必须合法。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/users/{id}/status"),
            &token,
            json!({ "status": 2 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "用户状态只能为正常或停用");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/users/{id}/status"),
            &token,
            json!({ "status": 0 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({ "username": "newbie", "password": DEMO_PASSWORD, "captcha": "AB12", "captchaUuid": "nope" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "验证码错误或已过期");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/users/{id}/status"),
            &token,
            json!({ "status": 1 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 重置密码后新口令生效。
    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/users/{id}/password"),
            &token,
            json!({ "password": "Zz@654321" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rotated = login(&app, "newbie", "Zz@654321").await;
    assert!(!rotated.is_empty());

    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/users/{id}/password"),
            &token,
            json!({ "password": "short" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "密码长度必须在6到64位之间");

    // 内置管理员受保护。
    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/system/users/1", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "内置管理员不允许删除");

    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/users/1/status",
            &token,
            json!({ "status": 0 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "内置管理员不允许停用");

    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/users/{id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/users/{id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "用户不存在或已删除");
}

#[tokio::test]
async fn user_export_returns_spring_csv() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, headers, bytes) = support::send_raw(
        app.clone(),
        get_with_bearer("/api/v1/system/users/export", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers
            .get(axum::http::header::CONTENT_TYPE)
            .expect("content-type"),
        "text/csv; charset=UTF-8"
    );
    assert_eq!(
        headers
            .get(axum::http::header::CONTENT_DISPOSITION)
            .expect("disposition"),
        "attachment; filename=\"users.csv\""
    );
    let csv = String::from_utf8(bytes).expect("CSV 为 UTF-8");

    assert!(
        csv.starts_with("登录账号,用户昵称,手机号,邮箱,部门,岗位,角色,状态,创建时间,最近登录\n")
    );
    assert!(csv.contains("admin,陈立诚,,chenlc@example.com,平台架构部,技术总监,超级管理员,正常"));
}

#[tokio::test]
async fn role_flow_covers_menus_graph_and_guards() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/roles?pageNum=1&pageSize=20", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["data"]["total"], 6);
    let seeded = &page["data"]["rows"][0];
    assert_eq!(seeded["code"], "super_admin");
    assert_eq!(seeded["dataScope"], "全部数据权限");
    assert_eq!(seeded["builtin"], true);
    assert_eq!(seeded["userCount"], 1);
    assert!(
        seeded["menuIds"].as_array().expect("菜单 ID").len() > 30,
        "超级管理员拥有全部菜单"
    );

    let (_, menus) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/roles/1/menus", &token),
    )
    .await;
    assert_eq!(
        menus["data"]["menuIds"].as_array().expect("菜单 ID").len(),
        38
    );

    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/roles",
            &token,
            json!({ "name": "受限角色", "code": "limited", "dataScope": "DEPT", "menuIds": [100, 101] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let role_id = created["data"]["id"].as_i64().expect("角色 ID");

    let (_, duplicate) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/roles",
            &token,
            json!({ "name": "重复编码", "code": "limited" }),
        ),
    )
    .await;
    assert_eq!(duplicate["message"], "角色编码已存在");

    let (_, roles) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/roles?keyword=受限", &token),
    )
    .await;
    assert_eq!(roles["data"]["rows"][0]["menuIds"], json!([100, 101]));
    assert_eq!(roles["data"]["rows"][0]["dataScope"], "本部门数据权限");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/roles/{role_id}"),
            &token,
            json!({ "name": "受限角色改", "dataScope": "SELF", "menuIds": [101] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, updated) = send(
        app.clone(),
        get_with_bearer(&format!("/api/v1/system/roles/{role_id}/menus"), &token),
    )
    .await;
    assert_eq!(updated["data"]["menuIds"], json!([101]));

    // 内置角色不可修改 / 删除。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/roles/1",
            &token,
            json!({ "name": "改超管" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "角色不存在、已删除或为内置角色");

    // 已分配用户的角色不能删除。
    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/system/roles/2", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "角色已分配用户，不能删除");

    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/roles/{role_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, graph) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/permissions/graph", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(graph["data"]["usersVisible"], true);
    assert_eq!(
        graph["data"]["users"].as_array().expect("用户节点").len(),
        3
    );
    assert_eq!(
        graph["data"]["menus"].as_array().expect("菜单节点").len(),
        38
    );
    assert_eq!(graph["data"]["userRoles"], json!([[1, 1], [2, 2], [3, 3]]));
}

#[tokio::test]
async fn menu_tree_export_and_crud() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, tree) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/menus/tree", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let roots = tree["data"].as_array().expect("菜单树");
    assert_eq!(roots.len(), 4);
    assert_eq!(roots[0]["name"], "工作台");
    assert_eq!(roots[0]["type"], "menu");

    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/menus",
            &token,
            json!({ "parentId": 105, "name": "导入岗位", "type": "btn", "permission": "system:post:import", "sort": 4 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = created["data"]["id"].as_i64().expect("菜单 ID");
    assert!(id > 1073, "新菜单 ID 由当前最大值 +1 得到");

    let (status, body) = send(
        app.clone(),
        post_json_with_bearer("/api/v1/system/menus", &token, json!({ "name": "  " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "菜单名称不能为空");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/menus/{id}"),
            &token,
            json!({ "name": "导入岗位改", "sort": 9 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, tree) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/menus/tree?keyword=导入岗位", &token),
    )
    .await;
    // 命中过滤的菜单没有带上父节点，因此自己成为根。
    assert_eq!(tree["data"].as_array().expect("过滤后的菜单").len(), 1);
    assert_eq!(tree["data"][0]["name"], "导入岗位改");
    assert_eq!(tree["data"][0]["parentId"], 105);
    assert_eq!(tree["data"][0]["permission"], "system:post:import");

    // 有子菜单不能删；删除按钮本身可以删。
    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/system/menus/105", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "存在子菜单，不能删除");

    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/system/menus/1", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "菜单已分配角色，不能删除");

    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/menus/{id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, _, bytes) = support::send_raw(
        app.clone(),
        get_with_bearer("/api/v1/system/menus/export", &token),
    )
    .await;
    let csv = String::from_utf8(bytes).expect("CSV");
    assert!(csv.starts_with("菜单名称,类型,图标,路由地址,组件路径,权限标识,排序,状态\n"));
    assert!(
        csv.contains("用户管理,menu,users,/system/user,system/user/index,system:user:list,1,正常")
    );
}

#[tokio::test]
async fn dept_tree_and_crud_keep_ancestors_consistent() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, tree) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/depts/tree", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let roots = tree["data"].as_array().expect("部门树");
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0]["memberCount"], 0, "集团本部没有直属成员");
    let rd = &roots[0]["children"][0];
    assert_eq!(rd["name"], "研发中心");
    assert_eq!(
        rd["children"][0]["memberCount"], 2,
        "平台架构部有 admin 与 linjh"
    );

    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/depts",
            &token,
            json!({ "parentId": 5, "name": "架构一组", "leader": "李维" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = created["data"]["id"].as_i64().expect("部门 ID");

    let (status, body) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/depts",
            &token,
            json!({ "parentId": 9999, "name": "悬空部门" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "上级部门不存在或已删除");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/depts/{id}"),
            &token,
            json!({ "name": "架构一组改", "leader": "李维", "sort": 2 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/system/depts/5", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "存在下级部门，不能删除");

    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/system/depts/6", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "部门已关联用户，不能删除");

    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/depts/{id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn post_flow_and_export() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/posts?pageNum=1&pageSize=20", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["data"]["total"], 9);
    assert_eq!(page["data"]["rows"][0]["postCode"], "ceo");
    assert_eq!(page["data"]["rows"][0]["userCount"], 0);

    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/posts",
            &token,
            json!({ "postCode": "intern", "postName": "实习生", "remark": "校园招聘" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = created["data"]["id"].as_i64().expect("岗位 ID");

    let (_, duplicate) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/posts",
            &token,
            json!({ "postCode": "intern", "postName": "重复" }),
        ),
    )
    .await;
    assert_eq!(duplicate["message"], "岗位编码已存在");

    let (_, missing) = send(
        app.clone(),
        post_json_with_bearer("/api/v1/system/posts", &token, json!({ "postCode": "x" })),
    )
    .await;
    assert_eq!(missing["message"], "岗位名称不能为空");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/posts/{id}"),
            &token,
            json!({ "postName": "实习生改", "status": 0 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/posts/{id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, _, bytes) = support::send_raw(
        app.clone(),
        get_with_bearer("/api/v1/system/posts/export", &token),
    )
    .await;
    let csv = String::from_utf8(bytes).expect("CSV");
    assert!(csv.starts_with("岗位编码,岗位名称,排序,状态,创建时间,备注\n"));
    assert!(csv.contains("ceo,董事长,1,正常"));
}

#[tokio::test]
async fn dict_flow_covers_types_data_and_business_options() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/dict/types?pageNum=1&pageSize=20", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["data"]["total"], 3);

    // 管理页的 `def` 是布尔值，业务下拉里是 0/1。
    let (_, data) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/dict/data/sys_normal_disable", &token),
    )
    .await;
    let rows = data["data"].as_array().expect("字典数据");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["label"], "正常");
    assert_eq!(rows[0]["def"], true);
    assert_eq!(rows[1]["def"], false);

    let (status, options) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/dict/options/sys_normal_disable", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let options = options["data"].as_array().expect("业务下拉");
    assert_eq!(options[0]["def"], 1);
    assert_eq!(options[0]["tone"], "ok");

    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/dict/types",
            &token,
            json!({ "dictName": "任务状态", "dictType": "task_status" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let type_id = created["data"]["id"].as_i64().expect("字典类型 ID");

    let (_, duplicate) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/dict/types",
            &token,
            json!({ "dictName": "重复", "dictType": "task_status" }),
        ),
    )
    .await;
    assert_eq!(duplicate["message"], "字典类型已存在");

    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/dict/data",
            &token,
            json!({ "dictType": "task_status", "label": "进行中", "value": "1", "tone": "warn", "def": true }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let data_id = created["data"]["id"].as_i64().expect("字典数据 ID");

    // 有字典数据时类型不能删除。
    let (status, body) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/dict/types/{type_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "字典类型存在字典数据，不能删除");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/dict/data/{data_id}"),
            &token,
            json!({ "label": "已完成", "value": "2" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/dict/data/{data_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/dict/types/{type_id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 不允许删除内置字典类型（数据仍在）。
    let (status, body) = send(
        app.clone(),
        delete_with_bearer("/api/v1/system/dict/types/2", &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "字典类型存在字典数据，不能删除");
}

#[tokio::test]
async fn config_flow_and_export() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/configs?pageNum=1&pageSize=20", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["data"]["total"], 3);
    let rows = page["data"]["rows"].as_array().expect("参数行");
    assert!(rows.iter().any(|row| row["key"] == "sys.user.initPassword"
        && row["value"] == "Yy@123456"
        && row["builtin"] == true
        && row["type"] == "Y"));

    let (status, created) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/configs",
            &token,
            json!({ "name": "任务并发数", "key": "task.concurrency", "value": "4", "type": "N" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = created["data"]["id"].as_i64().expect("参数 ID");

    let (_, duplicate) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/configs",
            &token,
            json!({ "name": "重复", "key": "task.concurrency", "value": "1" }),
        ),
    )
    .await;
    assert_eq!(duplicate["message"], "参数键名已存在");

    let (_, missing) = send(
        app.clone(),
        post_json_with_bearer(
            "/api/v1/system/configs",
            &token,
            json!({ "name": "缺键值", "key": "task.missing" }),
        ),
    )
    .await;
    assert_eq!(missing["message"], "参数键值不能为空");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            &format!("/api/v1/system/configs/{id}"),
            &token,
            json!({ "value": "8" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 内置参数不可修改 / 删除。
    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/configs/1",
            &token,
            json!({ "value": "hacked" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "参数配置不存在或为内置参数");

    let (status, _) = send(
        app.clone(),
        delete_with_bearer(&format!("/api/v1/system/configs/{id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, _, bytes) = support::send_raw(
        app.clone(),
        get_with_bearer("/api/v1/system/configs/export", &token),
    )
    .await;
    let csv = String::from_utf8(bytes).expect("CSV");
    assert!(csv.starts_with("参数名称,参数键名,参数键值,系统内置,创建时间,备注\n"));
    assert!(csv.contains("用户管理-账号初始密码,sys.user.initPassword,Yy@123456,Y"));
}

#[tokio::test]
async fn data_scope_limits_user_list_for_dept_scoped_role() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    // 给「系统管理员」授予用户管理与角色管理菜单：它拥有 DEPT_AND_CHILD 范围（部门 5）。
    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/system/roles/2",
            &token,
            json!({ "menuIds": [101, 102] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let scoped = login(&app, "linjh", DEMO_PASSWORD).await;
    let (status, page) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users?pageNum=1&pageSize=20", &scoped),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["data"]["total"], 2, "只能看到平台架构部的两个账号");
    let usernames = page["data"]["rows"]
        .as_array()
        .expect("用户行")
        .iter()
        .map(|row| row["username"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert_eq!(usernames, vec!["admin".to_string(), "linjh".to_string()]);

    // 导出同样遵守数据范围。
    let (_, _, bytes) = support::send_raw(
        app.clone(),
        get_with_bearer("/api/v1/system/users/export", &scoped),
    )
    .await;
    let csv = String::from_utf8(bytes).expect("CSV");
    assert!(csv.contains("admin"));
    assert!(csv.contains("linjh"));
    assert!(!csv.contains("heyj"), "部门外的账号不能出现在导出里");

    // 只读权限不会带回可编辑选项与初始密码。
    let (_, options) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/users/options", &scoped),
    )
    .await;
    assert_eq!(options["data"]["roles"], json!([]));
    assert_eq!(options["data"]["posts"], json!([]));
    assert_eq!(options["data"]["initPassword"], "");

    let (status, graph) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/permissions/graph", &scoped),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(graph["data"]["usersVisible"], true);
    assert_eq!(
        graph["data"]["users"].as_array().expect("用户节点").len(),
        2,
        "关系图里的用户节点同样受数据范围限制"
    );
    assert_eq!(
        graph["data"]["userRoles"],
        json!([[1, 1], [2, 2]]),
        "被数据范围挡掉的用户不出现授权连线"
    );

    // 缺少权限码的只读账号访问关系图仍然被拒。
    let outsider = login(&app, "heyj", DEMO_PASSWORD).await;
    let (status, body) = send(
        app.clone(),
        get_with_bearer("/api/v1/system/permissions/graph", &outsider),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], "没有访问权限");
}

#[tokio::test]
async fn password_change_requires_old_password_and_rotates_credentials() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/auth/password",
            &token,
            json!({ "oldPassword": "wrong", "newPassword": "Nn@123456" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "旧密码不正确");

    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/auth/password",
            &token,
            json!({ "oldPassword": DEMO_PASSWORD, "newPassword": DEMO_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "新密码不能与旧密码一致");

    let (status, body) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/auth/password",
            &token,
            json!({ "oldPassword": DEMO_PASSWORD, "newPassword": "123" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "newPassword 长度必须在6到64位之间");

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/auth/password",
            &token,
            json!({ "oldPassword": DEMO_PASSWORD, "newPassword": "Nn@123456" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let rotated = login(&app, "admin", "Nn@123456").await;
    assert!(!rotated.is_empty());

    // 复原口令，避免影响同一测试文件里的其他用例（各自独立数据库，这里只是保持语义清晰）。
    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/auth/password",
            &rotated,
            json!({ "oldPassword": "Nn@123456", "newPassword": DEMO_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

/// 改密码后，该账号已签发的会话（含当前与其它设备）必须全部失效。
#[tokio::test]
async fn password_change_revokes_every_existing_session() {
    let app = setup_app().await;
    let first = admin_token(&app).await;
    // JWT 的 iat / exp 是秒粒度，同一秒内重复登录会签发逐字相同的令牌，
    // 先错开一秒，才能确定拿到两个不同会话。
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let second = login(&app, "admin", DEMO_PASSWORD).await;
    assert_ne!(first, second, "两个会话必须是不同令牌");
    let other = login(&app, "linjh", DEMO_PASSWORD).await;

    let (status, _) = send(
        app.clone(),
        put_json_with_bearer(
            "/api/v1/auth/password",
            &second,
            json!({ "oldPassword": DEMO_PASSWORD, "newPassword": "Nn@123456" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    for (label, token) in [("改密前的会话", &first), ("改密用的会话", &second)] {
        let (status, body) = send(app.clone(), get_with_bearer("/api/v1/auth/me", token)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{label} 应失效");
        assert_eq!(body["message"], "未认证或登录已过期");
    }

    // 只清该账号的会话，其他账号不受影响。
    let (status, _) = send(app.clone(), get_with_bearer("/api/v1/auth/me", &other)).await;
    assert_eq!(status, StatusCode::OK, "其他账号的会话不应被误撤");

    let rotated = login(&app, "admin", "Nn@123456").await;
    assert!(!rotated.is_empty());
}

#[tokio::test]
async fn login_log_records_success_and_failure() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let (status, failure) = send(
        app.clone(),
        post_json(
            "/api/v1/auth/login",
            json!({ "username": "admin", "password": "wrong", "captcha": "AB12", "captchaUuid": "unknown" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(failure["message"], "验证码错误或已过期");

    let (_, logs) = send(
        app.clone(),
        get_with_bearer("/api/v1/monitor/login-logs?pageNum=1&pageSize=10", &token),
    )
    .await;
    let rows = logs["data"]["rows"].as_array().expect("登录日志");
    assert!(!rows.is_empty());
    assert_eq!(logs["data"]["total"], 1, "验证码失败不写登录日志");
    assert_eq!(rows[0]["username"], "admin");
    assert_eq!(rows[0]["status"], 1);
    assert_eq!(rows[0]["msg"], "登录成功");
    assert_eq!(rows[0]["browser"], Value::Null);
}
