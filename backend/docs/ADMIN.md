# 后台管理模块

> 本文档说明后台管理接口的契约：路由清单、权限模型、
> 留痕规则、存储映射与差异取舍。快速上手见 [README](../README.md)，分层约束见 [架构设计](ARCHITECTURE.md)。

请求前缀统一为 `/api/v1`。业务代码分布：

```
src/api/controllers/auth_controller.rs       登录、验证码、当前账号、动态路由、修改密码
src/api/controllers/account_controller.rs    本人资料与会话、2FA 绑定/恢复/关闭、管理员重置
src/api/controllers/admin/                   系统管理八个页面各一个控制器
src/api/controllers/monitor_controller.rs    操作日志与登录日志
src/api/controllers/dashboard_controller.rs  工作台
src/api/controllers/generator_controller.rs  代码生成
src/api/dto/admin.rs                         查询串解析与生成器请求体
src/api/extractors/admin_user.rs             后台会话提取器（权限码校验、数据范围）
src/api/extractors/api_json.rs               请求体统一反序列化（非法 JSON → 400）
src/domain/services/admin/                   AdminUseCase 的 9 个子 trait
src/services/admin/                          各子 trait 的实现
src/middleware/audit.rs                       写操作留痕
```

## 接口清单

除本人会话列表使用游标外，列表接口统一用 `pageNum` / `pageSize` 分页（页码从 1 开始，`pageSize` 上限 100，
排序由各接口的 SQL 固定给出）。`pageNum` / `pageSize` 与 `status` 等过滤参数同语义：
非整数或负数一律 `400`，不静默回落默认值。

`pageSize` 超过 100 返回 `400`「每页条数不能超过100」（不截断后照常返回，避免客户端
把半页当成整页）；不传或传 `0` 取默认值 10。

### 认证（`/auth`）

| 方法 | 路径 | 权限 | 说明 |
| --- | --- | --- | --- |
| GET | `/auth/captcha` | 公开 | 返回 `uuid` 与内联 SVG 图片（`image` 为 `data:image/svg+xml;base64,...`），明文只留在服务端缓存里比对，Redis TTL 300s；参数 `sys.account.captchaEnabled=false` 时返回 `enabled=false` 且不生成验证码 |
| POST | `/auth/login` | 公开 | 校验码 → 口令 → 账号状态；未启用 2FA 直接返回 JWT 与 `expiresIn`，启用则返回一次性挑战 |
| POST | `/auth/login/2fa` | 公开 | 用挑战 + 动态码/恢复码完成登录，签发 JWT 与会话 |
| POST | `/auth/logout` | 登录态 | 删除 Redis 会话令牌，令牌随后失效 |
| GET | `/auth/me` | 登录态 | 账号、部门、角色、权限码 |
| GET | `/auth/routes` | 登录态 | 动态菜单树（只含 `dir` / `menu` 且可见） |
| PUT | `/auth/password` | 登录态 | 改密（校验旧密码、新旧不能相同、长度 6–64） |
| GET | `/auth/session` | 登录态 | 骨架自带的演示会话接口 |
| GET | `/auth/profile` | 登录态 | 本人资料与 2FA 状态，无需菜单权限 |
| PUT | `/auth/profile` | 登录态 | 只修改昵称、手机、邮箱 |
| GET | `/auth/sessions` | 登录态 | 本人有效会话，游标分页，无需菜单权限 |
| POST | `/auth/sessions/{id}/revoke` | 登录态 + 密码 + 已启用的第二因素 | 仅撤销本人其他会话，无需菜单权限 |
| POST | `/auth/2fa/setup` | 登录态 + 密码 | 创建待确认绑定信息 |
| POST | `/auth/2fa/enable` | 登录态 + 密码 + 动态码 | 确认绑定，返回一次性恢复码 |
| POST | `/auth/2fa/disable` | 登录态 + 密码 + 第二因素 | 关闭 2FA |
| POST | `/auth/2fa/recovery-codes` | 登录态 + 密码 + 第二因素 | 替换全部恢复码 |

### 系统管理（`/system`）

