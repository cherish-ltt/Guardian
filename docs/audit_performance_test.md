# 审计日志性能测试指南

## 测试概述

本文档提供了审计日志功能的性能测试指南，用于验证审计日志不影响主流程性能。

## 测试工具

- **Apache Bench (ab)**: HTTP 压力测试工具
- **wrk**: 现代化的 HTTP 压力测试工具
- **curl**: 单次请求测试

## 测试场景

### 1. 登录性能测试

**目标**: 验证登录接口在有审计日志情况下的性能

**使用 Apache Bench**:
```bash
# 创建请求文件
cat > login_request.json << EOF
{
  "username": "admin",
  "password": "password123"
}
EOF

# 运行性能测试
ab -n 1000 -c 10 -p login_request.json -T application/json \
  http://localhost:6123/guardian-auth/v1/auth/login
```

**使用 wrk**:
```bash
wrk -t 10 -c 100 -d 30s \
  -s login_request.lua \
  http://localhost:6123/guardian-auth/v1/auth/login
```

其中 `login_request.lua`:
```lua
request = function()
  local body = '{"username": "admin", "password": "password123"}'
  return wrk.format('POST', '/guardian-auth/v1/auth/login', {
    ['Content-Type'] = 'application/json'
  }, body)
end
```

**预期结果**:
- 平均响应时间 < 200 ms
- P95 响应时间 < 500 ms
- P99 响应时间 < 1000 ms
- 成功率 > 99%

### 2. 批量写入性能测试

**目标**: 验证批量写入机制的性能

**步骤**:
1. 启动 Guardian 服务
2. 连续发送 100 个请求
3. 监控数据库写入情况
4. 计算批量写入的吞吐量

**监控脚本**:
```bash
#!/bin/bash

for i in {1..100}; do
  echo "发送请求 $i"
  time curl -s -X POST http://localhost:6123/guardian-auth/v1/auth/login \
    -H "Content-Type: application/json" \
    -d "{\"username\": \"user${i}\", \"password\": \"password123\"}" \
    > /dev/null
  
  if [ $((i % 10)) -eq 0 ]; then
    echo "等待批量写入..."
    sleep 1
  fi
done

echo "查询审计日志数量"
psql -h localhost -U guardian -d guardian_auth -c \
  "SELECT COUNT(*) FROM guardian_audit_logs WHERE created_at >= NOW() - INTERVAL '5 minutes';"
```

**预期结果**:
- 批量写入间隔约 3 秒
- 每 10 条日志立即批量写入
- 数据库写入吞吐量 > 100 logs/s

### 3. 并发性能测试

**目标**: 验证高并发下审计日志的性能

**测试命令**:
```bash
# 50 个并发，每个并发 20 个请求
wrk -t 50 -c 50 -d 30s \
  -s login_request.lua \
  http://localhost:6123/guardian-auth/v1/auth/login
```

**监控脚本**:
```bash
#!/bin/bash

while true; do
  echo "=== $(date) ==="
  echo "审计日志数量:"
  psql -h localhost -U guardian -d guardian_auth -c \
    "SELECT COUNT(*) FROM guardian_audit_logs WHERE created_at >= NOW() - INTERVAL '1 minute';"
  
  echo "数据库连接数:"
  psql -h localhost -U guardian -d guardian_auth -c \
    "SELECT COUNT(*) FROM pg_stat_activity WHERE datname = 'guardian_auth';"
  
  sleep 5
done
```

**预期结果**:
- 平均响应时间 < 500 ms
- P95 响应时间 < 1000 ms
- 数据库连接数稳定
- 无数据库连接泄漏

### 4. 内存使用测试

**目标**: 验证审计日志不导致内存泄漏

**测试步骤**:
1. 记录初始内存使用
2. 连续发送 1000 个请求
3. 记录结束后内存使用
4. 计算内存增长

