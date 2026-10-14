# 中间件栈与配置

> 本文档详解 HTTP 中间件栈、容量参数与压测调参方法，配置总览见 [README](../README.md)。

## 中间件栈

所有中间件在 `src/middleware/stack.rs` 里用 `tower::ServiceBuilder` 一次装配，并通过
`middleware::apply` 包在整个 `Router` 外层，控制器对它们完全无感。

之所以不用 `Router::layer` 施加这些层：`PathRouter::layer` 会对**每条路由**复制一份 layer
（`endpoint.layer(layer.clone())`），有状态的 `ConcurrencyLimit` / `RateLimit` 会被放大成
「每路由一份」，全局并发/限流语义直接失效。此外，栈在 Router 外层意味着未匹配路由的 404
请求也会占用并发/限流额度，这是有意为之（非法流量同样消耗资源）。

执行顺序（外 → 内）：

| 顺序 | 中间件 | 解决的问题 | 失败响应 |
| --- | --- | --- | --- |
| 1 | `SetRequestIdLayer` | 请求没有 `x-request-id` 时生成 UUID | — |
| 2 | `PropagateRequestIdLayer` | 把 `x-request-id` 写回响应头，便于客户端关联 | — |
| 3 | `TraceLayer` | 记录 method / uri / status / 耗时，`request_id` 进 tracing span | — |
| 4 | `SetMultipleResponseHeadersLayer` | `nosniff`、`X-Frame-Options: DENY`、`Referrer-Policy`；生产额外 `HSTS` | — |
| 5 | `HandleError` | 把背压层的 overload 错误转成响应 | `503` |
| 6 | `LoadShedLayer` | 下游不可及时立即拒绝，不排队等待 | `503` |
| 7 | `BufferLayer` | 固定长度等待队列，杜绝无限堆积 | 队列满 → `503` |
| 8 | `ConcurrencyLimitLayer` | 在途请求数上限（全局） | 超出 → 进入 7 排队 |
| 9 | `HandleError` | 把限流 overload 错误转成响应 | `429` |
| 10 | `BufferLayer` | 让非 `Clone` 的 `RateLimit` 满足 `Clone` 约束，克隆共享同一 worker | — |
| 11 | `LoadShedLayer` | 限流窗口未重置时快速拒绝，而不是让请求挂着等窗口 | `429` |
| 12 | `RateLimitLayer` | 进程级全局限速（每个周期 N 个请求） | 超出 → `429` |
| 13 | `CompressionLayer` | 按 `Accept-Encoding` 压缩响应，>32 字节且非图片/gRPC/SSE 才压缩 | — |
| 14 | `TimeoutLayer` | 单个请求处理超时（进入本层后至响应产生的耗时，外层排队不计入；排队由 `ConcurrencyLimit` + `Buffer` 承担） | `408` |
| 15 | `RequestBodyLimitLayer` | 请求体大小上限，按 `Content-Length` 先行拒绝 | `413` |
| 16 | `CorsLayer` | 按 `MIDDLEWARE_CORS_ALLOWED_ORIGINS` 白名单放行跨域；白名单为空时不发任何 CORS 头 | 无（预检返回 `200` 空响应） |

关键点：

- **并发限制与限速是两个维度**。并发限制约束「同时在途的请求数」，限流约束「单位时间的请求数」，
  两者都要有：前者防打满线程/连接池，后者防突发流量。
- **背压 = `ConcurrencyLimit` + 有界队列 + `LoadShed`**。容量内正常执行；短暂超出时请求在有界队列里
  有限等待；队列也满时立即返回 `503`，不会堆积到拖垮进程。
- **`BufferLayer` 不是额外的并发池**：它只是一个固定长度的 mpsc 队列，克隆共享同一个 worker，
  因此不会把并发度放大，`axum::serve` 要求的 `Clone` 也由此满足。
- **`x-request-id` 贯穿全链路**：客户端已带则透传，未带则生成；`TraceLayer` 的 span 里带有
  `request_id` 字段，所以 handler 里的日志会自动带上同一个 ID，可与响应头对齐排查。
- 中间件产生的错误统一走 `ApiResponse` 结构（`src/middleware/error_response.rs`），
  与业务错误 `AppError` 的响应格式一致。
- **扩展点**：V1 只做进程级全局限速。需要按 IP / 用户 / 路径限流时，在 `api/router()` 里
  对特定路由单独叠一个 `tower::limit::RateLimitLayer` 即可：`Router::layer` 会按路由复制 layer，
  这在这里正是需要的「每路由一份额度」语义；也可以换成读 `ConnectInfo` 或已解析用户信息的自定义
  限流键，但不应把这种逻辑写进 `middleware/stack.rs` 的全局栈。

## 中间件配置

