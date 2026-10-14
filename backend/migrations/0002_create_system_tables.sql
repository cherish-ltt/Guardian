-- 后台管理底座的 17 张表，按 SQLite 语义重写 MySQL 版 `V1__init_schema.sql`。
--
-- 类型映射：BIGINT/INT -> INTEGER，TINYINT -> INTEGER，VARCHAR/TEXT/CHAR -> TEXT，
-- DATETIME -> TEXT（`YYYY-MM-DD HH:MM:SS`，应用侧显式写入本地时间）。
-- 时间用字符串保存，让 MySQL 版的 `NOW()` / `DATE_FORMAT()` 语义在 SQLite 下仍是
-- 可直接比较、可直接截断的文本，避免不同时区下的偏移。
-- 列名、约束名与 MySQL 版保持一致，便于与 Spring 版逐列对照。

CREATE TABLE IF NOT EXISTS sys_dept (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id INTEGER NOT NULL DEFAULT 0,
    ancestors TEXT NOT NULL DEFAULT '',
    dept_name TEXT NOT NULL,
    leader TEXT,
    phone TEXT,
    email TEXT,
    sort INTEGER NOT NULL DEFAULT 0,
    status INTEGER NOT NULL DEFAULT 1,
    deleted INTEGER NOT NULL DEFAULT 0,
    create_by TEXT,
    update_by TEXT,
    create_time TEXT DEFAULT (datetime('now', 'localtime')),
    update_time TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE INDEX IF NOT EXISTS idx_sys_dept_parent ON sys_dept (parent_id);
CREATE INDEX IF NOT EXISTS idx_sys_dept_status ON sys_dept (status);

CREATE TABLE IF NOT EXISTS sys_post (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    post_code TEXT NOT NULL,
    post_name TEXT NOT NULL,
    sort INTEGER NOT NULL DEFAULT 0,
    status INTEGER NOT NULL DEFAULT 1,
    remark TEXT,
    deleted INTEGER NOT NULL DEFAULT 0,
    create_by TEXT,
    update_by TEXT,
    create_time TEXT DEFAULT (datetime('now', 'localtime')),
    update_time TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_sys_post_code ON sys_post (post_code);

CREATE TABLE IF NOT EXISTS sys_user (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL,
    password TEXT NOT NULL,
    nickname TEXT NOT NULL,
    phone TEXT,
    email TEXT,
    dept_id INTEGER,
    status INTEGER NOT NULL DEFAULT 1,
    last_login_time TEXT,
    deleted INTEGER NOT NULL DEFAULT 0,
    create_by TEXT,
    update_by TEXT,
    create_time TEXT DEFAULT (datetime('now', 'localtime')),
    update_time TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_sys_user_username ON sys_user (username);
CREATE INDEX IF NOT EXISTS idx_sys_user_dept ON sys_user (dept_id);
CREATE INDEX IF NOT EXISTS idx_sys_user_status ON sys_user (status);

CREATE TABLE IF NOT EXISTS sys_role (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    role_name TEXT NOT NULL,
    role_code TEXT NOT NULL,
    data_scope TEXT NOT NULL DEFAULT 'SELF',
    sort INTEGER NOT NULL DEFAULT 0,
    status INTEGER NOT NULL DEFAULT 1,
    builtin INTEGER NOT NULL DEFAULT 0,
    remark TEXT,
    deleted INTEGER NOT NULL DEFAULT 0,
    create_by TEXT,
    update_by TEXT,
    create_time TEXT DEFAULT (datetime('now', 'localtime')),
    update_time TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_sys_role_code ON sys_role (role_code);

CREATE TABLE IF NOT EXISTS sys_menu (
    id INTEGER PRIMARY KEY,
    parent_id INTEGER NOT NULL DEFAULT 0,
    menu_name TEXT NOT NULL,
    menu_type TEXT NOT NULL,
    icon TEXT,
    path TEXT,
    component TEXT,
    permission TEXT,
    sort INTEGER NOT NULL DEFAULT 0,
    visible INTEGER NOT NULL DEFAULT 1,
    status INTEGER NOT NULL DEFAULT 1,
    create_by TEXT,
    update_by TEXT,
    create_time TEXT DEFAULT (datetime('now', 'localtime')),
    update_time TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE INDEX IF NOT EXISTS idx_sys_menu_parent ON sys_menu (parent_id);
CREATE INDEX IF NOT EXISTS idx_sys_menu_permission ON sys_menu (permission);

CREATE TABLE IF NOT EXISTS sys_user_role (
    user_id INTEGER NOT NULL,
    role_id INTEGER NOT NULL,
    PRIMARY KEY (user_id, role_id)
);

CREATE TABLE IF NOT EXISTS sys_user_post (
    user_id INTEGER NOT NULL,
    post_id INTEGER NOT NULL,
    PRIMARY KEY (user_id, post_id)
);

CREATE TABLE IF NOT EXISTS sys_role_menu (
    role_id INTEGER NOT NULL,
    menu_id INTEGER NOT NULL,
    PRIMARY KEY (role_id, menu_id)
);

CREATE TABLE IF NOT EXISTS sys_role_dept (
    role_id INTEGER NOT NULL,
    dept_id INTEGER NOT NULL,
    PRIMARY KEY (role_id, dept_id)
);

CREATE TABLE IF NOT EXISTS sys_dict_type (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dict_name TEXT NOT NULL,
    dict_type TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 1,
    remark TEXT,
    create_by TEXT,
    update_by TEXT,
    create_time TEXT DEFAULT (datetime('now', 'localtime')),
    update_time TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_sys_dict_type ON sys_dict_type (dict_type);

CREATE TABLE IF NOT EXISTS sys_dict_data (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dict_type TEXT NOT NULL,
    dict_label TEXT NOT NULL,
    dict_value TEXT NOT NULL,
    sort INTEGER NOT NULL DEFAULT 0,
    status INTEGER NOT NULL DEFAULT 1,
    tone TEXT,
    is_default INTEGER NOT NULL DEFAULT 0,
    remark TEXT,
    create_by TEXT,
    update_by TEXT,
    create_time TEXT DEFAULT (datetime('now', 'localtime')),
    update_time TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE INDEX IF NOT EXISTS idx_sys_dict_data_type ON sys_dict_data (dict_type);

CREATE TABLE IF NOT EXISTS sys_config (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    config_name TEXT NOT NULL,
    config_key TEXT NOT NULL,
    config_value TEXT NOT NULL,
    config_type TEXT NOT NULL DEFAULT 'N',
    builtin INTEGER NOT NULL DEFAULT 0,
    remark TEXT,
    create_by TEXT,
    update_by TEXT,
    create_time TEXT DEFAULT (datetime('now', 'localtime')),
    update_time TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_sys_config_key ON sys_config (config_key);

CREATE TABLE IF NOT EXISTS sys_oper_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    module TEXT NOT NULL,
    action TEXT NOT NULL,
    request_method TEXT,
    request_url TEXT,
    request_params TEXT,
    oper_name TEXT,
    dept_name TEXT,
    ip TEXT,
    location TEXT,
    cost INTEGER,
    result INTEGER NOT NULL DEFAULT 1,
    error_msg TEXT,
    oper_time TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE INDEX IF NOT EXISTS idx_sys_oper_log_time ON sys_oper_log (oper_time);
CREATE INDEX IF NOT EXISTS idx_sys_oper_log_oper_name ON sys_oper_log (oper_name);

CREATE TABLE IF NOT EXISTS sys_login_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT,
    ip TEXT,
    location TEXT,
    browser TEXT,
    os TEXT,
    status INTEGER NOT NULL DEFAULT 1,
    msg TEXT,
    login_time TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE INDEX IF NOT EXISTS idx_sys_login_log_time ON sys_login_log (login_time);
CREATE INDEX IF NOT EXISTS idx_sys_login_log_username ON sys_login_log (username);

CREATE TABLE IF NOT EXISTS sys_file (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    original_name TEXT NOT NULL,
    storage_name TEXT NOT NULL,
    url TEXT NOT NULL,
    content_type TEXT,
    size INTEGER NOT NULL DEFAULT 0,
    create_by TEXT,
    update_by TEXT,
    create_time TEXT DEFAULT (datetime('now', 'localtime')),
    update_time TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE TABLE IF NOT EXISTS gen_table (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,
    table_comment TEXT,
    class_name TEXT NOT NULL,
    module_name TEXT NOT NULL,
    business_name TEXT NOT NULL,
    synced INTEGER NOT NULL DEFAULT 0,
    create_by TEXT,
    update_by TEXT,
    create_time TEXT DEFAULT (datetime('now', 'localtime')),
    update_time TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_gen_table_name ON gen_table (table_name);

CREATE TABLE IF NOT EXISTS gen_table_column (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_id INTEGER NOT NULL,
    column_name TEXT NOT NULL,
    java_field TEXT NOT NULL,
    java_type TEXT NOT NULL,
    jdbc_type TEXT NOT NULL,
    column_comment TEXT,
    is_insert INTEGER NOT NULL DEFAULT 1,
    is_edit INTEGER NOT NULL DEFAULT 1,
    is_list INTEGER NOT NULL DEFAULT 1,
    is_query INTEGER NOT NULL DEFAULT 0,
    query_type TEXT NOT NULL DEFAULT '=',
    form_type TEXT NOT NULL DEFAULT '文本框',
    required INTEGER NOT NULL DEFAULT 0,
    sort INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_gen_table_column_table ON gen_table_column (table_id);
