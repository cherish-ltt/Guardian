#!/usr/bin/env python3
"""
Guardian 数据库初始化脚本
创建数据库和所有表结构（包含完整的 COMMENT 注释）
注意：所有表不使用外键，只通过逻辑关联
"""

import psycopg2
from psycopg2 import sql
from psycopg2.extensions import ISOLATION_LEVEL_AUTOCOMMIT

# 数据库配置
CONFIG = {
    "host": "127.0.0.1",
    "port": 5432,
    "user": "postgres",
    "password": "123456",
    "database": "guardian_auth",
}


def create_database():
    """创建数据库"""
    print("🔧 正在创建数据库...")

    # 连接到 PostgreSQL（默认数据库）
    conn = psycopg2.connect(
        host=CONFIG["host"],
        port=CONFIG["port"],
        user=CONFIG["user"],
        password=CONFIG["password"],
        database="postgres",
    )
    conn.set_isolation_level(ISOLATION_LEVEL_AUTOCOMMIT)
    cursor = conn.cursor()

    # 检查数据库是否存在
    cursor.execute(
        sql.SQL("SELECT 1 FROM pg_database WHERE datname = {}").format(
            sql.Literal(CONFIG["database"])
        )
    )
    exists = cursor.fetchone()

    if exists:
        print(f"✅ 数据库 '{CONFIG['database']}' 已存在")
    else:
        # 创建数据库
        cursor.execute(
            sql.SQL("CREATE DATABASE {}").format(sql.Identifier(CONFIG["database"]))
        )
        print(f"✅ 数据库 '{CONFIG['database']}' 创建成功")

    cursor.close()
    conn.close()


