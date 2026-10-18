//! 后台管理的种子数据，按 Spring 版 `V2__seed_data.sql` 逐行对齐。
//!
//! 全部语句都用 `INSERT OR IGNORE`，重复启动不会重复插入，也不需要先查后写。
//! 账号与口令来自 `.env`（见 `SeedConfig`）：明文口令不写进迁移文件与种子 SQL，
//! 用户口令用 argon2 现场生成（Spring 版种子里的 BCrypt 字面量无法在本项目复用）。

use anyhow::{Context, Result};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value};

use crate::infrastructure::config::{SeedConfig, SeedUser};
use crate::util::password;

/// 播种演示数据；账号与口令由 `SeedConfig` 提供。
pub async fn seed_admin_data(database: &DatabaseConnection, seed: &SeedConfig) -> Result<()> {
    for (name, statement) in STATEMENTS {
        database
            .execute_unprepared(statement)
            .await
            .with_context(|| format!("failed to seed {name}"))?;
    }

    seed_users(database, &seed.users, &seed.admin_password).await?;
    seed_user_assignments(database, seed.users.len()).await?;

    Ok(())
}

/// 播种账号；已经种过（库里已有 ID `1..=N` 的账号）时不再计算口令哈希。
///
/// 每个账号单独哈希：即使共用同一个口令，也必须各自带随机盐，
/// 不能把一份 PHC 字符串复制给多个账号。
async fn seed_users(
    database: &DatabaseConnection,
    users: &[SeedUser],
    admin_password: &str,
) -> Result<()> {
    if seeded_user_count(database, users.len() as i64).await? >= users.len() as i64 {
        return Ok(());
    }

    let mut sql = String::from(USER_INSERT_PREFIX);
    let mut values = Vec::with_capacity(users.len() * 6);
    for (index, user) in users.iter().enumerate() {
        let hashed = password::hash_password(admin_password)
            .map_err(|error| anyhow::anyhow!("failed to hash seed password: {error}"))?;

        if index > 0 {
            sql.push_str(", ");
        }
        sql.push_str(USER_INSERT_ROW);
        values.extend([
            Value::from(index as i64 + 1),
            Value::from(user.username.as_str()),
            Value::from(hashed),
            Value::from(user.nickname.as_str()),
            Value::from(user.email.as_str()),
            Value::from(user.dept_id),
        ]);
    }

    database
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            sql,
            values,
        ))
        .await
        .context("failed to seed sys_user")?;

    Ok(())
}

/// 按账号顺序分配演示角色与岗位。
///
/// 第一个账号是超级管理员（拥有全部菜单），其余账号是演示角色。
/// 账号数超过内置映射时，多出的账号不分配，登录后没有菜单。
async fn seed_user_assignments(database: &DatabaseConnection, user_count: usize) -> Result<()> {
    let assigned = user_count.min(USER_ROLE_IDS.len());
    if assigned == 0 {
        return Ok(());
    }

    for (table, column, ids) in [
        ("sys_user_role", "role_id", USER_ROLE_IDS),
        ("sys_user_post", "post_id", USER_POST_IDS),
    ] {
        let pairs = (0..assigned)
            .map(|index| format!("({}, {})", index + 1, ids[index]))
            .collect::<Vec<String>>()
            .join(", ");
        let statement = format!("INSERT OR IGNORE INTO {table} (user_id, {column}) VALUES {pairs}");

        database
            .execute_unprepared(&statement)
            .await
            .with_context(|| format!("failed to seed {table}"))?;
    }

    Ok(())
}

async fn seeded_user_count(database: &DatabaseConnection, last_id: i64) -> Result<i64> {
    let statement = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "SELECT COUNT(*) AS value FROM sys_user WHERE id <= ?",
        vec![Value::from(last_id)],
    );
    let row = database
        .query_one_raw(statement)
        .await
        .context("failed to count seeded users")?
        .ok_or_else(|| anyhow::anyhow!("counting seeded users returned no result"))?;

    row.try_get::<i64>("", "value")
        .context("failed to read seeded user count")
}

