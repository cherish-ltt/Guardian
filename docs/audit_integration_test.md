# 审计日志集成测试指南

## 测试概述

本文档提供了审计日志功能的集成测试指南。由于集成测试需要运行中的 Guardian 服务，以下测试需要在服务启动后手动执行。

## 前置条件

1. 启动 Guardian 服务
2. 确保 PostgreSQL 数据库已初始化
3. 确保有测试用的管理员账户

## 测试场景

### 1. 登录审计日志测试

**目标**: 验证登录操作正确记录到审计日志

**步骤**:
1. 发送登录请求
```bash
curl -X POST http://localhost:6123/guardian-auth/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "password123"
  }'
```

2. 等待 4 秒（等待批量写入）

3. 查询审计日志
```sql
SELECT * FROM guardian_audit_logs
WHERE action = 'login'
ORDER BY created_at DESC
LIMIT 1;
```

**预期结果**:
- 审计日志中有一条记录
- action = 'login'
- username = 'admin'
- resource = '/guardian-auth/v1/auth/login'
- method = 'POST'
- status_code = 200
- result 包含登录成功信息

### 2. 2FA 设置审计日志测试

**目标**: 验证 2FA 设置操作正确记录到审计日志

**步骤**:
1. 登录获取 access token
2. 发送 2FA 设置请求
```bash
curl -X POST http://localhost:6123/guardian-auth/v1/auth/2fa/setup \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>"
```

3. 等待 4 秒

4. 查询审计日志
```sql
SELECT * FROM guardian_audit_logs
WHERE action = '2fa_setup'
ORDER BY created_at DESC
LIMIT 1;
```

**预期结果**:
- 审计日志中有一条记录
- action = '2fa_setup'
- result = '{"success": true}'
- 不包含 secret 和 backup_codes（敏感信息）

### 3. 管理员创建审计日志测试

**目标**: 验证管理员创建操作正确记录到审计日志

**步骤**:
1. 登录获取 access token
2. 创建管理员
```bash
curl -X POST http://localhost:6123/guardian-auth/v1/admins \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "username": "test-admin",
    "password": "password123",
    "is_super_admin": false,
    "role_ids": []
  }'
```

3. 等待 4 秒

4. 查询审计日志
```sql
SELECT * FROM guardian_audit_logs
WHERE action = 'admin_create'
ORDER BY created_at DESC
LIMIT 1;
```

**预期结果**:
- 审计日志中有一条记录
- action = 'admin_create'
- params 包含 username 和 is_super_admin
- 不包含 password（敏感信息）

### 4. 批量写入时序测试

**目标**: 验证批量写入机制正确工作

**步骤**:
1. 连续发送 9 个登录请求
```bash
for i in {1..9}; do
  curl -X POST http://localhost:6123/guardian-auth/v1/auth/login \
    -H "Content-Type: application/json" \
    -d "{\"username\": \"testuser${i}\", \"password\": \"password123\"}" &
done
wait
```

2. 等待 4 秒

3. 查询审计日志数量
```sql
SELECT COUNT(*) FROM guardian_audit_logs
WHERE action = 'login'
AND created_at >= NOW() - INTERVAL '1 minute';
```

4. 发送第 10 个请求
```bash
curl -X POST http://localhost:6123/guardian-auth/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "testuser10", "password": "password123"}'
```

5. 等待 500 毫秒

6. 再次查询审计日志数量
```sql
SELECT COUNT(*) FROM guardian_audit_logs
WHERE action = 'login'
AND created_at >= NOW() - INTERVAL '1 minute';
```

**预期结果**:
- 第 4 秒后应该有 9 条日志（定时器触发批量写入）
- 第 10 个请求后应该立即有 10 条日志（队列达到 10 条触发批量写入）

### 5. 敏感信息过滤测试

**目标**: 验证敏感信息不被记录到审计日志

**步骤**:
1. 创建管理员（包含密码）
2. 设置 2FA（包含 secret 和 backup_codes）
3. 查询这些操作的审计日志

**预期结果**:
- 创建管理员的审计日志不包含密码
- 2FA 设置的审计日志不包含 secret 和 backup_codes
- result 字段仅包含成功/失败信息

### 6. 性能测试

**目标**: 验证审计日志不影响 API 性能

**步骤**:
1. 使用 Apache Bench 进行压力测试
```bash
ab -n 1000 -c 10 -p login_request.json \
  -T application/json \
  http://localhost:6123/guardian-auth/v1/auth/login
```

2. 记录平均响应时间和请求成功率

**预期结果**:
- 平均响应时间小于 500 ms
- 成功率大于 95%
- 审计日志开销小于 5%

## 自动化测试脚本

### login_audit_test.sh

```bash
#!/bin/bash

BASE_URL="http://localhost:6123"

echo "测试 1: 登录审计日志"
RESPONSE=$(curl -s -X POST "${BASE_URL}/guardian-auth/v1/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "password123"}')

echo "响应: $RESPONSE"

sleep 4

echo "测试 2: 查询审计日志"
# 需要数据库访问权限
# psql -h localhost -U guardian -d guardian_auth -c "SELECT * FROM guardian_audit_logs WHERE action = 'login' ORDER BY created_at DESC LIMIT 1;"

echo "测试完成"
```

### 2fa_audit_test.sh

```bash
#!/bin/bash

BASE_URL="http://localhost:6123"

echo "测试 1: 获取访问令牌"
LOGIN_RESPONSE=$(curl -s -X POST "${BASE_URL}/guardian-auth/v1/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "password123"}')

ACCESS_TOKEN=$(echo $LOGIN_RESPONSE | jq -r '.data.access_token')

echo "访问令牌: $ACCESS_TOKEN"

echo "测试 2: 设置 2FA"
SETUP_RESPONSE=$(curl -s -X POST "${BASE_URL}/guardian-auth/v1/auth/2fa/setup" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${ACCESS_TOKEN}")

echo "2FA 设置响应: $SETUP_RESPONSE"

sleep 4

echo "测试 3: 验证 2FA"
VERIFY_RESPONSE=$(curl -s -X POST "${BASE_URL}/guardian-auth/v1/auth/2fa/verify" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${ACCESS_TOKEN}" \
  -d '{"code": "123456"}')

echo "2FA 验证响应: $VERIFY_RESPONSE"

sleep 4

echo "测试完成"
```

## 故障排查

### 问题: 审计日志未写入

**检查项**:
1. 确认服务正在运行
2. 确认数据库连接正常
3. 检查日志中是否有错误信息
4. 确认等待足够时间（至少 4 秒）

### 问题: 审计日志内容不正确

**检查项**:
1. 检查中间件是否正确提取请求信息
2. 检查 controller 是否正确设置 AuditContext
3. 检查批处理器是否正确发送日志到数据库

### 问题: 批量写入不工作

**检查项**:
1. 检查 mpsc 通道是否正常工作
2. 检查定时器是否正常触发
3. 检查数据库连接是否正常

## 结论

通过以上测试，可以验证审计日志功能的正确性和性能。如果所有测试都通过，说明审计日志功能已正确实现。

---

**创建时间**: 2026-01-15
**版本**: v1.0
