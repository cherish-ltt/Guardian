# Guardian Frontend

Guardian 后台管理系统的前端：Vue 3 + TypeScript + Vite + Vue Router + GSAP，珍珠玻璃主题与自有组件库，
对接本仓库的 axum 后端（[../backend](../backend)）。

前端来自上游项目 [wbr664393425-lgtm/scaffold-master](https://github.com/wbr664393425-lgtm/scaffold-master)
的 `scaffold-vue` 模块，沿用其 MIT 许可（见 [LICENSE](LICENSE)）。相对上游只做了与后端配套的改动：
默认 API 前缀由 `/api` 改为 `/api/v1`、接口契约测试常量同步调整、
`localStorage` 键前缀由 `scaffold.` 改为 `guardian.`；新增本人资料与安全设置、登录第二因素和独立权限管理员重置入口，
其他管理页面沿用原有组件与 composable。

## 开发

```bash
npm ci
npm run dev
npm run test:unit
npm run build
```

默认地址 `http://localhost:5173`，`/api/v1` 代理到 `http://localhost:8080`，可用 `VITE_API_PROXY_TARGET`
指定其他本地后端。该变量与 `VITE_API_BASE_URL` 一样可以从 `frontend/.env` 读取（写法 `VITE_API_PROXY_TARGET=http://…`），
也可以直接作为 shell 环境变量传入，后者优先。后端准备（SQLite 与 Redis、默认账号）与生产环境清理清单见
[后端说明](../backend/README.md)，接口清单见 [后台管理模块](../backend/docs/ADMIN.md)。

前端请求的默认前缀是 `/api/v1`（`src/api/http.ts`），可用 `VITE_API_BASE_URL` 覆盖。

顶栏右侧有一个指向本仓库的 GitHub 图标入口，地址取自 `src/data/index.ts` 的 `APP.repo`。
生产部署若不希望外露该链接，把 `APP.repo` 置为空字符串即可整体不渲染（无需改动模板）；
彻底移除则是删掉 `src/components/GithubLink.vue` 与 `src/components/layout/Topbar.vue` 里的一行
`<GithubLink />`。

默认管理员为 `admin` / `admin123456`（账号与口令由后端 `.env` 的 `SEED_USERS` / `SEED_ADMIN_PASSWORD` 播种），
验证码必须使用当前页面显示的字符。非管理员菜单来自角色授权；
无菜单账号显示明确的权限分配提示。

## 个人资料与二次验证

顶栏「个人资料」进入 `/profile`，所有登录账号可访问（不要求菜单授权）。昵称、手机、邮箱可编辑；
账号、部门、角色只读。修改密码复用既有弹窗。认证器绑定支持本地生成二维码或手动密钥，确认动态码后才启用；
生成的恢复码仅保留本页内存，保存后重新登录。登录框只收账号、密码、验证码：账号启用 2FA 时，密码验证通过后会跳转独立页
`/login/2fa` 输入动态码或未使用恢复码，验证成功才建立会话；挑战只存在内存，刷新即作废。

用户管理的「重置2FA」按钮由 `system:user:reset2fa` 权限控制，需输入操作者密码，不使用重置密码权限替代。
后端密钥配置与恢复流程见 [生产部署](../docs/DEPLOYMENT.md#二次认证密钥与恢复)。

## 目录

| 路径 | 中文职责 |
| --- | --- |
| `src/api` | 统一 HTTP、会话存储、认证、系统管理与生成器接口 |
| `src/router/index.ts` | 静态入口、动态菜单路由、登录守卫与会话失效跳转 |
| `src/composables/auth.ts` | 当前用户、权限、动态菜单与会话版本 |
| `src/components/ui` | 按钮、输入、下拉、表格、弹窗、抽屉、提示等基础控件 |
| `src/components/business` | ListPage、SearchForm、FormDialog、PermissionButton 及字典、部门、用户选择 |
| `src/components/layout` | 外壳、侧栏、顶栏、命令搜索、外观与密码弹窗 |
| `src/components/GithubLink.vue` | 顶栏的 GitHub 仓库入口（独立组件，可按需摘除） |
| `src/composables` | 列表、字典 / 部门缓存、下拉浮层定位、外观、动效、焦点与确认框 |
| `src/styles` | 令牌、基础、材质、控件、外壳、页面六层样式 |
| `src/views` | 登录与两步验证、工作台、个人资料、10 个管理页、无权限与未匹配页面 |
| `scripts/session-regression.py` | 真实浏览器会话、路由竞态与低权限回归 |
| `scripts/account-regression.mjs` | 资料、2FA 绑定与恢复码的浏览器回归 |

## 扩展约定

新列表先复用 `ListPage` 与列表 composable，新表单先复用 `FormDialog`，权限动作使用 `PermissionButton`；
接口复用 `api/http.ts`。在 `auth.ts` 的 `COMPONENT_MAP` 注册页面，再通过菜单管理配置路径、组件标识与权限码。

所有下拉选择框统一走 `ui/Select` 与 `ui/MultiCheck`：浮层 Teleport 到 body、层级随所在弹窗（`composables/focus.ts` 的
`floatingLayerFor`）、空间不足向上展开（`composables/popover.ts`），触发器外观共用 `.select-trigger`。
搜索框默认按选项数自动开启（`ui/select-shared.ts` 的 `SELECT_SEARCH_THRESHOLD`），需要强制开启或关闭时显式传
`filterable`。部门、用户、字典选择器只是它的业务包装，不要另写一套下拉。

退出和 401 均通过统一会话清理撤销动态路由与缓存；不得在页面直接修改认证存储或路由就绪标记。
旧请求只能更新发起时仍有效的会话。

## 后端契约要点

- 统一响应 `{code, message, data?, traceId, timestamp}`：`code` 与 HTTP 状态码一致，
  `http.ts` 只认 `message` 作为错误文案。
- 分页返回 `{pageNum, pageSize, total, rows}`；列表页的导出走 CSV（`text/csv`），
  代码生成下载走 ZIP。
- 401 触发统一会话清理并跳登录；403 表示权限不足，页面按权限码隐藏入口。

## 浏览器回归

资料/2FA 浏览器回归（独立测试库；脚本创建并删除无菜单权限用户）：

```bash
npm install --no-save --package-lock=false @playwright/test@1.64.1
npx playwright install chromium
GUARDIAN_TEST_URL=http://localhost:5173 GUARDIAN_TEST_PASSWORD='<测试管理员口令>' node scripts/account-regression.mjs
```

需要未启用 2FA 的测试管理员；`GUARDIAN_TEST_PASSWORD` **必须**提供（脚本不内置口令），
验证资料保存、绑定、两步登录（登录框保持三项、跳转独立验证页）、恢复码登录/更新/关闭和移动端布局。

已有 Python Playwright 与 Chromium、前后端均启动时，可运行原有会话回归：

```bash
GUARDIAN_TEST_PASSWORD='<测试管理员口令>' python3 scripts/session-regression.py
```

建议连接独立本地校验库；脚本创建并删除自身的角色 / 用户夹具，不清空任何表。
`GUARDIAN_TEST_PASSWORD` **必须**提供；脚本需要 Vite 开发服务器来访问应用实际加载的 HTTP 模块。

两个脚本都不内置口令，登录用的验证码由 `scripts/captcha_solver.py` 从 `/auth/captcha`
返回的 SVG 图片里还原（明文只留在服务端），因此需要本机有 `python3`。渲染器改动时要同步
更新求解器里的字模与几何常量。

## 许可与上游

- 本目录代码派生自 `scaffold-master` 的 `scaffold-vue`，沿用上游 MIT 许可并保留版权声明，
  见 [LICENSE](LICENSE)；本仓库整体采用 `MIT OR Apache-2.0` 双许可，见 [根目录说明](../README.md)。
- 第三方依赖（`vue`、`vue-router`、`gsap`、`qrcode` 等）各自遵循其自身许可证，**不属于本项目
  MIT 授权范围**；使用与再分发时须遵守对应条款并保留原有版权与许可声明。
- GSAP 依赖遵循其 [Standard "No Charge" License](https://gsap.com/community/standard-license/)，
  该许可与 MIT 不同，商业使用前请自行核对其条款。
- 主题、密度与减少动效规则来自上游 `scaffold-master` 的 `DESIGN.md`（未随本仓库分发），
  前后端链路见同一仓库的 `PROJECT_MAP.md`；本仓库的落地位置是 `src/styles/tokens.css` 与
  `src/composables/appearance.ts`。
