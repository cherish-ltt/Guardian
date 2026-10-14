# 后台管理模块

> 本文档说明后台管理接口的契约：路由清单、权限模型、
> 留痕规则、存储映射与差异取舍。快速上手见 [README](../README.md)，分层约束见 [架构设计](ARCHITECTURE.md)。

请求前缀统一为 `/api/v1`。业务代码分布：

```
src/api/controllers/auth_controller.rs       登录、验证码、当前账号、动态路由、修改密码
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
| POST | `/auth/login` | 公开 | 校验码 → 口令 → 账号状态，返回 JWT 与 `expiresIn` |
| POST | `/auth/logout` | 登录态 | 删除 Redis 会话令牌，令牌随后失效 |
| GET | `/auth/me` | 登录态 | 账号、部门、角色、权限码 |
| GET | `/auth/routes` | 登录态 | 动态菜单树（只含 `dir` / `menu` 且可见） |
| PUT | `/auth/password` | 登录态 | 改密（校验旧密码、新旧不能相同、长度 6–64） |
| GET | `/auth/session` | 登录态 | 骨架自带的演示会话接口 |

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
| GET | `/system/dict/options/{dict_type}` | 同上 | 业务下拉（`def` 为 0/1，带 `tone`） |
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

`src/middleware/audit.rs` 挂在 `nest` 之后的路由层，按「请求方法 + 路径段模式（`:id` 通配）」匹配 31 条规则，
命中后记录模块与动作，例如 `POST /system/posts` → 「岗位管理 / 新增」、`POST /tool/generator/import` →
「代码生成 / 导入表」。

与 Spring `@OperLog` 切面的差异（有意为之）：

- 记录**请求体**（切面记录的是方法入参），`password` / `token` / `secret` / `authorization`
  的值替换为 `******`，超过 4000 字符截断。
- 认证或权限被拒（401 / 403）不写日志：这两种情况下请求根本没进业务方法。
- 留痕写入失败只 `error!` 记日志，不改变业务响应。
- 操作人只解 JWT，不再查库（处理器已完成完整鉴权）。

## 存储映射（MySQL → SQLite）

- 表结构见 `migrations/0002_create_system_tables.sql`：17 张表，列名与 MySQL 版一致。
- `DATETIME` 存 `TEXT`（`YYYY-MM-DD HH:MM:SS` 本地时间），字典序比较等价时间先后；
  查询用 `strftime` / `julianday` / `substr(x, 1, 16)` 等价 `NOW()` / `CURDATE()` / `DATE_FORMAT()`。
- 种子数据在 `src/infrastructure/databases/seed.rs`：11 个部门、9 个岗位、6 个角色、
  3 个用户及其角色 / 岗位关联、38 条菜单、3 类字典与 7 条字典项、3 条参数配置；
  全部 `INSERT OR IGNORE`，可重复启动。**不含**任何 `sys_oper_log` / `sys_login_log` 示例行（与 Spring 一致）。
- 种子账号与口令都来自 `.env`：`SEED_USERS`（`用户名:昵称:邮箱:部门ID`，逗号分隔）决定播种哪些账号，
  `SEED_ADMIN_PASSWORD`（默认 `admin123456`）是它们的初始口令，也是新增用户未传口令时的默认值；
  播种时用 argon2 现场哈希，没有照抄参考实现里的 BCrypt 字面量。

## 会话与验证码存储

`src/infrastructure/services/auth_store.rs` 定义 `AuthStore` 接口：

- `MemoryAuthStore`：集成测试用，进程内存 + TTL。
- `RedisAuthStore`：`CAPTCHA_KEY_PREFIX="captcha:"`、`TOKEN_KEY_PREFIX="auth:token:"`，TTL 300s；
  连接串与口令来自 `REDIS_URL` / `REDIS_PASSWORD`（`RedisConfig::connection_url()`）。

登录后 JWT 与 Redis 会话令牌必须同时有效：登出即删除令牌，旧令牌立刻失效（返回
「未认证或登录已过期」）。测试通过 `Container::bootstrap_with_auth_store(config, Container::memory_auth_store())`
装配，不依赖外部 Redis。

## 与 Spring 版的差异

| 项 | Spring 版 | 本实现 | 原因 |
| --- | --- | --- | --- |
| 路由前缀 | `/api` | `/api/v1` | 与骨架既有前缀统一；前端只需改 `http.ts` 默认前缀 |
| 路径参数 | `@PathVariable Long id` | `{dict_type}` 共用同名参数 | axum 要求同一位置参数同名，GET 传类型、PUT / DELETE 传主键 |
| 响应体 | `{code, message, data, traceId}` | 同前 + `timestamp` | `timestamp` 是骨架既有字段，前端不读 |
| 5xx 文案 | `系统内部错误` | 同前 | 4xx 文案逐字透出（「账号或密码错误」等） |
| 请求体非法 | Spring 抛 `HttpMessageNotReadableException` | `ApiJson` 统一转 400「请求体格式错误」 | 前端 `http.ts` 只认 `message` |
| 生成器 `formType` | 按 MySQL 列类型（`varchar` → 文本框） | SQLite 只有 `TEXT`，按 `text` → 文本域 | SQLite 无长度信息，无法区分 `varchar` / `text` |
| 生成器模板 | Java 文本块 | 逐字对齐的 Rust 字符串 | 集成测试按字节断言 7 个文件的内容 |
| 生成产物包名 | `com.scaffold.*` | 同前 | 生成器输出与参考实现逐字一致，不因本项目改名而漂移 |
| 时间字段 | `DATETIME` | `TEXT` 本地时间字符串 | SQLite 无原生日期类型 |
| 登录日志 IP | 不写 | 不写 | 保持与 Spring 一致；操作日志按 `x-forwarded-for` → `x-real-ip` 取 |

## 测试

`tests/support/mod.rs` 提供装配装置（临时 SQLite + 内存会话、登录、带令牌请求、CSV / ZIP 原始响应读取）。
新增接口的契约测试：

- `tests/admin_api_tests.rs`：认证全流程、七个系统管理页面的 CRUD / 树 / 导出 / 权限 403 / 数据范围 / 404 / 405 文案。
- `tests/monitor_api_tests.rs`：留痕内容与脱敏、日志统计 / 时间线 / 热力图 / 导出 / 删除清空、工作台聚合。
- `tests/generator_api_tests.rs`：候选表与导入幂等、字段元数据、7 个生成文件的内容、ZIP 条目、留痕动作。

测试要求与命令见 [测试与构建检查](TESTING.md)。