const USER_INSERT_PREFIX: &str = "INSERT OR IGNORE INTO sys_user \
     (id, username, password, nickname, phone, email, dept_id, status, create_by, update_by) \
     VALUES ";

const USER_INSERT_ROW: &str = "(?, ?, ?, ?, '', ?, ?, 1, 'system', 'system')";

/// 演示角色与岗位按账号顺序分配，对应 `sys_role` 的超级管理员 / 系统管理员 / 业务管理员。
const USER_ROLE_IDS: [i64; 3] = [1, 2, 3];
const USER_POST_IDS: [i64; 3] = [3, 5, 6];

/// 语句顺序与 Spring 版一致：先组织，再菜单授权，最后字典与参数。
const STATEMENTS: [(&str, &str); 8] = [
    (
        "sys_dept",
        "-- 本地开发示例数据；组织与用户仅供演示，电话留空，邮箱使用 example.com。
         INSERT OR IGNORE INTO sys_dept (id, parent_id, ancestors, dept_name, leader, phone, email, sort, status, create_by, update_by)
         VALUES
             (1, 0, '0', 'Guardian 科技集团', '陈立诚', '', 'hq@example.com', 1, 1, 'system', 'system'),
             (2, 1, '0,1', '研发中心', '林嘉豪', '', 'rd@example.com', 1, 1, 'system', 'system'),
             (3, 1, '0,1', '产品中心', '苏婉清', '', 'pm@example.com', 2, 1, 'system', 'system'),
             (4, 1, '0,1', '职能中心', '郑文博', '', 'hr@example.com', 3, 1, 'system', 'system'),
             (5, 2, '0,1,2', '平台架构部', '高远', '', 'arch@example.com', 1, 1, 'system', 'system'),
             (6, 2, '0,1,2', '应用开发部', '何雅静', '', 'app@example.com', 2, 1, 'system', 'system'),
             (7, 2, '0,1,2', '测试质量部', '罗启明', '', 'qa@example.com', 3, 1, 'system', 'system'),
             (8, 3, '0,1,3', '产品设计部', '唐泽宇', '', 'design@example.com', 1, 1, 'system', 'system'),
             (9, 3, '0,1,3', '用户研究部', '曾静宜', '', 'ux@example.com', 2, 0, 'system', 'system'),
             (10, 4, '0,1,4', '人力资源部', '邓伟华', '', 'people@example.com', 1, 1, 'system', 'system'),
             (11, 4, '0,1,4', '财务部', '沈书宁', '', 'finance@example.com', 2, 1, 'system', 'system');",
    ),
    (
        "sys_post",
        "INSERT OR IGNORE INTO sys_post (id, post_code, post_name, sort, status, remark, create_by, update_by)
         VALUES
             (1, 'ceo', '董事长', 1, 1, '集团最高管理岗位', 'system', 'system'),
             (2, 'gm', '总经理', 2, 1, '分管业务线总经理', 'system', 'system'),
             (3, 'tech_dir', '技术总监', 3, 1, '研发中心负责人', 'system', 'system'),
             (4, 'pm', '产品经理', 4, 1, '负责产品规划与设计', 'system', 'system'),
             (5, 'arch', '架构师', 5, 1, '平台架构设计岗位', 'system', 'system'),
             (6, 'dev', '研发工程师', 6, 1, '前后端研发岗位', 'system', 'system'),
             (7, 'qa', '测试工程师', 7, 1, '质量保障岗位', 'system', 'system'),
             (8, 'hr', '人事专员', 8, 1, '人力资源岗位', 'system', 'system'),
             (9, 'finance', '财务专员', 9, 0, '财务核算岗位', 'system', 'system');",
    ),
    (
        "sys_role",
        "INSERT OR IGNORE INTO sys_role (id, role_name, role_code, data_scope, sort, status, builtin, remark, create_by, update_by)
         VALUES
             (1, '超级管理员', 'super_admin', 'ALL', 1, 1, 1, '系统最高权限，不可删除', 'system', 'system'),
             (2, '系统管理员', 'sys_admin', 'DEPT_AND_CHILD', 2, 1, 0, '负责系统日常配置与维护', 'system', 'system'),
             (3, '业务管理员', 'biz_admin', 'DEPT', 3, 1, 0, '维护本部门业务数据', 'system', 'system'),
             (4, '研发工程师', 'developer', 'SELF', 4, 1, 0, '使用代码生成与开发模块', 'system', 'system'),
             (5, '审计专员', 'auditor', 'CUSTOM', 5, 1, 0, '查看操作日志与登录日志', 'system', 'system'),
             (6, '只读访客', 'guest', 'SELF', 6, 0, 0, '仅可查看授权报表', 'system', 'system');",
    ),
    ("sys_menu", MENU_INSERT),
    (
        "sys_role_menu",
        "-- 超级管理员拥有全部菜单；其余角色由管理员在「角色管理」中按需授权
         INSERT OR IGNORE INTO sys_role_menu (role_id, menu_id)
         SELECT 1, id FROM sys_menu;",
    ),
    (
        "sys_dict_type",
        "INSERT OR IGNORE INTO sys_dict_type (id, dict_name, dict_type, status, remark, create_by, update_by)
         VALUES
             (1, '用户性别', 'sys_user_sex', 1, '用户性别列表', 'system', 'system'),
             (2, '系统开关', 'sys_normal_disable', 1, '系统正常 / 停用状态', 'system', 'system'),
             (3, '是否', 'sys_yes_no', 1, '通用是 / 否选项', 'system', 'system');",
    ),
    (
        "sys_dict_data",
        "INSERT OR IGNORE INTO sys_dict_data (id, dict_type, dict_label, dict_value, sort, status, tone, is_default, create_by, update_by)
         VALUES
             (1, 'sys_user_sex', '男', '0', 1, 1, 'info', 1, 'system', 'system'),
             (2, 'sys_user_sex', '女', '1', 2, 1, 'purple', 0, 'system', 'system'),
             (3, 'sys_user_sex', '未知', '2', 3, 1, 'neutral', 0, 'system', 'system'),
             (4, 'sys_normal_disable', '正常', '1', 1, 1, 'ok', 1, 'system', 'system'),
             (5, 'sys_normal_disable', '停用', '0', 2, 1, 'danger', 0, 'system', 'system'),
             (6, 'sys_yes_no', '是', 'Y', 1, 1, 'ok', 1, 'system', 'system'),
             (7, 'sys_yes_no', '否', 'N', 2, 1, 'neutral', 0, 'system', 'system');",
    ),
    (
        "sys_config",
        "INSERT OR IGNORE INTO sys_config (id, config_name, config_key, config_value, config_type, builtin, remark, create_by, update_by)
         VALUES
             (1, '用户管理-账号初始密码', 'sys.user.initPassword', 'Yy@123456', 'Y', 1, '初始化密码', 'system', 'system'),
             (3, '主框架-默认外观', 'sys.index.appearance', 'light', 'N', 0, '可选 light / dark / auto', 'system', 'system');",
    ),
];

