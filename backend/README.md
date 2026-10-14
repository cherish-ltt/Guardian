<div align="center">
  <h1>Guardian Backend</h1>
  <p>axum + sea-orm · DDD 洋葱架构 · SQLite + Redis</p>
</div>

Guardian 的后端服务：提供登录会话（验证码 + JWT + Redis 撤销）、系统管理（用户 / 角色 / 菜单 / 部门 /
岗位 / 字典 / 参数）、操作与登录日志、工作台聚合与代码生成，配套前端见 [../frontend](../frontend)。

代码模板来自 [cherish-ltt/AxumServiceScaffold](https://github.com/cherish-ltt/AxumServiceScaffold)：
分层结构、中间件栈、统一响应、日志与事务实现沿用该骨架，业务模块（后台管理）在本仓库实现。
接口契约对齐 `scaffold-master` 的 Spring Boot 版本，逐条差异见 [后台管理模块](docs/ADMIN.md)。

## 目录结构

```
src/
├── api/               协议适配层：controllers、dto、extractors
├── services/          应用服务层：用例编排、事务边界
├── domain/            领域层（内核）：模型、用例接口 trait、领域错误
├── infrastructure/    基础设施层：配置、数据库、仓储适配器、AuthStore 实现
├── middleware/        中间件栈、错误响应、trace、审计留痕
├── entities/          sea-orm 实体
├── util/              CSV、口令哈希、时间格式化
├── container.rs       依赖装配中心
└── main.rs            启动入口
migrations/            sqlx 版本化迁移（0001 转账示例表、0002 系统表 17 张）
tests/                 单元测试与 oneshot 黑盒契约测试
docs/                  架构、中间件、日志、事务、响应、测试、后台模块文档
```

## 技术栈

| 关注点 | 选型 |
| --- | --- |
| Web 框架 | `axum`（`tower-http` 中间件、`tower::ServiceExt::oneshot` 测试） |
| 异步运行时 | `tokio` |
| ORM 与迁移 | `sea-orm` + `sqlx`（SQLite，`sqlx::migrate!` 执行 `migrations/`） |
| 鉴权 | `jsonwebtoken`（`aws_lc_rs` 后端）、`argon2` 口令哈希 |
| 会话与验证码 | `redis`（connection-manager 自动重连） |
| 接口文档 | `utoipa` + `swagger-ui`，由可选 feature `docs` 门控（不进 release） |
| 序列化与日志 | `serde` / `serde_json`、`tracing` + `tracing-subscriber` |
| 其它 | `uuid`（traceId 与主键）、`zip`（代码生成下载）、`csv`、`anyhow`、`thiserror` |

依赖版本在 `Cargo.toml` 中以 `=` 精确锁定；`preset` / `http-client` 两个可选 feature 提供
`rayon` / `reqwest` / `validator` 等预置库，默认构建不编译。

## 分层约束

依赖方向 `api -> services -> domain <- infrastructure`，由 `container.rs` 注入实现：

- `domain`：不依赖 axum、sea-orm、JWT 等外部框架；放领域模型、用例接口 trait 与领域错误。
- `services`：只面向领域接口写用例，事务边界只在这一层（`begin` / `commit` / `rollback`）。
- `api`：只做协议适配（提取参数、调用例、包响应），不写业务判断。
- `infrastructure`：可替换的外部实现（配置、数据库、仓储适配器、Redis 会话存储）。

仓储通过 trait 接口 + 无状态适配器结构体解耦，泛型参数注入 `services`；对外用例接口保持
`dyn` 兼容，由 `container.rs` 统一装配。后台管理模块按页面拆分为 8 个用例 trait 加 1 个权限图 trait，
合并成 `AdminUseCase` 视图。分层细节见 [架构设计](docs/ARCHITECTURE.md)。

## 中间件与请求链路

`src/middleware/stack.rs` 按固定顺序组装（外到内）：

请求 ID / `Trace` 日志 → 安全响应头 → CORS 白名单 → 限流 → 并发限制与背压 → 超时 →
请求体上限 → 压缩（`CompressionLayer`），最后是 `traceId` 注入与审计留痕。

- **统一响应**：`{code, message, data?, traceId, timestamp}`，`code` 与 HTTP 状态码派生自同一真值，
  见 [统一响应结构](docs/RESPONSE.md)。
- **traceId**：优先沿用请求头 `x-trace-id`，缺失时生成 32 位十六进制，同时写入响应头；
  CSV / ZIP 等非 JSON 响应只加响应头。
- **审计留痕**：按「请求方法 + 路径段模式」匹配 31 条规则，记录模块、动作、操作人、耗时与
  请求体（敏感字段脱敏），写入失败不影响业务响应。
- **容量参数**：全部集中在 `.env`（超时、Body 上限、并发、背压队列、限流额度、CORS 白名单），
  调参见 [中间件栈与配置](docs/MIDDLEWARE.md)。
- **日志**：按天滚动文件 + 分批落盘 + 启动配置快照，见 [日志](docs/LOGGING.md)。

## 数据库与迁移

只支持 SQLite：迁移文件与老库补列 / 建索引逻辑（`pragma_table_info`）都是 SQLite 语法，
`DATABASE_URL` 不以 `sqlite:` 开头会在启动时直接拒绝。数据库文件不入库，首次启动自动完成
建表 → 补列建索引 → 幂等播种；要回到干净状态删除 `guardian.db` 再启动即可。

时间字段统一存 `TEXT`（`YYYY-MM-DD HH:MM:SS` 本地时间，`src/util/time.rs`），比较用字典序，
SQL 侧用 `strftime` / `julianday` / `substr` 等价 MySQL 的 `NOW()` / `CURDATE()` / `DATE_FORMAT()`。

## 会话与验证码

`AuthStore` 抽象有两个实现：`RedisAuthStore`（生产，键前缀 `captcha:` 与 `auth:token:`，TTL 300s）
与 `MemoryAuthStore`（测试）。登录要求 JWT 与 Redis 会话令牌同时有效，登出即删除令牌、
旧令牌立刻失效。Redis 连接在启动时建立（`connection-manager` 自动重连），不可用会直接启动失败。

## 快速开始

### 1. 准备环境变量

```bash
cp .env-example .env
```

默认模板使用 SQLite 与本地 Redis：

```env
DATABASE_URL=sqlite://guardian.db?mode=rwc
REDIS_URL=redis://127.0.0.1:6379
# 留空表示未设置密码
REDIS_PASSWORD=
```

### 2. 启动服务

```bash
cargo run
```

默认监听 `http://127.0.0.1:8080`；启用 `docs` feature（开发时 `cargo run --features docs`）可访问
`http://127.0.0.1:8080/swagger-ui` 与 `/api-doc/openapi.json`。收到 Ctrl-C 或 SIGTERM 后停止接收新请求并优雅关闭。

健康检查语义：

- `/api/v1/system/health`：进程存活检查，固定 `200` 且 `database_status=not_checked`，不查数据库；
  它只证明进程能响应 HTTP，不代表依赖已就绪。
- `/api/v1/system/ready`：数据库就绪检查，数据库不可用时返回 `503`。
- `/api/v1/health`：带 `status` / `time` / `serviceName` 的概览检查。

### 3. 默认账号

首次启动播种的演示管理员为 `admin` / `admin123456`（口令哈希在播种时用 argon2 现场计算）。
种子数据清单与生产环境的清理项见下一节。

## 生产环境注意事项

`.env-example` 与启动播种都是为「本地演示可开箱运行」准备的，上线前必须处理下列内容。

必须替换或清理：

| 位置 | 内容 | 处理 |
| --- | --- | --- |
| `.env` | `JWT_SECRET=change-me-...` | 换成足够长的随机串；`APP_ENV=production` |
| `.env` | `DATABASE_URL` 指向本地 `guardian.db` | 指向部署目录下的库文件，或删除该文件后重新初始化 |
| `sys_config` | `sys.user.initPassword`（内置演示值 `Yy@123456`） | 改成强口令，或删除该行让新增用户必须显式传口令 |
| `sys_user` | 演示账号 `chensy`、`wumin` | 删除（连同 `sys_user_role` / `sys_user_post` 关联） |
| `sys_user` | `admin` 的口令 | 首次登录后立即修改（播种口令是公开的） |
| `sys_dept` / `sys_post` | 演示组织（云原科技集团等 11 个部门）与 9 个岗位 | 替换为真实组织与岗位 |
| `transfer_accounts` / `transfer_records` / `transfer_audits` | 演示账户 `acc_alice`、`acc_bob`（各 100000 分）与流水 | 清空或按业务替换；`migrations/0001` 是事务示例 |
| `sys_oper_log` / `sys_login_log` | 本地联调产生的日志 | 上线前清空 |
| Redis | `captcha:*`、`auth:token:*` | 切换环境时清空，避免旧会话残留 |
| `log/` | 本地运行日志 | 不入库，部署时按需清理 |

建议保留的默认数据（清掉会导致页面无法正常使用）：

- `sys_menu` 的 38 条目录 / 菜单 / 按钮记录：前端动态路由与按钮权限码都来自这里；
  若要自定义菜单，请在此基础上增删，不要整表清空。
- `sys_role` 中的内置超级管理员角色（`super_admin`）及其菜单授权：`AdminUser` 提取器对
  `super_admin` 直接放行，该角色被删除后需要自行保证权限闭环。
- `sys_user` 的 `admin` 账号（改口令后保留），否则没有任何账号能登录后台。
- `sys_dict_type` / `sys_dict_data`：业务下拉数据，按需替换为真实字典。

另外，示例模块（`examples`）与转账示例接口（`transactions`，含 `dev-*` 调试入口）
在真实项目中应当替换或删除：`dev-login` / `dev-transfer` 只在 debug 构建注册，release 构建不存在。

## 接口清单

基础能力接口：

- `GET /`
- `GET /api/v1/system/health`、`GET /api/v1/system/ready`、`GET /api/v1/health`
- `POST /api/v1/auth/dev-login`（仅调试构建）、`GET /api/v1/auth/me`
- `POST /api/v1/examples/echo`、`GET /api/v1/examples`、`GET /api/v1/examples/{id}`
- `POST /api/v1/transactions/dev-transfer`（仅调试构建）、`GET /api/v1/transactions`、`GET /api/v1/transactions/{id}`

后台管理接口（`/api/v1/auth`、`/api/v1/system`、`/api/v1/monitor`、`/api/v1/dashboard`、`/api/v1/tool/generator`）
见 [后台管理模块](docs/ADMIN.md)。

## 文档导航

- [后台管理模块](docs/ADMIN.md)：接口清单、权限与数据范围、留痕、存储映射、与 Spring 版的差异
- [架构设计](docs/ARCHITECTURE.md)：洋葱架构、目录结构、每层职责
- [中间件栈与配置](docs/MIDDLEWARE.md)：执行顺序、容量参数、压测调参
- [事务示例](docs/TRANSACTION.md)：转账参考实现、回滚与幂等验证
- [统一响应结构](docs/RESPONSE.md)：响应体约定与 `traceId`
- [日志](docs/LOGGING.md)：分批落盘、启动配置日志
- [测试与构建检查](docs/TESTING.md)：构建检查命令、黑盒测试约定
- [扩展开发](docs/DEVELOPMENT.md)：新增模块步骤、内置工具、预置依赖
- [更新日志](docs/CHANGELOG.md)

## 贡献指南

提交代码前请先阅读 [AGENTS.md](AGENTS.md)，它是本项目的开发规范，包含 Git 提交规范、Rust CI 标准、
依赖与版本锁定要求、DDD + 洋葱架构约束、生产数据清理约定。

仓库使用 `prek`（Rust 编写的 Git hook 管理器）管理提交前检查，配置在仓库根目录的 `.pre-commit-config.yaml`
（Rust 钩子通过 `--manifest-path backend/Cargo.toml` 作用于本 crate）：

```bash
prek init   # 安装钩子，之后每次 git commit 自动运行格式、lint 与测试检查
```

所有变更必须通过以下检查后才能提交：

```bash
prek run --all-files
cargo fmt --all -- --check
cargo clippy --all-targets --features docs -- -D warnings
cargo build
cargo test --features docs
```

单元测试覆盖率使用 `cargo llvm-cov` 检测，要求不低于 80%。

## 许可证

本项目采用 [MIT OR Apache-2.0](https://opensource.org/licenses) 双许可，使用者可任选其一遵循：

- MIT 许可证全文见 [LICENSE-MIT](LICENSE-MIT)
- Apache-2.0 许可证全文见 [LICENSE-APACHE](LICENSE-APACHE)