| 方法 | 路径 | 权限 | 说明 |
| --- | --- | --- | --- |
| GET | `/system/users` | `system:user:list` | 返回 `deptId`、`roleIds`、`postIds` 的分页列表，支持 `keyword` / `status` / `deptId` / `postId`，受数据范围约束 |
| GET | `/system/users/options` | 同上 | 表单下拉（角色、岗位、部门树、初始密码，只读权限返回空） |
| GET | `/system/users/export` | 同上 | `users.csv` |
| POST | `/system/users` | `system:user:add` | 新增（口令留空取系统参数 `sys.user.initPassword`，参数为空才回退 `SEED_ADMIN_PASSWORD`） |
| PUT | `/system/users/{id}` | `system:user:edit` | 修改（`roleIds` / `postIds` 全量替换） |
| PUT | `/system/users/{id}/status` | `system:user:edit` | 启用 / 停用，内置管理员不可停用 |
| PUT | `/system/users/{id}/password` | `system:user:resetPwd` | 重置口令 |
| POST | `/system/users/{id}/2fa/reset` | `system:user:reset2fa` | 校验操作者密码后重置他人 2FA |
| DELETE | `/system/users/{id}` | `system:user:remove` | 逻辑删除，内置管理员不可删除 |
| GET | `/system/roles` | `system:role:list` | 分页列表（`dataScope` 返回中文标签） |
| GET | `/system/roles/export` | 同上 | `roles.csv` |
| GET | `/system/roles/{id}/menus` | 同上 | 角色已授权菜单 ID |
| GET | `/system/permissions/graph` | 同上 | 权限关系图（用户 / 角色 / 菜单节点与连线，用户节点受数据范围约束） |
| POST / PUT / DELETE | `/system/roles`、`/system/roles/{id}` | `system:role:add` / `edit` / `remove` | 角色维护，内置角色不可改删，有用户占用不可删 |
| GET | `/system/menus/tree` | `system:menu:list` | 菜单树（`keyword` 命中项不补父节点） |
| GET | `/system/menus/export` | 同上 | `menus.csv` |
| POST / PUT / DELETE | `/system/menus`、`/system/menus/{id}` | `system:menu:add` / `edit` / `remove` | 有子菜单或已授权角色不可删 |
| GET | `/system/depts/tree` | `system:dept:list` | 部门树（含直属成员数） |
| GET | `/system/depts/export` | 同上 | `depts.csv` |
| POST / PUT / DELETE | `/system/depts`、`/system/depts/{id}` | `system:dept:add` / `edit` / `remove` | 上级链随 `parentId` 重算；有下级或已关联用户不可删 |
| GET | `/system/posts` | `system:post:list` | 分页列表（含占用人数） |
| GET | `/system/posts/export` | 同上 | `posts.csv` |
| POST / PUT / DELETE | `/system/posts`、`/system/posts/{id}` | `system:post:add` / `edit` / `remove` | 编码唯一 |
| GET | `/system/dict/types` | `system:dict:list` | 字典类型分页列表 |
| GET | `/system/dict/types/export` | 同上 | `dict-types.csv` |
| GET | `/system/dict/data/{dict_type}` | 同上 | 指定类型的字典数据（管理页用，`def` 为布尔） |
| GET | `/system/dict/options/{dict_type}` | 登录态 | 业务下拉（`def` 为 0/1，带 `tone`）；供任意已登录页面渲染，不校验 `system:dict:list` |
| POST / PUT / DELETE | `/system/dict/types`、`/system/dict/types/{id}` | `system:dict:add` / `edit` / `remove` | 有字典数据不可删 |
| POST / PUT / DELETE | `/system/dict/data`、`/system/dict/data/{dict_type}` | 同上 | `{dict_type}` 位置：GET 传类型，PUT / DELETE 传主键；同一字典类型只保留一个默认项（`def=true`），设新的会清掉旧的 |
| GET | `/system/configs` | `system:config:list` | 参数分页列表 |
| GET | `/system/configs/export` | 同上 | `configs.csv` |
| POST / PUT / DELETE | `/system/configs`、`/system/configs/{id}` | `system:config:add` / `edit` / `remove` | 内置参数不可改删，键名唯一 |

新增用户采用指定密码或系统初始密码，密码长度须为 6 至 64 个 Unicode 字符；重置和修改密码同样适用此限制。

### 监控（`/monitor`）

| 方法 | 路径 | 权限 | 说明 |
| --- | --- | --- | --- |
| GET | `/monitor/oper-logs` | `monitor:operlog:list` | 分页列表（`keyword` / `result` / `startTime` / `endTime`），`ip` 按查看权限打码 |
| GET | `/monitor/oper-logs/stats` | 同上 | 统计卡 + 趋势（`range=24h` 按小时、`7d` / `30d` 按天，缺省桶补零） |
| GET | `/monitor/oper-logs/timeline` | 同上 | 泳道时间线（同一模块相邻操作合并，间隔超过 900s 断开） |
| GET | `/monitor/oper-logs/export` | 同上 | `oper-logs.csv`，与列表共用关键词、结果及时间条件，最多 5000 行 |
| DELETE | `/monitor/oper-logs/{id}`、`/monitor/oper-logs` | `monitor:operlog:remove` | 删除单条 / 清空 |
| GET | `/monitor/login-logs` | `monitor:loginlog:list` | 分页列表（`keyword` / `status` / `startTime` / `endTime`），`ip` 按查看权限打码 |
| GET | `/monitor/login-logs/stats` | 同上 | 统计卡 + 趋势 |
| GET | `/monitor/login-logs/heatmap` | 同上 | 7×24 热力格与异常告警（`days` 可选，阈值默认 5） |
| GET | `/monitor/login-logs/export` | 同上 | `login-logs.csv`，与列表共用关键词、状态及时间条件，最多 5000 行 |
| DELETE | `/monitor/login-logs/{id}`、`/monitor/login-logs` | `monitor:loginlog:remove` | 删除单条 / 清空 |
| GET | `/monitor/ip-locations?ip=` | `monitor:ip:view:full` | 单条属地现查，返回 `{ip, location}` |
| POST | `/monitor/ip-locations` | `monitor:ip:view:full` | 整页批量现查，请求 `{"ips": [...]}`，返回顺序与请求一致 |