全部集中在 `.env`，读取入口是 `AppConfig::middleware`（`src/infrastructure/config.rs`），
中间件代码里不出现字面量：

| 变量 | 默认值 | 含义 |
| --- | --- | --- |
| `MIDDLEWARE_REQUEST_TIMEOUT_SECS` | `10` | 单请求整体超时（秒），超时返回 `408` |
| `MIDDLEWARE_MAX_BODY_BYTES` | `2097152`（2 MiB） | 请求体上限，超限返回 `413` |
| `MIDDLEWARE_MAX_CONCURRENCY` | `256` | 同时在途请求上限 |
| `MIDDLEWARE_BACKPRESSURE_QUEUE` | `256` | 并发已满时允许排队的请求数，超出立即 `503` |
| `MIDDLEWARE_RATE_LIMIT_REQUESTS` | `32768` | 每个限流周期允许的请求数，超出 `429` |
| `MIDDLEWARE_RATE_LIMIT_PERIOD_SECS` | `1` | 限流周期长度（秒） |
| `MIDDLEWARE_HSTS_ENABLED` | 跟随 `APP_ENV` | 生产环境默认下发 `HSTS`，开发环境默认关闭 |
| `MIDDLEWARE_CORS_ALLOWED_ORIGINS` | 空 | 允许跨域的来源白名单，逗号分隔；留空不下发任何 CORS 头，单值 `*` 放行全部来源 |

任一容量参数为 `0` 都会导致启动失败，避免「以为限制了其实没限制」；
`MIDDLEWARE_CORS_ALLOWED_ORIGINS` 的每个条目必须是合法 header 值，否则启动失败。

上表的默认值与 `.env-example` 示例文件一致，适配「单实例 + SQLite/中小型数据库」的中等配置
机器；上线前必须按压测结果调整：

- `MIDDLEWARE_MAX_CONCURRENCY`：由下游能承受的最大在途请求数决定，通常取压测中找到的
  「p99 开始明显劣化」并发点的 80%。
- `MIDDLEWARE_BACKPRESSURE_QUEUE`：吸收突发用，取 `MAX_CONCURRENCY` 的 1/4 ~ 1 倍即可；
  队列越大，被排队的请求尾延迟越高（队列里的请求最长可能等一个 `TIMEOUT`）。
- `MIDDLEWARE_RATE_LIMIT_REQUESTS` / `PERIOD`：由下游 QPS 容量决定；单实例场景是进程级计数，
  **不是分布式限流**，多实例部署需要换到网关层或基于共享存储的实现。
- `MIDDLEWARE_REQUEST_TIMEOUT_SECS`：取业务正常 p99 的 3~5 倍；过小会误杀慢请求，
  过大则慢请求会长时间占住并发额度。
- `MIDDLEWARE_MAX_BODY_BYTES`：按最大上传接口的实际需求设置，默认 2 MiB 对 JSON API 足够宽裕。
- `MIDDLEWARE_CORS_ALLOWED_ORIGINS`：生产环境应显式列出前端域名，不要用 `*`；
  留空表示服务不供浏览器跨域调用（同源与后端调用不受影响）。

## 最小压测方法

先确认并发上限，再确定限速额度。以 [oha](https://github.com/hatoo/oha) 为例：

```bash
cargo install oha

# 1. 把限流放宽（甚至暂时设为很大），排除限流干扰，逐级提高并发观察 p99 与错误率
oha -c 64  -z 30s --no-tui http://127.0.0.1:8080/api/v1/system/health
oha -c 128 -z 30s --no-tui http://127.0.0.1:8080/api/v1/system/health
oha -c 256 -z 30s --no-tui http://127.0.0.1:8080/api/v1/system/health

# 2. 固定并发在上一步的容量内，逐级提高目标 QPS，观察 429 出现点与下游延迟
oha -c 128 -q 500  -z 30s --no-tui http://127.0.0.1:8080/api/v1/system/health
oha -c 128 -q 2000 -z 30s --no-tui http://127.0.0.1:8080/api/v1/system/health
```

读结果的方式：

- 并发压测出现 `503` 是**预期**的背压行为，关注点是「503 立即返回」且成功请求的 p99 稳定；
  如果成功请求的 p99 随并发持续恶化，说明 `MAX_CONCURRENCY` 设得比下游容量大。
- `503` 的比例长期很高说明 `MAX_CONCURRENCY` 偏小，容量被浪费。
- `429` 只应在压测目标 QPS 超过 `RATE_LIMIT_REQUESTS / PERIOD` 时出现，且延迟应接近零
  （快速拒绝而不是排队等窗口）。
- 超时 `408` 的理想值是 0；出现 `408` 应当先查慢查询/外部依赖，而不是直接调大超时。