**测试脚本**:
```bash
#!/bin/bash

echo "=== 内存使用测试 ==="

# 获取初始内存
if pgrep -x Guardian > /dev/null; then
  INITIAL_MEM=$(ps -o rss= -p $(pgrep -x Guardian) | awk '{sum += $1} END {print sum}')
  echo "初始内存: ${INITIAL_MEM} KB"
fi

# 发送 1000 个请求
for i in {1..1000}; do
  curl -s -X POST http://localhost:6123/guardian-auth/v1/auth/login \
    -H "Content-Type: application/json" \
    -d "{\"username\": \"memuser${i}\", \"password\": \"password123\"}" \
    > /dev/null
  
  if [ $((i % 100)) -eq 0 ]; then
    echo "已发送 $i 个请求"
  fi
done

# 等待所有日志写入
sleep 10

# 获取结束内存
if pgrep -x Guardian > /dev/null; then
  FINAL_MEM=$(ps -o rss= -p $(pgrep -x Guardian) | awk '{sum += $1} END {print sum}')
  echo "结束内存: ${FINAL_MEM} KB"
  
  if [ -n "$INITIAL_MEM" ] && [ -n "$FINAL_MEM" ]; then
    MEM_INCREASE=$((FINAL_MEM - INITIAL_MEM))
    MEM_PER_REQUEST=$((MEM_INCREASE / 1000))
    echo "内存增长: ${MEM_INCREASE} KB"
    echo "每请求内存增长: ${MEM_PER_REQUEST} KB"
  fi
fi

echo "=== 测试完成 ==="
```

**预期结果**:
- 内存增长 < 50 MB（1000 个请求）
- 每请求内存增长 < 50 KB
- 无明显内存泄漏（长时间运行后内存稳定）

### 5. CPU 使用测试

**目标**: 验证审计日志不会导致 CPU 使用过高

**监控脚本**:
```bash
#!/bin/bash

echo "=== CPU 使用测试 ==="

# 记录初始 CPU
echo "测试 10 秒..."
for i in {1..10}; do
  if pgrep -x Guardian > /dev/null; then
    CPU_USAGE=$(ps -o %cpu= -p $(pgrep -x Guardian) | awk '{sum += $1} END {print sum}')
    echo "第 ${i} 秒: CPU 使用率 ${CPU_USAGE}%"
  fi
  sleep 1
done

echo "=== 测试完成 ==="
```

**预期结果**:
- 平均 CPU 使用率 < 50%
- 无 CPU 使用率峰值 > 90%

## 性能基准

### 登录接口性能基准

| 指标 | 基准值 | 可接受值 |
|------|--------|----------|
| 平均响应时间 | < 200 ms | < 500 ms |
| P95 响应时间 | < 500 ms | < 1000 ms |
| P99 响应时间 | < 1000 ms | < 2000 ms |
| 成功率 | > 99% | > 95% |
| 吞吐量 | > 500 req/s | > 100 req/s |

### 批量写入性能基准

| 指标 | 基准值 | 可接受值 |
|------|--------|----------|
| 批量写入间隔 | ~3 秒 | < 5 秒 |
| 批量大小 | 10 条 | 5-20 条 |
| 写入吞吐量 | > 100 logs/s | > 50 logs/s |
| 队列容量 | 1000 条 | 500-2000 条 |

### 资源使用基准

| 指标 | 基准值 | 可接受值 |
|------|--------|----------|
| 内存增长（1000 请求） | < 50 MB | < 100 MB |
| 每请求内存增长 | < 50 KB | < 100 KB |
| 平均 CPU 使用率 | < 50% | < 80% |
| 数据库连接数 | < 50 | < 100 |

## 故障排查

### 问题: 性能不达标

**检查项**:
1. 检查数据库连接池配置
2. 检查索引是否正确创建
3. 检查批量写入大小是否合适
4. 检查是否有慢查询

**优化建议**:
- 增加批量写入大小（从 10 增加到 20）
- 减少批量写入间隔（从 3 秒减少到 2 秒）
- 优化数据库查询

### 问题: 内存持续增长

**检查项**:
1. 检查是否有内存泄漏
2. 检查 mpsc 通道是否正确关闭
3. 检查是否有未释放的资源

**优化建议**:
- 限制队列大小
- 定期清理历史日志
- 使用内存分析工具检测泄漏

## 自动化测试脚本

### performance_test.sh

```bash
#!/bin/bash

BASE_URL="http://localhost:6123"

echo "=== 审计日志性能测试 ==="

echo "测试 1: 登录接口性能"
echo "发送 1000 个请求，10 个并发"

START_TIME=$(date +%s%N)

ab -n 1000 -c 10 -p login_request.json -T application/json \
  ${BASE_URL}/guardian-auth/v1/auth/login > ab_result.txt

END_TIME=$(date +%s%N)

echo ""
echo "=== 结果 ==="
cat ab_result.txt | grep "Requests per second\|Time per request\|Transfer rate"
echo ""
echo "测试完成"
echo "总耗时: $(( (END_TIME - START_TIME) / 1000000 )) ms"
```

## 结论

通过以上性能测试，可以验证审计日志功能的性能表现。如果所有测试都达标，说明审计日志功能对性能影响在可接受范围内。

---

**创建时间**: 2026-01-15
**版本**: v1.0
