# Guardian

基于 `axum + sea-orm` 与 `Vue 3` 的后台管理系统：登录会话、系统管理、监控、工作台与代码生成。

## 来源与致谢

本仓库的两部分是上游项目的延续与改写，功能对照、术语与接口契约都以上游为准：

| 部分 | 上游项目 | 关系 |
| --- | --- | --- |
| `backend/` | [cherish-ltt/AxumServiceScaffold](https://github.com/cherish-ltt/AxumServiceScaffold) | 沿用其 DDD + 洋葱架构骨架与中间件栈，在其上实现完整后台管理模块 |
| `frontend/` | [wbr664393425-lgtm/scaffold-master](https://github.com/wbr664393425-lgtm/scaffold-master) | 取自其 `scaffold-vue` 模块（Vue 3 + TS + 珍珠玻璃主题），改用本仓库的 axum 后端 |

后端把上游 Spring Boot 版本的后台接口用 Rust 重写，接口路径、字段、错误文案与权限码逐条对齐，
数据库由 MySQL 换成 SQLite，登录会话与验证码改存 Redis。

## 目录结构

```
Guardian/
├── backend/    # axum + sea-orm 后端（洋葱架构，SQLite + Redis）
└── frontend/   # Vue 3 + TypeScript 前端（Vite，代理 /api/v1 到后端）
```

两个子项目各自独立安装依赖与启动，前端通过 Vite 代理把 `/api/v1/*` 转发到后端 `127.0.0.1:8080`。

## 功能范围

| 模块 | 说明 |
| --- | --- |
| 登录会话 | 图形验证码、登录、登出（会话撤销）、当前账号、动态菜单路由、修改密码 |
| 系统管理 | 用户 / 角色 / 菜单 / 部门 / 岗位 / 字典 / 参数配置的增删改查、树形结构、下拉选项、CSV 导出、权限关系图 |
| 系统监控 | 操作日志与登录日志（列表、统计、时间线、热力图、导出、删除、清空）与写操作留痕 |
| 工作台 | 统计卡、登录趋势、近 7 天操作量、近期动态、角色分布 |
| 代码生成 | 可导入表、导入、字段配置、代码预览、生成、ZIP 下载 |
| 基础能力 | 统一响应与 `traceId`、JWT 鉴权、中间件栈、日志分批落盘、事务示例、健康检查、Swagger（`docs` feature） |

接口清单、权限与数据范围、留痕规则见 [backend/docs/ADMIN.md](backend/docs/ADMIN.md)。

## 快速开始

前置：Rust `1.99.0`、Node.js（`npm`）、Redis。

```bash
# 1. 后端：准备环境变量（默认 SQLite + 本地 Redis）
cd backend
cp .env-example .env          # 按需填写 REDIS_URL / REDIS_PASSWORD
cargo run                      # http://127.0.0.1:8080

# 2. 前端：安装依赖并启动
cd ../frontend
npm ci
npm run dev                    # http://localhost:5173，/api/v1 代理到后端
```

首次启动会自动执行迁移并幂等播种演示数据，默认账号 `admin` / `admin123456`。

> 生产环境部署前必须清理演示数据并替换示例密钥，清单见
> [backend/README.md](backend/README.md#生产环境注意事项) 与 [backend/AGENTS.md](backend/AGENTS.md)。

## 验证命令

```bash
# 后端
cd backend
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features

# 前端
cd frontend
npm ci
npm run test:unit
npm run build
```

## 文档导航

- 后端总览与运行说明：[backend/README.md](backend/README.md)
- 后台模块与接口契约：[backend/docs/ADMIN.md](backend/docs/ADMIN.md)
- 架构、中间件、日志、事务、响应结构、测试：[backend/docs](backend/docs)
- 前端目录与扩展约定：[frontend/README.md](frontend/README.md)

## 许可证

本仓库采用 `MIT OR Apache-2.0` 双许可，使用者可任选其一遵循：

- MIT 许可证全文见 [LICENSE-MIT](LICENSE-MIT)
- Apache-2.0 许可证全文见 [LICENSE-APACHE](LICENSE-APACHE)

`frontend/` 派生自 `scaffold-master` 的 `scaffold-vue` 模块，沿用上游 MIT 许可并保留其版权声明，
见 [frontend/LICENSE](frontend/LICENSE)；后端的同款许可文件见 [backend](backend) 目录。

---

<div align="center">
  <sub>Guardian · axum + Vue 后台管理系统</sub>
</div>
