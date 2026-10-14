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
      - "backend/**.rs"
      - "backend/**.proto"
      - "backend/Cargo.toml"
      - "backend/Cargo.lock"
      - "backend/.rustfmt.toml"
      - "backend/.clippy.toml"
      - ".github/workflows/backend.yml"
  pull_request:
    branches: [ "main", "master" ]
    paths:
      - "backend/**.rs"
      - "backend/**.proto"
      - "backend/Cargo.toml"
      - "backend/Cargo.lock"
      - "backend/.rustfmt.toml"
      - "backend/.clippy.toml"
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

      - name: Run tests
        run: cargo test --features docs --verbose
```

---


- 必须包含完整的包元数据（满足可发布到 [crates.io](https://crates.io) 的要求），例如：
  - `name`、`version`、`edition`、`authors`、`description`、`license`、`repository` 等。
- 依赖项必须**归类**，使用 `#` 注释说明每组依赖的用途。
- 每个依赖必须使用 `version = "x.y.z"` **锁定具体版本**（使用 `=` 号），不得使用范围限定符。
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
tokio = { version = "=1.42.0", features = ["full"] }

# 开发依赖
[dev-dependencies]
# 基准测试
criterion = { version = "=0.5.1" }

# 编译优化配置
[profile.dev]
opt-level = 1
[profile.dev.package."*"]
opt-level = 3
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
# 已启用的 lint 列在 Cargo.toml 的 [lints.clippy]（作用于本包全部 target）。

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
- 故意**不启用**的两个 lint：`too_many_lines`（pedantic，30 行阈值会把配置快照、中间件装配
  这类线性代码全部判为超标，拆分反而降低可读性）与 `expect_used`（restriction，在集成测试的
  辅助函数里噪声过大）。改代码而不是调阈值。

所有代码必须通过 `cargo clippy --all-targets --features docs -- -D warnings` 检查，无警告
（与 CI 一致；`docs` feature 必须打开，否则文档代码不在检查范围内）。`--all-features` 会额外
编译 `preset` / `http-client` 两个可选依赖 bundle，pre-commit 钩子用它做本地兜底。

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
  超时 10s、Body 2 MiB、周期 1s）；修改任一侧都必须同步另一侧与 `docs/MIDDLEWARE.md` 的「中间件配置」表格。
- 中间件的错误响应必须复用 `ApiResponse::error(status, message)`，与业务错误 `AppError`
  的响应格式保持一致；不要在中间件里手写 JSON。
- 并发限制与限速是两个维度：并发限制约束在途请求数，限速约束单位时间请求数；
  两者均为**进程级**实现，多实例部署需在网关或共享存储层补齐分布式限流。
- **CORS 也必须在栈里**：`CorsLayer` 由 `middleware/stack.rs` 的 `cors_layer(config)` 装配
  （栈的最内层），来源白名单读 `MIDDLEWARE_CORS_ALLOWED_ORIGINS`；禁止在 `create_app`
  或 controller 里直接挂 `CorsLayer::permissive()`，默认必须是不发任何 CORS 头。
- 调整执行顺序、新增或移除中间件时，必须同步更新 `docs/MIDDLEWARE.md` 的「中间件栈」章节与
  `docs/CHANGELOG.md`，并在 `tests/middleware_tests.rs` 补对应断言（至少覆盖超时、Body 超限、
  背压 503、限流 429、`request_id` 关联）。
- 需要断言日志内容的测试必须单独成进程（如 `tests/middleware_log_tests.rs`）：`tracing` 的
  callsite interest 缓存是进程级的，其他测试线程若在没有订阅者时命中过 tower-http 的 span
  callsite，该 callsite 会被缓存为 `never`，而 `tracing::subscriber::set_global_default` 并不重建
  这个缓存（需显式调用 `tracing::callsite::rebuild_interest_cache()`）。
- 依赖约束：`tower` 必须是正式依赖（`buffer`/`limit`/`load-shed`）；`tower-http` 需包含
  `cors`、`trace`、`request-id`、`limit`、`timeout`、`set-header`、`compression-gzip`。
- 限流（`429`）与背压（`503`）触发时是批量拒绝，逐条告警无信息量：
  `src/middleware/error_response.rs` 不得为这两类拒绝打印逐条 `warn` 日志，
  每次请求（含被拒绝的请求）由 `TraceLayer` 的访问日志（INFO）覆盖；
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
  - `tests/config_tests.rs`：覆盖 `AppConfig::from_env` 的默认值、覆盖项与全部校验分支。
  - `tests/logging_tests.rs`：覆盖日志初始化与配置校验。
- 环境变量是进程级全局状态，依赖环境变量的测试必须通过共享 `Mutex` 串行执行，并在结束后恢复原值。
- 集成测试使用临时 SQLite 文件数据库，每个测试独立装配 `Container`，互不共享状态。
- 覆盖率检测命令：`cargo llvm-cov --summary-only`，要求行覆盖率 ≥ 80%。

### 10.2 jsonwebtoken 加密后端

- `jsonwebtoken` 从 v11 起默认不启用任何加密后端，直接调用签发/校验会在运行时 panic。
- `Cargo.toml` 中必须保持 `jsonwebtoken = { version = "=11", features = ["aws_lc_rs"] }`，与 rustls 使用的 `aws-lc-rs` 后端保持一致。
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
| 必须替换 | `.env` 的 `SEED_ADMIN_PASSWORD`（播种演示账号与新增用户的默认口令） | 换成强口令；该值不落迁移文件与种子 SQL |
| 必须替换 | `sys_config` 的 `sys.user.initPassword`（内置演示值） | 改成强口令或删除该行，强制新增用户显式传口令 |
| 必须替换 | `sys_user` 中 `admin` 的口令 | 播种口令公开可知，首次登录后立即修改 |
| 应清理 | `sys_user` 的 `linjh`、`heyj` 及 `sys_user_role` / `sys_user_post` 关联 | 演示账号 |
| 应清理 | `sys_dept` 的演示组织（含负责人姓名）、`sys_post` 的演示岗位 | 替换为真实组织与岗位 |
| 应清理 | `transfer_accounts` / `transfer_records` / `transfer_audits`（`acc_alice`、`acc_bob`） | 事务示例数据，生产应按业务替换或清空 |
| 应清理 | `sys_oper_log` / `sys_login_log` 的本地联调记录、Redis 的 `captcha:*` 与 `auth:token:*`、`log/` | 环境切换时清空 |
| 建议保留 | `sys_menu` 的目录 / 菜单 / 按钮记录 | 前端动态路由与权限码的来源，清空会导致页面不可用 |
| 建议保留 | `sys_role` 的内置 `super_admin` 角色及其菜单授权 | `AdminUser` 提取器对该角色直接放行 |
| 建议保留 | `sys_user` 的 `admin` 账号（改口令后保留） | 否则没有任何账号能登录后台 |
| 按需替换 | `sys_dict_type` / `sys_dict_data` | 业务下拉数据 |

- 演示接口（`examples`、`transactions` 与 `dev-login` / `dev-transfer`）在真实项目中应替换或删除；
  `dev-*` 只在 debug 构建注册，release 构建不含。
- 新增种子数据必须幂等（`INSERT OR IGNORE` 或先查后写），并同步更新 `README.md` 的生产清理清单。
