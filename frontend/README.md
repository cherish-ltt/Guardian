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

顶栏「个人资料」进入 `/profile`，无需改密的登录账号可访问（不要求菜单授权）。昵称、手机、邮箱可编辑；
账号、部门、角色只读。修改密码复用既有弹窗。认证器绑定支持本地生成二维码或手动密钥，确认动态码后才启用；
生成的恢复码仅保留本页内存，保存后重新登录。登录框只收账号、密码、验证码：账号启用 2FA 时，密码验证通过后会跳转独立页
`/login/2fa` 输入动态码或未使用恢复码，验证成功才建立会话；挑战只存在内存，刷新即作废。

「登录会话」展示本人有效登录、当前项、浏览器 / 系统及按浏览器本地格式显示的登录 / 到期时间，支持刷新和加载更多。
撤销其他会话须输入当前密码，已启用 2FA 时还需动态码或恢复码；当前会话仍通过账号菜单退出。
列表不代表实时在线，客户端名称仅供识别，不展示 IP 或原始令牌。撤销失败保留列表项，重试需重新提供安全证明。
新前端须与 v0.11.0 后端配套更新，旧会话兼容与故障语义见 [发布说明](../docs/version/v0.11.0.md)。

用户管理的「重置2FA」按钮由 `system:user:reset2fa` 权限控制，需输入操作者密码，不使用重置密码权限替代。
后端密钥配置与恢复流程见 [生产部署](../docs/DEPLOYMENT.md#二次认证密钥与恢复)。

用户管理的新增 / 重置密码弹窗可开启「要求用户登录后先修改密码」，每次打开默认关闭。
登录及页面刷新先查询当前身份，`passwordChangeRequired` 为真时进入外壳之外的静态 `/password-change`，
不加载菜单、资料或业务页面；无菜单账号也能完成，启用 2FA 的账号仍须先完成第二因素。
修改成功后统一重新登录，记住状态的其他标签页同步退出；后端 403 中的改密标记和普通权限不足分别处理。
验收截图：[桌面浅色](../docs/screenshots/password-required-desktop.png)、[移动深色](../docs/screenshots/password-required-mobile.png)。

## 日志时间筛选

操作日志和登录日志的开始 / 结束时间使用 `@vuepic/vue-datepicker`：点击日期框，在同一面板选择日期与
时、分、秒，点击「确定」应用；「取消」或 Escape 放弃未确认的修改，清空按钮移除单侧边界，重置清空两侧。
四个筛选控件等高对齐，时间说明独立成行；窄屏竖排，弹层自动避让视口边缘。

`ui/DateTimePicker` 只绑定 `yyyy-MM-dd HH:mm:ss` 字符串，语义仍是服务端本地时间，不转 UTC 时间戳。
组件内部固定中性时区以避免浏览器夏令时缺口改写墙上时间；它不代表接口改为 UTC。
筛选包含开始、不包含结束，仅作用于明细、分页与 CSV 导出，不改变上方统计窗口。

`scripts/log-window-regression.py` 在独立测试数据库上实际点选日期和三个时间选项，并检查秒级边界、取消、
清空 / 重置、分页 / CSV、浏览器夏令时缺口，以及 1720 / 1440 / 820 / 390 / 320px 的控件边界与弹层。
运行要求与脚本头部环境变量说明一致，不应连接生产数据库。

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
| `scripts/self-session-regression.mjs` | 本人会话列表、单项撤销隔离、故障与慢响应、浅深主题回归 |
| `scripts/generator-remove-regression.mjs` | 单表配置移除、末页回退、重导、会话切换与迟到响应及桌面 / 窄屏回归 |
| `scripts/password-requirement-regression.mjs` | 待改密登录、重置开关、刷新与深链、重复提交、跨标签页及迟到响应回归 |
| `scripts/log-window-regression.py` | 日志时间窗口、翻页、CSV、重置及桌面 / 窄屏回归 |

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

代码生成页面的「移除配置」需确认表名及永久丢失的配置范围；取消不会请求，失败保留行，成功刷新列表与候选。
操作保留业务表、业务数据及已生成文件；重新导入建立新配置，不恢复旧自定义设置。
确认、删除及后续刷新均绑定发起时的会话；会话失效或页面卸载后不再执行旧确认、更新列表或显示旧结果。

- 统一响应 `{code, message, data?, traceId, timestamp}`：`code` 与 HTTP 状态码一致，
  `http.ts` 只认 `message` 作为错误文案。
- 管理列表分页返回 `{pageNum, pageSize, total, rows}`；本人会话使用游标分页 `{items, nextCursor}`。
  列表页的导出走 CSV（`text/csv`），代码生成下载走 ZIP。
- 401 触发统一会话清理并跳登录；带 `passwordChangeRequired: true` 的 403 进入改密流程，普通 403 表示权限不足。
- 操作与登录日志支持可选开始 / 结束时间，输入 `YYYY-MM-DD HH:MM:SS` 服务端本地时间，
  包含开始、不含结束；只影响明细与 CSV（最多 5000 行），不转换浏览器时区或改变统计窗口。
  清空可只保留单边界，重置同时清除时间并返回第一页。

## 浏览器回归

日志时间筛选回归使用独立 SQLite 测试库和已启动的前后端，脚本按唯一关键词创建并清理夹具，
不得连接生产数据库。已安装 Python Playwright / Chromium 后运行：

```bash
GUARDIAN_TEST_URL=http://127.0.0.1:5173 \
GUARDIAN_TEST_DB=/absolute/path/to/test.db \
GUARDIAN_TEST_PASSWORD='<测试管理员口令>' \
python scripts/log-window-regression.py
```

可用 `GUARDIAN_TEST_BROWSER` 指定本机 Chromium 路径，`GUARDIAN_TEST_SCREENSHOTS` 指定截图目录。
脚本使用纽约浏览器时区验证时间文本原样发送，并检查 1440 / 390 像素宽度下的输入可用性。
验收截图：[桌面](../docs/screenshots/log-window-desktop.png)、[窄屏](../docs/screenshots/log-window-mobile.png)。

资料/2FA 浏览器回归（独立测试库；脚本创建并删除无菜单权限用户）：

```bash
npm install --no-save --package-lock=false @playwright/test@1.64.1
npx playwright install chromium
GUARDIAN_TEST_URL=http://localhost:5173 GUARDIAN_TEST_PASSWORD='<测试管理员口令>' node scripts/account-regression.mjs
```

需要未启用 2FA 的测试管理员；`GUARDIAN_TEST_PASSWORD` **必须**提供（脚本不内置口令），
验证资料保存、绑定、两步登录（登录框保持三项、跳转独立验证页）、恢复码登录/更新/关闭和移动端布局。

本人会话回归复用上述 Playwright 安装及独立测试环境：

```bash
GUARDIAN_TEST_URL=http://localhost:5173 GUARDIAN_TEST_PASSWORD='<测试管理员口令>' node scripts/self-session-regression.mjs
```

可设置 `GUARDIAN_BROWSER_CHANNEL=chrome` 使用本机 Chrome，或通过 `GUARDIAN_PLAYWRIGHT_MODULE` 指向已安装的模块入口。
脚本创建两个无菜单权限账号和四个独立浏览器上下文，检查当前标记、跨账号隔离、失败不删行、慢请求重复提交、
目标会话真实 401、其他会话保持登录以及换号后的旧响应隔离；结束后删除本次用户。
桌面 / 移动端的浅深主题截图写入忽略的 `.test-artifacts/`，表格窄屏沿用内部横向滚动。
验收截图：[桌面浅色](../docs/screenshots/self-sessions-desktop.png)、[移动深色](../docs/screenshots/self-sessions-mobile.png)。
本脚本使用 Chrome 引擎；真实 2FA 撤销与 Redis 多页边界由后端回归覆盖。

生成配置移除回归复用上述 Playwright 安装，需要 `sqlite3` 和独立 SQLite 测试库：

```bash
GUARDIAN_TEST_URL=http://localhost:5173 \
GUARDIAN_TEST_DATABASE=/absolute/path/to/test.db \
GUARDIAN_TEST_PASSWORD='<测试管理员口令>' \
node scripts/generator-remove-regression.mjs
```

脚本连接 Vite 开发服务，创建 11 张唯一命名的业务测试表、导入配置及一个无菜单权限账号，验证取消、删除失败、
末页回退、候选刷新、重导新 ID、预览和业务数据保留；还覆盖确认期间 401 / 跨标签换号、DELETE 和刷新迟到的
成功 / 失败响应，确认新会话不继承旧删除、刷新或提示。结束后删除自身账号、配置和测试表；不得连接生产或共享数据库。
桌面 / 窄屏截图写入忽略的 `.test-artifacts/`；环境变量 `GUARDIAN_BROWSER_CHANNEL` 与
`GUARDIAN_PLAYWRIGHT_MODULE` 的含义同上。

强制改密回归复用同一独立环境与环境变量，需 Vite 开发服务器（迟到响应测试使用应用实际 HTTP 模块）：

```bash
GUARDIAN_TEST_URL=http://localhost:5173 GUARDIAN_TEST_PASSWORD='<测试管理员口令>' node scripts/password-requirement-regression.mjs
```

脚本自建自删两个无菜单账号，验证受限登录与深链、刷新、桌面/移动浅深主题、错误密码、慢请求防重复、
成功改密后的旧令牌 401 与跨标签页退出、管理员重置开关以及旧账号迟到 403 不影响新账号。
截图写入 `.test-artifacts/`；2FA 顺序、数据库迁移和事务回滚由后端用例覆盖。

已有 Python Playwright 与 Chromium、前后端均启动时，可运行原有会话回归：

```bash
GUARDIAN_TEST_PASSWORD='<测试管理员口令>' python3 scripts/session-regression.py
```

建议连接独立本地校验库；脚本创建并删除自身的角色 / 用户夹具，不清空任何表。
`GUARDIAN_TEST_PASSWORD` **必须**提供；脚本需要 Vite 开发服务器来访问应用实际加载的 HTTP 模块。

这些脚本都不内置口令，登录用的验证码由 `scripts/captcha_solver.py` 从 `/auth/captcha`
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