def get_ddl_statements():
    """返回所有 DDL 语句（包含 COMMENT，无外键）"""
    statements = []

    # ========== 创建辅助函数 ==========
    statements.extend(
        [
            """-- 创建 UUIDv7 生成函数
CREATE OR REPLACE FUNCTION uuidv7() RETURNS UUID AS $$
DECLARE
    v_timestamp BIGINT;
    v_rand_a BIGINT;
    v_rand_b BIGINT;
    v_uuid UUID;
BEGIN
    v_timestamp := (EXTRACT(EPOCH FROM CLOCK_TIMESTAMP()) * 1000)::BIGINT;
    v_rand_a := (RANDOM() * 65535)::BIGINT;
    v_rand_b := (RANDOM() * 4294967295)::BIGINT;
    v_uuid := v_timestamp::BIT(48)::BIT(128) << 80
             | v_rand_a::BIT(16)::BIT(128) << 64
             | '0111'::BIT(4)::BIT(128) << 60
             | v_rand_b::BIT(62)::BIT(128);
    RETURN v_uuid;
END;
$$ LANGUAGE plpgsql;""",
            """-- 创建自动更新 updated_at 字段的函数
CREATE OR REPLACE FUNCTION update_updated_at_column() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;""",
        ]
    )

    # ========== guardian_admins 表 ==========
    statements.extend(
        [
            """-- 创建管理员表
CREATE TABLE IF NOT EXISTS guardian_admins (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    username VARCHAR(64) UNIQUE NOT NULL,
    password_hash VARCHAR(60) NOT NULL,
    two_fa_secret TEXT,
    is_super_admin BOOLEAN DEFAULT false,
    status SMALLINT DEFAULT 1,
    last_login_at TIMESTAMPTZ,
    login_attempts INTEGER DEFAULT 0,
    locked_until TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);""",
            """-- 表注释
COMMENT ON TABLE guardian_admins IS '管理员表';""",
            """-- 字段注释
COMMENT ON COLUMN guardian_admins.id IS '管理员ID（UUIDv7）';
COMMENT ON COLUMN guardian_admins.username IS '用户名';
COMMENT ON COLUMN guardian_admins.password_hash IS 'argon2密码哈希';
COMMENT ON COLUMN guardian_admins.two_fa_secret IS '2FA密钥（ChaCha20加密存储）';
COMMENT ON COLUMN guardian_admins.is_super_admin IS '是否超级管理员';
COMMENT ON COLUMN guardian_admins.status IS '状态：1=正常，0=禁用';
COMMENT ON COLUMN guardian_admins.last_login_at IS '最后登录时间';
COMMENT ON COLUMN guardian_admins.login_attempts IS '登录失败尝试次数';
COMMENT ON COLUMN guardian_admins.locked_until IS '锁定截止时间';
COMMENT ON COLUMN guardian_admins.created_at IS '创建时间';
COMMENT ON COLUMN guardian_admins.updated_at IS '更新时间';""",
            """-- 创建索引
CREATE INDEX IF NOT EXISTS idx_guardian_admins_username ON guardian_admins(username);
CREATE INDEX IF NOT EXISTS idx_guardian_admins_status ON guardian_admins(status);""",
            """-- 创建触发器：自动更新 updated_at
CREATE TRIGGER guardian_admins_updated_at BEFORE UPDATE ON guardian_admins
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();""",
        ]
    )

    # ========== guardian_roles 表 ==========
    statements.extend(
        [
            """-- 创建角色表
CREATE TABLE IF NOT EXISTS guardian_roles (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    code VARCHAR(32) UNIQUE NOT NULL,
    name VARCHAR(64) NOT NULL,
    description TEXT,
    is_system BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);""",
            """-- 表注释
COMMENT ON TABLE guardian_roles IS '角色表';""",
            """-- 字段注释
COMMENT ON COLUMN guardian_roles.id IS '角色ID（UUIDv7）';
COMMENT ON COLUMN guardian_roles.code IS '角色代码（唯一）';
COMMENT ON COLUMN guardian_roles.name IS '角色名称';
COMMENT ON COLUMN guardian_roles.description IS '角色描述';
COMMENT ON COLUMN guardian_roles.is_system IS '是否系统内置角色（不可删除）';
COMMENT ON COLUMN guardian_roles.created_at IS '创建时间';
COMMENT ON COLUMN guardian_roles.updated_at IS '更新时间';""",
            """-- 创建索引
CREATE INDEX IF NOT EXISTS idx_guardian_roles_code ON guardian_roles(code);""",
            """-- 创建触发器：自动更新 updated_at
CREATE TRIGGER guardian_roles_updated_at BEFORE UPDATE ON guardian_roles
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();""",
        ]
    )

    # ========== guardian_permissions 表 ==========
    statements.extend(
        [
            """-- 创建权限表（无外键，parent_id 为逻辑关联）
CREATE TABLE IF NOT EXISTS guardian_permissions (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    code VARCHAR(64) UNIQUE NOT NULL,
    name VARCHAR(128) NOT NULL,
    description TEXT,
    resource_type VARCHAR(32) NOT NULL,
    http_method VARCHAR(10),
    resource_path VARCHAR(255),
    parent_id UUID,
    sort_order INTEGER DEFAULT 0,
    is_system BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);""",
            """-- 表注释
COMMENT ON TABLE guardian_permissions IS '权限表';""",
            """-- 字段注释
COMMENT ON COLUMN guardian_permissions.id IS '权限ID（UUIDv7）';
COMMENT ON COLUMN guardian_permissions.code IS '权限代码（唯一）';
COMMENT ON COLUMN guardian_permissions.name IS '权限名称';
COMMENT ON COLUMN guardian_permissions.description IS '权限描述';
COMMENT ON COLUMN guardian_permissions.resource_type IS '资源类型：api/menu/button';
COMMENT ON COLUMN guardian_permissions.http_method IS 'HTTP方法：GET/POST/PUT/DELETE等';
COMMENT ON COLUMN guardian_permissions.resource_path IS '资源路径';
COMMENT ON COLUMN guardian_permissions.parent_id IS '父权限ID（逻辑关联，不使用外键）';
COMMENT ON COLUMN guardian_permissions.sort_order IS '排序序号';
COMMENT ON COLUMN guardian_permissions.is_system IS '是否系统内置权限（不可删除）';
COMMENT ON COLUMN guardian_permissions.created_at IS '创建时间';
COMMENT ON COLUMN guardian_permissions.updated_at IS '更新时间';""",
            """-- 创建索引
CREATE INDEX IF NOT EXISTS idx_guardian_permissions_code ON guardian_permissions(code);
CREATE INDEX IF NOT EXISTS idx_guardian_permissions_resource ON guardian_permissions(resource_type, resource_path);
CREATE INDEX IF NOT EXISTS idx_guardian_permissions_parent ON guardian_permissions(parent_id);""",
            """-- 创建触发器：自动更新 updated_at
CREATE TRIGGER guardian_permissions_updated_at BEFORE UPDATE ON guardian_permissions
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();""",
        ]
    )

    # ========== guardian_admin_roles 表 ==========
    statements.extend(
        [
            """-- 创建管理员角色关联表（无外键，admin_id 和 role_id 为逻辑关联）
CREATE TABLE IF NOT EXISTS guardian_admin_roles (
    admin_id UUID NOT NULL,
    role_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (admin_id, role_id)
);""",
            """-- 表注释
COMMENT ON TABLE guardian_admin_roles IS '管理员角色关联表';""",
            """-- 字段注释
COMMENT ON COLUMN guardian_admin_roles.admin_id IS '管理员ID（逻辑关联 guardian_admins.id，不使用外键）';
COMMENT ON COLUMN guardian_admin_roles.role_id IS '角色ID（逻辑关联 guardian_roles.id，不使用外键）';
COMMENT ON COLUMN guardian_admin_roles.created_at IS '创建时间';""",
            """-- 创建索引
CREATE INDEX IF NOT EXISTS idx_guardian_admin_roles_admin_id ON guardian_admin_roles(admin_id);
CREATE INDEX IF NOT EXISTS idx_guardian_admin_roles_role_id ON guardian_admin_roles(role_id);""",
        ]
    )

    # ========== guardian_role_permissions 表 ==========
    statements.extend(
        [
            """-- 创建角色权限关联表（无外键，role_id 和 permission_id 为逻辑关联）
CREATE TABLE IF NOT EXISTS guardian_role_permissions (
    role_id UUID NOT NULL,
    permission_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_id, permission_id)
);""",
            """-- 表注释
COMMENT ON TABLE guardian_role_permissions IS '角色权限关联表';""",
            """-- 字段注释
COMMENT ON COLUMN guardian_role_permissions.role_id IS '角色ID（逻辑关联 guardian_roles.id，不使用外键）';
COMMENT ON COLUMN guardian_role_permissions.permission_id IS '权限ID（逻辑关联 guardian_permissions.id，不使用外键）';
COMMENT ON COLUMN guardian_role_permissions.created_at IS '创建时间';""",
            """-- 创建索引
CREATE INDEX IF NOT EXISTS idx_guardian_role_permissions_role_id ON guardian_role_permissions(role_id);
CREATE INDEX IF NOT EXISTS idx_guardian_role_permissions_permission_id ON guardian_role_permissions(permission_id);""",
        ]
    )

    # ========== guardian_token_blacklist 表 ==========
    statements.extend(
        [
            """-- 创建令牌黑名单表
CREATE TABLE IF NOT EXISTS guardian_token_blacklist (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    token_id VARCHAR(128) UNIQUE NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);""",
            """-- 表注释
COMMENT ON TABLE guardian_token_blacklist IS '令牌黑名单表';""",
            """-- 字段注释
COMMENT ON COLUMN guardian_token_blacklist.id IS '记录ID（UUIDv7）';
COMMENT ON COLUMN guardian_token_blacklist.token_id IS '令牌标识（JWT的jti）';
COMMENT ON COLUMN guardian_token_blacklist.expires_at IS '过期时间';
COMMENT ON COLUMN guardian_token_blacklist.created_at IS '创建时间';""",
            """-- 创建索引
CREATE INDEX IF NOT EXISTS idx_guardian_token_blacklist_token_id ON guardian_token_blacklist(token_id);
CREATE INDEX IF NOT EXISTS idx_guardian_token_blacklist_expires_at ON guardian_token_blacklist(expires_at);""",
        ]
    )

    # ========== guardian_audit_logs 表 ==========
    statements.extend(
        [
            """-- 创建审计日志表
CREATE TABLE IF NOT EXISTS guardian_audit_logs (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    trace_id VARCHAR(64),
    admin_id UUID,
    username VARCHAR(64),
    action VARCHAR(32) NOT NULL,
    resource VARCHAR(255) NOT NULL,
    method VARCHAR(10) NOT NULL,
    params JSONB,
    result JSONB,
    status_code INTEGER NOT NULL,
    ip_address INET,
    user_agent TEXT,
    duration_ms INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);""",
            """-- 表注释
COMMENT ON TABLE guardian_audit_logs IS '审计日志表';""",
            """-- 字段注释
COMMENT ON COLUMN guardian_audit_logs.id IS '日志ID（UUIDv7）';
COMMENT ON COLUMN guardian_audit_logs.trace_id IS '请求追踪ID（用于关联同一请求的多个操作）';
COMMENT ON COLUMN guardian_audit_logs.admin_id IS '操作管理员ID（逻辑关联，不使用外键）';
COMMENT ON COLUMN guardian_audit_logs.username IS '操作用户名';
COMMENT ON COLUMN guardian_audit_logs.action IS '操作类型：login/logout/create/update/delete等';
COMMENT ON COLUMN guardian_audit_logs.resource IS '操作资源：如/admins/123';
COMMENT ON COLUMN guardian_audit_logs.method IS 'HTTP方法：GET/POST/PUT/DELETE';
COMMENT ON COLUMN guardian_audit_logs.params IS '请求参数（JSONB格式）';
COMMENT ON COLUMN guardian_audit_logs.result IS '操作结果（JSONB格式）';
COMMENT ON COLUMN guardian_audit_logs.status_code IS 'HTTP状态码';
COMMENT ON COLUMN guardian_audit_logs.ip_address IS '客户端IP地址';
COMMENT ON COLUMN guardian_audit_logs.user_agent IS '客户端User-Agent';
COMMENT ON COLUMN guardian_audit_logs.duration_ms IS '请求耗时（毫秒）';
COMMENT ON COLUMN guardian_audit_logs.created_at IS '创建时间';""",
            """-- 创建索引
CREATE INDEX IF NOT EXISTS idx_guardian_audit_logs_trace_id ON guardian_audit_logs(trace_id);
CREATE INDEX IF NOT EXISTS idx_guardian_audit_logs_admin_id ON guardian_audit_logs(admin_id);
CREATE INDEX IF NOT EXISTS idx_guardian_audit_logs_username ON guardian_audit_logs(username);
CREATE INDEX IF NOT EXISTS idx_guardian_audit_logs_created_at ON guardian_audit_logs(created_at);""",
        ]
    )

    return statements


