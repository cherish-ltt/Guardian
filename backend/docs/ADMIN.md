# 后台管理模块

> 本文档说明后台管理接口的契约：路由清单、权限模型、
> 留痕规则、存储映射与差异取舍。快速上手见 [README](../README.md)，分层约束见 [架构设计](ARCHITECTURE.md)。

请求前缀统一为 `/api/v1`。业务代码分布：

```
src/api/controllers/auth_controller.rs       登录、验证码、当前账号、动态路由、修改密码
src/api/controllers/account_controller.rs    本人资料、2FA 绑定/恢复/关闭、管理员重置
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

### 认证（`/auth`）

| 方法 | 路径 | 权限 | 说明 |
| --- | --- | --- | --- |
| GET | `/auth/captcha` | 公开 | 4 位验证码，明文回显给登录页，Redis TTL 300s |
| POST | `/auth/login` | 公开 | 校验码 → 口令 → 账号状态；未启用 2FA 直接返回 JWT 与 `expiresIn`，启用则返回一次性挑战 |
| POST | `/auth/login/2fa` | 公开 | 用挑战 + 动态码/恢复码完成登录，签发 JWT 与会话 |
| POST | `/auth/logout` | 登录态 | 删除 Redis 会话令牌，令牌随后失效 |
| GET | `/auth/me` | 登录态 | 账号、部门、角色、权限码 |
| GET | `/auth/routes` | 登录态 | 动态菜单树（只含 `dir` / `menu` 且可见） |
| PUT | `/auth/password` | 登录态 | 改密（校验旧密码、新旧不能相同、长度 6–64） |
| GET | `/auth/session` | 登录态 | 骨架自带的演示会话接口 |
| GET | `/auth/profile` | 登录态 | 本人资料与 2FA 状态，无需菜单权限 |
| PUT | `/auth/profile` | 登录态 | 只修改昵称、手机、邮箱 |
| POST | `/auth/2fa/setup` | 登录态 + 密码 | 创建待确认绑定信息 |
| POST | `/auth/2fa/enable` | 登录态 + 密码 + 动态码 | 确认绑定，返回一次性恢复码 |
| POST | `/auth/2fa/disable` | 登录态 + 密码 + 第二因素 | 关闭 2FA |
| POST | `/auth/2fa/recovery-codes` | 登录态 + 密码 + 第二因素 | 替换全部恢复码 |

### 系统管理（`/system`）

| 方法 | 路径 | 权限 | 说明 |
| --- | --- | --- | --- |
| GET | `/system/users` | `system:user:list` | 分页列表，支持 `keyword` / `status` / `deptId` / `postId`，受数据范围约束 |
| GET | `/system/users/options` | 同上 | 表单下拉（角色、岗位、部门树、初始密码，只读权限返回空） |
| GET | `/system/users/export` | 同上 | `users.csv` |
| POST | `/system/users` | `system:user:add` | 新增（口令留空用 `sys.user.initPassword`） |
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
| POST / PUT / DELETE | `/system/dict/data`、`/system/dict/data/{dict_type}` | 同上 | `{dict_type}` 位置：GET 传类型，PUT / DELETE 传主键 |
| GET | `/system/configs` | `system:config:list` | 参数分页列表 |
| GET | `/system/configs/export` | 同上 | `configs.csv` |
| POST / PUT / DELETE | `/system/configs`、`/system/configs/{id}` | `system:config:add` / `edit` / `remove` | 内置参数不可改删，键名唯一 |

### 监控（`/monitor`）

| 方法 | 路径 | 权限 | 说明 |
| --- | --- | --- | --- |
| GET | `/monitor/oper-logs` | `monitor:operlog:list` | 分页列表（`keyword` / `result`） |
| GET | `/monitor/oper-logs/stats` | 同上 | 统计卡 + 趋势（`range=24h` 按小时、`7d` / `30d` 按天，缺省桶补零） |
| GET | `/monitor/oper-logs/timeline` | 同上 | 泳道时间线（同一模块相邻操作合并，间隔超过 900s 断开） |
| GET | `/monitor/oper-logs/export` | 同上 | `oper-logs.csv` |
| DELETE | `/monitor/oper-logs/{id}`、`/monitor/oper-logs` | `monitor:operlog:remove` | 删除单条 / 清空 |
| GET | `/monitor/login-logs` | `monitor:loginlog:list` | 分页列表（`keyword` / `status`） |
| GET | `/monitor/login-logs/stats` | 同上 | 统计卡 + 趋势 |
| GET | `/monitor/login-logs/heatmap` | 同上 | 7×24 热力格与异常告警（`days` 可选，阈值默认 5） |
| GET | `/monitor/login-logs/export` | 同上 | `login-logs.csv` |
| DELETE | `/monitor/login-logs/{id}`、`/monitor/login-logs` | `monitor:loginlog:remove` | 删除单条 / 清空 |

### 工作台与代码生成

| 方法 | 路径 | 权限 | 说明 |
| --- | --- | --- | --- |
| GET | `/dashboard/summary` | 登录态 | 6 张统计卡、登录趋势、近 7 天操作量、近期动态、角色分布 |
| GET | `/tool/generator/tables` | `tool:gen:list` | 已导入的生成配置（`keyword`） |
| GET | `/tool/generator/importable-tables` | 同上 | 数据库表候选（`imported` 标记是否已导入） |
| POST | `/tool/generator/import` | 同上 | 按 `tableNames` 导入，重复导入复用已有配置 |
| GET | `/tool/generator/tables/{id}/columns` | 同上 | 字段配置（`javaType` / `formType` / 增删改查开关） |
| GET | `/tool/generator/tables/{id}/preview` | 同上 | 7 个文件的代码预览（默认展开 Controller） |
| PUT | `/tool/generator/tables/{id}` | 同上 | 只接受 `tableComment` 与 `synced` |
| POST | `/tool/generator/tables/{id}/generate` | 同上 | 生成结果提示 |
| GET | `/tool/generator/tables/{id}/download` | 同上 | ZIP（条目为完整相对路径） |

### 其它

`GET /api/v1/health`（新增：`status` / `time` / `serviceName`）、骨架既有的
`GET /api/v1/system/health`、`GET /api/v1/system/ready`、示例与事务接口保持不变。

## 权限与数据范围

`src/api/extractors/admin_user.rs` 负责两件事：

- **权限码**：`AdminUser::require(code)` 命中 `super_admin` 直接放行，否则要求权限码在 `/auth/me`
  返回的集合里，缺失返回 403「没有访问权限」。权限码由「启用角色 → `sys_role_menu` → 启用菜单」推导。
- **数据范围**：`AdminUser::data_scope()` 解析启用角色的范围，`ALL` 不限；`DEPT_AND_CHILD` / `DEPT`
  展开为可见部门（`sys_dept.ancestors` 以逗号分隔的上级链，用 `(',' || ancestors || ',') LIKE '%,id,%'`
  替代 MySQL 的 `FIND_IN_SET`）；`CUSTOM` 取 `sys_role_dept`；其余按「仅本人」。用户列表、用户导出
  与权限关系图的用户节点都吃这套过滤。

## 留痕（审计）

`src/middleware/audit.rs` 挂在 `nest` 之后的路由层，按「请求方法 + 路径段模式（`:id` 通配）」匹配 37 条规则，
命中后记录模块与动作，例如 `POST /system/posts` → 「岗位管理 / 新增」、`POST /tool/generator/import` →
「代码生成 / 导入表」。

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
| `location` | 按 `ip` 推导：回环「本机」、私网与链路本地「内网」；其余查离线地名库（`vendor/ip2region`），命中写库里的原始串，未命中写「外网」 |
| `browser` / `os` | 请求 `User-Agent` 经 `src/util/user_agent.rs`（woothee 规则表）解析出的名称，识别不出时留空 |

`src/api/extractors/client_source.rs` 把来源收成一个提取器：控制器只需声明参数，
不自己读头或扩展；服务层签名收 `&ClientSource`，会话、验证码与事务边界都不受影响。

## 存储映射（MySQL → SQLite）

- 表结构见 `migrations/0002_create_system_tables.sql`：17 张表，列名与 MySQL 版一致。
- `DATETIME` 存 `TEXT`（`YYYY-MM-DD HH:MM:SS` 本地时间），字典序比较等价时间先后；
  查询用 `strftime` / `julianday` / `substr(x, 1, 16)` 等价 `NOW()` / `CURDATE()` / `DATE_FORMAT()`。
- 种子数据在 `src/infrastructure/databases/seed.rs`：11 个部门、9 个岗位、6 个角色、
  3 个用户及其角色 / 岗位关联、38 条菜单、3 类字典与 7 条字典项、2 条参数配置；
  全部 `INSERT OR IGNORE`，可重复启动。**不含**任何 `sys_oper_log` / `sys_login_log` 示例行（与 Spring 一致）。
- 种子账号与口令都来自 `.env`：`SEED_USERS`（`用户名:昵称:邮箱:部门ID`，逗号分隔）决定播种哪些账号，
  `SEED_ADMIN_PASSWORD`（默认 `admin123456`）是它们的初始口令，也是新增用户未传口令时的默认值；
  播种时用 argon2 现场哈希，没有照抄参考实现里的 BCrypt 字面量。
  每个账号各自哈希：即使共用同一口令，也各带随机盐，不会把一份 PHC 字符串复制给多个账号。

## 会话与验证码存储

`src/infrastructure/services/auth_store.rs` 定义 `AuthStore` 接口：

- `MemoryAuthStore`：集成测试用，进程内存 + TTL。
- `RedisAuthStore`：`CAPTCHA_KEY_PREFIX="captcha:"`、`TOKEN_KEY_PREFIX="auth:token:"`、`USER_KEY_PREFIX="auth:user:"`，TTL 300s；
  连接串与口令来自 `REDIS_URL` / `REDIS_PASSWORD`（`RedisConfig::connection_url()`）。

登录后 JWT 与 Redis 会话令牌必须同时有效：登出即删除令牌，旧令牌立刻失效（返回
「未认证或登录已过期」）。测试通过 `Container::bootstrap_with_auth_store(config, Container::memory_auth_store())`
装配，不依赖外部 Redis。

改密码会撤销该账号的全部会话（`AuthStore::revoke_user_tokens`）：Redis 侧按
`auth:user:<username>` 集合索引该账号已签发的令牌，一次清空集合与对应 `auth:token:*` 键，
其它账号的会话不受影响。因此改完密码后，当前会话与其它设备上的旧会话都需重新登录。

## 本人资料与二次认证

所有可登录账号均可自愿启用/关闭，不强制开启；仅登录增加第二因素，不改变既有改密契约。
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
  连续 5 次安全证明失败封禁安全验证 300 秒（数据库持久化）；管理员重置会清除目标封禁。
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

| 项 | Spring 版 | 本实现 | 原因 |
| --- | --- | --- | --- |
| 个人资料与 2FA | 本轮环境未提供参考目录，未核实上游对应契约 | 按本文新增本人接口、第二因素与独立重置权限 | 本次明确需求，非声称逐字兼容 |
| 基础数据缓存 | 本轮未核实上游缓存策略 | 单实例写屏障/换代、有界回源及空值缓存 | 本次明确的一致性及故障约束 |
| 验证码开关 | `sys.account.captchaEnabled` 参数控制是否校验验证码 | 无该参数，登录恒校验验证码 | 该参数在 Spring 版之外无人读取，留着就是假开关；需要关闭时直接改代码而不是改参数 |
| 路由前缀 | `/api` | `/api/v1` | 与骨架既有前缀统一；前端只需改 `http.ts` 默认前缀 |
| 路径参数 | `@PathVariable Long id` | `{dict_type}` 共用同名参数 | axum 要求同一位置参数同名，GET 传类型、PUT / DELETE 传主键 |
| 响应体 | `{code, message, data, traceId}` | 同前 + `timestamp` | `timestamp` 是骨架既有字段，前端不读 |
| 5xx 文案 | `系统内部错误` | 同前 | 4xx 文案逐字透出（「账号或密码错误」等） |
| 请求体非法 | Spring 抛 `HttpMessageNotReadableException` | `ApiJson` 统一转 400「请求体格式错误」 | 前端 `http.ts` 只认 `message` |
| 生成器 `formType` | 按 MySQL 列类型（`varchar` → 文本框） | SQLite 只有 `TEXT`，按 `text` → 文本域 | SQLite 无长度信息，无法区分 `varchar` / `text` |
| 生成器模板 | Java 文本块 | 逐字对齐的 Rust 字符串 | 集成测试按字节断言 7 个文件的内容 |
| 生成产物包名 | `com.scaffold.*` | 同前 | 生成器输出与参考实现逐字一致，不因本项目改名而漂移 |
| 时间字段 | `DATETIME` | `TEXT` 本地时间字符串 | SQLite 无原生日期类型 |
| 留痕来源 IP | 操作日志按 `x-forwarded-for` → `x-real-ip` 取；登录日志不写 | 两者都取保护层按可信代理解出的真实对端（未命中可信代理时忽略转发头） | 转发头可被调用方伪造，日志里的来源必须可追溯；部署时需配置 `MIDDLEWARE_TRUSTED_PROXIES`，见 [生产部署](../../docs/DEPLOYMENT.md) |
| 登录日志客户端 | 只写账号与结果 | 另写 IP、地点、浏览器、操作系统 | 排查需要知道「谁从哪里用什么客户端登录」；`User-Agent` 用 woothee 规则表解析，版本号不写入 |
| 日志地点 | 无（只有 IP） | 「本机 / 内网 / 外网」加离线地名（如 `中国|广东省|茂名市|电信|CN`） | Spring 版不解析归属地，后台日志页只能看 IP；本地网段与保留地址段不查库，库文件与配置见 [生产部署](../../docs/DEPLOYMENT.md) |

## 测试

`tests/support/mod.rs` 提供装配装置（临时 SQLite + 内存会话、登录、带令牌请求、CSV / ZIP 原始响应读取）。
新增接口的契约测试：

- `tests/admin_api_tests.rs`：认证全流程（含登录 / 退出日志的来源与浏览器采集）、七个系统管理页面的 CRUD / 树 / 导出 / 权限 403 / 数据范围 / 404 / 405 文案。
- `tests/monitor_api_tests.rs`：留痕内容（含可信代理下的来源 IP）与脱敏、日志统计 / 时间线 / 热力图 / 导出 / 删除清空、工作台聚合。
- `tests/generator_api_tests.rs`：候选表与导入幂等、字段元数据、7 个生成文件的内容、ZIP 条目、留痕动作。

测试要求与命令见 [测试与构建检查](TESTING.md)。
