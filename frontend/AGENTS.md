# AGENTS.md（前端）

Guardian 前端的开发规范。取自上游 `scaffold-master` 的 `scaffold-vue` 模块，
页面、组件、路由与 composable 的业务逻辑保持上游写法，只在与后端配套处做最小改动。

## 目录职责

| 路径 | 职责 |
| --- | --- |
| `src/api` | 统一 HTTP、会话存储、认证、系统管理与生成器接口 |
| `src/router/index.ts` | 静态入口、动态菜单路由、登录守卫与会话失效跳转 |
| `src/composables` | 列表、字典 / 部门缓存、外观、动效、权限与确认框 |
| `src/components/ui` | 基础控件，不含业务语义 |
| `src/components/business` | 列表页、搜索表单、表单弹窗、权限按钮与业务选择器 |
| `src/components/layout` | 外壳、侧栏、顶栏、命令搜索与弹窗 |
| `src/views` | 页面，一个菜单对应一个页面 |
| `src/data` | 品牌常量与导航分组 |

## 硬性约定

- **接口只走 `src/api/http.ts`**：统一响应 `{code, message, data?, traceId, timestamp}`，
  错误文案只取 `message`；401 触发统一会话清理，页面不得自行处理令牌。
- **权限动作使用 `PermissionButton`**，页面不得自己解析权限码。
- **新列表复用 `ListPage` 与列表 composable，新表单复用 `FormDialog`**，不新造同类组件。
- **新页面注册流程**：在 `src/composables/auth.ts` 的 `COMPONENT_MAP` 注册，再到菜单管理里配置
  路径、组件标识与权限码。
- **退出与 401 只走统一会话清理**：不得在页面直接改认证存储或路由就绪标记；
  异步回调只能更新发起时仍有效的会话版本。
- **品牌与版本**：名称、简称、版权集中在 `src/data/index.ts`；版本号由 `vite.config.ts` 的
  `define` 从 `package.json` 的 `version` 注入（`APP_VERSION`），不得在页面手写版本字面量。
- **本地存储键统一以 `guardian.` 开头**（令牌、外观、视图偏好、引导状态等）。
- 样式沿用 `src/styles` 的六层结构与既有令牌，不新增平行体系。

## 提交前检查

```bash
npm ci
npm run test:unit
npm run build
```

- CI 在仓库根的 `.github/workflows/frontend.yml`，与上面三条命令一致（Node 24 + npm 缓存）。
- `npm run test:unit` 覆盖 `src/api/__tests__` 与 `src/utils/__tests__`（`node --test` 直跑 TS）。
- 改接口路径或响应结构时同步更新 `src/api/__tests__` 的契约断言。
- 需要真实浏览器验证时用 `scripts/session-regression.py`（要求前后端已启动、
  已安装 Python Playwright 与 Chromium；脚本自建自删夹具，不清空表）。
