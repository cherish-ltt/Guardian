# 日志

> 本文档详解日志的分批落盘机制与启动配置快照，配置总览见 [README](../README.md)。

## 日志写入：分批落盘

文件日志采用分批写入（`src/logging.rs`）：事件先进入内存缓冲，满 `LOG_BATCH_MAX_EVENTS`
条或每 `LOG_BATCH_FLUSH_INTERVAL_SECS` 秒落盘一次，避免高频场景下逐条写文件浪费 CPU；
控制台层保持实时输出。参数在 `.env` 配置，默认值见下表：

| 变量 | 默认值 | 含义 |
| --- | --- | --- |
| `LOG_BATCH_MAX_EVENTS` | `50` | 缓冲满 N 条触发一次落盘 |
| `LOG_BATCH_FLUSH_INTERVAL_SECS` | `3` | 距上次落盘超过 N 秒强制落盘一次 |
| `LOG_MAX_LOG_FILES` | `60` | 日志最大文件数，超过后按轮转策略清理 |
| `LOG_OUT_DIR` | `./log` | 日志目录；相对路径以进程工作目录为基准 |

- 任一项设为 `0` 都会导致启动失败（参数必须大于 0）；
- `LOG_FILTER` 写错（如使用了不存在的日志级别）同样**拒绝启动**，不会静默回退到默认过滤规则；
- 进程退出时（`BatchGuard` drop）会停止定时刷盘线程并落盘剩余缓冲，不会丢日志；
- 每条日志先格式化为完整一行再写入缓冲，保证「条数」计数与日志行一一对应。

限流（`429`）与背压（`503`）触发时是批量拒绝，逐条告警日志没有信息量，`src/middleware/error_response.rs`
不再为这两类拒绝打日志；每次请求（含被拒绝的请求）仍由中间件栈里的请求观测层以
INFO 级别记录 method / path / client_ip / status，需要排查风暴时可查访问日志或用 `LOG_FILTER` 调整级别。
请求观测层取代了原先 `TraceLayer` 的响应/失败回调，同一次 5xx 不会再出现多条访问日志。

## 低噪声请求诊断

每个请求只有一条完成日志：正常请求 INFO `request completed`；达到 `MIDDLEWARE_SLOW_REQUEST_MS`
（默认 1000 ms）、408 或业务服务端失败时 WARN `request diagnostic`，额外携带 `route`、`queue_ms`
与 `stages`。`429` 与容量拒绝不逐条告警；`AppError` 与 panic 的内部原因仍记 ERROR，用同一个
`request_id` 关联。不记录 query、Authorization、请求体或 SQL 参数——但路径参数仍可能含业务标识，
不要把密钥放进 URL。

排查入口：

```bash
# 取到出问题请求的响应头 x-request-id，再到服务日志目录检索（本地默认 ./log）
rg 'request-id-from-response' log/
# 检索慢请求/超时及特定路由模板
rg 'request diagnostic|route="/api/v1/transactions/\{id\}"' log/
```

- `elapsed_ms`：包含全局排队、处理与响应头生成；不等于客户端完整下载时间。
- `queue_ms`：到进入处理层前诊断上下文的耗时；未进入时显示当前已等待时间，`route` 为 `unmatched`。
- `stages`：事务示例已测量 `db.begin`、`db.find_account`、`db.debit`、`db.credit`、`db.insert_record`、
  `db.insert_audit`、`db.commit`、`db.rollback`，以及读流水/审计/分页；就绪检查使用 `db.ready`。
  `elapsed_ms` 为该阶段累计墙钟耗时，`calls` 为次数，`interrupted` 表示超时/取消/异常中断。
- `queue_ms` 大而数据库阶段不大：先看过载与排队；`db.begin` 大：检查连接池与 SQLite 写锁；
  `db.ready` 或查询阶段大：检查连接池、慢查询与数据库状态。这些时间包含取连接等待，
  不能直接当成纯 SQL 执行时间，也不能精确区分连接池等待与 SQLite 锁等待。
- 新业务在 services 中用 `crate::middleware::measure("固定阶段名", future).await` 标记外部 IO 边界即可，
  不逐阶段打印；阶段名不得拼接用户输入。不在请求上下文里调用时行为不变。
  `tokio::spawn` 不继承 task-local 阶段上下文，独立任务不能据此得到完整阶段数据。
- 这里没有公开的诊断/指标端点、SQL 查询明细或全量分布式追踪；未标记的代码无法自动细分，
  并行阶段的累计时间也不能相加当成总耗时。本功能提供的是线上请求的阶段证据。

使用默认 `LOG_FILTER` 即能看到诊断；若只想保留慢请求与异常，可设
`LOG_FILTER=info,guardian_backend::middleware::diagnostics=warn`，普通访问日志随之减少。
请求 span 使用独立的 `http_request` target，保持其 INFO 可确保错误与诊断仍带请求关联字段。
修改过滤规则须重启进程，目前不提供远程动态开关。

## 启动配置日志

`cargo run` 启动时会把本次**实际生效**的配置以手绘 ASCII 列表框形式打印出来（`src/logging.rs` 的
`log_startup_config`），并且同样写入日志文件，便于事后回溯进程启动那一刻用的参数：

- 框内分为「application / server / database / jwt / redis / logging / middleware」七组（分组名与键均为英文），
  字段直接取自 `AppConfig`，渲染为 `key=value` 形式；`application` 组的 `version` 是例外，
  取自 `Cargo.toml` 的 `version`（`env!("CARGO_PKG_VERSION")`），发版改 `Cargo.toml` 即自动生效；
- `DATABASE_URL` 中的口令被替换为 `***`（`user:password@host` 与 `?password=` 两种写法都支持，
  口令里含 `@` 或 `:` 也不会漏出），`JWT_SECRET` 只输出字符数（如 `secret=***(56 chars)`），
  密钥不会被写进日志；
- 打印时机在日志初始化之后、`Container::bootstrap` 之前（`src/main.rs`），
  因此数据库引导失败时也能看到配置快照。

`.env-example` 是入库的示例文件，其中的容量参数与 [中间件配置](MIDDLEWARE.md) 的默认值保持一致
（`MIDDLEWARE_MAX_CONCURRENCY=256`、`MIDDLEWARE_BACKPRESSURE_QUEUE=256`、
`MIDDLEWARE_RATE_LIMIT_REQUESTS=32768`、`MIDDLEWARE_CLIENT_MAX_CONCURRENCY=32`、
`MIDDLEWARE_CLIENT_RATE_LIMIT_REQUESTS=1024`、`MIDDLEWARE_CLIENT_MAX_ENTRIES=16384`）；
两者不一致时以启动日志打印的实际值为准。
