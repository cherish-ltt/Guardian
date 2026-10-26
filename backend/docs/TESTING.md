# 测试与构建检查

> 本文档详解测试组织方式与构建验证流程，快速上手见 [README](../README.md)。

## 构建检查

建议执行：

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --features docs -- -D warnings
cargo check --all-targets
cargo check --release
cargo check --release --features docs
cargo test --features docs
```

（CI 同款命令见仓库根的 `.github/workflows/backend.yml`；`default` feature 为空，
当前可选 feature 只剩 `docs`，因此 `--all-features` 与 `--features docs` 等价。）

这样可以同时确认：

- 启用 `docs` feature 时 Swagger 与 OpenAPI 文档正常（开发用 `cargo run --features docs`）；
- 发布模式下不会暴露开发登录接口；
- `docs` feature 与 `debug_assertions` 的四种组合都能编译（二者是正交开关，
  release + docs 下调试接口的文档元数据会被自动裁剪）；
- 所有 feature 和 release 构建均可通过；
- 测试和 Clippy 检查通过。

应用启动时通过 `sqlx::migrate!` 执行 `migrations/` 目录下的版本化迁移（记录写进 `_sqlx_migrations` 表），
在此基础上补齐老库缺列、建索引并幂等播种演示账户。生产环境建议在部署阶段执行明确的迁移脚本，
不要使用示例 JWT_SECRET。

回滚这类“写了一半”的行为应当有测试兜底：可参考
`services/transaction.rs` 的单测（在事务内主动失败后断言数据库无残留）与
`tests/api_tests.rs` 中的 `transfer_rolls_back_every_write_when_force_fail_is_on`。

## HTTP 黑盒测试（`oneshot`）

`tests/api_tests.rs` 是接口层的黑盒测试：不直接调用 controller 或 service，
而是用 `tower::ServiceExt::oneshot` 把 `create_app(container)` 产出的 `Router`
当成一次性 `Service` 驱动，直接断言请求进、响应出，无需监听真实端口。

约定：

- 每个测试用 `setup_app()` 独立装配一套 `Container` 与临时 SQLite 文件库，互不共享状态；
  会话用内存实现（`Container::memory_auth_store()`），不依赖外部 Redis。
- `tests/api_tests.rs` 自带 `setup_app` / `send` / `send_response` 辅助函数；后台与监控接口改用
  `tests/support/mod.rs` 的公共装置（同样提供 `send()`，并用 `support::send_raw()` 取
  `(StatusCode, HeaderMap, Vec<u8>)` 断言 CSV / ZIP 等非 JSON 响应）。
- `send()` 返回 `(StatusCode, Value)`，断言同时覆盖 HTTP 状态码与统一响应结构。
- `Router` 在 `oneshot` 后即被消费，需要多次请求时先 `app.clone()`。
- 请求必须注入真实或测试用的 `ConnectInfo<SocketAddr>`（`tests/support/mod.rs` 的 `send` / `send_raw`
  会自己补一个本地对端）；缺少对端时保护层失败关闭，返回 `500` 而不是静默绕过。
- 黑盒测试锁定的是对外契约：HTTP 状态码、`{code, message, data?, traceId, timestamp}` 统一响应结构、
  CORS 预检与 `allow` 响应头，以及 400/401/404/405/409/415/422 等错误语义。
- 框架生成的纯文本 400/404/405/413/415/422 由请求观测层归一化成统一 JSON，不回显解析细节；
  后台接口的请求体走 `ApiJson`，非法 JSON 统一为 400「请求体格式错误」。
- `tests/client_protection_tests.rs` 验证代理防伪、IP 额度隔离、跨路由共享与取消后释放；
  `tests/middleware_tests.rs` 覆盖超时含排队、panic 隔离、HEAD 无响应体与非法请求 ID 替换；
  `tests/middleware_log_tests.rs` 单独成进程，覆盖低噪声日志、路由模板、query 不泄漏，
  并用真实 SQLite 连接池占满验证 `db.ready` 阶段的中断诊断。

新增接口时按同样方式补一条 `oneshot` 用例即可，无需启动服务或 mock 端口。

### 覆盖率检测

```bash
cargo llvm-cov --features docs --summary-only
```

`AGENTS.md` 第 10.1 节要求**行覆盖率 ≥ 80%**：全局口径，不按文件考核，CI 由
`scripts/check-coverage.py` 卡这条门槛；
命令与 CI 的测试口径保持一致（同样带 `--features docs`，否则 `docs` 门控的代码不在统计范围内）。
当前实测：**行 96.66% / 函数 92.40% / region 94.51%**（320 个用例，其中 2 个需真实 Redis 默认忽略；`cargo-llvm-cov 0.9.1`）。
这只是当时的快照，改动后需重新测量，不要把数字当常量。

下面两处不计入覆盖率目标，也无需为它们写测试：

| 文件 | 不统计的原因 |
| --- | --- |
| `src/main.rs` | 二进制入口：进程启动、信号监听与优雅退出，单测进程覆盖不到（要覆盖必须拉起真实进程与端口）。 |
| `src/infrastructure/services/redis_store.rs` | 生产用的 `RedisAuthStore` 需要外部 Redis 实例；集成测试统一用 `MemoryAuthStore`，不让测试依赖外部服务。 |
| `vendor/ip2region/` | 从上游复制的第三方查询客户端（见该目录 `README.md`）；覆盖率命令只统计本包，且它的行为由上游保证。 |

除这两处外，剩余未覆盖行只有两类，都不值得为覆盖率補断言：

- **防御性兜底**：如各仓储的 `.ok_or_else(|| AppError::internal(...))`（数据库时钟无返回值）、
  `offset` 整数溢出的 `checked_*` 守卫、`seed_user_assignments` 的 `assigned == 0`、
  `Deref` 实现等没有实际触发路径的分支。
- **需要故障注入的错误分支**：写盘失败回退缓冲（`logging.rs`）、响应体收集失败
  （`middleware/trace.rs`）、留痕写入失败（`middleware/audit.rs`）、事务回滚失败
  （`services/support.rs`）—— 它们的存在本身就是“不让未预期失败掩盖真因”。

### 后台管理与监控接口测试

`tests/support/mod.rs` 提供公共装置：临时 SQLite 装配、登录（自动取验证码）与带令牌请求构造、
原始响应读取。基于它的九个集成测试文件：

| 文件 | 覆盖范围 |
| --- | --- |
| `tests/contract_tests.rs` | 全部 91 个 `(方法, 路径)` 的鉴权与错误语义矩阵：77 个受保护端点缺令牌一律 401、65 个权限码端点用零权限账号一律 403、10 个公开端点免令牌；404 / 405 / 400 的统一文案 |
| `tests/admin_api_tests.rs` | 认证全流程（含登录 / 退出日志的客户端来源与浏览器采集）、用户 / 角色 / 菜单 / 部门 / 岗位 / 字典 / 参数的 CRUD 与校验文案、导出、403 与数据范围、404 / 405 文案 |
| `tests/monitor_api_tests.rs` | 写操作留痕的内容（含可信代理下的来源 IP）与脱敏、操作 / 登录日志的列表与统计、时间线、热力图、导出、删除与清空、工作台聚合 |
| `tests/generator_api_tests.rs` | 候选表与导入幂等、字段元数据、10 个生成文件的内容、ZIP 条目、留痕动作 |
| `tests/account_api_tests.rs` | 本人资料字段白名单、2FA 绑定/启用/关闭/恢复码、动态码重放、恢复码并发单次消费、会话撤销、日志脱敏、人工重置的权限与密码校验、失败限次 |
| `tests/account_security_tests.rs` | 待绑定密钥密文存储与过期、错误动态码不启用、`0003` 权限迁移只授予一次且不恢复已撤销授权、非超管不能重置超管 |
| `tests/account_fault_tests.rs` | Redis 会话清理失败时仍返回已提交的恢复码、旧令牌被会话版本拒绝；业务缓存故障时已认证读请求降级 SQLite，认证存储故障仍返回 503 |
| `tests/login_challenge_tests.rs` | 两步登录：第一步不签发 JWT、挑战一次性、过期/密码变更/失败限次均失效、并发提交只成功一次 |
| `tests/cache_tests.rs` | 并发未命中只回源一次、写屏障阻止旧值回填、缓存故障降级与回源上限 503、空值缓存与错误不缓存、键数上限；真实 Redis 的读写 / TTL / 换代与验证码原子取用用例默认 `#[ignore]` |

`tests/contract_tests.rs` 的四张端点表与 `src/api/controllers` 的路由注册一一对应：
新增或删除端点时必须同步这四张表，否则 `endpoint_tables_cover_every_registered_route`
会报错。这个用例会解析 `api::router()` 的 `Debug` 输出，还原 Router 实际注册的
`(方法, 路径)`，再与四张表双向比对：新增路由没登记报「未登记」，删了路由而表没跟上
报「已失效」——它就是为了让「加了接口但忘了加鉴权」在测试里暴露。

这些用例按字节断言生成的 Java / TS / Vue / SQL 文件内容，因此修改代码生成模板必须同步更新断言。
