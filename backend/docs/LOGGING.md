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
不再为这两类拒绝打日志；每次请求（含被拒绝的请求）仍由中间件栈里的 `TraceLayer` 以
INFO 级别记录 method / uri / status，需要排查风暴时可查访问日志或用 `LOG_FILTER` 调整级别。

## 启动配置日志

`cargo run` 启动时会把本次**实际生效**的配置以手绘 ASCII 列表框形式打印出来（`src/logging.rs` 的
`log_startup_config`），并且同样写入日志文件，便于事后回溯进程启动那一刻用的参数：

- 框内分为「application / server / database / jwt / logging / middleware」六组（分组名与键均为英文），
  字段直接取自 `AppConfig`，渲染为 `key=value` 形式；
- `DATABASE_URL` 中的口令被替换为 `***`（`user:password@host` 与 `?password=` 两种写法都支持，
  口令里含 `@` 或 `:` 也不会漏出），`JWT_SECRET` 只输出字符数（如 `secret=***(56 chars)`），
  密钥不会被写进日志；
- 打印时机在日志初始化之后、`Container::bootstrap` 之前（`src/main.rs`），
  因此数据库引导失败时也能看到配置快照。

`.env-example` 是入库的示例文件，其中的容量参数与 [中间件配置](MIDDLEWARE.md) 的默认值保持一致
（`MIDDLEWARE_MAX_CONCURRENCY=256`、`MIDDLEWARE_BACKPRESSURE_QUEUE=256`、
`MIDDLEWARE_RATE_LIMIT_REQUESTS=32768`）；两者不一致时以启动日志打印的实际值为准。
