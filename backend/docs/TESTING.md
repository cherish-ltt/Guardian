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

（CI 同款命令见 `.github/workflows/rust-ci.yml`；`default` feature 为空，
`--all-features` 等价于启用 `preset`、`http-client`、`docs` 三个可选 feature，
因此 `--all-features` 会比 CI 多编译一批预置库。）

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
- 黑盒测试锁定的是对外契约：HTTP 状态码、`{code, message, data?, traceId, timestamp}` 统一响应结构、
  CORS 预检与 `allow` 响应头，以及 400/401/404/405/409/415/422 等错误语义。
- 请求体未通过 JSON 提取器时（400/415/422）响应是 axum 生成的纯文本，不属于统一响应结构，
  这类用例只断言状态码；后台接口的请求体走 `ApiJson`，非法 JSON 统一为 400「请求体格式错误」。

新增接口时按同样方式补一条 `oneshot` 用例即可，无需启动服务或 mock 端口。

### 后台管理与监控接口测试

`tests/support/mod.rs` 提供公共装置：临时 SQLite 装配、登录（自动取验证码）与带令牌请求构造、
原始响应读取。基于它的三个契约测试文件：

| 文件 | 覆盖范围 |
| --- | --- |
| `tests/admin_api_tests.rs` | 认证全流程、用户 / 角色 / 菜单 / 部门 / 岗位 / 字典 / 参数的 CRUD 与校验文案、导出、403 与数据范围、404 / 405 文案 |
| `tests/monitor_api_tests.rs` | 写操作留痕的内容与脱敏、操作 / 登录日志的列表与统计、时间线、热力图、导出、删除与清空、工作台聚合 |
| `tests/generator_api_tests.rs` | 候选表与导入幂等、字段元数据、7 个生成文件的内容、ZIP 条目、留痕动作 |

这些用例按字节断言生成的 Java / TS / Vue / SQL 文件内容，因此修改代码生成模板必须同步更新断言。
