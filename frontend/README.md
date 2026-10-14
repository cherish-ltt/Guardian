# Guardian Frontend

Guardian 后台管理系统的前端：Vue 3 + TypeScript + Vite + Vue Router + GSAP，珍珠玻璃主题与自有组件库，
对接本仓库的 axum 后端（[../backend](../backend)）。

前端来自上游项目 [wbr664393425-lgtm/scaffold-master](https://github.com/wbr664393425-lgtm/scaffold-master)
的 `scaffold-vue` 模块，沿用其 MIT 许可（见 [LICENSE](LICENSE)）。相对上游只做了与后端配套的改动：
默认 API 前缀由 `/api` 改为 `/api/v1`、接口契约测试常量同步调整、
`localStorage` 键前缀由 `scaffold.` 改为 `guardian.`，页面、组件、路由与 composable 的业务逻辑未改动。

## 开发

```bash
npm ci
npm run dev
npm run test:unit
npm run build
```

默认地址 `http://localhost:5173`，`/api/v1` 代理到 `http://localhost:8080`，可用 `VITE_API_PROXY_TARGET`
指定其他本地后端。后端准备（SQLite 与 Redis、默认账号）与生产环境清理清单见
[后端说明](../backend/README.md)，接口清单见 [后台管理模块](../backend/docs/ADMIN.md)。

前端请求的默认前缀是 `/api/v1`（`src/api/http.ts`），可用 `VITE_API_BASE_URL` 覆盖。

默认管理员为 `admin` / `admin123456`（口令由后端 `.env` 的 `SEED_ADMIN_PASSWORD` 播种），
验证码必须使用当前页面显示的字符。非管理员菜单来自角色授权；
无菜单账号显示明确的权限分配提示。

## 目录

| 路径 | 中文职责 |
| --- | --- |
| `src/api` | 统一 HTTP、会话存储、认证、系统管理与生成器接口 |
| `src/router/index.ts` | 静态入口、动态菜单路由、登录守卫与会话失效跳转 |
| `src/composables/auth.ts` | 当前用户、权限、动态菜单与会话版本 |
| `src/components/ui` | 按钮、输入、下拉、表格、弹窗、抽屉、提示等基础控件 |
| `src/components/business` | ListPage、SearchForm、FormDialog、PermissionButton 及字典、部门、用户选择 |
| `src/components/layout` | 外壳、侧栏、顶栏、命令搜索、外观与密码弹窗 |
| `src/composables` | 列表、字典 / 部门缓存、外观、动效、焦点与确认框 |
| `src/styles` | 令牌、基础、材质、控件、外壳、页面六层样式 |
| `src/views` | 登录、工作台、10 个管理页、无权限与未匹配页面 |
| `scripts/session-regression.py` | 真实浏览器会话、路由竞态与低权限回归 |

## 扩展约定

新列表先复用 `ListPage` 与列表 composable，新表单先复用 `FormDialog`，权限动作使用 `PermissionButton`；
接口复用 `api/http.ts`。在 `auth.ts` 的 `COMPONENT_MAP` 注册页面，再通过菜单管理配置路径、组件标识与权限码。

退出和 401 均通过统一会话清理撤销动态路由与缓存；不得在页面直接修改认证存储或路由就绪标记。
旧请求只能更新发起时仍有效的会话。

## 后端契约要点

- 统一响应 `{code, message, data?, traceId, timestamp}`：`code` 与 HTTP 状态码一致，
  `http.ts` 只认 `message` 作为错误文案。
- 分页返回 `{pageNum, pageSize, total, rows}`；列表页的导出走 CSV（`text/csv`），
  代码生成下载走 ZIP。
- 401 触发统一会话清理并跳登录；403 表示权限不足，页面按权限码隐藏入口。

## 浏览器回归

已有 Python Playwright 与 Chromium、前后端均启动时，可运行：

```bash
python3 scripts/session-regression.py
```

建议连接独立本地校验库；脚本创建并删除自身的角色 / 用户夹具，不清空任何表。
管理员密码可用 `GUARDIAN_TEST_PASSWORD` 覆盖；脚本需要 Vite 开发服务器来访问应用实际加载的 HTTP 模块。

## 许可与上游

- 本目录代码派生自 `scaffold-master` 的 `scaffold-vue`，沿用上游 MIT 许可并保留版权声明，
  见 [LICENSE](LICENSE)；本仓库整体采用 `MIT OR Apache-2.0` 双许可，见 [根目录说明](../README.md)。
- GSAP 依赖遵循其 [Standard "No Charge" License](https://gsap.com/community/standard-license/)。
- 主题、密度与减少动效规则来自上游 `DESIGN.md`，前后端链路见上游 `PROJECT_MAP.md`。
