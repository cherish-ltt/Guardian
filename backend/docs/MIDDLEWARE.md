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
| 1 | 请求 ID 校验 + `SetRequestIdLayer` | 只保留 1–64 字节的字母、数字、`-_.` 请求 ID，其余（含重复头）替换为 UUID | — |
| 2 | `PropagateRequestIdLayer` | 把 `x-request-id` 写回响应头，便于客户端关联 | — |
| 3 | 客户端识别（`client_ip::identify`） | 读取 TCP 对端，按显式可信代理链解析真实来源 IP | 转发头非法 → `400`，缺对端 → `500` |
| 4 | `SetMultipleResponseHeadersLayer` | `nosniff`、`X-Frame-Options: DENY`、`Referrer-Policy`；生产额外 `HSTS`，同样覆盖超时响应 | — |
| 5 | 请求观测与总超时（`diagnostics::observe`） | 建立请求 span（含排队计时）、归一化框架错误、每请求输出一条访问或诊断日志 | `408` 统一 JSON |
| 6 | `CatchPanicLayer` | 把请求处理 panic 隔离成安全响应，保留内部错误日志 | `500` 统一 JSON |
| 7 | 客户端保护（`client_limits::protect`） | 按来源 IP 限并发与固定窗口请求量，跨路由共享额度 | `429` / `503` |
| 8 | `HandleError` + `LoadShed` + `Buffer` | 有界背压，队列满后快速拒绝 | `503` |
| 9 | `ConcurrencyLimitLayer` | 在途请求数上限（全局） | 超出 → 进入 8 排队 |
| 10 | `HandleError` + `Buffer` + `LoadShed` + `RateLimit` | 进程级全局限速，窗口内额度用尽快速拒绝 | `429` |
| 11 | `CompressionLayer` | 按 `Accept-Encoding` 压缩响应，>32 字节且非图片/gRPC/SSE 才压缩 | — |
| 12 | 诊断上下文（`diagnostics::enter`） | 记录全局排队耗时，把阶段测量上下文传进 handler | — |
| 13 | `trace::inject_trace_id` | 补 `x-trace-id` 响应头与 JSON 响应体的 `traceId`（必须在压缩层之内，否则读到的是压缩后的字节） | — |
| 14 | `RequestBodyLimitLayer` | 请求体大小上限，按 `Content-Length` 先行拒绝，分块传输按实际累计字节拒绝 | `413` |
| 15 | `CorsLayer` | 按 `MIDDLEWARE_CORS_ALLOWED_ORIGINS` 白名单放行跨域；白名单为空时不发任何 CORS 头 | 无（预检返回 `200` 空响应） |
| 16 | 路由模板采集（`diagnostics::route`） | 匹配成功后记录 `MatchedPath`，供诊断日志使用；无状态，不分配额度 | — |

关键点：

- **并发限制与限速是两个维度**。并发限制约束「同时在途的请求数」，限流约束「单位时间的请求数」，
  两者都要有：前者防打满线程/连接池，后者防突发流量。
- **背压 = `ConcurrencyLimit` + 有界队列 + `LoadShed`**。容量内正常执行；短暂超出时请求在有界队列里
  有限等待；队列也满时立即返回 `503`，不会堆积到拖垮进程。
- **`BufferLayer` 不是额外的并发池**：它只是一个固定长度的 mpsc 队列，克隆共享同一个 worker，
  因此不会把并发度放大，`axum::serve` 要求的 `Clone` 也由此满足。
- **`x-request-id` 贯穿全链路**：合法客户端 ID 原样保留，非法或缺失时生成 UUID；请求 span 里带有
  请求 ID 与可信来源 IP，handler 日志与响应头可以对齐。转发链与非法原始 ID 都不写入日志。
- **客户端额度**：在全局队列之前按规范化 IP 分配并发与固定窗口速率额度，跨路由共享；
  同一 NAT 出口共用额度。拒绝请求不排队，速率超限带 `Retry-After`（秒），并发超限不猜测重试时间；
  注册表满时新来源返回 `503`。活动项不驱逐，空闲项定期清理，不依赖无限增长的 IP 缓存。
- **可信代理**：默认忽略所有转发头。仅当真实 TCP 对端命中显式 IP/CIDR 时解析
  `X-Forwarded-For`，并从右向左剥离可信代理；重复头或参与解析的地址非法时返回 `400`。
  不使用 `X-Real-IP` / `Forwarded` 兜底，网关必须覆盖或正确追加 `X-Forwarded-For`。
  单值 `*` 表示信任所有直连对端（等价于完全采信客户端自报的转发头），仅用于本地调试：
  生产环境遇到前缀长度为 0 的条目会拒绝启动，且 `*` 不得与具体条目混写。
  自定义启动入口必须注入 `ConnectInfo`；缺少真实对端信息返回 `500`，不合成一个假 IP。
- **留痕与保护用的是同一套客户端身份**：`sys_oper_log.oper_ip` 与 `sys_login_log.ip` 都取
  `client_ip::source_of` 从请求扩展里读出的真实对端 IP（即上一条的可信代理链解析结果），
  不再直接采信 `X-Forwarded-For` / `X-Real-IP`：未经校验的转发头能被调用方伪造，不能当作来源。
  代价是与 Spring 版有差异（见 [后台管理模块](ADMIN.md) 的「与 Spring 版的差异」）。