日志明细与导出时间条件：

- `startTime` / `endTime` 均可省略，支持只传一个边界；全部省略保持原有行为。
- 格式严格为 `YYYY-MM-DD HH:MM:SS`，按服务端本地时间文本解释；区间为
  `[startTime, endTime)`（包含起点、不含终点），与关键词及结果 / 状态取交集。
  例如 `?startTime=2026-01-02%2010:00:00&endTime=2026-01-02%2011:00:00`。
- 无效日期、缺时间部分、显式空串 / 空白、非规范格式、开始不早于结束均返回明确的 400，
  不会静默忽略条件；前端清空输入时省略对应参数，重置清除时间并返回第一页。
- 列表、`total` 与 CSV 使用相同过滤及时间倒序 / ID 倒序；导出忽略分页，最多 5000 行，
  不是全量归档。权限、IP 三档脱敏与 CSV 公式中和保持不变。
- 明细条件不影响概览、趋势、泳道或热力图的独立窗口；不转换浏览器时区，
  不迁移历史数据。历史时间没有偏移信息，无法区分夏令时重复时刻。

`location` 的取值与日志地点同口径：回环「本机」、RFC1918 私网 / 链路本地 / IPv6 ULA「内网」
（这两档不查库，未配置地名库时同样成立），其余查库，库未命中或落在保留地址段为「外网」。

真实 IP 的查看粒度由三个按钮级权限码决定，角色在「角色管理」里勾选，只有超级管理员能授权：

| 权限码 | 默认持有 | 列表里的 `ip` |
| --- | --- | --- |
| `monitor:ip:view:full` | 超级管理员（`super_admin` 拥有全部菜单） | 完整地址，可使用属地查询 |
| `monitor:ip:view:partial` | 系统管理员（`sys_admin`，种子默认授权） | `61.135.x.x` / `2001:db8:85a3:0:x:x:x:x` |
| （都不授予） | 其余角色 | `61.x.x.21` / `2001:x:x:x:x:x:x:7334` |

打码在 axum 返回前完成，库里存的是原始地址；本地网段、回环与公网地址一视同仁。
属地查询只对完全查看档开放，其余档位返回 403「没有访问权限」。

### 工作台与代码生成

| 方法 | 路径 | 权限 | 说明 |
| --- | --- | --- | --- |
| GET | `/dashboard/summary` | 登录态 | 6 张统计卡、登录趋势、近 7 天操作量、近期动态、角色分布 |
| GET | `/tool/generator/tables` | `tool:gen:list` | 已导入的生成配置（`keyword`） |
| GET | `/tool/generator/importable-tables` | 同上 | 数据库表候选（`imported` 标记是否已导入） |
| POST | `/tool/generator/import` | 同上 | 按 `tableNames` 导入，重复导入复用已有配置 |
| GET | `/tool/generator/tables/{id}/columns` | 同上 | 字段配置（`rustField` / `rustType` / `nullable` / `formType` / 增删改查开关） |
| GET | `/tool/generator/tables/{id}/preview` | 同上 | 10 个文件的代码预览（`activeFile` 为默认展开的控制器路径） |
| PUT | `/tool/generator/tables/{id}` | 同上 | 只接受 `tableComment` 与 `synced` |
| DELETE | `/tool/generator/tables/{id}` | 同上 | 永久移除该生成配置及列配置，保留业务表、数据和已生成文件 |
| POST | `/tool/generator/tables/{id}/generate` | 同上 | 生成结果提示 |
| GET | `/tool/generator/tables/{id}/download` | 同上 | ZIP（条目为完整相对路径） |

代码生成器仅支持 SQLite 单列 `INTEGER` 行号主键；文本主键、复合主键、无主键和非行号主键在导入时会被拒绝。

移除按配置 ID 定位，在同一 IMMEDIATE 事务中确认父配置存在、删除列配置再删除父配置；任一步失败均回滚。
业务表已不存在仍可移除残留配置；目标配置不存在返回 400「代码生成表不存在」。成功返回 `data: null`。
业务表仍存在且符合导入限制时可重新导入，建立新的配置 ID 和默认列配置，不恢复旧自定义选项；
旧 ID 的迟到移除或更新返回不存在，不影响新配置。并发导入可能合法地在移除后建立新配置。
页面使用确认框提示永久丢失范围，取消不请求，成功后刷新列表与候选；移除不替代保留设置的结构同步。
审计动作是「代码生成 / 移除配置」，记录含 ID 的 URL；401/403 不留痕，审计失败不回滚已完成的移除。

