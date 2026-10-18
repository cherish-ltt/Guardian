<div align="center">
<h1>Guardian</h1>
<p>基于 Rust 的生产级后台管理系统</p>
  <p>
  <a href="https://github.com/cherish-ltt/Guardian/blob/main/LICENSE-MIT">
    <img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="license"/>
  </a>
  <a href="https://www.rust-lang.org">
    <img src="https://img.shields.io/badge/rust-1.99.0+-orange.svg" alt="rust"/>
  </a>
  <a href="https://github.com/cherish-ltt/Guardian">
    <img src="https://img.shields.io/badge/version-v0.4.0-blue.svg" alt="version"/>
  </a>
  <a href="https://github.com/cherish-ltt/Guardian">
    <img src="https://img.shields.io/badge/status-stable-green.svg" alt="status"/>
  </a>
  </p>
</div>

## 项目简介

Guardian 是一个功能完整的后台管理系统，后端基于 `axum + sea-orm`（SQLite + Redis），
前端基于 `Vue 3 + TypeScript + Vite`。包含认证、菜单与按钮权限、数据权限、组织管理、岗位、
字典、参数、操作与登录日志、工作台和代码生成，新增项目可直接在此基础上实现业务模块。

架构骨架取自 [AxumServiceScaffold](https://github.com/cherish-ltt/AxumServiceScaffold)，
前端取自 [scaffold-master](https://github.com/wbr664393425-lgtm/scaffold-master) 的 `scaffold-vue` 模块，
详见下方「来源与致谢」。

## 核心特性

- **登录会话**：图形验证码（一次有效）、JWT + Redis 会话令牌双重校验、登出即撤销、修改密码即撤销该账号全部会话。
- **个人资料与 2FA**：本人资料、改密、可选认证器动态码；一次性恢复码与独立权限管理员重置。绑定前须配置 `TWO_FACTOR_KEY`，见 [部署说明](docs/DEPLOYMENT.md#二次认证密钥与恢复)。
- **基础数据缓存**：字典、参数、部门和管理菜单使用 Redis → SQLite；单实例修改后立即可见，空值短缓存、TTL 抖动及有界回源，不缓存用户权限和统计。
- **系统管理**：用户 / 角色 / 菜单 / 部门 / 岗位 / 字典 / 参数配置的增删改查、树形结构、下拉选项、
  CSV 导出与权限关系图；菜单与按钮权限码驱动前端动态路由。
- **数据权限**：`ALL` / `DEPT_AND_CHILD` / `DEPT` / `CUSTOM` / 仅本人，用户列表、导出与权限图统一受控。
- **系统监控**：写操作留痕（请求体脱敏、失败留痕、401/403 不留痕）、操作与登录日志的列表、
  统计、时间线、热力图、导出、删除与清空；两份日志都记录真实来源 IP（按可信代理链解析）、
  地点与浏览器 / 操作系统。
- **工作台**：统计卡、登录趋势、近 7 天操作量、近期动态与角色分布。
- **代码生成**：候选表、导入、字段配置、代码预览、生成与 ZIP 下载。
- **工程化**：DDD + 洋葱架构分层、统一响应与 `traceId`、完整中间件栈（限流 / 背压 / 超时 / Body 上限 /
  压缩 / CORS 白名单 / 安全响应头）、日志分批落盘、事务示例、健康检查与 Swagger（`docs` feature）。

## 目录结构

```
Guardian/
├── backend/    # axum + sea-orm 后端（洋葱架构，SQLite + Redis）
└── frontend/   # Vue 3 + TypeScript 前端（Vite，代理 /api/v1 到后端）
```

两个子项目各自独立安装依赖与启动，前端通过 Vite 代理把 `/api/v1/*` 转发到后端 `127.0.0.1:8080`。

## 快速开始

前置：Rust `1.99.0`、Node.js（`npm`）、Redis。

```bash
# 1. 后端：准备环境变量（默认 SQLite + 本地 Redis）
cd backend
cp .env-example .env          # 按需填写 REDIS_URL / REDIS_PASSWORD / SEED_ADMIN_PASSWORD
cargo run                      # http://127.0.0.1:8080

# 2. 前端：安装依赖并启动
cd ../frontend
npm ci
npm run dev                    # http://localhost:5173，/api/v1 代理到后端
```

首次启动会自动执行迁移并幂等播种演示数据（组织、岗位、角色、菜单、字典、参数与三个演示账号），
演示账号与口令取自 `.env`：`SEED_USERS` 决定播种哪些账号，`SEED_ADMIN_PASSWORD` 是它们的初始口令
（模板默认 `admin` / `admin123456`）。

> 生产环境部署前必须替换示例密钥、默认口令并清理演示数据，且要按部署形态配置可信代理
> （否则日志里的来源 IP 与按 IP 的额度都不可信）。完整说明见 [生产部署](docs/DEPLOYMENT.md)，
> 数据清理清单见 [backend/README.md](backend/README.md#生产环境注意事项) 与 [backend/AGENTS.md](backend/AGENTS.md)。

## 验证命令

```bash
# 后端（与 CI、AGENTS.md、docs/TESTING.md 一致；当前 default 为空，
# 唯一可选 feature 是 docs，所以 --features docs 就是全量构建）
cd backend
cargo fmt --all -- --check
cargo clippy --all-targets --features docs -- -D warnings
cargo test --features docs

# 前端
cd frontend
npm ci
npm run test:unit
npm run build
```

## 来源与致谢

| 部分 | 上游项目 | 关系 |
| --- | --- | --- |
| `backend/` | [cherish-ltt/AxumServiceScaffold](https://github.com/cherish-ltt/AxumServiceScaffold) | 沿用其 DDD + 洋葱架构骨架与中间件栈，在其上实现完整后台管理模块 |
| `frontend/` | [wbr664393425-lgtm/scaffold-master](https://github.com/wbr664393425-lgtm/scaffold-master) | 取自其 `scaffold-vue` 模块（Vue 3 + TS + 珍珠玻璃主题），改用本仓库的 axum 后端 |

后端把上游 Spring Boot 版本的后台接口用 Rust 重写，接口路径、字段、错误文案与权限码逐条对齐，
数据库由 MySQL 换成 SQLite，登录会话与验证码改存 Redis。

## 文档导航

- 生产部署（真实客户端 IP、反向代理 / CDN、上线清单）：[docs/DEPLOYMENT.md](docs/DEPLOYMENT.md)
- 各版本发布说明（前后端分列）：[docs/version](docs/version)
- 后端总览与运行说明：[backend/README.md](backend/README.md)
- 后台模块与接口契约：[backend/docs/ADMIN.md](backend/docs/ADMIN.md)
- 架构、中间件、日志、事务、响应结构、测试：[backend/docs](backend/docs)
- 前端目录与扩展约定：[frontend/README.md](frontend/README.md)
- 开发规范：[AGENTS.md](AGENTS.md)

## 许可证

本仓库采用 [MIT OR Apache-2.0](https://opensource.org/licenses) 双许可，使用者可任选其一遵循：

- MIT 许可证全文见 [LICENSE-MIT](LICENSE-MIT)
- Apache-2.0 许可证全文见 [LICENSE-APACHE](LICENSE-APACHE)

`frontend/` 派生自 `scaffold-master` 的 `scaffold-vue` 模块，沿用上游 MIT 许可并保留其版权声明，
见 [frontend/LICENSE](frontend/LICENSE)。

------

<div align="center">
  <p>Built with ❤️ by the Guardian team</p>
</div>