- **错误文案遵循后台契约**：中间件产生的错误统一走 `ApiResponse` 结构
  （`src/middleware/error_response.rs` 与各保护层），4xx 用中文原样透出，5xx 统一「系统内部错误」，
  不把内部细节透给客户端；真正需要排查的失败仍记 `error!` 日志。
- **框架错误归一化**：axum 生成的纯文本 400 / 404 / 405 / 413 / 415 / 422 由观测层改写成统一 JSON，
  不回显解析细节；已经是 JSON 的业务响应不改写，405 保留 `Allow` 响应头。
- **扩展点**：需要按用户、租户或路径分配额度时，可在 `api/router()` 里对特定路由单独叠一层
  `tower::limit::RateLimitLayer`（`Router::layer` 会按路由复制 layer，这里正是需要的
  「每路由一份额度」语义）；按来源 IP 的全局额度已经由客户端保护层负责，不要再往
  `middleware/stack.rs` 的全局栈里塞业务维度。

## 中间件配置

全部集中在 `.env`，读取入口是 `AppConfig::middleware`（`src/infrastructure/config.rs`），
中间件代码里不出现字面量：

| 变量 | 默认值 | 含义 |
| --- | --- | --- |
| `MIDDLEWARE_REQUEST_TIMEOUT_SECS` | `10` | 从进入请求保护层到响应头生成（含全局排队）的超时，超时返回 `408` 统一 JSON；不限制响应流发送时间 |
| `MIDDLEWARE_MAX_BODY_BYTES` | `2097152`（2 MiB） | 请求体上限，超限返回 `413` |
| `MIDDLEWARE_MAX_CONCURRENCY` | `256` | 同时在途请求上限 |
| `MIDDLEWARE_BACKPRESSURE_QUEUE` | `256` | 并发已满时允许排队的请求数，超出立即 `503` |
| `MIDDLEWARE_RATE_LIMIT_REQUESTS` | `32768` | 每个限流周期允许的请求数，超出 `429` |
| `MIDDLEWARE_RATE_LIMIT_PERIOD_SECS` | `1` | 限流周期长度（秒） |
| `MIDDLEWARE_HSTS_ENABLED` | 跟随 `APP_ENV` | 生产环境默认下发 `HSTS`，开发环境默认关闭 |
| `MIDDLEWARE_CORS_ALLOWED_ORIGINS` | 空 | 允许跨域的来源白名单，逗号分隔；留空不下发任何 CORS 头，单值 `*` 放行全部来源 |
| `MIDDLEWARE_TRUSTED_PROXIES` | 空 | 显式可信代理 IP/CIDR，逗号分隔；留空完全忽略转发头，单值 `*` 信任所有直连对端（生产环境拒绝启动，且不得与具体条目混写） |
| `MIDDLEWARE_CLIENT_MAX_CONCURRENCY` | `32` | 单个来源 IP 在途请求（含全局排队）上限，超出 `429` |
| `MIDDLEWARE_CLIENT_RATE_LIMIT_REQUESTS` | `1024` | 单个来源 IP 每窗口请求数上限，超出 `429` 并带 `Retry-After` |
| `MIDDLEWARE_CLIENT_RATE_LIMIT_PERIOD_SECS` | `1` | 客户端固定窗口长度（秒），不得大于 `MIDDLEWARE_CLIENT_IDLE_SECS` |
| `MIDDLEWARE_CLIENT_MAX_ENTRIES` | `16384` | 客户端注册表容量上限，满后新来源 `503`，活动项不驱逐 |
| `MIDDLEWARE_CLIENT_IDLE_SECS` | `60` | 空闲来源保留期（秒），不得小于客户端窗口 |
| `MIDDLEWARE_SLOW_REQUEST_MS` | `1000` | 慢请求诊断阈值（毫秒），达到后输出阶段汇总 |

任一容量参数为 `0` 都会导致启动失败，避免「以为限制了其实没限制」；
`MIDDLEWARE_CORS_ALLOWED_ORIGINS` 的每个条目必须是合法 header 值，
`MIDDLEWARE_TRUSTED_PROXIES` 的每个条目必须是 IP 或 CIDR，否则启动失败；
`MIDDLEWARE_CLIENT_IDLE_SECS` 小于客户端窗口同样拒绝启动。

上表的默认值与 `.env-example` 示例文件一致，适配「单实例 + SQLite/中小型数据库」的中等配置
机器；上线前必须按压测结果调整：

- `MIDDLEWARE_MAX_CONCURRENCY`：由下游能承受的最大在途请求数决定，通常取压测中找到的
  「p99 开始明显劣化」并发点的 80%。
- `MIDDLEWARE_BACKPRESSURE_QUEUE`：吸收突发用，取 `MAX_CONCURRENCY` 的 1/4 ~ 1 倍即可；
  队列越大被排队的请求尾延迟越高，总超时包含排队，到期会取消请求并释放额度。
