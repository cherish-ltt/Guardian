# 更新日志

本项目的所有重要变更都会记录在本文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循 [语义化版本 2.0.0](https://semver.org/lang/zh-CN/)。

> `0.1.0` 是本项目（Guardian）的首个版本，架构骨架来自
> [AxumServiceScaffold](https://github.com/cherish-ltt/AxumServiceScaffold)。
> 下方 `0.5.0` 及更早的条目属于上游项目的历史记录，版本号与本项目不连续。

## [0.6.3] - 2026-10-21

统一前端所有下拉选择框：用户表单里的「岗位」「角色」多选下拉改为与「所属部门」同一种下拉，
浮层置顶不被弹窗裁切并带搜索；其他页面的下拉沿用同一套浮层与外观，搜索框按选项数自动开启。
分前后端的发布说明见 [docs/version/v0.6.3.md](../../docs/version/v0.6.3.md)。

### 变更

- 岗位 / 角色多选下拉改用与部门下拉同一套浮层：面板 `Teleport` 到 `body`、层级随所在弹窗、
  空间不足时向上展开，补齐搜索、方向键 / 回车 / Esc 与 `role="listbox"` 语义，仍是多选。
- `filterable` 改为 `true` / `false` / `'auto'` 三态，默认按选项数自动开启搜索框，
  阈值由 `src/components/ui/select-shared.ts` 的 `SELECT_SEARCH_THRESHOLD` 统一。
- 浮层定位抽到 `src/composables/popover.ts`，由 `Select` 与 `MultiCheck` 共用；
  左右位置夹在视口内，面板自身滚动不再触发重新摆位。
- 删除内联绝对定位的 `.multi-check-panel` / `.multi-check-option` / `.multi-check-caret` 等旧样式。

## [0.6.2] - 2026-10-20

修复字典「默认值」可以同时勾选多项的问题：同一字典类型只保留一个默认项，
存量多默认数据在升级时自动收敛。
分前后端的发布说明见 [docs/version/v0.6.2.md](../../docs/version/v0.6.2.md)。

### 修复

- 字典数据的创建 / 更新改在写事务内执行，写入前先清掉同类型其它项的默认标记，
  改类型时按更新后的最终状态收敛。
- 新增迁移 `0009_dict_default_unique.sql`：先把同类型多余的默认项降级为普通项
  （保留排序最靠前的一项），再建 `is_default` 部分唯一索引，绕过接口直写也会被拒绝。
- 前端字典数据表单补充唯一默认的说明，样本册行内设默认失败时重新拉取服务端数据恢复。

## [0.6.1] - 2026-10-20

修复 `0.6.0` 生成器的两个产物缺陷与代码预览的路径显示问题。
分前后端的发布说明见 [docs/version/v0.6.1.md](../../docs/version/v0.6.1.md)。

### 修复

- 生成物的仓储补齐 `impl XxxRepository`：`count_*` / `list_*` 按字段配置生成可执行的 SQL，
  `row_of` 构造列表行。此前只生成 trait 与空结构体，服务层却直接调用，生成物必然编译失败。
- 主键不再被当成可空列（SQLite 对 `INTEGER PRIMARY KEY` 报 `notnull=0`），
  新增迁移 `0008_gen_column_primary_not_null.sql` 纠正已导入配置；`0007` 保持发布时的内容。
- 前端代码预览的文件分组标题改为自动换行，长路径不再截断。

## [0.6.0] - 2026-10-20

代码生成器改为产出本项目的 axum + sea-orm 分层骨架，不再生成 Spring/Java 代码。
分前后端的发布说明见 [docs/version/v0.6.0.md](../../docs/version/v0.6.0.md)。

### 变更

- 生成器 7 个 Java 模板换成 10 个产物：sea-orm 实体、领域模型、`XxxUseCase`、仓储接口、
  用例实现、axum 控制器、前端 api / 页面、菜单 SQL、接入说明。写操作在服务层留 `todo!`。
- `gen_table_column` 的 `java_field` / `java_type` 改名为 `rust_field` / `rust_type`，
  类型值改为 Rust 类型；迁移 `0007` 转换存量行。
- 业务名从 camelCase 改为 snake_case，影响生成产物路径。
- `GenPreview.activeFile` 返回默认展开文件的完整路径，不再是文件名。
- `docs/ADMIN.md` 与 `AGENTS.md` 的生成器契约条款同步更新。

## [0.5.2] - 2026-10-19

对 `0.5.0` 与 `0.5.1` 的后端改动做了一轮复核，修掉两个安全与数据一致性缺陷。
分前后端的发布说明见 [docs/version/v0.5.2.md](../../docs/version/v0.5.2.md)。

### 安全

- 重置口令时在同一事务内递增 `auth_version`。此前只删会话存储里的令牌，
  Redis 清理失败或令牌索引缺失时旧 JWT 仍能用到 TTL 到期。
- 新增 `AccountRepository::bump_auth_version` 只更新单列，避免 `save_security`
  整行覆盖把并发写入的其他安全字段写回旧值。

### 修复

- `dataScope=CUSTOM` 的 `deptIds` 校验部门存在性，不再向 `sys_role_dept`
  写入不存在的部门 ID 留下悬空关联。
- 补验证码字模完整性测试：字形缺失时渲染会静默跳过该字符。
- 修正 `docs/ADMIN.md` 与 `auth_controller` 里「验证码明文回显」的过时描述。

### CI

- 覆盖率步骤合并为一次 `cargo llvm-cov`，不再重复跑 `cargo test`。
- `scripts/check-coverage.py` 对缺文件、非法 JSON、结构不符给出明确报错。

## [0.5.1] - 2026-10-19

修复 `0.5.0` 前端产物里的一个回归缺陷。只改了前端，后端代码无变动，版本号按约定同步递增。
分前后端的发布说明见 [docs/version/v0.5.1.md](../../docs/version/v0.5.1.md)。

### 修复

- `FormDialog.vue`、`SearchForm.vue`、`NoAccess.vue` 三处的 `import` 被误放到
  `<script setup>` 之外，import 不生效导致 `Modal` / `Button` / `EmptyState` 未注册：
  所有列表页底部漏出整块未受控的表单（关不掉），查询区按钮点击无效，无权限页内容空白。
- 类型检查与 lint 对这类错误不敏感，新增 `src/components/__tests__/sfc-structure.test.ts`
  扫描全部 `.vue`，断言 `import` / `export` 都在 `<script>` 块内。

## [0.5.0] - 2026-10-19

本次按生产级标准补齐安全、数据一致性与前端稳定性上的缺口，并给 CI 加了覆盖率护栏。
对外行为有若干不兼容变更，分前后端的发布说明见 [docs/version/v0.5.0.md](../../docs/version/v0.5.0.md)。

### 不兼容变更

- `GET /auth/captcha` 不再返回 `code`，改为 `image`（base64 SVG）。
- `POST /system/users/{id}/reset-password` 的 `password` 改为必填，不再有内置默认口令。
- 列表接口的 `pageNum` / `pageSize` 非法值返回 `400`，不再静默回落默认值。
- `/examples/*` 只在 debug 构建注册，release 包访问返回 `404`。
- 新增用户未传口令时改用 `sys.user.initPassword`，参数为空才回退 `SEED_ADMIN_PASSWORD`。

### 安全

- 验证码改为服务端生成内联 SVG，明文只留在会话存储里比对，随机源换成系统随机数。
- 登录密码阶段接入失败计数：连续 5 次错误锁定 5 分钟；账号不存在时也付出一次 Argon2 成本。
- 删除硬编码重置口令，新口令必填；管理员重置他人口令后撤销该账号全部会话。
- `PUT /system/users/{id}` 补上「内置管理员不允许停用」校验。

### 修复

- 用户 / 角色 / 岗位的唯一索引改为 `WHERE deleted = 0` 的部分索引（新增 `0006` 迁移），
  软删后重建同名记录不再被数据库拦成 409。
- CUSTOM 数据权限补上写入能力：`RoleWriteCommand` 增加 `deptIds`，角色仓储新增
  `sys_role_dept` 的读与全量替换；删除角色一并清理。
- 新增或编辑用户时在事务内校验 `roleIds` / `postIds` 存在，失败整体回滚。
- 新增用户口令改为先读 `sys.user.initPassword`，与文档和前端提示一致。
- `/system/health` 的 OpenAPI 去掉不存在的 503 分支。
- 分页参数与过滤参数统一错误语义；移除从未被使用的 `sortField` / `sortOrder`。
- 示例接口按 debug 构建门控，release 不再暴露 `/examples/*`。

### CI 与构建

- musl 目标只设 `CC_<target>=musl-gcc`，去掉 `CARGO_TARGET_<TARGET>_LINKER=musl-gcc`：
  后者会让产物变成 dynamic-pie，运行到 musl 的 `_start_c` 段错误；打包前新增静态链接校验。
- `release.yml` 的 Rust 工具链固定 `1.99.0`，与 `backend.yml` 一致。
- `backend.yml` 的 paths 统一成 `backend/**/*.rs` 风格，并补 `backend/vendor/**/*.rs`。
- 新增 `scripts/check-coverage.py`，CI 卡全局行覆盖率 80% 门槛。

## [0.4.1] - 2026-10-18

本次让后端产物明确区分 glibc 与 musl 运行时，共四个下载包，并简化了包内结构。
分前后端的发布说明见 [docs/version/v0.4.1.md](../../docs/version/v0.4.1.md)。

### 变更

- **产物按 libc 区分**：产物名由 `guardian-backend-<tag>-linux-<arch>` 改为带 libc，
  共四个包：`x86_64-linux-gnu`、`aarch64-linux-gnu`、`x86_64-linux-musl`、`aarch64-linux-musl`。
  gnu 包动态链接 glibc，musl 包静态链接；此前未标明运行时，下载者无法判断能否在 Alpine 上运行。
- **包内结构**：二进制由 `target/release/guardian-backend` 改为根位置的 `guardian-backend`，
  与 `.env-example` 并列，不再附带 `migrations/`（迁移 SQL 已由 `sqlx::migrate!` 编译期嵌入）。
- **版本号统一到 `0.4.1`**：`backend/Cargo.toml`、`frontend/package.json`、
  `frontend/package-lock.json`、根 README 徽章与 `src/api/dto/system.rs` 的 OpenAPI 示例同步。

### 构建

- 四个目标都在对应架构的原生 runner 上编译，musl 目标的 C 工具链来自 apt 的 `musl-tools`
  （noble 的 amd64 与 arm64 都有）；不能用 `aarch64-linux-gnu-gcc` 充当 musl 链接器
  （glibc 工具链，会因缺 `stat64` 等符号链接失败）。

## [0.4.0] - 2026-10-18

本次让日志 `location` 列对 IPv6 来源也能给出地名，并统一前后端版本号。
分前后端的发布说明见 [docs/version/v0.4.0.md](../../docs/version/v0.4.0.md)。

### 新增

- **地名解析支持 IPv6**：`IpLocation` 由单个查询器改为 IPv4 / IPv6 各一个，按来源 IP 的协议
  分别查询离线库；新增 `IP_LOCATION_V6_DB_PATH` 配置项，与 `IP_LOCATION_DB_PATH` 并列且互相独立。
  只配其中一个时另一个协议的来源仍按本地网段与「外网」判定。
- **库版本与配置不符时启动失败**：加载库时用一条本机地址探测一次，库头记录的 IP 版本与
  配置的变量不符直接拒绝启动，不会静默退回「外网」。

### 变更

- **版本号统一到 `0.4.0`**：`backend/Cargo.toml`、`frontend/package.json`、
  `frontend/package-lock.json`、根 README 徽章与 `src/api/dto/system.rs` 的 OpenAPI 示例同步。
- **启动配置日志**：`ip_location` 组新增 `ip_location_v6_db_path`。

### 测试

- `tests/config_tests.rs` 的路径用例同时覆盖两个变量（含空白视为未配置），
  `src/infrastructure/ip_location.rs` 的失败用例分别覆盖 v4 与 v6 库路径。

## [0.3.0] - 2026-10-18

本次把登录验证码开关做成真正生效的功能，并统一前后端版本号。
分前后端的发布说明见 [docs/version/v0.3.0.md](../../docs/version/v0.3.0.md)。

### 新增

- **登录验证码开关**：`sys.account.captchaEnabled` 重新种入 `sys_config`（`builtin = 0`，
  可在「参数配置」页直接开关）。关闭时 `GET /api/v1/auth/captcha` 返回 `enabled = false`
  且不生成验证码、不写 Redis，`POST /api/v1/auth/login` 跳过验证码校验；
  参数缺失时按开启处理，不会因读不到参数而放宽登录。`CaptchaChallenge` 相应增加
  `enabled` 字段，登录页据此隐藏验证码输入框。

### 变更

- **版本号统一到 `0.3.0`**：`backend/Cargo.toml`、`frontend/package.json`、根 README 徽章
  与 `src/api/dto/system.rs` 的 OpenAPI 示例同步。

### 测试

- `tests/admin_api_tests.rs` 新增 `captcha_is_enabled_by_default` 与
  `disabled_captcha_skips_verification`，后者同时断言关闭后口令仍被校验；
  `config_flow_and_export` 的参数总数断言改为 3 条。
- 前端 `src/api/__tests__/auth.test.ts` 新增关闭态用例，断言 `captcha()` 返回值与
  登录请求体不包含验证码字段。

## [0.2.0] - 2026-10-17

本次把上游 `v0.7.0`（客户端保护、低噪声诊断与版本可追溯）的功能原样同步进来，
并在同一版本内补齐后台二次认证、个人资料页与基础数据缓存。

### 新增

- **登录日志采集客户端来源**：每次登录尝试与退出都写入 `sys_login_log` 的 `ip` / `location` /
  `browser` / `os`。IP 取保护层按可信代理链解析出的真实对端（与操作日志同源，不再采信请求头原值），
  地点按「本机 / 内网 / 外网」三档判定（回环、RFC1918 私网与链路本地、其余），
  浏览器与操作系统由新增依赖 `woothee` 从 `User-Agent` 解析、识别不出时留空。
  登录 / 退出用例签名改为收 `&ClientSource`，控制器通过新提取器
  (`src/api/extractors/client_source.rs`) 取得来源。
- **生产部署文档**：新增 [docs/DEPLOYMENT.md](../../docs/DEPLOYMENT.md)，覆盖直连 / Nginx /
  CDN 回源直连 / CDN + Nginx 四种形态下的真实客户端 IP 获取与防伪造要点、验证方法、
  环境变量、启动与进程管理、前端构建与静态托管、安全边界、日志与磁盘、备份与上线清单。
- **按真实来源 IP 的客户端保护**（`src/middleware/client_ip.rs`、`client_limits.rs`）：
  进程级单 IP 并发与固定窗口请求额度，跨路由共享；有界注册表（满时新来源 `503`）、空闲清理、
  取消释放许可；速率拒绝带 `Retry-After`。
- **显式可信代理**：只有直连对端命中 `MIDDLEWARE_TRUSTED_PROXIES` 时才采信 `X-Forwarded-For`，
  从右向左剥离可信代理，IPv4 映射 IPv6 归一化，重复或非法转发头返回 `400`；缺少 `ConnectInfo` 时
  失败关闭（`500`）。单值 `*` 信任全部对端，仅限本地调试（生产环境拒绝启动）。
- **请求观测与失败隔离**（`src/middleware/diagnostics.rs`）：每请求一条访问或诊断日志，含路由模板、
  排队耗时与数据库阶段耗时；总超时覆盖全局排队；`CatchPanicLayer` 把请求 panic 隔离成安全 500；
  框架生成的纯文本 400/404/405/413/415/422 归一化成统一 JSON（405 保留 `Allow`）。
- **生产启动守卫**：`APP_ENV=production` 时拒绝 debug 构建（`AppConfig::validate_runtime`，
  `main` 与 `create_app` 双入口）；`HTTP` 监听改用 `into_make_service_with_connect_info`。
- **阶段耗时标记**：事务示例与就绪检查的关键 IO 用 `middleware::measure("固定阶段名", future)` 包裹。
- **本人资料与二次认证**：新增 `migrations/0003_user_security.sql`（`sys_user_security`：
  加密有效 / 待确认 TOTP 密钥、恢复码哈希、会话版本、失败计数与封禁截止）与
  `0004_login_challenge.sql`（一次性登录挑战）。可登录账号自愿启用认证器
  （SHA1 / 6 位 / 30 秒），每次启用生成 10 枚一次性恢复码；密钥用独立 `TWO_FACTOR_KEY`
  做 AES-256-GCM 加密并以账号 ID 作 AAD，数据库只有密文与哈希。
- **登录拆为两步**：`POST /auth/login` 通过验证码与口令后，未启用 2FA 直接返回 JWT；
  启用则只返回 300 秒一次性挑战（`twoFactorRequired` / `challenge`），**不签发半会话**；
  `POST /auth/login/2fa` 校验动态码或恢复码后才签发 JWT 与会话。挑战只存 SHA256，
  成功、被拒、过期、密码变更或 2FA 重置后立即失效；失败 5 次封禁安全验证 300 秒。
- **本人资料接口**：`GET/PUT /auth/profile` 只允许改昵称、手机、邮箱（`deny_unknown_fields`，越界字段 400），
  账号、部门、角色只读；配合前端新增的静态受保护页 `/profile`。
- **人工重置 2FA**：新增独立权限码 `system:user:reset2fa`（迁移中一次性授予 `super_admin` 与
  已有 `system:user:resetPwd` 的角色，之后独立分配），`POST /system/users/{id}/2fa/reset`
  要求操作者输入本人密码，禁止重置自己与非超管重置超管。
- **基础数据缓存**（`src/services/hot_cache.rs`、`src/infrastructure/services/cache_store.rs`）：
  部门树、管理菜单树、用户选项中的部门、字典类型 / 数据 / 业务选项、参数列表走
  API → Redis → SQLite。键含进程随机命名空间、数据代数与查询的 SHA256，TTL 加 0–20% 抖动，
  空结果短缓存、错误不缓存，键数与单值大小有上限；64 个分片锁合并并发未命中，
  回源并发由 semaphore 限制、超量返回 503。用户权限、`/auth/routes` 定制菜单、统计聚合与导出不缓存。

### 变更

- **删除 `sys_file` 表**：该表在 0002 里建好后没有任何 controller / service / repository 引用，
  也没有上传接口，留着只会让人以为文件模块存在。用追加迁移 `0005_drop_sys_file.sql` 删除，
  不改 0002（改已发布的迁移会让已应用它的库 checksum 不匹配而拒启）。
- **日志地点改为分级解析**：`location` 不再只有「本机 / 内网」两档。回环记「本机」，RFC1918 私网、
  链路本地与 IPv6 ULA 记「内网」，其余地址查可选的离线地名库（ip2region xdb，`IP_LOCATION_DB_PATH`），
  命中写入库里的原始串（如 `中国|广东省|茂名市|电信|CN`），未命中记「外网」。
  库文件不随仓库分发，未配置时地点只有三档，配置了却加载不了会启动失败。
  解析实现 vendored 在 `vendor/ip2region`（官方 Rust 客户端，锁提交 `1bf6cab`，去掉会拖入 clap 等
  约 70 个包的 `maker` 依赖）。
- **留痕来源改为可信解析**：`sys_oper_log.oper_ip` 与 `sys_login_log.ip` 都改用保护层解析出的
  真实对端，不再直接取 `X-Forwarded-For` / `X-Real-IP`（与 Spring 版的差异已记入
  [后台管理模块](ADMIN.md)）；因此部署在代理后必须正确配置 `MIDDLEWARE_TRUSTED_PROXIES`。
- 中间件栈重排：新增客户端识别、请求观测与总超时、panic 隔离、客户端保护与诊断上下文；
  移除 `TraceLayer` 与 `TimeoutLayer`（访问日志与超时改由请求观测层承担），因此
  `tower-http` 不再需要 `trace` / `timeout` feature，新增 `catch-panic`、`tower/util`（`map_response`）、
  `axum/matched-path`（路由模板）与 `ipnet`（可信网段）。
- `MiddlewareConfig` 新增 `trusted_proxies` 与 6 项客户端/诊断参数，均走 `.env`；
  新增校验：容量非 0、`MIDDLEWARE_CLIENT_IDLE_SECS` 不小于限流窗口、可信代理条目合法、
  生产环境不得信任全部对端；启动配置快照同步输出这些字段。
- 新增错误文案遵循后台契约：4xx 用中文原样透出（如「请求过于频繁」「请求头 X-Forwarded-For 格式错误」），
  5xx 统一「系统内部错误」；不照搬上游的英文文案。
- `traceId` 改为由请求观测层解析一次并放进请求上下文，`x-trace-id` 响应头覆盖包括限流、
  背压与 panic 在内的全部响应；响应体 `traceId` 仍由压缩层之内的注入层改写，异常点已记入
  [统一响应结构](RESPONSE.md)。
- **缓存一致性写法**：基础数据写操作先取得写屏障并递增代数，屏障覆盖数据库修改；
  读锁覆盖读缓存、回源与回填，避免并发旧读回填。因此单实例内「API 修改成功后的后续请求」
  不读旧值；旧键靠 TTL 清理，不依赖删除 Redis 成功。多实例或在线直接改库不在保证范围内。
- **缓存故障降级**：Redis 业务缓存超时 / 错误 / 坏数据触发短暂熔断，在会话认证已通过的前提下
  回源 SQLite；认证、验证码与会话仍强制经过 `AuthStore`，Redis 整体不可用时不会绕过校验。
- **会话版本**：`AccessClaims` 增加带默认值的 `auth_version` 与 `jti`（UUID，避免同秒签发相同 JWT），
  认证时比对数据库版本；启用 / 关闭 2FA、更新恢复码与人工重置都会递增版本。
  提交后清理 Redis 旧令牌失败只记告警，旧令牌仍由版本校验拒绝，不影响返回一次性恢复码。
- 审计脱敏新增 `secondFactor` / `recovery`，无效 JSON 不再原样写入留痕；`/api/v1/auth/*`
  响应统一带 `Cache-Control: no-store`。
- **启动配置快照覆盖全部字段**：`log_startup_config` 新增 `[two_factor]`（只输出密钥是否已配置）、
  `[cache]`、`[seed]`（口令脱敏 + 各账号用户名 / 昵称 / 邮箱 / 部门）与 `[ip_location]` 分节，
  并在 `[middleware]` 补 `cors_allowed_origins`。`tests/logging_interval_tests.rs` 从 `AppConfig`
  的 `Debug` 输出提取全部字段名逐个断言，新增字段漏打印会直接测试失败。
- 新增依赖 `totp-rs`（`base32` / `constant_time_eq` 随 lock 引入），加密复用既有 `aws-lc-rs` 与 `base64`；
  `AppConfig` 新增 `two_factor_key` 与 `CacheConfig`（TTL、负缓存 TTL、IO 超时、回源上限、键数上限、
  单值上限），`.env-example`、`.env` 与 `tests/config_tests.rs` 同步。
- 版本号提升到 `0.2.0`（`Cargo.toml` / `Cargo.lock` / OpenAPI 版本示例）。

### 测试与文档

- 新增 `tests/client_protection_tests.rs`（代理防伪、额度隔离、跨路由共享、取消释放）；
  `tests/admin_api_tests.rs` 与 `tests/monitor_api_tests.rs` 补登录 / 退出日志的来源、地点、
  浏览器与操作系统采集断言（含可信代理下的真实 IP 与不可信转发头不被采信）；
  `tests/middleware_tests.rs` 补含排队的超时、panic 隔离、HEAD 无响应体、非法请求 ID 替换；
  `tests/middleware_log_tests.rs` 补低噪声日志、路由模板、query 不泄漏与池占满时的
  `db.ready` 中断诊断；`tests/config_tests.rs` 补可信代理与客户端参数分支；
  集成测试统一注入 `ConnectInfo`。
- 新增 `tests/config_tests.rs` 的 `IP_LOCATION_DB_PATH` 分支与 `src/infrastructure/ip_location.rs`
  的单测（本地网段、公网回退、保留地址段、库不可用时报错）。
- 新增 `tests/account_api_tests.rs`（资料白名单、2FA 生命周期、动态码重放、恢复码并发单次消费、
  会话撤销与日志脱敏、人工重置权限与密码校验、失败限次）、`tests/account_security_tests.rs`
  （密文存储与绑定过期、错误动态码不启用、`0003` 权限迁移只授予一次）、
  `tests/account_fault_tests.rs`（Redis 清理失败仍返回恢复码、旧令牌被版本拒绝、缓存故障降级）、
  `tests/login_challenge_tests.rs`（第一步无 JWT、挑战一次性、过期 / 密码变更 / 限次、并发只成功一次）
  与 `tests/cache_tests.rs`（并发未命中只回源一次、写屏障、降级与 503、空值缓存、真实 Redis 用例默认忽略）。
- `tests/contract_tests.rs` 端点矩阵扩到 91 条（新增 `POST /auth/login/2fa` 等公开与登录态端点），
  `tests/admin_api_tests.rs` 的菜单计数随新增按钮同步。
- 适配 `docs/MIDDLEWARE.md`（栈顺序、配置表、部署与生产边界）、`docs/LOGGING.md`
  （低噪声请求诊断）、`docs/ADMIN.md`（登录日志字段来源与差异表）、`docs/TESTING.md`、
  根 `README.md` 与 `backend/README.md`（部署文档入口）、`AGENTS.md`（新增 10.14 / 10.15 /
  10.16 / 10.17）；`docs/ADMIN.md` 补两步登录契约、本人接口、重置权限与缓存一致性边界，
  `docs/DEPLOYMENT.md` 补 `TWO_FACTOR_KEY` 配置 / 备份 / 唯一管理员丢失第二因素的线下恢复与
  缓存降级参数，`docs/TESTING.md` 更新端点计数、用例清单与覆盖率快照。

## [0.1.0] - 2026-10-14

### 新增

- **后台管理模块**：把 `scaffold-master`（Spring Boot + MyBatis + MySQL）的后台接口按同一契约重写到
  axum + sea-orm 上，前端 `scaffold-vue` 不改业务逻辑即可跑通。包含：
  - 认证：图形验证码、登录、登出（会话撤销）、当前账号、动态路由、修改密码；
  - 系统管理：用户（含状态变更、重置口令、数据范围过滤）、角色（含授权菜单、权限关系图）、
    菜单树、部门树（`ancestors` 上级链）、岗位、字典类型 / 字典数据 / 业务下拉、参数配置，
    各页面均支持 CSV 导出；
  - 监控：操作日志与登录日志的列表、统计、时间线、热力图、导出、删除与清空；
  - 工作台：统计卡、登录趋势、近 7 天操作量、近期动态、角色分布；
  - 代码生成：候选表、导入、字段配置、代码预览、生成与 ZIP 下载（产物为本项目的 axum + sea-orm 分层骨架，
    见 [v0.6.0 发布说明](../../docs/version/v0.6.0.md)）。
- **迁移与实体**：`migrations/0002_create_system_tables.sql` 建 17 张系统表，`src/entities` 补齐对应实体，
  `src/infrastructure/databases/seed.rs` 幂等播种部门 / 岗位 / 角色 / 菜单 / 字典 / 参数与演示账号。
- **会话与验证码存 Redis**：`AuthStore` 抽象 + `MemoryAuthStore`（测试）与 `RedisAuthStore`（生产），
  新增 `REDIS_URL` / `REDIS_PASSWORD` 配置；登录要求 JWT 与 Redis 会话令牌同时有效。
- **`traceId`**：响应体与响应头注入追踪 ID，沿用请求头 `x-trace-id`，CSV / ZIP 只加响应头。
- **写操作留痕**：`src/middleware/audit.rs` 按「方法 + 路径段模式」匹配 31 条规则，
  记录请求体（敏感字段脱敏）、操作人、耗时与错误文案。
- 接口契约测试：`tests/admin_api_tests.rs`、`tests/monitor_api_tests.rs`、`tests/generator_api_tests.rs`
  与公共装置 `tests/support/mod.rs`。
- 启动配置日志（`src/logging.rs` 的 `log_startup_config`）在 `app_name` / `app_env` 之后增加 `version` 行，
  取值 `env!("CARGO_PKG_VERSION")`（编译期取自 `Cargo.toml` 的 `version`），发版改 `Cargo.toml` 即自动生效。
- 改密码撤销该账号的全部会话（`AuthStore::revoke_user_tokens`）：Redis 侧新增
  `auth:user:<username>` 集合索引已签发的令牌，一次清空索引与对应 `auth:token:*` 键，
  其它账号的会话不受影响；前端在改密成功后同步清理本地会话并回登录页。

### 修复

- **查询串解码在 `%` 后接多字节字符时 panic**：`QueryString::parse` 的 `decode` 用
  `&value[index + 1..index + 3]` 做 `str` 区间切片取两位十六进制文本，`%` 后面若是多字节
  字符（请求行里出现字面非 ASCII 字节，如 `?keyword=%中`），`index + 3` 会落在该字符的
  字节区间内部，切片直接 panic。`decode` 服务于全部 `RawQuery` 端点，且 `oneshot` 之外
  的真实服务进程会因 handler panic 中断该请求，因此任意后台列表与导出接口都能被一条
  构造的查询串打崩。现改为逐字节解析：用 `hex_digit` 对后两个字节逐个判定十六进制，
  合法才消费 3 个字节，否则保留字面 `%`；输出端仍走 `from_utf8_lossy`，
  非法字节序列替换而不是崩溃。
- **播种账号共用同一份口令哈希**：`seed_users` 原先在循环外只哈希一次，多个账号会写入
  逐字相同的 PHC 字符串（含相同盐）。现改为每个账号各自哈希，盐彼此独立；
  当前所有播种账号本就共用 `SEED_ADMIN_PASSWORD`，不存在越权，但改为按账号独立加盐后，
  以后给账号配不同口令也不会静默写错。

### 变更

- **依赖瘦身与 feature 收紧**：去掉全部在 `src/`、`tests/` 里零调用点的依赖
  （`preset` bundle 的 `rayon` / `validator` / `regex` / `base64` / `once_cell` / `rand` /
  `derive_more` / `itertools`，与 `http-client` bundle 的 `reqwest`），并把「聚合 feature 全开」
  改为按实际调用面逐项列出：
  - `tokio`：`full` → `macros` / `net` / `rt-multi-thread` / `signal` / `time`；
  - `axum`：`default-features = false`（丢掉 `form` / `matched-path` / `original-uri` / `tower-log`），
    只留 `http1` / `json` / `query` / `tokio` / `tracing`；
  - `sea-orm`：`default-features = false`（丢掉 `with-json` / `with-chrono` / `with-uuid` /
    `with-time` / `with-rust_decimal` / `stream`），只留 `macros` / `runtime-tokio` /
    `sqlx-sqlite` 与 `sqlite-use-returning-for-3_35`；`runtime-tokio-rustls` 改 `runtime-tokio`
    （只支持 SQLite，TLS 后端是纯开销），顺带把 `rustls` / `ring` / `webpki-roots` 移出依赖图；
  - `chrono`：去掉未被使用的 `serde` feature。

  默认依赖图由 243 个 crate 降到 227 个（减 16 个）——其中 `rustls` / `ring` / `webpki-roots` /
  `rustls-webpki` 与 `rust_decimal` / `ouroboros` / `arrayvec` 都是上面两处「默认集全开」带进来的。
- **release 产物优化**：`[profile.release]` 新增 `lto = true` 与 `codegen-units = 1`，均不改变业务语义
  （代价是 release 构建由约 1m 变为约 3.5m；CI 只跑 debug 构建与 `cargo check --release`，不受影响）。
  未加 `strip`，保留符号表以便生产 panic 能打出可读栈帧。
  两项合并后 release 二进制由 19.83 MB 降到 14.09 MB（−28.9%，已小于原先 `strip` 后的 14.92 MB）。

  `AGENTS.md` 第 3 节、第 5 节、10.2 与新增的第 10.12 / 10.13 节，以及 `README.md`、
  `docs/DEVELOPMENT.md`、`docs/TESTING.md` 同步说明。
- 项目定名为 Guardian：crate 更名 `guardian-backend`（版本 `0.1.0`），前端包更名 `guardian-frontend`，
  默认数据库文件为 `guardian.db`，日志与响应中的服务名、JWT 签发者与受众同步调整，
  前端 `localStorage` 键前缀由 `scaffold.` 改为 `guardian.`。
- 演示账号改为从 `.env` 读取：`SEED_USERS`（`用户名:昵称:邮箱:部门ID`，逗号分隔）决定播种哪些账号，
  `SEED_ADMIN_PASSWORD`（默认 `admin123456`）是它们的初始口令、也是新增用户未传口令时的默认值；
  两者都不再写死在迁移文件与种子 SQL 中，解析失败启动即拒。角色与岗位按账号顺序分配，
  超过三个的账号不分配。README 与 AGENTS.md 同步列出生产环境的数据清理清单，
  `sys.user.initPassword` 仍保留上游的内置值。
- 演示数据改为 Guardian 命名：组织为 `Guardian 科技集团` 及其下属研发 / 产品 / 职能中心与 8 个部门，
  演示账号为 `admin`（陈立诚）、`linjh`（林嘉豪）、`heyj`（何雅静），部门负责人姓名一并替换；
  前端品牌文案与版权改为 Guardian。
- 后台接口统一使用 `/api/v1` 前缀（与骨架既有约定一致），前端 `http.ts` 默认前缀随之调整。
- 前端：页面与登录页展示的版本号由 `vite.config.ts` 的 define 注入，取自 `package.json` 的 version，
  不再手写常量。
- `AppError` 增加 `Business`（400，文案原样透出）、`Forbidden`（403）、`SessionExpired`（401）；
  5xx 统一文案为「系统内部错误」。
- 仓储层分页参数由 `offset` / `limit` 两个标量改为领域对象 `Page`，同时满足 clippy 的
  参数数量阈值（`.clippy.toml` 的阈值不调高，改代码）。

### 移除

- 移除上游的旧品牌文档资产（`docs/assets/logo.png`、`docs/assets/flow-1.png` 与 `docs/live-panel/` 动效面板），
  README 改用文本描述架构与流程。
- 移除三处无调用点的死角代码（不影响行为）：`repositories/sql.rs` 的 `float` 与 `flag`、
  `util/time.rs` 的 `today`、`repositories/permission_graph.rs` 的 `GRAPH_MENU_SELECT`
  与 `map_graph_menu`。其中 `GRAPH_MENU_SELECT` 是 `repositories/menu.rs` 里 `MENU_SELECT`
  的陈旧副本（少了 `m.component`、多了 `ORDER BY`），`map_graph_menu` 也已被
  `services/admin/graph.rs` 从领域模型 `MenuFlatRow` 直接构造 `GraphMenu` 取代。

### 测试

- **端点契约矩阵**：新增 `tests/contract_tests.rs`，把 Router 上注册的全部 83 个 `(方法, 路径)`
  按「需要权限码 64 个 / 只需登录 10 个 / 完全公开 9 个」三张表逐条锁定，表格驱动断言：
  - 74 个受保护端点缺令牌一律 401 + 「未认证或登录已过期」；脚手架自带、走 `CurrentUser`
    的 4 个接口（`/auth/session`、`/examples/{id}`、`/transactions`、`/transactions/{id}`）
    保留英文原文 `Unauthorized: missing Authorization header`，两套提取器的文案差异被显式固定下来。
  - 64 个权限码端点用零权限账号（种子里的 `linjh`）访问一律 403 + 「没有访问权限」。
  - 超管令牌不会被鉴权层拦下（证明表里的路径确实挂在 Router 上），9 个公开端点免令牌返回 200。
  - 404 / 405 / 400 文案矩阵：未知路径「资源不存在」、方法不支持「不支持的请求方法: {method}」、
    查询参数非法「参数 {key} 必须是整数」、后台请求体走 `ApiJson` 时统一 400「请求体格式错误」。
- **后台与监控接口黑盒补全**（`tests/admin_api_tests.rs`、`tests/monitor_api_tests.rs`）：
  补齐此前没有任何用例的 4 个端点 —— `GET /system/depts/export`、`GET /system/dict/types/export`、
  `GET /system/roles/export`、`PUT /system/dict/types/{id}`，并断言导出响应的
  `content-type: text/csv; charset=UTF-8` 与 `content-disposition` 附件名；
  新增列表筛选（`keyword` / `status` 下推到 SQL）、非超管动态路由只返回被授权菜单、
  停用账号立即失效既有会话并留下「账号已停用」失败日志、验证码缺失统一 400 且不写登录日志、
  顶层部门 `ancestors` 为空串、操作日志导出的 13 列内容、登录失败阈值取自
  `sys.login.alertThreshold` 且达到阈值时在热力图给出告警、时间线非法 `date` 一律 400、
  转发头里的非本机地址记为「内网」、工作台汇总统计已落库操作量、写操作指向不存在的目标时
  报业务错、可选字段缺省回落到稳定默认值等行为用例。
- **单元测试**：补 `util/csv.rs` 的 `number_cell`、`domain/models/audit_log.rs` 的 `truncate`
  （按字符计数、多字节安全）、`domain/models/monitor.rs` 的 `window_start` 三档窗口起点、
  `api/dto/admin.rs` 的非法 `%` 转义保留原字符、`services/dashboard.rs` 的未知统计键兜底，
  单元测试由 99 个增加到 105 个。
- **覆盖率**：`cargo llvm-cov --features docs --summary-only` 实测行覆盖率由 93.73% 提升到
  **97.40%**（region 90.63% → 95.55%，函数 90.33% → 93.71%），满足 `AGENTS.md` 第 10.1 节
  ≥80% 的要求；`docs/TESTING.md` 记录命令、口径，以及未纳入目标的 `src/main.rs`（二进制入口）
  与 `src/infrastructure/services/redis_store.rs`（需外部 Redis）及原因。

## [0.6.0] - 2026-10-13

> **以下为上游项目历史**（`0.6.0` 及更早）。它们的版本号与本项目不连续，
> 因此下文会再次出现 `0.2.0` 与 `0.1.0`——那是上游的旧版本，不是本项目的重复条目。

### 修复

- **日志落盘的磁盘 IO 移出锁外**：`BatchFileWriter::flush` 原先在持有缓冲锁的情况下
  `write_all` + `flush`，定时刷盘或满批落盘时所有写日志的线程会一起阻塞在磁盘 IO 上。
  现改为先在锁内 `mem::take` 换出缓冲并重置计数，再释放锁写文件；写盘失败时把内容
  放回缓冲队首，不丢日志。
- **配置兜底值与 `.env-example` 对齐**：`LOG_OUT_DIR` 代码默认值由 `/var/log/axum-app`
  改为 `./log`（不写环境变量时不再向系统目录写入，普通用户与容器内可直启动），
  `DATABASE_IDLE_SECS` 由 `30` 改为 `60`；`tests/config_tests.rs` 断言与 `docs/LOGGING.md`
  参数表同步。
- **`DATABASE_URL` 显式限制为 SQLite**：迁移文件与 `schema.rs` 的补列/索引都是 SQLite 语法，
  非 `sqlite:` 连接串会在启动时以难以定位的方式失败。现由 `AppConfig::from_env` 校验 scheme 并
  报错退出（只输出 scheme，不回显完整连接串以免泄露口令）；`tests/config_tests.rs`
  补 `postgres://` / `mysql://` / 裸路径三类用例，README 与 AGENTS.md 10.5 同步说明。
- **`LOG_FILTER` 非法不再静默回退**：`logging::init` 原先用
  `unwrap_or_else(|_| EnvFilter::new("info,tower_http=info"))`，过滤规则写错时会静默换一套
  级别运行（与“非法配置即拒绝启动”的约定相悖）。现改为 `with_context(...)?` 报错退出，
  并把解析提前到创建文件句柄之前；`tests/logging_tests.rs` 补对应用例，`docs/LOGGING.md` 同步。
- **分页边界与示例列表一致性**：仓储层 `list_records` 的偏移量改用
  `page.checked_sub(1).and_then(|p| p.checked_mul(size))`，不再依赖「service 已保证 page >= 1」
  （原式 `page - 1` 在 page 为 0 时 debug 下先 panic）；`examples` 列表补上 `page` 上限 10000，
  与事务接口一致；`docs` 下 Example DTO 的 `schema(example=...)` 改为与真实响应文案大小写一致。
- **409 响应泄露数据库内部结构**：`From<DbErr>` 原本把 `UniqueConstraintViolation` 的
  原始 detail 直接当作文案返回，客户端会收到
  `UNIQUE constraint failed: transfer_records.request_id`（表名 + 列名）。
  现改为固定文案 `resource already exists` + 服务端 `debug` 日志，`AGENTS.md`
  第 10.5 节同步约束。
- **Clippy 阈值失效修正**：`.clippy.toml` 里的 `cognitive-complexity-threshold`、
  `too-many-lines-threshold`、`allow-unwrap-in-tests` 此前从未生效（对应 lint 默认
  `allow`，只写阈值不启用 lint 等于假配置）。现改为：在 `src/lib.rs` 显式启用
  `clippy::cognitive_complexity` 与 `clippy::unwrap_used`，并拆分 `error.rs` 里的
  `into_response` 使其低于复杂度阈值；移除不具备可行性的 `too-many-lines-threshold`。
  `.clippy.toml` 与 `AGENTS.md` 第 5 节（以及 README / `docs/TESTING.md` 的检查命令）
  统一说明实际生效范围与不启用的两个 lint。
- **默认 feature 收紧与依赖清理**：`default` 由 `["preset", "http-client"]` 改为空，
  预置库改为按需开启（`--features preset,http-client`）；移除未被引用的
  `axum/multipart` feature、`mime` 依赖以及 `uuid` 的 `v4`/`serde` feature。
  README、`docs/DEVELOPMENT.md`、`docs/TESTING.md` 同步说明。
- **CORS 纳入中间件栈并改为可配置**：原先 `create_app` 直接挂 `CorsLayer::permissive()`，
  既绕过「所有中间件集中在 `middleware/stack.rs`」的约定，又无条件放行任意来源。
  现新增 `MIDDLEWARE_CORS_ALLOWED_ORIGINS` 白名单（逗号分隔），装配移到 `middleware/stack.rs`
  作为栈的最内层；默认空名单 → 不下发任何 CORS 头，单值 `*` 仍可显式放行全部来源。
  同步 `.env`、`.env-example`、`tests/config_tests.rs`、`tests/middleware_tests.rs`、`tests/api_tests.rs`
  与 `docs/MIDDLEWARE.md`。
- **SQLite 事务隔离级别失效**：转账事务原本以 `IsolationLevel::Serializable` 开启，
  但 sea-orm 在 SQLite 上会**忽略隔离级别**（只打一条 warn），实际始终是 `BEGIN DEFERRED`，
  既让文档承诺落空，又让每次转账都写一条误导性告警。现改用
  `begin_with_options(sqlite_transaction_mode = IMMEDIATE)`，在 `BEGIN` 时就取得写锁，
  消除 DEFERRED 下「读后升级写锁」的冲突窗口；`docs/TRANSACTION.md` 与 `AGENTS.md`
  第 10.5 节同步说明「控制并发用事务模式而非隔离级别」。
- **账户余额缺少乐观锁**：`update_account_balance` 原本用全字段 `ActiveModel::update()`，
  `WHERE` 中不带 `version`，读改写期间余额被他人推进会**静默覆盖**（与 AGENTS.md 10.5 不符）。
  现改为 `update_many` + `balance/version/updated_at` 表达式 + `WHERE id AND version = <读取时的值>`，
  影响行数为 0 时返回 `409 Conflict`；仓储接口新增 `expected_version` 参数，
  并在 `infrastructure/repositories/transaction.rs` 补单元测试。
- **`--release --features docs` 构建断裂**：`docs` feature 与 `debug_assertions` 是两个
  正交开关，`utoipa::path` 只为 debug 专用 handler 生成路由元数据，而
  `src/docs.rs` 的 `OpenApi` 无条件引用 `dev_login` / `dev_transfer`，导致
  release + docs 组合下 `__path_dev_login` / `__path_dev_transfer` 找不到而编译失败。
  现拆出仅在 `debug_assertions` 下编译的 `DevApiDoc`，由 `merge_from` 合入主文档，
  两个开关的四种组合都可编译；同时裁剪 release 下仅有 debug 用例使用的测试辅助函数。
- **分页偏移溢出**（审查 BUG-1）：`list_transfers` 新增页码上限校验
  （`page` 必须位于 1~10000，`MAX_PAGE`），查询层用 `checked_mul` 兜底防止
  `(page - 1) * size` 在 `u64` 上溢出；`docs/TRANSACTION.md` 接口表同步标注分页范围。
- **release 下接口测试级联失败**（审查 BUG-2）：`tests/api_tests.rs` 中依赖
  dev-login / dev-transfer 调试路由的辅助函数与用例（共 17 处）补上
  `#[cfg(debug_assertions)]`，release 构建不再因路由不存在而失败。
- **审计日志排序不稳定**（审查 M-4）：`list_audits` 在 `created_at` 基础上追加
  `id` 二级排序，同一事务内多条审计日志顺序固定。
- **测试健壮性**（审查 M-2/M-3/M-5/M-6/M-7）：`config_tests.rs` 的 `with_env`
  改用 `Drop` 守卫恢复环境变量（断言 panic 也会还原）；`api_tests.rs` 的
  `read_json` 改 `expect`（不再吞解析错误）；`logging_tests.rs` 断言日志文件内容
  而非仅存在；`logging_interval_tests.rs` 用轮询代替固定 `sleep`；
  `middleware_log_tests.rs` 补充全局订阅者单进程约束注释。
- **示例列表分页上限**（审查 M-10）：`examples` 列表的 `size` 上限 100，与事务接口一致。
- **文档与元数据修正**（审查 M-1/M-8/M-9）：`MIDDLEWARE.md` 超时描述改为
  「进入超时层后的处理耗时，外层排队不计入」；`logging.rs` 注释去重；
  `Cargo.toml` 的 `repository` 修正为 `cherish-ltt/AxumServiceScaffold`。

### 变更

- **OpenAPI 文档改为 `docs` feature 门控**（审查 S-3）：`utoipa` / `utoipa-swagger-ui`
  转为 optional 依赖，Swagger UI 与 `/api-doc/openapi.json` 仅在 `--features docs`
  构建下挂载（`cargo run --features docs`），不进 default 与 release 编译；
  controller / DTO 的 `utoipa` 注解统一改投 `feature = "docs"`，仅调试专用行为
  （dev 路由、`force_fail` 字段）保留 `debug_assertions`。CI 的 clippy 与 test
  步骤改用 `--features docs`。
- **建表迁移到 `sqlx::migrate`**（审查 S-5）：`migrations/` 目录新增版本化迁移文件
  （`0001_create_transfer_tables.sql`），`databases::run_migrations(database, url)`
  以 `sqlx::migrate!` 执行并记录到 `_sqlx_migrations`，随后补齐老库缺列、建索引
  并幂等播种；`schema.rs` 不再负责建表。
- **默认日志文件数统一为 60**（审查 S-1）：`LOG_MAX_LOG_FILES` 代码兜底默认值
  与 `.env-example` 一致，`docs/LOGGING.md` 参数表同步。
- **依赖版本锁定收紧**（审查 S-2）：`tracing` 锁定到 `=0.1.44`、
  `tracing-subscriber` 锁定到 `=0.3.23`。
- **Swagger 示例值校正**（审查 S-4）：`system` 响应的 `version` 示例更新为 `0.5.0`
  （与 `Cargo.toml` 同步），`database_status` 示例改为实际值 `not_checked`。
- **移除未使用的环境变量**（审查 S-6）：删除 `.env` / `.env-example` 中未被代码读取的
  `UPLOAD_IMAGE_MAX_BYTES`。
- **运行时输出英文化**：日志消息、API `message` 与错误消息、中间件文案、OpenAPI
  description / schema example、示例与种子数据改为英文（测试内 `expect`/`assert` 失败
  消息与代码注释、`docs/`、SQL 注释保持中文）；`src/` 下 20 余个文件与 `tests/`
  断言语义同步，对外消息结构不变。
- **启动配置改为 ASCII 列表框**：`log_startup_config` 由六条结构化 `info!` 改为单条
  多行手绘 ASCII 框（`+====+` 标题 + 六组 `key=value` 行），纯 ASCII 字符避免乱码，
  键仍输出为 `key=value`（如 `batch_max_events=50`）以兼容既有断言；
  `docs/LOGGING.md`「启动配置日志」同步。
- **启动配置框首行换行**：框体前补一空行，使 `+====+` 与日志前缀分行打印，
  避免标题与 INFO 行拼接破坏框结构。

### 说明

- 未知路由 404 仍返回 axum 内置纯文本（审查 S-7）：提取器错误与 404/405 不属于统一
  响应结构，测试只锁定状态码契约，维持既有取舍。
- 以下设计取舍经确认保持现状：`/system/health` 不查询数据库（`database_status` 固定
  `not_checked`，由 `/system/ready` 负责真实探测）、调试专用 `force_fail` 语义、
  `examples` 接口纯内存固定数据。

### 文档

- **过时描述修正**：`docs/DEVELOPMENT.md` 与 `docs/ARCHITECTURE.md` 仍写「在
  `infrastructure/databases/schema.rs` 建表」，实际建表早已迁到 `migrations/`
  （`schema.rs` 只做老库补列、补索引与幂等播种），已改为现状描述并补上
  「只支持 SQLite」的说明；`docs/RESPONSE.md` 的 `timestamp` 由「毫秒级本地时间戳」
  更正为「Unix epoch 毫秒（UTC 基准）」；README 的健康检查说明由「失败时不应继续接收
  流量」改为如实描述（`/health` 恒 200、不查库，接流与否请看 `/ready`）。
- README 重构为简洁门面文档：只保留项目简介、架构总览与流程图、已内置能力清单、
  文档导航、快速开始、默认接口、贡献指南与许可证；中间件栈与配置、事务示例、
  统一响应结构、日志、测试与构建检查、扩展开发等详情内容拆分到 `docs/` 下独立 md
  文件并通过链接跳转（`docs/ARCHITECTURE.md`、`docs/MIDDLEWARE.md`、
  `docs/TRANSACTION.md`、`docs/RESPONSE.md`、`docs/LOGGING.md`、`docs/TESTING.md`、
  `docs/DEVELOPMENT.md`）。图片、流程图、logo 与架构动效仍保留在 README。
- AGENTS.md 中引用 README 章节的规范同步改为指向 `docs/` 下对应文档
  （10.4 统一响应结构 → `docs/RESPONSE.md`；10.6 中间件栈/配置 → `docs/MIDDLEWARE.md`；
  10.7 日志分批落盘 → `docs/LOGGING.md`）。

## [0.5.0] - 2026-10-09

### 变更

- 依赖版本升级：`tokio` 1.53.1→1.53.2、`sea-orm` 2.0.3→2.0.4、`uuid` 1.26.1→1.27.0。
- 限流（`429`）与背压（`503`）拒绝请求时不再逐条打印 `warn` 日志
  （`src/middleware/error_response.rs`）：这两类拒绝总是批量出现，逐条告警无信息量；
  每次请求仍由 `TraceLayer` 以 INFO 记录 method / uri / status。

### 新增

- **日志分批写入**：文件日志改为内存缓冲批量落盘（`src/logging.rs`，`BatchFileWriter` /
  `BatchGuard`），满 `LOG_BATCH_MAX_EVENTS`（默认 50）条或每 `LOG_BATCH_FLUSH_INTERVAL_SECS`
  （默认 3）秒写一次文件，降低高频场景下逐条写文件的 CPU 开销；控制台层保持实时输出，
  进程退出时落盘剩余缓冲。新增环境变量：`LOG_BATCH_MAX_EVENTS`、`LOG_BATCH_FLUSH_INTERVAL_SECS`
  （任一为 0 拒绝启动），`.env`、`.env-example`、`tests/config_tests.rs`、
  `tests/logging_batch_tests.rs` 同步更新。

- **启动配置快照**：`logging::log_startup_config`（`src/logging.rs`）在启动时按
  「应用 / 服务 / 数据库 / JWT / 日志 / 中间件」六组结构化打印本次生效的配置；
  调用点位于 `src/main.rs` 日志初始化之后、`Container::bootstrap` 之前，
  数据库引导失败时也能看到配置。
- 脱敏：`DATABASE_URL` 中的口令被替换为 `***`（同时支持 `user:password@host` 与
  `?password=` 两种写法，口令中含 `@` 或 `:` 也不会漏出）；`JWT_SECRET` 只输出字符数，
  敏感信息不落日志。

### 变更

- `.env-public` 重命名为 `.env-example`（仍为入库的示例文件，用法不变：`cp .env-example .env`），
  避免与「公开」语义混淆。
- 中间件容量默认值与示例文件统一为按实测调整后的值：`MIDDLEWARE_MAX_CONCURRENCY` 512→256、
  `MIDDLEWARE_BACKPRESSURE_QUEUE` 512→256、`MIDDLEWARE_RATE_LIMIT_REQUESTS` 8192→32768；
  未显式配置这些环境变量时，代码兜底默认值与 `.env-example` 完全一致。

### 文档

- README 新增「贡献与使用本仓库」章节：从 `git clone` → `prek init`（安装 Git 钩子，
  配合 `.pre-commit-config.yaml`）→ `cp .env-example .env` → `cargo build`/`cargo test` →
  `cargo run` 到提交与推送的完整上手步骤链。
- README 新增「启动配置日志」小节；「中间件配置」表格的默认值同步为 256 / 256 / 32768，
  并说明默认值与 `.env-example` 保持一致。
- README 新增「架构动效预览」小节：内嵌 `docs/live-panel/axum-service-scaffold.gif`（30 秒循环）
  并链接到 `docs/live-panel/axum-service-scaffold.mp4`，同时说明面板配置（`docs/live-panel/config.json`）
  与示意值范围。GitHub 渲染会剥离手写 `<video>` 标签，仓库相对路径的 mp4 无法内嵌播放，
  故动效用 GIF 内嵌，完整画质用 mp4 点击跳转。
- README「构建检查」章节新增「HTTP 黑盒测试（`oneshot`）」小节：说明 `tests/api_tests.rs`
  以 `tower::ServiceExt::oneshot` 驱动 `create_app` 产出的 `Router`、每测试独立容器与临时 SQLite 库、
  `send` / `send_response` 的用法约定、`oneshot` 会消费 `Router` 需 `clone()`，
  以及提取器错误响应不属于统一响应结构的边界。

### 测试

- **HTTP 接口黑盒测试扩充**：`tests/api_tests.rs` 新增 12 个用例、共 42 个测试，
  全部通过 `tower::ServiceExt::oneshot` 把 `create_app` 产出的 `Router` 当作一次性 `Service` 驱动，
  不直接调用 controller/service，也不监听真实端口：
  - 统一响应结构：`success_response_follows_the_unified_envelope`、
    `error_responses_follow_the_unified_envelope`（断言 `code` 与状态码一致、`message` 为非空字符串、
    `timestamp` 为正毫秒值、`content-type: application/json`、错误响应不带 `data`）。
  - 提取器与协议错误：`malformed_json_body_is_rejected_with_400`（400）、
    `missing_json_content_type_is_rejected_with_415`（415）、
    `json_body_missing_required_field_is_rejected_with_422`（422）、
    `non_numeric_query_param_is_rejected_with_400`（400）、
    `unsupported_method_returns_405_with_allow_header`（405 且 `allow: GET,HEAD`）、
    `cors_preflight_is_allowed_for_every_origin`（预检 200 且放开来源与方法）。
  - 鉴权：`transaction_endpoints_require_bearer_token`、
    `malformed_authorization_header_is_rejected_with_401`（`Bearer` / `Bearer   ` 两种缺 token 写法）、
    `expired_token_is_rejected_with_401`（同密钥签发、已过期 5 分钟）、
    `dev_login_with_empty_roles_falls_back_to_default_role`（空 `roles` 回退为 `developer`）。
  - 辅助：新增 `send_response`（取原始 `Response` 以断言响应头）、`read_json`、
    `assert_unified_envelope` 与 `expired_token`。
  - 提取器层失败（400/415/422）与 404/405 的响应由 axum 生成（纯文本或空体），
    不属于统一响应结构，因此这些用例只锁定状态码契约。

## [0.4.0] - 2026-10-05

集成生产环境可用的 HTTP 中间件栈：请求 ID / Trace / 安全响应头 / 背压 / 限流 / 超时 /
Body 限制 / 压缩，全部由 `tower::ServiceBuilder` 统一装配，参数集中在 `.env`。

### 新增

- **中间件栈（`src/middleware/stack.rs`）**：`middleware::apply(app, &config.middleware)` 用
  `ServiceBuilder` 依次叠加 `SetRequestIdLayer`、`PropagateRequestIdLayer`、`TraceLayer`、
  `SetMultipleResponseHeadersLayer`、背压三件套（`LoadShedLayer` + `BufferLayer` +
  `ConcurrencyLimitLayer`）、限流链（`LoadShedLayer` + `BufferLayer` + `RateLimitLayer`）、
  `CompressionLayer`、`TimeoutLayer`、`RequestBodyLimitLayer`、`CorsLayer`。
- **配置（`MiddlewareConfig`）**：`MIDDLEWARE_REQUEST_TIMEOUT_SECS`（10）、
  `MIDDLEWARE_MAX_BODY_BYTES`（2 MiB）、`MIDDLEWARE_MAX_CONCURRENCY`（512）、
  `MIDDLEWARE_BACKPRESSURE_QUEUE`（512）、`MIDDLEWARE_RATE_LIMIT_REQUESTS`（8192）、
  `MIDDLEWARE_RATE_LIMIT_PERIOD_SECS`（1）、`MIDDLEWARE_HSTS_ENABLED`（默认跟随 `APP_ENV`）；
  任一容量参数为 0 时启动失败。
- **错误语义（`src/middleware/error_response.rs`）**：超时 408、Body 超限 413、
  限流 429、背压/overload 503，均复用 `ApiResponse` 结构。
- **请求 ID 贯穿**：`x-request-id` 缺失时生成 UUID，存在时透传；写入响应头，
  同时作为 `http_request` tracing span 的字段，handler 日志可与响应关联。
- **安全响应头**：`X-Content-Type-Options: nosniff`、`X-Frame-Options: DENY`、
  `Referrer-Policy: strict-origin-when-cross-origin`，生产环境额外下发 `HSTS`。
- **测试**：`tests/middleware_tests.rs` 新增 7 个集成测试（超时 408、Body 413、限流 429、
  背压 503 且不堆积、`request_id` 生成/透传、安全响应头与 HSTS 开关、gzip 压缩）；
  `tests/middleware_log_tests.rs` 单独验证日志关联（`request_id` 出现在 `http_request` span）；
  `tests/config_tests.rs` 新增 3 个配置测试（覆盖项、HSTS 默认值、容量参数为 0）；
  `src/middleware/error_response.rs` 补充错误分支的单元测试（覆盖率 100%）。
  其后共 97 个测试通过，行覆盖率 97.16%（`cargo llvm-cov`）。

### 变更

- `src/create_app.rs`：移除内联的 4 个 layer，改为 `middleware::apply`；
  `create_app` 的签名与调用方式不变。
- **依赖**：`tower` 由 dev-dependencies 提升为正式依赖（`buffer`、`limit`、`load-shed`）；
  `tower-http` 追加 `limit`、`timeout`、`set-header`、`compression-gzip` 特性。
- **配置结构**：`AppConfig` 新增 `middleware` 字段（结构体字面量构造会编译失败，测试已同步）。
- `.env` / `.env-example` 新增 `MIDDLEWARE_*` 配置段。
- 中间件栈包在整个 Router 外层而非 `Router::layer`，避免 `PathRouter::layer` 按路由复制
  有状态层导致全局并发/限流失效。

### 文档

- README 新增「中间件栈」「中间件配置」「最小压测方法」章节。
- AGENTS.md 新增「HTTP 中间件」规范（10.6）。

## [0.3.0] - 2026-09-27

内置「启动事务 → 读写数据 → 提交事务」的完整示例：一次转账在一个事务内读账户、
校验余额、更新双方余额、写流水与审计日志，任一步失败整体回滚；
并演示「写入过程中命中唯一约束冲突 → 跨表写入全部回滚」。

### 新增

- **事务示例（`services/transaction.rs`）**：`TransferService` 是脚手架内唯一的事务边界，
  以 `IsolationLevel::Serializable` 开启事务，成功后 `commit`，任一环节失败则 `rollback`，
  余额更新、流水、审计日志要么一起生效、要么一起消失。
- **唯一约束冲突 → 回滚**：`transfer_records.request_id` 作为幂等键建有唯一索引，
  重复提交时冲突发生在余额更新之后，账户余额、流水、审计日志被整体回滚，
  接口返回 `409 Conflict` 并带出数据库原因，用于演示「约束冲突不会留下半截数据」。
- **实体与建表**：`entities/transfer_account.rs`、`transfer_record.rs`、`transfer_audit.rs`
  三个实体；`infrastructure/databases/schema.rs` 负责建表、建索引与幂等播种两个演示账户
  （`acc_alice`、`acc_bob`，各 100000 分）。
- **克隆即可运行**：数据库文件不入库，首次启动或跑测试时自动建表、建索引并播种演示账户；
  老版本数据库启动时通过 `PRAGMA` 判断并自动补 `request_id` 列与唯一索引。
- **仓储适配器（`infrastructure/repositories/transaction.rs`）**：`TransferRepository`
  的每个方法接收 `C: ConnectionTrait`，传入连接表示不进事务、传入事务即在调用方事务内执行，
  同一套方法可被事务与普通查询复用。
- **HTTP 接口**：`POST /api/v1/transactions/dev-transfer`（调试构建专用，可传 `force_fail`）、
  `GET /api/v1/transactions/{id}`（流水详情与审计日志）、`GET /api/v1/transactions`（分页）。
- **回滚验证**：调试构建下 `force_fail = true` 会在事务内写入全部完成后主动失败，
  用于证明回滚确实撤销了已落库的数据；release 构建不注册该路由、DTO 中也没有该字段。
- **测试**：新增 11 个服务层单测与 9 个 HTTP 集成测试，覆盖提交、余额不足回滚、强制回滚、
  幂等键冲突回滚、老库补列、账户不存在、参数校验、余额溢出、分页与详情查询。
- **文档**：README 增加「事务示例」章节与完整调用示例；AGENTS.md 增加「事务与持久化规范」。

### 变更

- 新增 `AppError::Conflict`（HTTP 409），唯一约束冲突在 `From<DbErr>` 中统一识别并映射，
  不再被当作 500 数据库错误；409 与 400/401/404 一样向客户端透出可读原因。
- 金额统一使用最小货币单位的整数（`amount_cents: i64`），领域层与响应中不出现浮点金额，
  账户表同时维护自增 `version` 用于暴露并发丢失更新。
- `domain/repositories/mod.rs` 移除空的 `Repository` trait：仓储接口需要引用
  `sea_orm::ConnectionTrait`，继续放在 `domain` 会破坏「领域层不依赖 sea-orm」的边界，
  因此仓储接口与实现统一落在 `infrastructure/repositories`。
- 领域模型与用例接口新增 `domain/models/transaction.rs`、`domain/services/transaction.rs`。

### 兼容性

- 既有 `/api/v1/examples/*`、`/api/v1/system/*`、`/api/v1/auth/*` 接口与统一响应结构均未改动。
- 转账请求体的 `request_id` 为可选字段，不传时不参与幂等判定；老数据库启动时自动补列，
  历史流水该列为空，多个空值不触发唯一索引冲突。
- 数据库文件被 `.gitignore` 排除，克隆仓库后无需手工建库，启动与测试会自动完成初始化。
- 依赖版本提升至 `sea-orm 2.0.3`、`utoipa 6.0.0`、`utoipa-swagger-ui 10.0.1`、
  `thiserror 2.0.21`、`rand 0.10.3`、`reqwest 0.13.5`、`uuid 1.26.1`。

## [0.2.0] - 2026-09-22（上游）

统一响应结构的状态码真值，消除 HTTP 状态码与响应体 `code` 不一致的可能。

### 变更

- `ApiResponse` 内部改为只保存 `StatusCode`，响应体 `code` 在序列化时由状态码派生，
  HTTP 状态码与 `code` 字段不再可能不一致。
- 移除 `ApiResponse::with_parts(u16, ...)`，新增 `ApiResponse::with_status(StatusCode, ...)`
  作为唯一底层构造入口，并新增 `status()` 只读访问器。
- `ApiResponse` 实现 `IntoResponse`，controller 直接返回 `ApiResponse<T>`，不再手动包裹 `Json`。
- 移除 `ApiResponse` 的 `Deserialize` 实现（脚手架内无生产调用点）。
- README 补充「统一响应结构」约定。

### 兼容性

- JSON 响应结构保持不变：`{code, message, data?, timestamp}`，`data` 为空时仍省略该字段。
- Swagger 文档定义与既有集成测试断言无需调整。

## [0.1.0] - 2026-09-05（上游）

首个正式版本：一个面向 `axum + sea-orm` 的 Rust 空白脚手架，采用 DDD + 洋葱架构组织代码，
目标是可以直接复制作为新项目的基础骨架。

### 新增

- **洋葱架构骨架**：`api -> services -> domain -> infrastructure` 四层加装配层，
  依赖方向由外向内，`domain` 不依赖 axum、sea-orm 等外部框架。
- **Web 能力**：`axum 0.8` 路由、统一 API 返回结构 `ApiResponse`、统一错误映射 `AppError`。
- **鉴权**：`jsonwebtoken`（HS256）签发与校验访问令牌；`CurrentUser` 提取器解析
  `Authorization: Bearer <token>`；调试构建提供 `POST /api/v1/auth/dev-login` 调试登录。
- **示例接口**：系统健康检查（`/system/health`、`/system/ready`）、示例回显与分页列表
  （`/api/v1/examples` 系列接口）。
- **OpenAPI 文档**：`utoipa + swagger-ui`，调试构建下挂载 `/swagger-ui` 与
  `/api-doc/openapi.json`。
- **密码工具**：`src/util/password.rs` 提供 `argon2` 哈希与校验（`hash_password` /
  `verify_password`）。
- **基础设施**：`SeaORM` 连接池与连通性检查、`_schema_migrations` 迁移表、
  `JWT` 服务、环境变量配置加载与校验、按天滚动日志（`tracing-appender`）、
  请求 ID 与 Trace 中间件、CORS。
- **优雅关闭**：监听 Ctrl-C / SIGTERM 后停止接收新请求并关闭数据库连接池。
- **配置守卫**：拒绝过短的 `JWT_SECRET`、生产环境示例密钥、非法连接池与日志参数。
- **工程化**：GitHub Actions CI（格式、Clippy、构建、测试）、rustfmt/clippy 配置、
  61 个单元与集成测试，行覆盖率 94% 以上（`cargo llvm-cov`）。
- **预置依赖**：`rayon`、`reqwest`、`validator`、`regex`、`base64`、`once_cell`、
  `rand`、`derive_more`、`itertools` 等常用库按功能分组并锁定版本。
- **双许可**：采用 `MIT OR Apache-2.0` 双许可，文本见 `LICENSE-MIT` 与 `LICENSE-APACHE`。

### 修复

- 为 `jsonwebtoken` 显式启用 `aws_lc_rs` 加密后端，避免 v11 默认配置下
  签发/校验令牌时因缺少 CryptoProvider 而在运行时 panic。

### 说明

- 调试登录接口与 Swagger UI 仅在 debug 构建中提供，release 构建自动移除。
- 生产环境必须替换示例 `JWT_SECRET`，启动时会主动拒绝示例值。