def create_tables():
    """创建所有表和注释"""
    print("🔧 正在创建表结构...")

    # 连接到目标数据库
    conn = psycopg2.connect(
        host=CONFIG["host"],
        port=CONFIG["port"],
        user=CONFIG["user"],
        password=CONFIG["password"],
        database=CONFIG["database"],
    )
    conn.autocommit = True
    cursor = conn.cursor()

    # 获取所有 DDL 语句
    statements = get_ddl_statements()

    # 执行所有语句
    for i, statement in enumerate(statements, 1):
        try:
            cursor.execute(statement)
            print(f"  [{i}/{len(statements)}] ✅ 执行成功")
        except Exception as e:
            print(f"  [{i}/{len(statements)}] ❌ 执行失败: {e}")
            raise

    cursor.close()
    conn.close()

    print(f"✅ 所有表创建完成（共 {len(statements)} 条语句）")


def verify_tables():
    """验证表是否创建成功"""
    print("\n🔍 验证表结构...")

    conn = psycopg2.connect(
        host=CONFIG["host"],
        port=CONFIG["port"],
        user=CONFIG["user"],
        password=CONFIG["password"],
        database=CONFIG["database"],
    )
    cursor = conn.cursor()

    # 获取所有表
    cursor.execute("""
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = 'public'
        ORDER BY table_name
    """)
    tables = [row[0] for row in cursor.fetchall()]

    expected_tables = [
        "guardian_admins",
        "guardian_roles",
        "guardian_permissions",
        "guardian_admin_roles",
        "guardian_role_permissions",
        "guardian_token_blacklist",
        "guardian_audit_logs",
    ]

    print(f"  期望的表: {', '.join(expected_tables)}")
    print(f"  实际的表: {', '.join(tables)}")

    missing = set(expected_tables) - set(tables)
    if missing:
        print(f"❌ 缺少表: {', '.join(missing)}")
        return False
    else:
        print("✅ 所有表创建成功")

    # 验证表注释
    print("\n📝 验证表注释...")
    cursor.execute("""
        SELECT table_name, obj_description(table_name::regclass, 'pg_class') as comment
        FROM information_schema.tables
        WHERE table_schema = 'public'
        ORDER BY table_name
    """)
    for row in cursor.fetchall():
        table_name, comment = row
        if comment:
            print(f"  ✅ {table_name}: {comment}")
        else:
            print(f"  ⚠️  {table_name}: 无注释")

    # 验证无外键约束
    print("\n🔗 验证外键约束...")
    cursor.execute("""
        SELECT tc.table_name, kcu.column_name,
               ccu.table_name AS foreign_table_name,
               ccu.column_name AS foreign_column_name
        FROM information_schema.table_constraints AS tc
        JOIN information_schema.key_column_usage AS kcu
          ON tc.constraint_name = kcu.constraint_name
        JOIN information_schema.constraint_column_usage AS ccu
          ON ccu.constraint_name = tc.constraint_name
        WHERE tc.constraint_type = 'FOREIGN KEY'
    """)
    foreign_keys = cursor.fetchall()

    if foreign_keys:
        print(f"❌ 发现外键约束（应该没有）:")
        for fk in foreign_keys:
            print(f"   - {fk[0]}.{fk[1]} -> {fk[2]}.{fk[3]}")
        return False
    else:
        print("✅ 无外键约束，只有逻辑关联")

    # 验证触发器
    print("\n⚙️  验证触发器...")
    cursor.execute("""
        SELECT table_name, trigger_name
        FROM information_schema.triggers
        WHERE trigger_schema = 'public'
        ORDER BY table_name, trigger_name
    """)
    triggers = cursor.fetchall()
    if triggers:
        print(f"  触发器列表:")
        for table_name, trigger_name in triggers:
            print(f"    ✅ {table_name}.{trigger_name}")
    else:
        print("  ⚠️  未找到触发器")

    cursor.close()
    conn.close()

    return True


def main():
    """主函数"""
    print("=" * 60)
    print("🚀 Guardian 数据库初始化脚本")
    print("⚠️  注意：所有表不使用外键，只通过逻辑关联")
    print("=" * 60)
    print(f"📍 Host: {CONFIG['host']}:{CONFIG['port']}")
    print(f"👤 User: {CONFIG['user']}")
    print(f"💾 Database: {CONFIG['database']}")
    print("=" * 60)
    print()

    try:
        # 1. 创建数据库
        create_database()
        print()

        # 2. 创建表
        create_tables()
        print()

        # 3. 验证表
        verify_tables()
        print()

        print("=" * 60)
        print("✅ 数据库初始化完成！")
        print("=" * 60)
        print()
        print("📌 下一步:")
        print("  1. 运行 'cargo check' 验证依赖")
        print("  2. 运行 'cargo build' 验证编译")
        print()

    except Exception as e:
        print(f"\n❌ 初始化失败: {e}")
        import traceback

        traceback.print_exc()
        exit(1)


if __name__ == "__main__":
    main()
