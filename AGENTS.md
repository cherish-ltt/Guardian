# AGENTS.md

本仓库由两部分组成，各自的开发规范分别维护在子目录中，本文件只作为入口：

| 部分 | 规范文件 | 内容 |
| --- | --- | --- |
| `backend/`（axum + sea-orm） | [backend/AGENTS.md](backend/AGENTS.md) | Git 提交规范、Rust CI 标准、依赖与版本锁定、DDD + 洋葱架构约束、统一响应与事务约定、后台管理模块约定、生产数据清理清单 |
| `frontend/`（Vue 3 + TypeScript） | [frontend/AGENTS.md](frontend/AGENTS.md) | 目录职责、组件与 composable 复用约定、会话与权限处理、样式与动效规范、构建与测试要求 |

## 仓库级约定

- 提交信息使用 Conventional Commits（`feat` / `fix` / `docs` / `chore` / `test` / `refactor`），
  按「一个提交一件事」分批提交，不要把前后端或前后两次任务的改动混在一起。
- 改动只能落在本仓库内，不要修改上游参考实现（`AxumServiceScaffold`、`scaffold-master`）的目录。
- 后端接口契约以后台管理模块文档 [backend/docs/ADMIN.md](backend/docs/ADMIN.md) 为准；
  与参考实现的差异必须记录在该文档的「与 Spring 版的差异」一节。
- 仓库根目录的 CI 与 Git 钩子：
  - `.github/workflows/backend.yml`：后端格式、Clippy、构建与测试。
  - `.github/workflows/frontend.yml`：前端依赖安装、单元测试与类型检查 / 构建。
  - `.github/workflows/release.yml`：推送 `v*.*.*` tag 时构建前后端产物并发布 GitHub Release。
    后端二进制按「架构 × libc」出四个包：`x86_64-linux-gnu`、`aarch64-linux-gnu`、
    `x86_64-linux-musl`、`aarch64-linux-musl`（gnu 动态链接 glibc，musl 静态链接）；
    四个目标都在对应架构的原生 runner 上编译（x86_64 用 `ubuntu-24.04`，aarch64 用
    `ubuntu-24.04-arm`），musl 目标的 C 工具链来自 apt 的 `musl-tools`，不引入外部下载；
    前端产物与架构无关只构建一份；下载页另附本 tag 的 `.env-example`
    （文件名带版本，如 `env-example-v0.5.0`）。
  - 工作流都必须放在仓库根目录（GitHub 只识别根下的 `.github/workflows/`），
    用 `working-directory` 指向 `backend/` 与 `frontend/`。
  - `.pre-commit-config.yaml`：文件检查、密钥泄漏检测、ast-grep 规则与 Rust 钩子。
- 版本号前后端必须同时递增：`backend/Cargo.toml` 与 `frontend/package.json`（连同
  `frontend/package-lock.json`）一起改到同一个 `X.Y.Z`，**即使本次只有一端有改动**也要一起改，
  不允许两端版本号出现分叉。
- 版本发布说明统一放在仓库根 [docs/version](docs/version) 下的 `vX.Y.Z.md`
  （如 [docs/version/v0.4.1.md](docs/version/v0.4.1.md)）：
  - 内容按「后端」「前端」两节分别罗列本次变动，没有改动的部分保留小节标题并写「无」；
  - `release.yml` 会读 `docs/version/<tag>.md` 作为 Release 正文，文件缺失时发布失败；
  - 新增版本时必须先补该文件再打 tag，并同步 [backend/docs/CHANGELOG.md](backend/docs/CHANGELOG.md)。
- [docs/version/UNRELEASE.md](docs/version/UNRELEASE.md) 累积下一次发布要带上的变更：
  - 日常提交如果改动了对外行为、配置项或部署方式，在同一提交里往这个文件补一条，
    小节结构与正式发布说明一致（按「后端」「前端」分节，无改动的写「无」）；
  - **发版时必须从这里取出更新记录**：新增 `vX.Y.Z.md` 时把 `UNRELEASE.md` 的内容并入，
    作为发布说明与 Release 正文的一部分，而不是只按本 tag 的提交历史另写一遍；
  - 并完后清空 `UNRELEASE.md`（保留标题、说明与两个小节标题），下一轮变更重新累积。
- 生产部署前必须处理后端 README 的「生产环境注意事项」与 `backend/AGENTS.md` 第 10.11 节列出的
  演示数据与默认口令，并按 [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) 配置可信代理与网关。
- 部署说明统一放在仓库根 [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md)，由根 README 的「文档导航」
  链接；代理 / CDN 下的真实客户端 IP 获取与验证方法以该文档为准，其他文档只引用不复制。
- 许可证：本仓库采用 `MIT OR Apache-2.0` 双许可；`frontend/` 派生自上游 `scaffold-vue`，
  沿用其 MIT 许可并保留版权声明（见 [frontend/LICENSE](frontend/LICENSE)）。
