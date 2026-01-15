# 审计日志数据库迁移文档

## 概述

本文档记录了 Guardian 认证系统中审计日志功能的数据库表结构和索引设计。

## 表结构

### guardian_audit_logs 表

审计日志表记录所有关键操作的信息,包括登录、2FA 操作、管理员 CRUD、角色 CRUD 和权限 CRUD 等。

```sql
CREATE TABLE guardian_audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trace_id UUID,
    admin_id UUID,
    username VARCHAR(255),
    action VARCHAR(50) NOT NULL,
    resource VARCHAR(500) NOT NULL,
    method VARCHAR(10) NOT NULL,
    params JSONB,
    result JSONB,
    status_code INTEGER NOT NULL,
    ip_address INET,
    user_agent TEXT,
    duration_ms BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

### 字段说明

| 字段名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `id` | UUID(V7) | 是 | 主键,UUIDv7 格式确保全局唯一且有序 |
| `trace_id` | UUID | 否 | 请求追踪 ID,用于关联同一请求的多个操作 |
| `admin_id` | UUID(V7) | 否 | 操作者 ID,关联 guardian_admins 表 |
| `username` | VARCHAR(255) | 否 | 操作者用户名 |
| `action` | VARCHAR(50) | 是 | 操作类型,如 login/logout/create/update/delete 等 |
| `resource` | VARCHAR(500) | 是 | 操作的资源路径,如 /guardian-auth/v1/auth/login |
| `method` | VARCHAR(10) | 是 | HTTP 方法,如 GET/POST/PUT/DELETE |
| `params` | JSONB | 否 | 请求参数,JSONB 格式存储,便于查询和分析 |
| `result` | JSONB | 否 | 操作结果,JSONB 格式存储,包含成功/失败信息 |
| `status_code` | INTEGER | 是 | HTTP 状态码,如 200/400/500 等 |
| `ip_address` | INET | 否 | 客户端 IP 地址,支持 IPv4 和 IPv6 |
| `user_agent` | TEXT | 否 | 客户端 User-Agent 字符串 |
| `duration_ms` | BIGINT | 是 | 请求处理时长,单位毫秒 |
| `created_at` | TIMESTAMPTZ | 是 | 日志创建时间,时区感知的时间戳 |

## 索引设计

### 索引列表

为了优化查询性能,为审计日志表创建了以下索引:

```sql
-- 按 admin_id 查询,用于获取特定管理员的操作记录
CREATE INDEX idx_audit_logs_admin_id ON guardian_audit_logs(admin_id);

-- 按 username 查询,用于获取特定用户的操作记录
CREATE INDEX idx_audit_logs_username ON guardian_audit_logs(username);

-- 按 trace_id 查询,用于关联同一请求的多个操作
CREATE INDEX idx_audit_logs_trace_id ON guardian_audit_logs(trace_id);

-- 按 created_at 查询,用于按时间范围查询日志
CREATE INDEX idx_audit_logs_created_at ON guardian_audit_logs(created_at DESC);
```

### 索引使用场景

| 索引 | 查询场景 | 示例 |
|------|----------|------|
| `idx_audit_logs_admin_id` | 查询特定管理员的操作记录 | `SELECT * FROM guardian_audit_logs WHERE admin_id = 'xxx' ORDER BY created_at DESC LIMIT 100` |
| `idx_audit_logs_username` | 查询特定用户的操作记录 | `SELECT * FROM guardian_audit_logs WHERE username = 'admin' ORDER BY created_at DESC LIMIT 100` |
| `idx_audit_logs_trace_id` | 关联同一请求的多个操作 | `SELECT * FROM guardian_audit_logs WHERE trace_id = 'xxx' ORDER BY created_at ASC` |
| `idx_audit_logs_created_at` | 按时间范围查询日志 | `SELECT * FROM guardian_audit_logs WHERE created_at >= NOW() - INTERVAL '1 day' ORDER BY created_at DESC` |

## 初始化脚本

审计日志表的初始化已包含在以下脚本中:

- `scripts/init_db.py` - 数据库初始化脚本
- `scripts/public.sql` - 公开数据 SQL 文件

### 手动创建表

如果需要手动创建审计日志表,可以使用以下 SQL:

```sql
-- 创建审计日志表
CREATE TABLE guardian_audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trace_id UUID,
    admin_id UUID,
    username VARCHAR(255),
    action VARCHAR(50) NOT NULL,
    resource VARCHAR(500) NOT NULL,
    method VARCHAR(10) NOT NULL,
    params JSONB,
    result JSONB,
    status_code INTEGER NOT NULL,
    ip_address INET,
    user_agent TEXT,
    duration_ms BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 创建索引
CREATE INDEX idx_audit_logs_admin_id ON guardian_audit_logs(admin_id);
CREATE INDEX idx_audit_logs_username ON guardian_audit_logs(username);
CREATE INDEX idx_audit_logs_trace_id ON guardian_audit_logs(trace_id);
CREATE INDEX idx_audit_logs_created_at ON guardian_audit_logs(created_at DESC);

