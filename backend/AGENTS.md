# AGENTS.md

本文件定义了本项目的 Rust 开发规范与自动化流程。所有贡献者必须严格遵守，并在每次修改代码后及时更新本文件（如有新增规范或调整）。

---

## 1. Git 提交规范

- **范围**：每次提交应独立且完整地对应一个逻辑变更（如单一功能点、缺陷修复或配置调整），禁止混合多个不相关改动；按功能批次顺序组织，单次提交代码变动量建议控制在 500 行以内(仅建议，非强制，可适当突破)，避免大批量改动挤在同一条提交信息中。
- **格式**：`<type>: <中文描述>`
- **常用 type**：
  - `feat` – 新功能
  - `fix` – 修复 bug
  - `docs` – 文档更新
  - `style` – 代码格式（不影响逻辑）
  - `refactor` – 重构
  - `perf` – 性能优化
  - `test` – 测试相关
  - `build` – 构建系统或外部依赖变更
  - `ci` – CI 配置变更
  - `chore` – 杂项（如工具、配置等）
  - `revert` – 回退提交

示例：`feat: 添加用户登录接口`

---

## 2. Rust CI 标准（GitHub Actions）

CI 位于**仓库根目录**的 `.github/workflows/backend.yml`（GitHub 只识别根目录下的
`.github/workflows/`，放在 `backend/` 里不会被执行），通过 `defaults.run.working-directory: backend`
在 crate 目录内执行下列命令：

```yaml
name: Backend CI

on:
  push:
    branches: [ "main", "master" ]
    paths:
      - "backend/**/*.rs"
      - "backend/**/*.proto"
      - "backend/migrations/**/*.sql"
      - "backend/Cargo.toml"
      - "backend/Cargo.lock"
      - "backend/vendor/**/Cargo.toml"
      - "backend/vendor/**/*.rs"
      - "backend/.rustfmt.toml"
      - "backend/.clippy.toml"
      - "backend/.env-example"
      - "backend/docs/**/*.md"
      - "backend/AGENTS.md"
      - "scripts/check-coverage.py"
      - ".github/workflows/backend.yml"
  pull_request:
    branches: [ "main", "master" ]
    paths:
      - "backend/**/*.rs"
      - "backend/**/*.proto"
      - "backend/migrations/**/*.sql"
      - "backend/Cargo.toml"
      - "backend/Cargo.lock"
      - "backend/vendor/**/Cargo.toml"
      - "backend/vendor/**/*.rs"
      - "backend/.rustfmt.toml"
      - "backend/.clippy.toml"
      - "backend/.env-example"
      - "backend/docs/**/*.md"
      - "backend/AGENTS.md"
      - "scripts/check-coverage.py"
      - ".github/workflows/backend.yml"

env:
  CARGO_TERM_COLOR: always

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

defaults:
  run:
    working-directory: backend

jobs:
  check:
    name: Check & Test
    runs-on: ubuntu-latest
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: "1.99.0"
          components: rustfmt, clippy

      - name: Show rustup info
        run: rustup show

      # 覆盖率门槛依赖 cargo-llvm-cov；用 taiki-e 的 action 装预编译二进制，
      # 避免每次从源码编译。
      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@v2
        with:
          tool: cargo-llvm-cov

      - name: Cache Cargo dependencies
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: backend

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Run Clippy (lints)
        run: cargo clippy --all-targets --features docs -- -D warnings

      - name: Check release + docs combination
        run: cargo check --release --features docs

      - name: Build the project
        run: cargo build --verbose

      # 用 llvm-cov 跑测试：它本身就是一次完整测试执行，再单独 cargo test 等于跑两遍。
      # AGENTS.md 10.1 要求行覆盖率 >= 80%，此前只在文档里约定、CI 没有护栏；
      # 门槛按全局行覆盖率卡，不按文件。
      - name: Run tests with coverage
        run: |
          set -euo pipefail
          cargo llvm-cov --features docs --summary-only --json --output-path coverage.json
          python3 ../scripts/check-coverage.py coverage.json
```

---