### 其它

`GET /api/v1/health`（新增：`status` / `time` / `serviceName`）、骨架既有的
`GET /api/v1/system/health`、`GET /api/v1/system/ready` 保持不变。

`/api/v1/examples/*`、`/api/v1/auth/dev-login`、`/api/v1/transactions/dev-transfer`
都只在 debug 构建注册，release 构建访问返回 404；`/api/v1/transactions`、
`/api/v1/transactions/{id}` 两个只读事务示例在两种构建下都存在。

## 权限与数据范围

`src/api/extractors/admin_user.rs` 负责两件事：

- **权限码**：`AdminUser::require(code)` 命中 `super_admin` 直接放行，否则要求权限码在 `/auth/me`
  返回的集合里，缺失返回 403「没有访问权限」。权限码由「启用角色 → `sys_role_menu` → 启用菜单」推导。
- **数据范围**：`AdminUser::data_scope()` 解析启用角色的范围，`ALL` 不限；`DEPT_AND_CHILD` / `DEPT`
  展开为可见部门（`sys_dept.ancestors` 以逗号分隔的上级链，用 `(',' || ancestors || ',') LIKE '%,id,%'`
  替代 MySQL 的 `FIND_IN_SET`）；`CUSTOM` 取 `sys_role_dept`；其余按「仅本人」。用户列表、用户导出
  与权限关系图的用户节点都吃这套过滤。

## 留痕（审计）

`src/middleware/audit.rs` 挂在 `nest` 之后的路由层，按「请求方法 + 路径段模式（`:id` 通配）」匹配规则，
命中后记录模块与动作，例如 `POST /system/posts` → 「岗位管理 / 新增」、`POST /tool/generator/import` →
「代码生成 / 导入表」、`POST /auth/sessions/{id}/revoke` → 「账号安全 / 撤销会话」。

与 Spring `@OperLog` 切面的差异（有意为之）：

- 记录**请求体**（切面记录的是方法入参），`password` / `token` / `secret` / `authorization` /
  `secondFactor` / `recovery` 等敏感键的值替换为 `******`，超过 4000 字符截断；无效 JSON 不保留原文。
- 认证或权限被拒（401 / 403）不写日志：这两种情况下请求根本没进业务方法。
- 留痕写入失败只 `error!` 记日志，不改变业务响应。
- 操作人只解 JWT，不再查库（处理器已完成完整鉴权）。
- 来源 IP 取保护层的可信代理解析结果，而不是请求头原值（见 [中间件栈与配置](MIDDLEWARE.md)）。

## 登录日志

`sys_login_log` 在每次登录尝试与退出时各写一行，客户端来源与其他字段：

| 列 | 来源 |
| --- | --- |
| `username` / `status` / `msg` | 业务结果：密码错误、账号停用（成功登录前的失败也记录，验证码失败不写） |
| `ip` | 保护层解析出的真实对端 IP（可信代理命中时取 `X-Forwarded-For` 最右侧的不可信地址） |
| `browser` / `os` | 请求 `User-Agent` 经 `src/util/user_agent.rs`（woothee 规则表）解析出的名称，识别不出时留空 |

`location` 列不再写入（属地改为按需现查，见监控一节），历史行的该列保持原值不动。

`src/api/extractors/client_source.rs` 把来源收成一个提取器：控制器只需声明参数，
不自己读头或扩展；服务层签名收 `&ClientSource`，会话、验证码与事务边界都不受影响。

## 存储映射（MySQL → SQLite）

- 表结构见 `migrations/0002_create_system_tables.sql`：16 张表，列名与 MySQL 版一致
  （该文件原本建 17 张，`0005_drop_sys_file.sql` 删掉了从未使用的 `sys_file`）。
- 用户 / 角色 / 岗位走软删除（`deleted = 1`），唯一约束只约束未删除的行：
  `0006_partial_unique_indexes.sql` 把 `uk_sys_user_username` / `uk_sys_role_code` /
  `uk_sys_post_code` 改成 `WHERE deleted = 0` 的部分索引，删除后可以立刻重建同名记录。
- `DATETIME` 存 `TEXT`（`YYYY-MM-DD HH:MM:SS` 本地时间），字典序比较等价时间先后；
  查询用 `strftime` / `julianday` / `substr(x, 1, 16)` 等价 `NOW()` / `CURDATE()` / `DATE_FORMAT()`。
- 种子数据在 `src/infrastructure/databases/seed.rs`：11 个部门、9 个岗位、6 个角色、
  3 个用户及其角色 / 岗位关联、38 条菜单、3 类字典与 7 条字典项、3 条参数配置
  （启动后 `sys_menu` 实际是 39 条：`0003_user_security.sql` 另追加「重置2FA」按钮）；
  全部 `INSERT OR IGNORE`，可重复启动。**不含**任何 `sys_oper_log` / `sys_login_log` 示例行（与 Spring 一致）。