-- 添加注释
COMMENT ON TABLE guardian_audit_logs IS '审计日志表,记录所有关键操作';
COMMENT ON COLUMN guardian_audit_logs.id IS '主键,UUIDv7 格式确保全局唯一且有序';
COMMENT ON COLUMN guardian_audit_logs.trace_id IS '请求追踪 ID,用于关联同一请求的多个操作';
COMMENT ON COLUMN guardian_audit_logs.admin_id IS '操作者 ID,关联 guardian_admins 表';
COMMENT ON COLUMN guardian_audit_logs.username IS '操作者用户名';
COMMENT ON COLUMN guardian_audit_logs.action IS '操作类型,如 login/logout/create/update/delete 等';
COMMENT ON COLUMN guardian_audit_logs.resource IS '操作的资源路径';
COMMENT ON COLUMN guardian_audit_logs.method IS 'HTTP 方法,如 GET/POST/PUT/DELETE';
COMMENT ON COLUMN guardian_audit_logs.params IS '请求参数,JSONB 格式存储';
COMMENT ON COLUMN guardian_audit_logs.result IS '操作结果,JSONB 格式存储';
COMMENT ON COLUMN guardian_audit_logs.status_code IS 'HTTP 状态码';
COMMENT ON COLUMN guardian_audit_logs.ip_address IS '客户端 IP 地址';
COMMENT ON COLUMN guardian_audit_logs.user_agent IS '客户端 User-Agent 字符串';
COMMENT ON COLUMN guardian_audit_logs.duration_ms IS '请求处理时长,单位毫秒';
COMMENT ON COLUMN guardian_audit_logs.created_at IS '日志创建时间,时区感知的时间戳';
```

## 数据归档建议

由于审计日志会持续增长,建议定期归档历史日志:

### 归档策略

1. **时间范围**: 归档 6 个月前的日志
2. **归档方式**: 导出到 CSV/JSON 文件并存储到对象存储
3. **删除策略**: 归档后从数据库删除

### 归档 SQL 示例

```sql
-- 导出 6 个月前的日志到 CSV
COPY (
    SELECT * FROM guardian_audit_logs
    WHERE created_at < NOW() - INTERVAL '6 months'
    ORDER BY created_at
) TO '/tmp/audit_logs_archive.csv' WITH (FORMAT CSV, HEADER);

-- 删除 6 个月前的日志
DELETE FROM guardian_audit_logs
WHERE created_at < NOW() - INTERVAL '6 months';

-- 创建归档表(可选)
CREATE TABLE guardian_audit_logs_archive_2025 AS
SELECT * FROM guardian_audit_logs
WHERE created_at >= '2025-01-01'::TIMESTAMPTZ
AND created_at < '2026-01-01'::TIMESTAMPTZ;
```

## 查询示例

### 查询特定管理员的操作记录

```sql
SELECT * FROM guardian_audit_logs
WHERE admin_id = 'xxx-xxx-xxx-xxx-xxx'
ORDER BY created_at DESC
LIMIT 100;
```

### 查询特定用户的操作记录

```sql
SELECT * FROM guardian_audit_logs
WHERE username = 'admin'
ORDER BY created_at DESC
LIMIT 100;
```

### 查询最近 24 小时的登录失败记录

```sql
SELECT * FROM guardian_audit_logs
WHERE action = 'login'
AND result->>'success' = 'false'
AND created_at >= NOW() - INTERVAL '24 hours'
ORDER BY created_at DESC;
```

### 查询特定 IP 的所有操作

```sql
SELECT * FROM guardian_audit_logs
WHERE ip_address = '192.168.1.100'
ORDER BY created_at DESC
LIMIT 100;
```

### 查询同一 trace_id 的所有操作

```sql
SELECT * FROM guardian_audit_logs
WHERE trace_id = 'xxx-xxx-xxx-xxx-xxx'
ORDER BY created_at ASC;
```

### 统计每天的操作数量

```sql
SELECT
    DATE(created_at) as date,
    action,
    COUNT(*) as count
FROM guardian_audit_logs
WHERE created_at >= NOW() - INTERVAL '7 days'
GROUP BY DATE(created_at), action
ORDER BY date DESC, count DESC;
```

## 性能优化建议

1. **定期清理**: 建议定期归档或删除历史日志,保持表大小在合理范围内
2. **分区表**: 如果日志量非常大,可以考虑按时间范围创建分区表
3. **监控慢查询**: 定期检查慢查询日志,优化必要的索引
4. **批量写入**: 应用层使用批量写入策略,减少数据库压力

## 安全建议

1. **权限控制**: 限制对审计日志表的访问权限,只有授权人员可以查询
2. **数据加密**: 敏感字段考虑加密存储
3. **日志保留**: 根据合规要求设置合适的日志保留期限
4. **审计日志保护**: 审计日志本身应该不可篡改,考虑使用只读表或触发器保护

## 版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| v1.0 | 2026-01-15 | 初始版本,定义审计日志表结构和索引 |

---

**创建时间**: 2026-01-15
**版本**: v1.0
