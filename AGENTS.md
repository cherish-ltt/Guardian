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
  - `.github/workflows/backend.yml`：后端格式、Clippy、构建与测试（GitHub 只识别仓库根目录下的
    `.github/workflows/`）。
  - `.pre-commit-config.yaml`：文件检查、密钥泄漏检测、ast-grep 规则与 Rust 钩子。
- 生产部署前必须处理后端 README 的「生产环境注意事项」与 `backend/AGENTS.md` 第 10.11 节列出的
  演示数据与默认口令。
- 许可证：本仓库采用 `MIT OR Apache-2.0` 双许可；`frontend/` 派生自上游 `scaffold-vue`，
  沿用其 MIT 许可并保留版权声明（见 [frontend/LICENSE](frontend/LICENSE)）。