- 必须包含完整的包元数据（满足可发布到 [crates.io](https://crates.io) 的要求），例如：
  - `name`、`version`、`edition`、`authors`、`description`、`license`、`repository` 等。
- 依赖项必须**归类**，使用 `#` 注释说明每组依赖的用途。
- 每个依赖必须使用 `version = "x.y.z"` **锁定具体版本**（使用 `=` 号），不得使用范围限定符。
- **禁止聚合 feature**：不得写 `features = ["full"]` 这类「一把梭」开关，必须逐项列出代码真正用到的
  feature；依赖自带的默认 feature 集（`default = [...]`）同样要核对，用 `default-features = false`
  关掉未使用的部分再显式开启。
- **依赖与 feature 只保留被引用的部分**：`src/`、`tests/`、`benches/` 里查不到调用点的依赖必须删除，
  不得以「预置备用」「以后可能用」为理由保留；删完必须跑一次 `cargo tree` 对比规模。
  `woothee` 是唯一因日志采集引入的依赖（登录日志的浏览器 / 操作系统，规则表随 crate 发布、无需 feature）。
- 使用 `edition = "2024"` 以及环境中的 Rust 版本，例如:`rust-version = "1.99.0"`

示例结构：

```toml
[package]
name = "my_crate"
version = "0.1.0"
edition = "2024"
rust-version = "1.99.0"
authors = ["Your Name <email@example.com>"]
description = "A short description"
license = "MIT OR Apache-2.0"
repository = "https://github.com/your/repo"

# 核心依赖
[dependencies]
# 序列化
serde = { version = "=1.0.210", features = ["derive"] }

# 异步并发
# 不写 "full"：逐项列出实际用到的 feature
#（macros 覆盖 #[tokio::main] / #[tokio::test]，rt-multi-thread 覆盖多线程运行时）
tokio = { version = "=1.42.0", features = ["macros", "rt-multi-thread"] }

# 开发依赖
[dev-dependencies]
# 基准测试
criterion = { version = "=0.5.1" }

# 编译优化配置
[profile.dev]
opt-level = 1
[profile.dev.package."*"]
opt-level = 3
# release 以体积为优先
[profile.release]
lto = true
codegen-units = 1
```

---

## 4. 代码格式化（.rustfmt.toml）

项目根目录必须包含 `.rustfmt.toml`，内容如下：

```toml
edition = "2024"
max_width = 100
tab_spaces = 4
reorder_imports = true
reorder_modules = true
newline_style = "Auto"
match_block_trailing_comma = true
```

所有代码必须通过 `cargo fmt --all -- --check` 检查。

---

## 5. Clippy 配置（.clippy.toml）

项目根目录必须包含 `.clippy.toml`，内容如下：

```toml
# ── Clippy Configuration ──
# 本文件只配置「lint 的阈值 / 开关」；阈值只有在对应 lint 被启用时才会生效，
# 启用位置是 src/lib.rs 顶部的 #![warn(...)]（见下方说明）。

# clippy::cognitive_complexity 的阈值（已启用）
cognitive-complexity-threshold = 15
# clippy::too_many_arguments 的阈值（complexity 组，clippy 默认即启用）
too-many-arguments-threshold = 5
# clippy::unwrap_used 在测试代码中放行（已启用）
allow-unwrap-in-tests = true

msrv = "1.99.0"
```

启用位置（**必须**与 `.clippy.toml` 的注释保持一致）：

- `src/lib.rs` 顶部的 `#![warn(clippy::cognitive_complexity, clippy::unwrap_used)]` —— 覆盖全部生产代码
  与 `src/` 内的单元测试；`tests/` 集成测试不适用（用例的 setup / 断言天然长而分支多）。
- **只写阈值不启用 lint 等于阈值不生效**：`cognitive_complexity` 属于 nursery、`too_many_lines` 属于
  pedantic、`unwrap_used` 属于 restriction，三者默认都是 `allow`。新增阈值时必须同时在
  `src/lib.rs` 启用对应 lint，否则就是一份“看起来很严”的假配置。
- 启用点是 `#![warn(...)]` 而不是 `Cargo.toml` 的 `[lints.clippy]`：后者只对本包（crate）生效，
  而 `src/main.rs` 是独立的 binary crate root，用 `[lints]` 会漏掉它。写在 `lib.rs` 的内联属性
  也只覆盖 lib 及其子模块，所以 `src/main.rs` 目前**不受这两条 lint 约束**——它的代码量很小，
  真正需要覆盖时把属性也加到 `src/main.rs` 顶部。
- 故意**不启用**的两个 lint：`too_many_lines`（pedantic，30 行阈值会把配置快照、中间件装配
  这类线性代码全部判为超标，拆分反而降低可读性）与 `expect_used`（restriction，在集成测试的
  辅助函数里噪声过大）。改代码而不是调阈值。

所有代码必须通过 `cargo clippy --all-targets --features docs -- -D warnings` 检查，无警告
（与 CI 一致；`docs` feature 必须打开，否则文档代码不在检查范围内）。pre-commit 钩子与
根 `README.md` 的验证命令用同一组参数，新增 feature 时只需改这一处口径。

---

## 6. README.md

- 每次完成任务后需及时更新 `README.md`，至少包含：
  - 项目简介
  - 构建与运行说明
  - 主要功能或使用示例
  - 贡献指南（引用本 AGENTS.md）

---

## 7. .gitignore

必须排除以下内容（示例）：

```
# Rust
/target/
**/*.rs.bk
*.pdb

# macOS
.DS_Store

# IDE
.vscode/
.idea/
*.swp
```

---

## 8. 项目结构

- **使用DDD+洋葱结构，严格遵循此结构开发**
- **禁止单`mod.rs`文件写入大量代码，代码较多时候将代码拆分到更小的带具体名称的代码文件中(如 utils.rs)**
- **保持单代码文件简洁和更细致的 crate 划分以加速增量编译**
- 后台管理模块的代码位置：控制器 `src/api/controllers/admin/`、用例接口 `src/domain/services/admin/`、
  用例实现 `src/services/admin/`、仓储 `src/infrastructure/repositories/`、领域模型 `src/domain/models/`；
  契约与差异见 `docs/ADMIN.md`。

---

## 9. 通用原则

- **保持本文件（AGENTS.md）更新**：每次修正代码或引入新规范后，请同步更新此文档。
- **所有变更**必须通过 CI 检查（格式、lint、构建、测试）。
- **版本锁定**：工具链版本统一使用环境中的版本，但需>=1.99.0（如 CI 和 clippy 配置所示）。
- **遵循设计**：改动必须遵循原有结构设计，不得私自添加和修改，除非用户发出明确重构指令。
- **后续开发追加 AGENTS.md 内容**：写入第 10 章节。
- **测试**：编写单元测试，如果已经安装`cargo-llvm-cov`则检测测试覆盖率>=80%。

---

### 10.6 HTTP 中间件

- 所有中间件集中在 `src/middleware/stack.rs`，用 `tower::ServiceBuilder` 一次装配，
  通过 `middleware::apply(app, &config.middleware)` 实施；controller 与其他层不得再自行包中间件。
- **中间件栈必须包在整个 Router 外层**（`apply` 返回 `Router::new().fallback_service(stack)`）。
  禁止把有状态层（`ConcurrencyLimit`、`RateLimit`）交给 `Router::layer`：`PathRouter::layer`
  会给每条路由复制一份 layer，全局语义会被放大成「每路由一份」。
- `create_app` 的公开签名保持 `pub fn create_app(container: Arc<Container>) -> Router`，
  中间件在函数内部施加，`main.rs` 与集成测试的调用方式不变。
- Tower 的错误类型必须用 `axum::error_handling::HandleErrorLayer` 吸收（axum 要求 `Error = Infallible`），
  处理函数写在与 `stack.rs` 同目录的 `error_response.rs` 中，类型下转用
  `err.is::<tower::load_shed::error::Overloaded>()`。
- 非 `Clone` 的 Tower service（如 `RateLimit`）用 `BufferLayer` 兜住以满足 `axum::serve` /
  `fallback_service` 的 `Clone` 约束。Buffer 只是固定长度队列，克隆共享同一 worker，不增加并发度，
  也不得当作业务任务队列使用。
- 容量参数只允许定义在 `MiddlewareConfig`（`src/infrastructure/config.rs`）并从 `.env` 读取；
  中间件代码中不得出现字面量。新增参数必须同步 `.env`、`.env-example`、`tests/config_tests.rs`。
- 代码兜底默认值必须与 `.env-example` 保持一致（当前：并发 256、背压队列 256、限流 32768 req/s、
  超时 10s、Body 2 MiB、周期 1s、单 IP 并发 32、单 IP 速率 1024/窗口、客户端注册表 16384、
  空闲保留期 60s、慢请求阈值 1000ms、可信代理空）；修改任一侧都必须同步另一侧与
  `docs/MIDDLEWARE.md` 的「中间件配置」表格。
- 中间件的错误响应必须复用 `ApiResponse::error(status, message)`，与业务错误 `AppError`
  的响应格式保持一致；不要在中间件里手写 JSON。
- 并发限制与限速是两个维度：并发限制约束在途请求数，限速约束单位时间请求数；
  两者均为**进程级**实现，多实例部署需在网关或共享存储层补齐分布式限流。
- **CORS 也必须在栈里**：`CorsLayer` 由 `middleware/stack.rs` 的 `cors_layer(config)` 装配
  （栈的最内层），来源白名单读 `MIDDLEWARE_CORS_ALLOWED_ORIGINS`；禁止在 `create_app`
  或 controller 里直接挂 `CorsLayer::permissive()`，默认必须是不发任何 CORS 头。
- 调整执行顺序、新增或移除中间件时，必须同步更新 `docs/MIDDLEWARE.md` 的「中间件栈」章节与
  `docs/CHANGELOG.md`，并在 `tests/middleware_tests.rs` 补对应断言（至少覆盖含排队的超时、
  Body 超限、背压 503、限流 429、panic 隔离、框架错误归一化、`request_id` 关联），
  按来源 IP 的额度另在 `tests/client_protection_tests.rs` 覆盖。
- 需要断言日志内容的测试必须单独成进程（如 `tests/middleware_log_tests.rs`）：`tracing` 的
  callsite interest 缓存是进程级的，其他测试线程若在没有订阅者时命中过 tower-http 的 span
  callsite，该 callsite 会被缓存为 `never`，而 `tracing::subscriber::set_global_default` 并不重建
  这个缓存（需显式调用 `tracing::callsite::rebuild_interest_cache()`）。
- 依赖约束：`tower` 必须是正式依赖（`buffer`/`limit`/`load-shed`/`util`，`util` 提供
  `ServiceBuilder::map_response`）；`tower-http` 需包含 `catch-panic`、`compression-gzip`、`cors`、
  `limit`、`request-id`、`set-header`（`trace` 与 `timeout` 已无人使用：访问日志与总超时
  改由请求观测层承担）；可信代理解析需要 `ipnet`；`axum` 需启用 `matched-path`（路由模板）。
- 限流（`429`）与背压（`503`）触发时是批量拒绝，逐条告警无信息量：
  `src/middleware/error_response.rs` 不得为这两类拒绝打印逐条 `warn` 日志，
  每次请求（含被拒绝的请求）由请求观测层的一条访问/诊断日志覆盖；
  `internal()` 等真正的内部错误仍须保留 `error!` 日志。

### 10.7 日志分批写入

- 文件日志采用内存缓冲分批落盘（`src/logging.rs` 的 `BatchFileWriter` / `BatchGuard`）：
  满 `LOG_BATCH_MAX_EVENTS`（默认 50）条或每 `LOG_BATCH_FLUSH_INTERVAL_SECS`（默认 3）秒
  落盘一次，降低高频场景下逐条写文件的 CPU 开销；控制台层保持实时输出，不得加入缓冲。
- 每条日志必须先格式化为完整一行再写入缓冲（`LineFormat` 包一层 `FormatEvent`），
  保证「条数」计数与日志行一一对应，禁止直接计数 fmt 层的分段写入。
- 进程退出时必须停止定时刷盘线程并落盘剩余缓冲（`BatchGuard::drop`），不得丢日志。
- 批量参数属于日志配置（`LoggingConfig`），走 `.env`；新增/调整时必须同步 `.env`、
  `.env-example`、`tests/config_tests.rs`（默认值、覆盖项、0 值校验）、`docs/LOGGING.md` 的
  「日志写入：分批落盘」章节与 `docs/CHANGELOG.md`；任一项为 `0` 拒绝启动。
- 依赖约束：`tracing-subscriber`、`tracing-appender` 为正式依赖，不得为分批写入引入
  额外的异步通道/缓冲依赖（用 `std::thread` + `std::sync::mpsc` 实现定时刷盘）。

### 10.8 版本发布标签

- 版本发布必须打 annotated tag（`git tag -a`），message 风格与历史保持一致
  （如 `v0.4.0`/`v0.3.0`）：首行 `vX.Y.Z <一句功能标题>`，随后逐条列出本次主要变更，
  末行 `详见 docs/CHANGELOG.md`。
- 已推送的 tag 如需补充或修正 message，先删除本地与远程 tag 再重打重推
  （`git tag -d` + `git push origin :refs/tags/<tag>`），避免 tag 内容与实际发布不符。

### 10.9 OpenAPI 文档（`docs` feature）

- Swagger UI 与 OpenAPI 文档（`src/docs.rs`、`utoipa`/`utoipa-swagger-ui`）统一由
  `cfg(feature = "docs")`（Cargo 的 `docs` feature）门控，**禁止**回到
  `cfg(debug_assertions)`：`docs` feature 不进 `default`，release 与默认构建均不编译
  utoipa 及其文档代码。开发时用 `cargo run --features docs` 访问 `/swagger-ui` 与
  `/api-doc/openapi.json`。
- 仅调试专用的开发行为（dev-login / dev-transfer 路由注册、DTO 的 `force_fail` 字段
  与 `From` 实现等）仍使用 `cfg(debug_assertions)`，与文档开关相互独立。
- controller / DTO 上的 `#[cfg_attr(debug_assertions, utoipa::path(...))]`、
  `derive(ToSchema/IntoParams)`、`schema(example=...)` 等注解改投 `feature = "docs"`；
  CI 的 clippy 与 test 步骤使用 `--features docs`（见第 2 节），确保文档代码也在检查范围内。
- `system` 响应的 `docs_enabled` 字段跟随 `cfg!(feature = "docs")`。
- **`docs` feature 与 `debug_assertions` 正交，四种组合都必须可编译**：
  `utoipa::path` 仅为实际编译的 handler 生成 `__path_*`，因此仅调试存在的
  handler（`dev_login` / `dev_transfer`）的文档必须放在 `src/docs.rs` 里
  单独 `#[cfg(debug_assertions)]` 的 `DevApiDoc`，再通过
  `OpenApi::merge_from` 合入主文档；禁止直接写进主 `ApiDoc` 的 `paths(...)`
  （release + docs 下会因找不到 `__path_*` 编译失败）。CI 的
  `cargo check --release --features docs` 步骤（见第 2 节）负责守住这条约束。

**本文件是项目的“开发宪法”，所有 pull request 和代码审查均应参照其内容。**

## 10. 其他追加内容

### 10.1 测试组织与覆盖率

- 单元测试直接写在源码文件的 `#[cfg(test)] mod tests` 模块中（如 `src/error.rs`、`src/util/password.rs`）。
- 集成测试放在 `tests/` 目录，按职责拆分文件：
  - `tests/api_tests.rs`：通过 `create_app` + `tower::ServiceExt::oneshot` 覆盖 HTTP 接口层（controllers、DTO、extractor、JWT 鉴权、Swagger 文档）。
  - `tests/contract_tests.rs`：用表格驱动锁定全部端点的鉴权与错误语义（401 / 403 / 404 / 405 / 400）。
    新增或删除路由必须同步四张端点表，否则 `endpoint_tables_cover_every_registered_route` 会失败
    （它解析 `api::router()` 的 `Debug` 输出，与四张表双向比对）。
  - `tests/config_tests.rs`：覆盖 `AppConfig::from_env` 的默认值、覆盖项与全部校验分支。
  - `tests/logging_tests.rs`：覆盖日志初始化与配置校验。
  - `tests/middleware_tests.rs`：覆盖中间件栈的对外行为（含排队的超时、Body 超限、背压、限流、
    panic 隔离、框架错误归一化、CORS）；`tests/client_protection_tests.rs` 覆盖可信代理与按 IP 额度；
    `tests/middleware_log_tests.rs` 单独成进程断言日志内容。
- 环境变量是进程级全局状态，依赖环境变量的测试必须通过共享 `Mutex` 串行执行，并在结束后恢复原值。
- 集成测试使用临时 SQLite 文件数据库，每个测试独立装配 `Container`，互不共享状态。
- 覆盖率检测命令：`cargo llvm-cov --features docs --summary-only`，要求行覆盖率 ≥ 80%（全局口径，不按文件考核；
  与 CI 的测试口径一致）。`src/main.rs`（二进制入口）与 `src/infrastructure/services/redis_store.rs`
  （需外部 Redis）不计入该目标，原因见 `docs/TESTING.md` 的「覆盖率检测」。

### 10.2 jsonwebtoken 加密后端

- `jsonwebtoken` 从 v11 起默认不启用任何加密后端，直接调用签发/校验会在运行时 panic。
- `Cargo.toml` 中必须保持 `jsonwebtoken = { version = "=11", features = ["aws_lc_rs"] }`。
  SQLite 部署不经过 TLS，sqlx / sea-orm 侧不再启用 rustls（`runtime-tokio` 而非
  `runtime-tokio-rustls`），因此 `aws-lc-rs` 只服务 JWT；换用其它后端前必须评估口令哈希与
  JWT 验签的兼容性，不要在依赖评审里顺手改。
- 升级 `jsonwebtoken` 时需确认其加密后端特性仍然被显式启用。

### 10.3 双许可协议

- 项目采用 `MIT OR Apache-2.0` 双许可，使用者可任选其一遵循。
- `Cargo.toml` 的 `license` 字段必须保持 `MIT OR Apache-2.0`。
- 许可证文本分文件存放：`LICENSE-MIT`（MIT 全文）与 `LICENSE-APACHE`（Apache-2.0 全文），不要合并进单个 LICENSE 文件。
- README 的「许可证」章节需说明双许可及各自文本文件的位置。

### 10.4 统一响应结构

- 所有带响应体的接口统一返回 `ApiResponse<T>`，JSON 结构固定为 `{code, message, data?, traceId, timestamp}`，`data` 为空时省略该字段；`traceId` 由 `src/middleware/trace.rs` 在响应出栈时注入（只改 JSON 响应体，CSV / ZIP 只加 `x-trace-id` 响应头）。
- `code` 由 HTTP 状态码派生，唯一真值是 `ApiResponse` 内部私有的 `status: StatusCode`；禁止引入能独立设置 `code` 的构造入口。
- 需要非 200 语义时使用 `ApiResponse::with_status(StatusCode, message, data)`，不要新增绕过状态码的构造器。
- controller 直接返回 `ApiResponse<T>`（已实现 `IntoResponse`），不要再手动包裹 `Json`。
- `204 No Content` 等不带响应体的状态码不属于 `ApiResponse` 职责，由 handler 直接返回 `StatusCode`。
- `domain` 层不引入 `http::StatusCode`，错误状态码仍以 `AppError::http_code() -> u16` 表达，在外层 `src/error.rs` 才映射为 `StatusCode`。
- 修改响应结构时必须同步更新 `docs/RESPONSE.md` 的「统一响应结构」章节与 `docs/CHANGELOG.md`。

### 10.5 事务与持久化

- **事务边界只允许出现在 `services` 层**：`begin` / `commit` / `rollback` 不得出现在 controller、仓储或 `domain` 中。
- 事务必须显式开启与结束：成功路径显式 `commit`，失败路径显式 `rollback`，禁止依赖隐式提交或静默兜底。
- 回滚失败不得覆盖原始业务错误，只记录 `warn` 日志。
- 仓储方法签名统一接收 `C: ConnectionTrait`：传入 `&DatabaseConnection` 表示不进事务，传入 `&DatabaseTransaction` 表示在调用方事务内执行；禁止为事务单独复制一套方法。
- 仓储接口因引用 `sea_orm::ConnectionTrait` 而归入 `infrastructure/repositories`，`domain/repositories` 只保留边界说明；`domain` 不得依赖 `sea-orm`。
- 含泛型方法的 trait 无法作为 `dyn` 使用，仓储通过泛型参数注入 `services`；对外用例接口（`domain/services`）保持 `dyn` 兼容，由 `container.rs` 装配。
- 读改写场景必须开启写事务：SQLite 不支持在事务内切换隔离级别（`IsolationLevel` 会被
  sea-orm 忽略并打 warn），必须用 `begin_with_options(TransactionOptions { sqlite_transaction_mode: Some(SqliteTransactionMode::Immediate), .. })`；
  金额、数量用整数最小单位表示（如 `amount_cents: i64`），禁止浮点。
- 需要并发控制的表维护自增 `version` 列，更新时以 `WHERE id = ? AND version = <读取时的值>`
  做条件更新并自增 `version`；影响行数为 0 表示期间被其他事务改过，必须返回 `AppError::Conflict`（409），
  不得把 `update_many` 的「0 行」当成成功而静默覆盖。
- 新增实体放在 `src/entities` 下独立文件，并在 `migrations/` 目录新增对应的 sqlx 迁移文件
  （`CREATE TABLE`，由 `databases::run_migrations` 以 `sqlx::migrate!` 执行）；
  种子数据必须幂等（`NOT EXISTS` 或等价写法）。
- **当前只支持 SQLite**：迁移文件与 `schema.rs` 的补列/索引逻辑都是 SQLite 语法
  （`pragma_table_info`）。`DATABASE_URL` 必须在 `AppConfig::from_env` 阶段校验为 `sqlite:` 开头，
  非法 scheme 拒绝启动；要换 MySQL/PostgreSQL 必须同时把迁移与补列逻辑重写成后端无关的写法。
- 每个事务用例至少覆盖「提交成功」与「中途失败回滚后数据无残留」两类测试，回滚测试需断言失败前状态未被改变。
- 唯一约束冲突属于客户端可控冲突，在 `From<DbErr>` 中统一映射为 `AppError::Conflict`（409），
  响应文案用稳定描述（如 `resource already exists`），**不得**把数据库原始 detail（含表名、列名）
  透给客户端，只写 `debug` 日志；也不得当作 500 处理。幂等键一类业务唯一列必须建唯一索引，
  不能只靠先查后写。
- 数据库文件不得入库（`.gitignore` 排除）：建表、索引与种子数据必须由启动流程自动完成且幂等，表结构变更需同时提供老库的自动补齐路径。
- 参考实现：`src/services/transaction.rs`、`src/infrastructure/repositories/transaction.rs`。

---

### 10.10 后台管理模块（对齐 scaffold-master）

- **契约以 `scaffold-master` 的 Spring Boot 实现为准**：接口路径、请求 / 响应字段、错误文案、权限码与
  数据范围语义都逐条对照，改动前先读参考实现；差异必须写进 `docs/ADMIN.md` 的「与 Spring 版的差异」。
- **请求前缀固定 `/api/v1`**，中间件看到的路径是 `nest` 之后的相对路径（如 `/system/users`），
  `src/middleware/audit.rs` 的规则表按相对路径匹配。
- **鉴权与权限走 `AdminUser` 提取器**：`require(code)` 校验权限码，`data_scope()` 取数据范围；
  禁止在 controller 里手写权限判断或直接查库解析角色。
- **请求体统一用 `ApiJson<T>`**：非法 JSON 与类型不匹配统一为 400「请求体格式错误」，
  与 Spring 的 400 语义对齐；骨架既有示例接口保持 `Json<T>`（422），两者不要混用。
- **错误文案**：客户端可见的 4xx 用 `AppError::business`（文案原样透出，不加英文前缀），
  5xx 统一「系统内部错误」；不得把数据库 detail 透给客户端。
- **留痕规则表与业务接口同步维护**：新增写接口要在 `src/middleware/audit.rs` 的 `AUDIT_ROUTES`
  补一条「方法 + 路径段模式 + 模块 + 动作」，并在 `tests/monitor_api_tests.rs` 或对应测试文件里断言。
- **生成器模板是契约**：`src/services/generator.rs` 的 7 个模板与 Spring 文本块逐字一致，
  集成测试按字节断言，调整模板必须同步更新 `tests/generator_api_tests.rs`。
- **时间字段**：SQLite 没有原生日期类型，统一存 `TEXT`（`YYYY-MM-DD HH:MM:SS` 本地时间，
  `src/util/time.rs` 提供格式化与解析），比较用字典序，SQL 侧用 `strftime` / `julianday` / `substr`。
- **会话与验证码必须经过 `AuthStore`**：生产用 `RedisAuthStore`（`REDIS_URL` / `REDIS_PASSWORD`），
  测试用 `MemoryAuthStore`（`Container::memory_auth_store()`）；禁止绕开抽象直接操作 Redis 客户端。
- 新增页面级模块时按 `src/api/controllers/admin/` + `src/domain/services/admin/` + `src/services/admin/`
  三处同名文件扩展，`AdminUseCase` 只做子 trait 的合并视图，不写实现。

---

### 10.11 生产数据与演示数据

仓库默认「克隆即可运行」，因此启动时会幂等播种演示数据。**这些数据只服务于本地演示与联调**，
不得原样带上生产环境；部署前按下表处理：

| 处理方式 | 数据 | 说明 |
| --- | --- | --- |
| 必须替换 | `.env` 的 `JWT_SECRET`、`APP_ENV=production`、`DATABASE_URL` 指向部署路径 | 启动时会主动拒绝示例密钥 |
| 必须替换 | `.env` 的 `SEED_ADMIN_PASSWORD`（播种账号与新增用户的默认口令） | 换成强口令；该值不落迁移文件与种子 SQL |
| 必须替换 | `.env` 的 `SEED_USERS`（不设置会播种三个演示账号） | 换成真实账号清单；格式「用户名:昵称:邮箱:部门ID」，解析失败启动即拒 |
| 必须替换 | `sys_config` 的 `sys.user.initPassword`（内置演示值） | 改成强口令或删除该行，强制新增用户显式传口令 |
| 必须替换 | `sys_user` 中 `admin` 的口令 | 播种口令公开可知，首次登录后立即修改 |
| 应清理 | `sys_user` 的 `linjh`、`heyj` 及 `sys_user_role` / `sys_user_post` 关联 | 内置演示账号，用 `SEED_USERS` 覆盖后不会产生 |
| 应清理 | `sys_dept` 的演示组织（含负责人姓名）、`sys_post` 的演示岗位 | 替换为真实组织与岗位 |
| 应清理 | `transfer_accounts` / `transfer_records` / `transfer_audits`（`acc_alice`、`acc_bob`） | 事务示例数据，生产应按业务替换或清空 |
| 应清理 | `sys_oper_log` / `sys_login_log` 的本地联调记录、Redis 的 `captcha:*` 与 `auth:token:*`、`log/` | 环境切换时清空 |
| 建议保留 | `sys_menu` 的目录 / 菜单 / 按钮记录 | 前端动态路由与权限码的来源，清空会导致页面不可用 |
| 建议保留 | `sys_role` 的内置 `super_admin` 角色及其菜单授权 | `AdminUser` 提取器对该角色直接放行 |
| 建议保留 | `sys_user` 的首个账号（默认 `admin`，改口令后保留） | 唯一被授予超级管理员的账号，否则没有任何账号能登录后台 |
| 按需替换 | `sys_dict_type` / `sys_dict_data` | 业务下拉数据 |

- 演示接口（`examples`、`transactions` 与 `dev-login` / `dev-transfer`）在真实项目中应替换或删除；
  `dev-*` 只在 debug 构建注册，release 构建不含。
- 新增种子数据必须幂等（`INSERT OR IGNORE` 或先查后写），并同步更新 `README.md` 的生产清理清单。

### 10.12 依赖与 feature 收紧

- **不使用聚合 feature**：`tokio` 曾用 `features = ["full"]`，现按调用面逐项列出
  （`macros` / `net` / `rt-multi-thread` / `signal` / `time`）；新增能力时对着代码补 feature，
  不要回退到 `full`。`axum`、`sea-orm` 未关默认 feature 集同样属于「隐式全开」，
  用 `default-features = false` + 显式列表取代。
- **`axum` 的默认集**：`form` / `original-uri` / `tower-log` 未被使用，
  当前开 `http1` / `json` / `matched-path` / `query` / `tokio` / `tracing`（`matched-path` 供
  诊断日志取路由模板）。路由与提取器改动后若编译报
  `no method/type found`，先核对是不是用到了新 feature 再补声明，不要直接恢复默认集。
- **`sea-orm` 的默认集**：默认打开的 `with-json` / `with-chrono` / `with-uuid` / `with-time` /
  `with-rust_decimal` / `stream` 都未被使用（实体列只有 `i64` 与 `String`，也没有流式查询）。
  实体里新增这些类型的列时必须同步补对应 feature，否则编译失败。
  `sqlite-use-returning-for-3_35` 影响生成的 INSERT 语句，必须保留。
- **只支持 SQLite 就不启用 TLS**：数据库侧用 `runtime-tokio` 而非 `runtime-tokio-rustls`，
  避免把 `rustls` / `ring` / `webpki-roots` 编进产物；要支持 MySQL / PostgreSQL 时必须一并
  把 TLS 后端与迁移逻辑重新评估。
- **不保留「预置备用依赖」**：原先 `preset`（`rayon` / `validator` / `regex` / `base64` /
  `once_cell` / `rand` / `derive_more` / `itertools`）与 `http-client`（`reqwest`）两个 bundle
  在 `src/`、`tests/` 里零调用点，已整组删除。新增依赖必须当次就有调用点，
  并按第 3 节的规则锁定 `=` 版本、加分类注释。
- **依赖变更后的验证口径**：至少跑 `cargo tree -e normal --prefix none | sort -u | wc -l`
  对比前后 crate 数量，再跑第 2 节的完整 CI 四件套（fmt / clippy / check --release --features docs /
  test --features docs）；删 feature 属于编译期约束变更，只靠 `cargo check` 不能证明行为不变。
  涉及 `debug_assertions` 门控代码或 release profile 时，额外跑一次 `cargo test --release --features docs`。

### 10.13 发布产物优化

- `[profile.release]` 开 `lto = true` 与 `codegen-units = 1`：前者做跨 crate 内联，后者让优化器
  看到全部代码，两者都不改变业务语义。代价是 release 构建变慢（本 crate 约 1m → 3.5m），
  CI 只跑 debug 构建与 `cargo check --release`，不受影响。
- **不加 `strip`**：保留符号表以便生产 panic 与性能采样能打出可读栈帧；要再降体积可自行加
  `strip = "symbols"`（实测再减约 2.2 MB），但要接受栈帧只剩地址。
- 需求是“快”而不是“小”时，改回去只需删这两行；不要在业务代码里为体积做特殊处理。
- 修改 profile 后必须重跑 `cargo build --release` 并记录产物大小（`stat -c%s target/release/guardian-backend`）
  与 `docs/CHANGELOG.md`。

---

### 10.14 客户端 IP 与保护

- HTTP 监听必须通过 `into_make_service_with_connect_info::<SocketAddr>()` 注入真实 TCP 对端；
  集成测试要显式注入 `ConnectInfo<SocketAddr>`（`tests/support/mod.rs` 的 `send` / `send_raw` 已代劳），
  缺少对端信息时拒绝请求（500），不静默绕过保护。
- 客户端以规范化 IP 为键（IPv4 映射 IPv6 归一成 IPv4），共享出口共用额度；
  仅对 `MIDDLEWARE_TRUSTED_PROXIES` 里的显式 IP/CIDR 采信 `X-Forwarded-For`，从右向左剥离可信代理。
  不要相信所有转发头，也不要默认信任私网；不使用 `X-Real-IP` / `Forwarded` 兜底。
- 单个来源 IP 的并发许可必须在正常完成、取消、超时与异常退出时都释放；注册表有界，
  活动项不得驱逐，空闲保留期不得小于客户端限流窗口（启动时校验）。
- 保护层拒绝要用统一响应结构：速率超限带 `Retry-After`（秒），并发超限不猜测等待时间；
  5xx 文案统一「系统内部错误」，不得把容量细节透给客户端。
- 访问日志不记录 query、Authorization 或请求体；客户端 IP 与请求 ID 仅用于请求追溯。
  留痕模块（`src/middleware/audit.rs`）与登录日志的来源 IP 同样取保护层的可信解析结果
  （`client_ip::source_of`），不得直接读请求头原值。

### 10.15 请求诊断与失败隔离

- 请求观测与总超时必须放在全局排队层之外；超时涵盖排队至响应头产生，不保证响应流发送期限，
  也不代表已提交的外部副作用被撤销。取消时所有已持有的许可必须释放。
- 路由模板采集允许在统一装配点施加无状态 `route_layer`（`diagnostics::route`）；
  controller 不得自行装配中间件。
- 每请求仅一条访问或诊断事件（`request completed` / `request diagnostic`）；慢请求、408 与业务 5xx
  才输出阶段汇总，429/容量拒绝不逐条告警。services 的关键 IO 用固定阶段名 `measure` 包裹，
  不逐阶段打印，也不得把用户输入、SQL 或参数写进阶段名。
- 框架生成的纯文本 400/404/405/413/415/422 由观测层归一化成统一 JSON，不回显解析细节，
  不改写已有业务 JSON，405 保留 `Allow`；panic 保留 `error!` 日志并返回安全 500。
- `traceId` 的响应体注入留在压缩层之内（否则读到的是压缩后的字节）；由外层生成的响应
  （限流、背压、panic）至少保证 `x-trace-id` 响应头，异常点已记在 `docs/RESPONSE.md`。
- production 配置必须拒绝 debug 构建：`main` 启动校验与 `create_app` 嵌入入口都要守住边界；
  `docs` feature 仍独立，release + docs 必须能编译，生产文档访问由网关限制。

### 10.16 登录日志的客户端来源

- `sys_login_log` 的 `ip` / `location` / `browser` / `os` 必须采集：`ip` 取保护层按可信代理链解析出的
  真实对端（`client_ip::source_of`，与操作日志同源），**不得**直接读请求头原值；
  `location` 由 `IpLocation::resolve` 给出：回环「本机」、RFC1918 私网 / 链路本地 / IPv6 ULA
  「内网」（不查库），其余查离线 xdb 并写入库里原始串，未命中写「外网」；`browser` / `os` 由
  `src/util/user_agent.rs` 解析 `User-Agent`，识别不出时留空。
- 登录 / 退出用例的签名收 `&ClientSource`（`src/domain/models/identity.rs`），
  控制器通过 `src/api/extractors/client_source.rs` 提取，不自己读头或扩展。
  验证码失败不写日志、账号密码错误与账号停用要写失败日志的既有约定不变。
- 禁止把 `User-Agent` 原样或版本号写入日志列；`Column` 名称级的解析结果在
  `tests/admin_api_tests.rs` 与 `tests/monitor_api_tests.rs` 各有一条断言，
  调整采集字段或解析规则时同步这两处。
- 地名库是**可选**的：`IP_LOCATION_DB_PATH` 留空时不查库（地点只有三档），配置了却加载不了
  必须启动失败，不得静默降级。库按 IPv4 / IPv6 分别配置（`IP_LOCATION_DB_PATH` 与
  `IP_LOCATION_V6_DB_PATH`，各用一个 `Searcher`），两个都可留空。修改解析口径要同步 `docs/ADMIN.md` 的字段表、
  `docs/DEPLOYMENT.md` 的「离线 IP 地名库」与 `src/infrastructure/ip_location.rs` 的单测。

### 10.17 生产部署文档

- 部署相关说明统一维护在仓库根 [docs/DEPLOYMENT.md](../docs/DEPLOYMENT.md)，由根 README 的
  「文档导航」链接；`backend/docs/MIDDLEWARE.md` 只保留可信代理的规则与指向该文档的链接，
  不要在两处各写一份代理配置示例。
- 真实客户端 IP 的结论以代码为准：只有直连对端命中 `MIDDLEWARE_TRUSTED_PROXIES` 时才解析
  `X-Forwarded-For`，从右向左剥离可信代理。文档里的示例必须与服务端行为一致；
  调整解析规则或 `ConnectInfo` 注入方式时，同步更新该文档的「部署形态与真实客户端 IP」
  与「验证真实 IP 是否生效」两节。
- 生产清单（示例密钥、默认口令、演示数据）仍以 `README.md` 的「生产环境注意事项」为准，
  部署文档只引用、不复制。

### 10.18 环境变量与 `.env-example` 注释

- **每个变量必须有一条独立注释**：写清含义、单位、默认值带来的行为、取值边界，以及越界或依赖不可用时的
  表现（拒绝启动 / 返回 503 / 降级回源 / 留空即关闭）。用「一行概括整段」代替逐项说明不合规；
  与 `MIDDLEWARE_TRUSTED_PROXIES`、`CACHE_*` 一致，多行说明紧贴变量上方。
- 分组标题可以写 `# ── 分组名 ──`，但分组内仍要逐项注释；分组标题不能承载某个变量的含义。
- **密钥与口令类变量**（`JWT_SECRET`、`TWO_FACTOR_KEY`、`REDIS_PASSWORD` 等）注释必须说明生成方式、
  留空后果与备份要求，且 `.env-example` 只能写占位值或留空，不得写入任何真实密钥。
- 注释里的边界值必须与 `src/infrastructure/config.rs` 的校验分支一致（如 `CACHE_TTL_SECS` ≤ 86400、
  `CACHE_NEGATIVE_TTL_SECS` ≤ 正常 TTL、`CACHE_MAX_LOADS` ≤ 64）；改校验时必须同步注释，
  反之亦然，并由 `tests/config_tests.rs` 覆盖对应分支。
- **`.env-example` 与本地 `.env` 保持同样的注释、取值与顺序**：`.env` 不入库，两者漂移会让
  「按模板配置」与「本地能跑」变成两套行为。新增变量时同一次改动里补齐
  `.env-example`、`.env`、`config_tests.rs` 与对应文档表格（见第 10.6 节的同步清单）。

### 10.19 启动配置快照

- `log_startup_config`（`src/logging.rs`）必须覆盖 `AppConfig` 的**全部字段**，按结构分节输出；
  新增配置结构体或字段时，同一次改动里加上对应分节与行，不允许「先加字段、快照以后补」。
- **遗漏由测试守住，不靠人工比对**：`tests/logging_interval_tests.rs` 从 `AppConfig` 的 `Debug`
  输出提取所有字段名，逐个断言出现在日志中，并校验分节齐全；删掉任意一行输出都会使该用例失败。
- 敏感值只以脱敏形式出现：`JWT_SECRET`、`DATABASE_URL` / `REDIS_URL` 中的口令、
  `SEED_ADMIN_PASSWORD` 输出字符数；`TWO_FACTOR_KEY` 只输出 `(configured)` / `(not set)`；
  其它密钥类字段同理，禁止输出原文。测试同时断言这些原文不出现在日志里。
- 快照写的是**本次进程实际生效的值**，不是 `.env` 文件内容；同一变量在环境变量、`.env`
  与代码默认值之间存在覆盖关系时，以快照为准，文档描述必须与该行为一致。

### 10.20 vendor 第三方代码

- `vendor/` 下是从上游复制进来的第三方实现，目前只有 `vendor/ip2region`（登录 / 操作日志的
  离线 IP 地名）。它们的来源提交、许可与本仓库的差异记在各自的 `README.md`，不得只留代码不留出处。
- 优先用 crates.io 的 `=x.y.z` 依赖，vendor 只在依赖不可得或会拖入无关依赖树时才用。
  `ip2region` 官方 Rust 客户端未发布到 crates.io，且把生成工具 `maker` 当运行时依赖
  （`clap` / `chrono` / `itertools` 等约 70 个包），因此只 vendor 查询侧并自带最小头部解析。
- `Cargo.toml` 引用 vendor 依赖时同时写 `version = "=x.y.z"` 与 `path`，保持与其他依赖一致的
  锁版本口径；升级按 vendor 目录 `README.md` 的差异表重新套用上游文件。
- vendor 代码纳入本仓库的格式检查（`cargo fmt --all`）与编译，但不适用本仓库的
  `#![warn(...)]` lint 与覆盖率目标；改动 vendor 代码时不要顺手改它的实现逻辑。