- `MIDDLEWARE_RATE_LIMIT_REQUESTS` / `PERIOD`：由下游 QPS 容量决定；单实例场景是进程级计数，
  **不是分布式限流**，多实例部署需要换到网关层或基于共享存储的实现。
- `MIDDLEWARE_REQUEST_TIMEOUT_SECS`：取业务正常 p99 的 3~5 倍；过小会误杀慢请求，
  过大则慢请求会长时间占住并发额度。
- `MIDDLEWARE_MAX_BODY_BYTES`：按最大上传接口的实际需求设置，默认 2 MiB 对 JSON API 足够宽裕。
- `MIDDLEWARE_CLIENT_MAX_CONCURRENCY` / `CLIENT_RATE_LIMIT_REQUESTS`：按「单个来源正常需要多少额度」
  设置，要明显小于全局额度并给突发留余量；单机压测来自同一出口 IP，需同时放宽客户端与全局限额。
- `MIDDLEWARE_CLIENT_MAX_ENTRIES`：按预期同时活跃的来源数量估算；容量过小会让新来源被 `503` 拒绝。
- `MIDDLEWARE_CORS_ALLOWED_ORIGINS`：生产环境应显式列出前端域名，不要用 `*`；
  留空表示服务不供浏览器跨域调用（同源与后端调用不受影响）。

## 部署配置

### 真实来源 IP

一条规则：`MIDDLEWARE_TRUSTED_PROXIES` 填**服务直接看到的那一跳**（代理 / CDN 回源节点的 IP 或
CIDR），不是客户端的地址；留空则完全忽略转发头、只用连接来源 IP。有代理却不配置的表现是
所有用户共用代理 IP 的额度，日志里的来源也记成代理地址。

直连、Nginx、CDN 回源直连与 CDN + Nginx 四种形态的配置示例、防伪造要点与验证方法见
[生产部署](../../docs/DEPLOYMENT.md)。TLS、连接保护与阻止绕过网关由部署设施负责。

### 生产启动守卫

它是一道防止误上线的启动检查，不是登录鉴权或防火墙。debug 构建含 `dev-login` / `dev-transfer`
这类可直接指定身份的调试接口，因此 `APP_ENV=production` 时禁止运行 debug 构建：
`main` 与 `create_app`（嵌入自定义入口时）都会报错并停止启动。

生产部署先在 `.env` 中设置 `APP_ENV=production`、替换示例 JWT 密钥与示例口令，按
[README 的生产环境注意事项](../README.md) 清理演示数据，再执行：

```bash
cargo build --release
./target/release/guardian-backend
```

发布版本不注册开发登录与开发转账路由。`docs` feature 独立控制 Swagger（默认关闭）；
如生产显式启用，必须在网关限制文档访问。

## 生产保障边界

- 生产配置必须运行 release 二进制；`main` 与 `create_app` 都拒绝 production + debug，
  防止误暴露可伪造身份的调试登录。`docs` feature 仍独立，生产应在网关限制文档访问。
- 首选网关后单实例：TLS、TCP 连接数、请求头大小/读取超时、空闲连接回收、
  公网 DDoS 与可信代理入口隔离由网关负责。HSTS 只是响应头，不提供 TLS。
- 直接公网部署时本服务仍只有明文 HTTP；应用层保护在请求头解析之后才开始，
  替代不了 TLS 与连接层慢速攻击防护。未实现这些设施时不能称为安全公网入口。
- 所有限额都是进程级的；多实例额度随实例数增加，分布式公平性由网关或共享存储承担。
  SQLite 文件不能当成跨机器共享数据库，本项目也不提供集群故障转移。
- `408` 表示停止等待，不代表外部副作用已撤销；提交与超时竞争时结果可能未知，
  写请求必须靠业务幂等键兜底。不能用 HTTP 超时替代数据库事务回滚或外部调用自身超时。
- 超时与并发额度截至响应头产生，不覆盖 SSE、大文件等长响应发送；独立后台任务
  和不可让出的 CPU 工作也不会由此超时强制终止。
- 网关应在编排终止宽限期内停止转发并排空连接。存活/就绪端点同样受额度保护，
  不要把公网请求失败等同于进程死亡，探针频率与失败阈值要考虑短时过载。
- 文件日志按时间轮转而非大小轮转；磁盘配额、轮转归档与磁盘满告警由部署设施保证。

慢接口排查方法见 [日志：低噪声请求诊断](LOGGING.md#低噪声请求诊断)。

## 最小压测方法

先确认并发上限，再确定限速额度。以 [oha](https://github.com/hatoo/oha) 为例：

```bash
cargo install oha

# 单机压测来自同一 IP，应先按压测目标放宽客户端额度，不能只放宽全局额度

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
- `429` 只应在压测目标 QPS 超过 `RATE_LIMIT_REQUESTS / PERIOD`（或超过单个来源客户端额度）时
  出现，且延迟应接近零（快速拒绝而不是排队等窗口）。
- 超时 `408` 的理想值是 0；出现 `408` 应当先查慢查询/外部依赖，而不是直接调大超时。