/// 菜单树：目录 / 菜单 / 按钮三级，权限码与前端页面一一对应。
const MENU_INSERT: &str = "INSERT OR IGNORE INTO sys_menu (id, parent_id, menu_name, menu_type, icon, path, component, permission, sort, visible, status, create_by, update_by)
     VALUES
         (1, 0, '工作台', 'menu', 'dashboard', '/dashboard', 'dashboard/index', '', 1, 1, 1, 'system', 'system'),
         (100, 0, '系统管理', 'dir', 'gear', '/system', '', '', 2, 1, 1, 'system', 'system'),
         (101, 100, '用户管理', 'menu', 'users', '/system/user', 'system/user/index', 'system:user:list', 1, 1, 1, 'system', 'system'),
         (1011, 101, '新增用户', 'btn', '', '', '', 'system:user:add', 1, 1, 1, 'system', 'system'),
         (1012, 101, '编辑用户', 'btn', '', '', '', 'system:user:edit', 2, 1, 1, 'system', 'system'),
         (1013, 101, '删除用户', 'btn', '', '', '', 'system:user:remove', 3, 1, 1, 'system', 'system'),
         (1014, 101, '重置密码', 'btn', '', '', '', 'system:user:resetPwd', 4, 1, 1, 'system', 'system'),
         (102, 100, '角色管理', 'menu', 'role', '/system/role', 'system/role/index', 'system:role:list', 2, 1, 1, 'system', 'system'),
         (1021, 102, '新增角色', 'btn', '', '', '', 'system:role:add', 1, 1, 1, 'system', 'system'),
         (1022, 102, '编辑角色', 'btn', '', '', '', 'system:role:edit', 2, 1, 1, 'system', 'system'),
         (1023, 102, '删除角色', 'btn', '', '', '', 'system:role:remove', 3, 1, 1, 'system', 'system'),
         (103, 100, '菜单管理', 'menu', 'menu', '/system/menu', 'system/menu/index', 'system:menu:list', 3, 1, 1, 'system', 'system'),
         (1031, 103, '新增菜单', 'btn', '', '', '', 'system:menu:add', 1, 1, 1, 'system', 'system'),
         (1032, 103, '编辑菜单', 'btn', '', '', '', 'system:menu:edit', 2, 1, 1, 'system', 'system'),
         (1033, 103, '删除菜单', 'btn', '', '', '', 'system:menu:remove', 3, 1, 1, 'system', 'system'),
         (104, 100, '部门管理', 'menu', 'dept', '/system/dept', 'system/dept/index', 'system:dept:list', 4, 1, 1, 'system', 'system'),
         (1041, 104, '新增部门', 'btn', '', '', '', 'system:dept:add', 1, 1, 1, 'system', 'system'),
         (1042, 104, '编辑部门', 'btn', '', '', '', 'system:dept:edit', 2, 1, 1, 'system', 'system'),
         (1043, 104, '删除部门', 'btn', '', '', '', 'system:dept:remove', 3, 1, 1, 'system', 'system'),
         (105, 100, '岗位管理', 'menu', 'post', '/system/post', 'system/post/index', 'system:post:list', 5, 1, 1, 'system', 'system'),
         (1051, 105, '新增岗位', 'btn', '', '', '', 'system:post:add', 1, 1, 1, 'system', 'system'),
         (1052, 105, '编辑岗位', 'btn', '', '', '', 'system:post:edit', 2, 1, 1, 'system', 'system'),
         (1053, 105, '删除岗位', 'btn', '', '', '', 'system:post:remove', 3, 1, 1, 'system', 'system'),
         (106, 100, '字典管理', 'menu', 'dict', '/system/dict', 'system/dict/index', 'system:dict:list', 6, 1, 1, 'system', 'system'),
         (1061, 106, '新增字典', 'btn', '', '', '', 'system:dict:add', 1, 1, 1, 'system', 'system'),
         (1062, 106, '编辑字典', 'btn', '', '', '', 'system:dict:edit', 2, 1, 1, 'system', 'system'),
         (1063, 106, '删除字典', 'btn', '', '', '', 'system:dict:remove', 3, 1, 1, 'system', 'system'),
         (107, 100, '参数配置', 'menu', 'gear', '/system/config', 'system/config/index', 'system:config:list', 7, 1, 1, 'system', 'system'),
         (1071, 107, '新增参数', 'btn', '', '', '', 'system:config:add', 1, 1, 1, 'system', 'system'),
         (1072, 107, '编辑参数', 'btn', '', '', '', 'system:config:edit', 2, 1, 1, 'system', 'system'),
         (1073, 107, '删除参数', 'btn', '', '', '', 'system:config:remove', 3, 1, 1, 'system', 'system'),
         (200, 0, '系统监控', 'dir', 'log', '/monitor', '', '', 3, 1, 1, 'system', 'system'),
         (201, 200, '操作日志', 'menu', 'file', '/monitor/oper-log', 'monitor/operlog/index', 'monitor:operlog:list', 1, 1, 1, 'system', 'system'),
         (2011, 201, '清理操作日志', 'btn', '', '', '', 'monitor:operlog:remove', 1, 1, 1, 'system', 'system'),
         (202, 200, '登录日志', 'menu', 'lock', '/monitor/login-log', 'monitor/loginlog/index', 'monitor:loginlog:list', 2, 1, 1, 'system', 'system'),
         (2021, 202, '清理登录日志', 'btn', '', '', '', 'monitor:loginlog:remove', 1, 1, 1, 'system', 'system'),
         (300, 0, '系统工具', 'dir', 'tool', '/tool', '', '', 4, 1, 1, 'system', 'system'),
         (301, 300, '代码生成', 'menu', 'command', '/tool/generator', 'tool/gen/index', 'tool:gen:list', 1, 1, 1, 'system', 'system');
     -- 代码生成配置初始为空，按需导入当前数据库的真实表。";

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

    use super::{MENU_INSERT, STATEMENTS};
    use crate::infrastructure::config::{SeedConfig, SeedUser};

    /// 测试用播种配置：口令固定，账号由用例自己指定。
    fn seed_config(users: Vec<SeedUser>) -> SeedConfig {
        SeedConfig {
            admin_password: "seed-test-password".to_string(),
            users,
        }
    }

    fn demo_user(username: &str, nickname: &str) -> SeedUser {
        SeedUser {
            username: username.to_string(),
            nickname: nickname.to_string(),
            email: format!("{username}@example.com"),
            dept_id: 5,
        }
    }

    async fn setup_database() -> (DatabaseConnection, String) {
        let url = format!(
            "sqlite://{}?mode=rwc",
            std::env::temp_dir()
                .join(format!("guardian-seed-{}.db", uuid::Uuid::now_v7()))
                .display()
        );
        let database = Database::connect(&url).await.expect("连接临时数据库");

        (database, url)
    }

    async fn count(database: &DatabaseConnection, table: &str) -> i64 {
        let row = database
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Sqlite,
                format!("SELECT COUNT(*) AS value FROM {table}"),
            ))
            .await
            .expect("查询行数")
            .expect("查询行数返回结果");

        row.try_get("", "value").expect("读取行数")
    }

    /// 真实启动流程重复执行两遍：账号、菜单授权与字典都不应重复。
    #[tokio::test]
    async fn admin_seed_is_idempotent_and_keeps_demo_accounts() {
        let (database, url) = setup_database().await;
        let seed = SeedConfig {
            admin_password: "seed-test-password".to_string(),
            ..SeedConfig::default()
        };
        crate::infrastructure::databases::run_migrations(&database, &url, &seed)
            .await
            .expect("执行迁移与播种");
        crate::infrastructure::databases::run_migrations(&database, &url, &seed)
            .await
            .expect("重复执行迁移与播种");

        assert_eq!(count(&database, "sys_dept").await, 11);
        assert_eq!(count(&database, "sys_post").await, 9);
        assert_eq!(count(&database, "sys_role").await, 6);
        assert_eq!(count(&database, "sys_user").await, 3);
        assert_eq!(count(&database, "sys_user_role").await, 3);
        assert_eq!(count(&database, "sys_user_post").await, 3);
        assert_eq!(count(&database, "sys_dict_type").await, 3);
        assert_eq!(count(&database, "sys_dict_data").await, 7);
        assert_eq!(count(&database, "sys_config").await, 2);
        assert_eq!(
            count(&database, "sys_role_menu").await,
            count(&database, "sys_menu").await
        );

        let row = database
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Sqlite,
                "SELECT password, nickname FROM sys_user WHERE username = 'admin'".to_string(),
            ))
            .await
            .expect("查询种子管理员")
            .expect("种子管理员存在");
        let hashed: String = row.try_get("", "password").expect("读取口令");
        assert!(
            crate::util::password::verify_password("seed-test-password", &hashed)
                .expect("校验口令")
        );
    }

    /// 同一口令播种多个账号时，每个账号必须各自带随机盐。
    #[tokio::test]
    async fn seeded_accounts_use_distinct_salts() {
        let (database, url) = setup_database().await;
        let seed = SeedConfig {
            admin_password: "seed-test-password".to_string(),
            ..SeedConfig::default()
        };
        crate::infrastructure::databases::run_migrations(&database, &url, &seed)
            .await
            .expect("执行迁移与播种");

        let rows = database
            .query_all_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Sqlite,
                "SELECT password FROM sys_user ORDER BY id".to_string(),
            ))
            .await
            .expect("查询播种账号");
        assert_eq!(rows.len(), 3);

        let mut salts = Vec::new();
        for row in rows {
            let hashed: String = row.try_get("", "password").expect("读取口令");
            assert!(
                crate::util::password::verify_password("seed-test-password", &hashed)
                    .expect("校验口令")
            );
            // PHC 格式 `$argon2id$v=19$m=..,t=..,p=..$<salt>$<hash>`，索引 4 是盐。
            let salt = hashed.split('$').nth(4).expect("PHC 格式含盐").to_string();
            salts.push(salt);
        }
        salts.sort();
        salts.dedup();
        assert_eq!(salts.len(), 3, "每个账号的盐必须独立");
    }

    /// `SEED_USERS` 决定播种哪些账号，角色与岗位按顺序分配。
    #[tokio::test]
    async fn seed_follows_configured_accounts() {
        let (database, url) = setup_database().await;
        let seed = seed_config(vec![
            demo_user("only_admin", "单人管理员"),
            demo_user("helper", "协助账号"),
        ]);
        crate::infrastructure::databases::run_migrations(&database, &url, &seed)
            .await
            .expect("执行迁移与播种");

        let users = database
            .query_all_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Sqlite,
                "SELECT id, username, nickname, email FROM sys_user ORDER BY id".to_string(),
            ))
            .await
            .expect("查询播种账号");
        assert_eq!(users.len(), 2);
        assert_eq!(
            users[0]
                .try_get::<String>("", "username")
                .expect("读取账号"),
            "only_admin"
        );
        assert_eq!(
            users[1]
                .try_get::<String>("", "nickname")
                .expect("读取昵称"),
            "协助账号"
        );
        assert_eq!(
            users[1].try_get::<String>("", "email").expect("读取邮箱"),
            "helper@example.com"
        );
        assert_eq!(count(&database, "sys_user_role").await, 2);
        assert_eq!(count(&database, "sys_user_post").await, 2);

        // 超过内置映射（3 个）的账号不分配角色与岗位。
        let many = seed_config(vec![
            demo_user("u1", "账号一"),
            demo_user("u2", "账号二"),
            demo_user("u3", "账号三"),
            demo_user("u4", "账号四"),
        ]);
        let (database, url) = setup_database().await;
        crate::infrastructure::databases::run_migrations(&database, &url, &many)
            .await
            .expect("执行迁移与播种");

        assert_eq!(count(&database, "sys_user").await, 4);
        assert_eq!(count(&database, "sys_user_role").await, 3);
        assert_eq!(count(&database, "sys_user_post").await, 3);
    }

    #[test]
    fn statements_are_idempotent_inserts() {
        for (name, statement) in STATEMENTS {
            assert!(
                statement.contains("INSERT OR IGNORE"),
                "{name} 必须使用 INSERT OR IGNORE 保持幂等"
            );
        }
        assert!(MENU_INSERT.contains("INSERT OR IGNORE"));
    }

    #[test]
    fn default_seed_config_keeps_demo_accounts() {
        let seed = SeedConfig::default();
        assert_eq!(seed.users.len(), 3);
        assert_eq!(seed.users[0].username, "admin");
        assert_eq!(seed.users[0].dept_id, 5);
    }
}
