# AGENTS.md（前端）

Guardian 前端的开发规范。取自上游 `scaffold-master` 的 `scaffold-vue` 模块，
页面、组件、路由与 composable 的业务逻辑保持上游写法，只在与后端配套处做最小改动。

## 目录职责

| 路径 | 职责 |
| --- | --- |
| `src/api` | 统一 HTTP、会话存储、认证、系统管理与生成器接口 |
| `src/router/index.ts` | 静态入口、动态菜单路由、登录守卫与会话失效跳转 |
| `src/composables` | 列表、字典 / 部门缓存、外观、动效、权限与确认框 |
| `src/components/ui` | 基础控件，不含业务语义；统一从 `index.ts` 引入 |
| `src/components/business` | 列表页、搜索表单、表单弹窗、权限按钮与业务选择器；统一从 `index.ts` 引入 |
| `src/components/layout` | 外壳、侧栏、顶栏、命令搜索与弹窗 |
| `src/views` | 页面，一个菜单对应一个页面 |
| `src/data` | 品牌常量与导航分组 |
| `src/components/GithubLink.vue` | 顶栏的 GitHub 仓库入口，地址取自 `APP.repo` |

## 硬性约定

- **接口只走 `src/api/http.ts`**：统一响应 `{code, message, data?, traceId, timestamp}`，
  错误文案只取 `message`；401 触发统一会话清理，页面不得自行处理令牌。
- **权限动作使用 `PermissionButton`**，页面不得自己解析权限码。
- **新列表复用 `ListPage` 与列表 composable，新表单复用 `FormDialog`**，不新造同类组件。
- **组件一律从目录的 `index.ts` 引入**：`@/components/ui` 与 `@/components/business` 的 `index.ts`
  必须导出目录下全部 `.vue`，页面不得按 `@/components/business/Xxx.vue` 直引；
  新增组件时同一次改动里补导出。
- **新页面注册流程**：在 `src/composables/auth.ts` 的 `COMPONENT_MAP` 注册，再到菜单管理里配置
  路径、组件标识与权限码。
- **退出与 401 只走统一会话清理**：不得在页面直接改认证存储或路由就绪标记；
  异步回调只能更新发起时仍有效的会话版本。
- **品牌与版本**：名称、简称、版权集中在 `src/data/index.ts`；版本号由 `vite.config.ts` 的
  `define` 从 `package.json` 的 `version` 注入（`APP_VERSION`），不得在页面手写版本字面量。
- **仓库入口只有一个**：`src/components/GithubLink.vue`，地址读 `APP.repo`，置空即不渲染；
  不得在页面里再散写仓库地址。
- **本地存储键统一以 `guardian.` 开头**（令牌、外观、视图偏好、引导状态等）。
- 样式沿用 `src/styles` 的六层结构与既有令牌，不新增平行体系。

## 提交前检查

```bash
npm ci
npm run test:unit
npm run build
```

- CI 在仓库根的 `.github/workflows/frontend.yml`，与上面三条命令一致（Node 24 + npm 缓存）。
- `npm run test:unit` 覆盖 `src/api/__tests__`、`src/utils/__tests__` 与
  `src/components/__tests__`（`node --test` 直跑 TS）。
- 组件测试不引 vitest / jsdom：`src/components/__tests__/support.ts` 用 `@vue/compiler-sfc`
  编译 SFC、esbuild 去 TS 语法、`@vue/server-renderer` 输出 HTML 断言，三者都已在依赖树里。
  被测组件不能 import 其他 SFC（`@/` 别名与 `.vue` 导入在 Node 里无法解析），
  因此断言只落在纯展示控件上；带子组件或需要交互的页面用 `scripts/session-regression.py` 验证。
- 改接口路径或响应结构时同步更新 `src/api/__tests__` 的契约断言。
- 需要真实浏览器验证时用 `scripts/session-regression.py`（要求前后端已启动、
  已安装 Python Playwright 与 Chromium；脚本自建自删夹具，不清空表）。