- 种子账号与口令都来自 `.env`：`SEED_USERS`（`用户名:昵称:邮箱:部门ID`，逗号分隔）决定播种哪些账号，
  `SEED_ADMIN_PASSWORD`（默认 `admin123456`）是它们的初始口令，也是新增用户未传口令时的默认值；
  播种时用 argon2 现场哈希，没有照抄参考实现里的 BCrypt 字面量。
  每个账号各自哈希：即使共用同一口令，也各带随机盐，不会把一份 PHC 字符串复制给多个账号。

## 会话与验证码存储

`src/infrastructure/services/auth_store.rs` 定义 `AuthStore` 接口：

- `MemoryAuthStore`：集成测试用，进程内存 + TTL。
- `RedisAuthStore`：连接串与口令来自 `REDIS_URL` / `REDIS_PASSWORD`（`RedisConfig::connection_url()`），
  验证码 TTL 为 300 秒，会话 TTL 按签发 JWT 的有效期设置。

| Redis 键 | 内容与用途 |
| --- | --- |
| `captcha:<uuid>` | 一次性验证码，300 秒过期 |
| `auth:token:<JWT>` | 令牌所属账号；认证时必须存在，按 JWT 有效期过期 |
| `auth:user:<username>` | 该账号令牌的 SET 索引，兼容旧格式；新会话不会缩短索引中较长会话的剩余 TTL |
| `auth:session:<username>:<id>` | 仅服务端可读的归属、令牌与浏览器 / 系统名称映射，沿用令牌剩余 TTL；撤销后保留归属和令牌至原到期时间，供本人重试确认，不能用于认证 |

登录后 JWT 与 Redis 会话令牌必须同时有效：登出即删除令牌，旧令牌立刻失效（返回
「未认证或登录已过期」）。测试通过 `Container::bootstrap_with_auth_store(config, Container::memory_auth_store())`
装配，默认不依赖外部 Redis；真实 Redis 用例单独执行，见 [测试说明](TESTING.md)。

改密码会撤销该账号的全部会话（`AuthStore::revoke_user_tokens`）：Redis 侧按
`auth:user:<username>` 集合读取待撤销令牌，逐项删除对应 `auth:token:*` 并 `SREM` 索引成员，
不在最后 `DEL` 整个索引，避免误删并发新登录建立的索引；其他账号不受影响。
因此改完密码后，当前会话与其他设备上的旧会话都需重新登录。

旧 SET 索引无需迁移；读取仍有效且归属匹配的旧令牌时按其剩余 TTL 补建管理映射，不延长令牌有效期。
旧 JWT 缺少 `jti` 仍可管理；缺少 `auth_version` 按 0 处理，仍须匹配数据库当前版本。
旧会话没有客户端信息时浏览器与系统名称为空，不补采 IP 或完整 User-Agent。
Redis Cluster 未验证，部署限制见 [生产部署](../../docs/DEPLOYMENT.md#会话存储与撤销)。

## 本人会话管理

个人资料页 `/profile` 提供本人会话列表与单会话撤销，所有已登录账号均可使用，无需菜单权限。
超级管理员也只能管理本人会话，不能通过这组接口管理其他账号。

- `GET /auth/sessions?cursor=<游标>` 返回 `data: {items: [{id, current, issuedAt, expiresAt, browser, os}], nextCursor}`。
  `issuedAt` / `expiresAt` 为 Unix 秒；`current` 标记发起请求的会话。浏览器和系统只返回解析后的名称，
  未识别时为空字符串；不返回 JWT、Redis key、IP 或原始 User-Agent。
- `id` 是由令牌摘要生成的 Base64URL 标识（43 字符），用于选择会话，不能作为登录凭据。
  列表只包含账号启用、JWT 验签与归属正确、`auth_version` 匹配、`exp` 严格晚于当前时刻且 Redis 令牌有效的会话；
  过期、已撤销、换代或归属不符的索引残留不会显示，不受 JWT 验签宽限期影响。
  只清理无法验签、已过期或同一身份更旧版本的令牌；不同身份或较新版本只过滤，避免旧请求误撤销并发新登录。
- 首次省略 `cursor`（或传 `0`）；后续原样传回 `nextCursor`，为 `null` 时结束。
  游标是非负 `u64` 十进制字符串；显式空值、负数、小数或越界值返回 400。
  Redis 使用本人索引的 `SSCAN COUNT 50`，不扫描全库；`COUNT 50` 只是工作量提示，**不是每页固定条数或上限**。
  过滤后可能出现空页但仍有下一游标；扫描不是固定排序快照，客户端应按 `id` 去重并继续读取至 `nextCursor=null`。
- `POST /auth/sessions/{id}/revoke` 接受 `{password, secondFactor?}`：每次校验当前密码，
  已启用 2FA 时还必须提供有效动态码或未使用恢复码。密码与第二因素分别累计失败次数，
  各自连续失败 5 次封禁 300 秒；密码成功不清除第二因素的失败计数或封禁。
- 目标是当前会话时返回 400「当前会话请使用退出登录」；跨账号、未知 ID 或已无法校验的目标返回 404。
  成功返回 `data: {revoked: true}`，只使所选会话失效，其余会话、密码和 2FA 设置不变。
  本人映射仍在且 JWT 尚可验签，但目标已撤销、已过 `exp` 或已换代时，仍校验并消费证明、清理存储，
  返回 `data: {revoked: false}`；映射到期或 JWT 已超过验签宽限后返回 404。
- 密码失败计数、第二因素失败计数及一次性证明消费先在 SQLite 事务提交，再调用 Redis 撤销。
  Redis 存入、列表、查找或撤销失败返回 503；撤销失败可能发生在令牌已删除之后，不代表目标仍有效。
  已提交的动态码 / 恢复码消费不会回滚；恢复后用当前密码及新的动态码 / 恢复码对同一 ID 重试确认，
  未开启 2FA 时只需当前密码。故障边界见 [生产部署](../../docs/DEPLOYMENT.md#会话存储与撤销)。

## 本人资料与二次认证

所有可登录账号均可自愿启用/关闭，不强制开启；登录与安全操作使用第二因素，不改变既有改密契约。
前端静态受保护页 `/profile` 无需菜单授权，顶栏「个人资料」进入；账号、部门和角色只读。

- GET `/auth/profile` 返回 `username/nickname/phone/email/deptName/roles/twoFactorEnabled/recoveryCodesRemaining`。
  PUT 接受 `{nickname, phone, email}`；昵称 1–64 字符，手机最多 32 字节（数字及 +、-、空格、括号），
  邮箱最多 254 字节且包含非空的 @ 两侧；手机、邮箱可传 null 清空。额外字段一律 400。
- 登录分两阶段，原登录框只收账号、密码、验证码：

  - `POST /auth/login` 请求体不含第二因素。密码与账号状态通过后：未启用 2FA 直接返回 `{token, expiresIn}`；
    启用 2FA 返回 `{twoFactorRequired: true, challenge, expiresIn: 300}`，**此时不签发 JWT、不建立会话**。
    挑战是 32 字节随机串，库里只存 SHA256。
  - 前端拿到挑战后跳转独立页 `/login/2fa` 提交 `POST /auth/login/2fa` `{challenge, secondFactor}`，
    动态码或未使用恢复码正确才返回 `{token, expiresIn}`。密码不落前端持久化存储，挑战只放在内存里，
    刷新或离开该页即作废。
  - 挑战一次性：成功或被拒（包括失败计数达上限）都会清除；过期（300 秒）、密码此后变更、
    账号停用、2FA 关闭或重置后均视为无效，需回登录页重新走验证码与密码。
  - 第二因素失败 5 次与绑定/关闭等安全证明共用同一失败计数与 300 秒封禁。
  - 密码或验证码失败不进入第二因素，也不创建挑战；登录日志分别记「账号或密码错误」「二次验证失败」。
- setup 接受 `{password}`，返回 `{secret, otpauthUrl, expiresIn: 300}`。重复 setup 替换待绑定密钥，
  只有 enable 提交 `{password, secondFactor}` 且动态码正确才启用；未确认/过期绑定不影响登录。
- TOTP 为 SHA1 / 6 位 / 30 秒，允许前后各一个时间步；有效密钥和待绑定密钥用 AES-256-GCM 加密，
  独立密钥配置及恢复边界见 [生产部署](../../docs/DEPLOYMENT.md)。SQLite IMMEDIATE 事务串行消费时间步，
  同一时间步不可重放；刚绑定或登录后进行安全设置需等待新动态码或使用另一枚恢复码。
- enable 和 recovery-codes 返回 `{recoveryCodes: string[]}`：10 枚随机 128-bit 恢复码，仅存 SHA256 哈希。
  每枚仅能成功消费一次；更新后旧批次立即失效。绑定二维码、密钥、恢复码只放页面内存，不写浏览器存储；
  `/auth/*` 响应设置 `Cache-Control: no-store`，请勿在网关缓存认证响应。
- disable / recovery-codes 接受 `{password, secondFactor}`，可用动态码或未使用恢复码证明身份。
  密码与第二因素独立计数，各自连续失败 5 次封禁 300 秒（数据库持久化）；管理员重置会清除目标封禁。
- 启用、关闭、更新恢复码、管理员重置均在事务内增加 `auth_version`；认证每次比较 JWT 和数据库版本，
  并继续校验 Redis 会话。提交后清理 Redis 旧令牌；清理失败记录告警，旧令牌仍由版本校验拒绝，
  返回已提交结果避免丢失一次性恢复码。前端清理当前会话，保存恢复码后重新登录。
- 管理员重置接受 `{password}`（操作者密码），不能重置本人，非超管不能重置超管。
  迁移 `0003_user_security.sql` 新增权限 `system:user:reset2fa`，一次性默认授予超管和已有
  `system:user:resetPwd` 的角色。之后独立管理，不随重启恢复已撤销的授权；新角色需显式分配。

## 基础数据缓存

认证和权限校验先于业务缓存；读取路径为 API → Redis → SQLite，SQLite 为唯一业务真值。
当前缓存清单：

| 接口 | 缓存内容 |
| --- | --- |
| GET `/system/depts/tree` | 含成员数的部门树（按筛选参数区分） |
| GET `/system/menus/tree` | 管理用菜单树（按筛选参数区分） |
| GET `/system/users/options` | 仅部门选项，角色/岗位/初始密码不在本次缓存内 |
| GET `/system/dict/types` | 字典类型分页/筛选结果 |
| GET `/system/dict/data/{dict_type}` | 字典管理数据 |
| GET `/system/dict/options/{dict_type}` | 业务下拉字典 |
| GET `/system/configs` | 参数分页/筛选结果 |

不缓存用户权限、`/auth/routes` 用户定制菜单、用户资料、统计聚合和导出。
键包含进程随机命名空间、数据代数与完整查询的 SHA256；正常 TTL 加 0–20% 抖动，空数组/null/空分页短缓存，错误不缓存。
64 个固定分片锁合并健康 Redis 下的并发未命中；回源并发有界，超量 503。同分片等候仍受全局请求超时/背压约束。
Redis 超时/错误/坏数据触发短暂熔断，回源 SQLite；故障时不共享已完成的回源结果，不保证只查一次。
键登记数、单值大小均有上限，超限不回填；配置与故障边界见 [生产部署](../../docs/DEPLOYMENT.md)。

**仅保证单后端实例内，经 API 成功修改后发起的后续请求不读旧值**：相关写操作先取得写屏障换代，
屏障覆盖数据库修改；读锁覆盖读缓存、回源和回填，防止并发旧值回填。部门/菜单/字典/参数写入及影响部门
成员数的用户写入均换代；失败或取消也不会复用旧代。旧键 TTL 自动清理，不依赖删除 Redis 成功。
多实例或在线直接改库不在一致性保证内；浏览器已显示的内容不会自动推送更新。

## 与 Spring 版的差异

- 生成配置支持按 ID 原子移除父子元数据；该操作不执行真实表 DDL，不删除业务数据或生成文件，重导不保留旧配置 ID。
- 非超级管理员不得重置任何拥有有效超级管理员角色的账号的密码。
- 用户编辑时手机号和邮箱省略表示不修改，传空字符串表示清空。

| 项 | Spring 版 | 本实现 | 原因 |
| --- | --- | --- | --- |
| 本人会话管理 | 本轮未核实上游对应契约 | 本人有效会话游标列表、经安全证明撤销其他单会话、非凭据 ID、失败 503 与新证明重试 | 本次明确需求，保留旧 Redis SET 与 JWT 兼容，不声称兼容上游会话接口 |
| 日志明细时间窗口 | 列表与导出仅关键词、结果 / 状态 | 增加可选 `startTime` / `endTime`，服务端本地时间半开区间，非法边界返回 400 | 按事件窗口排查与导出，沿用既有时间索引及导出上限，不改变统计窗口 |
| 个人资料与 2FA | 本轮环境未提供参考目录，未核实上游对应契约 | 按本文新增本人接口、第二因素与独立重置权限 | 本次明确需求，非声称逐字兼容 |
| 基础数据缓存 | 本轮未核实上游缓存策略 | 单实例写屏障/换代、有界回源及空值缓存 | 本次明确的一致性及故障约束 |
| 验证码开关 | `sys.account.captchaEnabled` 参数控制是否校验验证码 | 同名参数，`builtin=0` 可在参数页直接开关 | 关闭后 `/auth/captcha` 返回 `enabled=false` 且不写 Redis，登录不再校验验证码 |
| 路由前缀 | `/api` | `/api/v1` | 与骨架既有前缀统一；前端只需改 `http.ts` 默认前缀 |
| 路径参数 | `@PathVariable Long id` | `{dict_type}` 共用同名参数 | axum 要求同一位置参数同名，GET 传类型、PUT / DELETE 传主键 |
| 字典默认项 | 未核实上游约束 | 同一字典类型最多一个默认项，写库时在同事务内清掉旧默认项，并由 `is_default` 部分唯一索引兜底；升级时自动把存量多默认收敛为排序最靠前的一项 | 默认值语义上是单选，旧实现允许多个，存量数据由迁移无感修正 |
| 响应体 | `{code, message, data, traceId}` | 同前 + `timestamp` | `timestamp` 是骨架既有字段，前端不读 |
| 5xx 文案 | `系统内部错误` | 同前 | 4xx 文案逐字透出（「账号或密码错误」等） |
| 请求体非法 | Spring 抛 `HttpMessageNotReadableException` | `ApiJson` 统一转 400「请求体格式错误」 | 前端 `http.ts` 只认 `message` |
| 生成器 `formType` | 按 MySQL 列类型（`varchar` → 文本框） | SQLite 只有 `TEXT`，按 `text` → 文本域 | SQLite 无长度信息，无法区分 `varchar` / `text` |
| 生成器模板 | Java 文本块：`domain` / `mapper` / `service` / `controller` 四个 Java 类 + 前端 api / 页面 + 菜单 SQL | 按本项目 axum 分层生成 Rust：sea-orm 实体、领域模型、用例接口、仓储、服务、控制器 + 前端 api / 页面 + 菜单 SQL + 接入说明，共 10 个文件 | 本项目已迁到 axum，Java 产物放不进项目也用不上 |
| 生成器字段命名 | `javaField` / `javaType`（`Long` / `BigDecimal` / `LocalDateTime`） | `rustField` / `rustType`（`i64` / `i32` / `f64` / `String`） | 产物是 Rust，字段名与类型都按 Rust 语义 |
| 生成器业务名 | `sys_user` → `userRole`（camelCase） | `sys_user` → `user`（snake_case） | Rust 模块、函数与前端目录都用 snake_case |
| 生成器默认展开 | `<类名>Controller.java` | `backend/src/api/controllers/<表名>.rs` | 产物不再有类名，`activeFile` 取完整路径 |
| 生成器可空列 | 一律生成引用类型，空值靠约定 | `nullable` 列生成 `Option<T>` | 生成物要与数据库的 NULL 语义一致 |
| 生成器前端产物 | `frontend/src/api/<模块>/<业务名>.ts`、`views/<模块>/<业务名>/index.vue` | `frontend/src/api/<业务名>.ts`、`views/<模块>/<类名>.vue` | 与本项目前端现有布局（api 平铺、页面按模块 PascalCase）一致 |
| 生成器写操作 | 模板只有空类体，业务逻辑靠手工补 | 仓储的列表 / 计数与服务的列表查询已接好，写操作在仓储与服务层留 `todo!` | 不静默返回伪成功，缺失实现显式暴露 |
| 时间字段 | `DATETIME` | `TEXT` 本地时间字符串 | SQLite 无原生日期类型 |
| 留痕来源 IP | 操作日志按 `x-forwarded-for` → `x-real-ip` 取；登录日志不写 | 两者都取保护层按可信代理解出的真实对端（未命中可信代理时忽略转发头） | 转发头可被调用方伪造，日志里的来源必须可追溯；部署时需配置 `MIDDLEWARE_TRUSTED_PROXIES`，见 [生产部署](../../docs/DEPLOYMENT.md) |
| 登录日志客户端 | 只写账号与结果 | 另写 IP、地点、浏览器、操作系统 | 排查需要知道「谁从哪里用什么客户端登录」；`User-Agent` 用 woothee 规则表解析，版本号不写入 |
| 日志地点 | 无（只有 IP） | 「本机 / 内网 / 外网」加离线地名（如 `中国|广东省|茂名市|电信|CN`） | Spring 版不解析归属地，后台日志页只能看 IP；本地网段与保留地址段不查库，库文件与配置见 [生产部署](../../docs/DEPLOYMENT.md) |

## 测试

`tests/support/mod.rs` 提供装配装置（临时 SQLite + 内存会话、登录、带令牌请求、CSV / ZIP 原始响应读取）。
新增接口的契约测试：

- `tests/admin_api_tests.rs`：认证全流程（含登录 / 退出日志的来源与浏览器采集）、七个系统管理页面的 CRUD / 树 / 导出 / 权限 403 / 数据范围 / 404 / 405 文案。
- `tests/monitor_api_tests.rs`：留痕内容（含可信代理下的来源 IP）与脱敏、日志统计 / 时间线 / 热力图 / 导出 / 删除清空、工作台聚合。
- `tests/generator_api_tests.rs`：候选表与导入幂等、字段元数据、10 个生成文件的内容、ZIP 条目、留痕动作。
- `tests/generator_remove_tests.rs`：配置移除与重导、业务数据保留、旧 ID 隔离、并发及事务回滚、权限和审计失败边界。
- `tests/session_api_tests.rs`：本人身份边界、单目标撤销、独立失败限次、第二因素防重放、旧 JWT 与严格有效性过滤。
- `tests/account_fault_tests.rs` / `tests/session_redis_tests.rs`：会话存储失败的 503、证明消费不回滚、真实 Redis 类型故障与旧索引 TTL；真实 Redis 用例默认忽略。

测试要求与命令见 [测试与构建检查](TESTING.md)。
