# 生产部署

本文件说明 Guardian 上线时要处理的事项，重点是如何在反向代理 / CDN 之后**拿到并信任真实客户端 IP**。
数据清理、演示账号与默认口令的清单不在这里复述，见
[backend/README.md 的生产环境注意事项](../backend/README.md#生产环境注意事项)；
中间件栈顺序与各容量参数含义见 [中间件栈与配置](../backend/docs/MIDDLEWARE.md)。

## 部署形态与真实客户端 IP

服务判断客户端身份的规则只有一条，所有形态都围绕它展开：

- 服务只认**真实 TCP 对端**（`ConnectInfo`，来自监听 socket，无法伪造）。
- 对端不在 `MIDDLEWARE_TRUSTED_PROXIES` 里：**完全忽略**请求头里的
  `X-Forwarded-For`，来源 IP 就是对端 IP。
- 对端命中可信代理：解析 `X-Forwarded-For`，**从右向左**逐个跳过可信代理，
  取第一个不可信地址作为客户端 IP。
- 不使用 `X-Real-IP` / `Forwarded` 兜底；转发头重复、或参与解析的地址不是合法 IP 时返回 `400`；
  请求上下文里没有对端信息时返回 `500`（不合成假 IP）。
- 该结果同时用于按 IP 的并发 / 速率额度、访问日志，以及
  `sys_oper_log.oper_ip` 与 `sys_login_log.ip`。日志里的来源因此不能被调用方伪造。

因此配置的核心只有 `MIDDLEWARE_TRUSTED_PROXIES`：**填「服务直接看到的那一跳」的地址**，
而不是客户端的地址。

### 形态一：用户直连服务

服务暴露在用户能直接访问的地址上，中间没有任何代理。

```env
MIDDLEWARE_TRUSTED_PROXIES=
```

留空即为正确配置：任何 `X-Forwarded-For` 都被忽略，来源就是对端 IP。
若服务直接暴露在公网，`X-Forwarded-For` 会被任意客户端随意填写，而本形态下这些值不参与任何判断。

### 形态二：Nginx 反向代理

用户访问 Nginx，Nginx 转发到服务。

```nginx
server {
    listen 443 ssl;
    server_name admin.example.com;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Request-Id $request_id;
        # 覆盖而不是追加：客户端自带的 X-Forwarded-For 被丢弃，链里只留真实对端
        proxy_set_header X-Forwarded-For $remote_addr;
    }
}
```

```env
SERVER_HOST=127.0.0.1        # 只监听本机，用户无法绕过 Nginx
MIDDLEWARE_TRUSTED_PROXIES=127.0.0.1/32
```

- `proxy_set_header X-Forwarded-For $remote_addr` 覆盖客户端自带的头；若改用
  `$proxy_add_x_forwarded_for`（追加语义），客户端伪造的前缀会留在链左侧——服务端从右向左解析时
  取的是最右侧的不可信地址，结果仍然正确，但链里会混入不可信数据，排查时容易误读。推荐覆盖。
- Nginx 与服务不在同一台机器（或不在同一容器网络）时，`MIDDLEWARE_TRUSTED_PROXIES`
  要填**服务实际看到的 Nginx 地址或网段**，不是 `127.0.0.1/32`。
- 多层 Nginx（如边缘 + 应用网关）时，把两层地址都列进来，服务会从右向左逐跳剥离。

### 形态三：CDN 回源，中间只有服务

CDN 节点直接回源到本服务，中间没有自建代理。此时「可信代理」就是 CDN 的回源节点地址段。

```env
MIDDLEWARE_TRUSTED_PROXIES=<CDN 回源网段列表>
```

两个注意点：

1. **网段必须来自厂商文档**，并随厂商变更同步（多数 CDN 会公布回源 IP 段，且可能调整）。
   填错或漏填的表现是：来源 IP 变成 CDN 节点地址，所有经 CDN 的用户共用同一份额度。
2. **不要用单值 `*` 图省事**。`*` 表示信任所有直连对端，等于完全采信客户端自报的
   `X-Forwarded-For`，来源 IP 从此可被伪造；而且 `APP_ENV=production` 时服务会直接拒绝启动。

若回源网段过宽或难以维护，建议改为「CDN 回源到 Nginx」（形态四），由 Nginx 用
`proxy_set_header X-Forwarded-For $remote_addr` 收敛成单个固定地址后再转发给服务。
有些厂商提供回源私有地址（如回源到内网 VPC），这比维护公网回源网段更稳。

### 形态四：CDN + 反向代理 + 服务

最常见的公网形态：用户 → CDN → Nginx → 服务。

```env
SERVER_HOST=127.0.0.1
MIDDLEWARE_TRUSTED_PROXIES=127.0.0.1/32
```

```nginx
location / {
    proxy_pass http://127.0.0.1:8080;
    proxy_set_header Host $host;
    # 只采信上一层代理给出的来源，覆盖客户端自带的头
    proxy_set_header X-Forwarded-For $remote_addr;
}
```

此时服务只信任本机 Nginx，CDN 与 Nginx 之间的跳数、CDN 回源网段变化都不需要服务关心：
Nginx 把「上一层是 CDN 节点」这一跳收敛掉了。**这是最省维护成本的接法。**

### 混合形态：部分流量走 CDN、部分直连

例如接口走 CDN，管理后台走直连，或多个入口并存。两种情况都能同时满足：

- 走 CDN 的请求经过代理，对端落在 `MIDDLEWARE_TRUSTED_PROXIES` 里，按转发头解析真实客户端。
- 直连的请求对端是用户网络地址，不在可信代理里，直接用对端 IP。

要注意的**不是** IP 解析，而是安全边界：如果直连入口不受保护，攻击者可以绕过 CDN 的
WAF / 限速 / 缓存策略直接打服务。所以直连入口要么绑内网地址（`SERVER_HOST` 设为内网 IP，
公网不可达），要么由安全组 / 防火墙只放行可信来源。

## 验证真实 IP 是否生效

装好之后用三条请求确认，不要只看配置项：

```bash
# 1. 不带转发头（走 Nginx / CDN 的真实路径）：来源应为真实客户端，不是代理地址
curl -s https://admin.example.com/api/v1/system/health -o /dev/null -D - | grep -i x-request-id
grep "request completed" ./log/app.$(date +%F).log | tail -1
#    日志里的 client_ip= 应为客户端地址；若显示代理地址或 127.0.0.1 说明可信代理没配对

# 2. 伪造转发头：经过 Nginx 覆盖后应被丢弃，来源仍是真实客户端
curl -s -H 'X-Forwarded-For: 1.2.3.4' https://admin.example.com/api/v1/system/health -o /dev/null

# 3. 直接打服务端口（应连不通；能连通说明服务暴露了，用户可绕过网关）
curl -s --max-time 3 http://127.0.0.1:8080/api/v1/system/health -o /dev/null -w '%{http_code}\n'
```

再用管理后台核对落库结果：`系统监控 → 登录日志` 的「来源」列应显示客户端 IP 与
「本机 / 内网 / 外网」（本地回环 / 私网与链路本地 / 其余），
「客户端」列显示浏览器与操作系统；`操作日志` 的 IP 列同理。两者都取保护层的解析结果，
只要日志里对了，额度划分与留痕就是对的。

反向代理配置需要重启（`nginx -s reload`）后才生效；环境变量改动需要重启服务。

## 环境变量

完整清单与默认值见 [backend/.env-example](../backend/.env-example)。上线必须处理的几项：

| 变量 | 说明 |
| --- | --- |
| `APP_ENV=production` | 声明生产环境；配合 release 构建使用，见下一节 |
| `JWT_SECRET` | 至少 32 字符的随机串；示例值在 `production` 下会被拒绝启动 |
| `TWO_FACTOR_KEY` | 独立的 Base64 编码 32 字节随机密钥；启用认证器前必须配置，见下节 |
| `DATABASE_URL` | 必须是 `sqlite:` 开头的 URL；其他 scheme 在启动阶段直接拒绝 |
| `SERVER_HOST` / `SERVER_PORT` | 有代理时绑 `127.0.0.1`，不要直接监听公网 |
| `REDIS_URL` / `REDIS_PASSWORD` | 验证码与会话撤销都依赖 Redis，缺了登录不可用 |
| `MIDDLEWARE_TRUSTED_PROXIES` | 可信代理地址，见上一节；不能用 `*` |
| `MIDDLEWARE_CORS_ALLOWED_ORIGINS` | 前后端不同源时必须显式列出来源；留空表示不下发任何 CORS 头 |
| `MIDDLEWARE_HSTS_ENABLED` | 默认跟随 `APP_ENV`（production 默认开启）；只在全站 HTTPS 时开启 |
| `LOG_OUT_DIR` / `LOG_MAX_LOG_FILES` | 日志目录与保留文件数，按磁盘容量规划 |
| `IP_LOCATION_DB_PATH` | IPv4 离线 IP 地名库路径，见下一节；留空则 IPv4 来源的地点只区分本机 / 内网 / 外网 |
| `IP_LOCATION_V6_DB_PATH` | IPv6 离线 IP 地名库路径，与上一条互相独立；留空则 IPv6 来源的地点只区分本机 / 内网 / 外网 |

按来源 IP 的额度、全局限流与背压都是**进程级**实现：多实例部署时每个实例各有一套额度，
分布式限流需要网关或共享存储承担。SQLite 是单机文件库，不要当成跨机器共享数据库。

## 二次认证密钥与恢复

用安全随机源生成 32 字节并编码为 Base64（例如 `openssl rand -base64 32`），写入受保护的环境配置
`TWO_FACTOR_KEY`，不要复用 JWT 密钥或提交到 Git。本仓库模板有意留空：未配置不能开始绑定，
已绑定账号的动态码验证无法解密时拒绝，不会跳过 2FA；未使用的恢复码仍可自助登录、关闭或重建。
TOTP 依赖时间，服务器和认证器应保持时钟同步。

- **备份**：SQLite 包含加密后的有效/待确认密钥、恢复码哈希和会话版本；加密密钥单独安全备份。
  只恢复数据库而丢失 `TWO_FACTOR_KEY` 不能恢复动态码验证。不要直接换密钥后继续使用旧密文；本版本无在线轮换工具。
- **自助恢复**：账号密码通过后进入独立验证页，用一枚恢复码完成登录，再用另一枚恢复码关闭 2FA，重新绑定；也可用有效动态码完成这些操作。
  两步登录都不签发半会话：挑战 300 秒一次性有效，前端不持久化密码或挑战。
  恢复码仅在生成时显示一次，保存到独立安全位置，不要和认证器只放在同一台设备。
- **人工恢复**：有 `system:user:reset2fa` 权限的管理员在用户管理重置目标账号，需输入自己的密码。
  默认权限迁移及非超管限制见 [后台管理模块](../backend/docs/ADMIN.md#本人资料与二次认证)。
- **唯一管理员丢失全部第二因素**：由持数据库权限的运维按身份核验流程线下处理，不提供匿名恢复接口或本地恢复命令。
  停服并备份后，仅对核实的目标 `sys_user_security` 行清空 `secret/pending_secret`、置
  `pending_expires/failures/blocked_until` 为 0、`last_step` 为 -1、`recovery_hashes` 为 `[]`，
  **递增而不是清零 `auth_version`**。不要删除整行来重置版本。重启后旧令牌不能使用，新登录立即重新绑定并保存恢复码。
- 安全设置提交后 Redis 会话清理失败会告警；数据库版本校验已拒绝旧令牌，不影响返回新恢复码。
  若客户端在提交后断网没有收到恢复码，可用认证器的新动态码登录重新生成，不会重复展示旧批次。
- 网关不得缓存认证接口，保留 `/auth/*` 的 `Cache-Control: no-store`；不得记录密码、动态码、绑定密钥或恢复码。

## 基础数据缓存与降级

适用接口清单及一致性语义见 [后台管理模块](../backend/docs/ADMIN.md#基础数据缓存)。
当前只支持单后端服务实例内的立即可见保证；不能在多实例共享 SQLite 的情况下假定本地换代会通知其他实例。
在线直接改基础表也不会失效缓存，应停服维护后重启。

| 配置 | 默认值 | 含义 |
| --- | --- | --- |
| `CACHE_TTL_SECS` | 300 | 正常值基础 TTL，另外加最多约 20% 抖动 |
| `CACHE_NEGATIVE_TTL_SECS` | 15 | 空结果 TTL，不得大于正常 TTL |
| `CACHE_TIMEOUT_MS` | 100 | 单次 Redis 业务缓存 IO 等待上限 |
| `CACHE_MAX_LOADS` | 4 | 每实例最多同时回源数，超量 503 |
| `CACHE_MAX_ENTRIES` | 4096 | 当前进程登记的缓存键数上限；满后新键不回填 |
| `CACHE_MAX_VALUE_BYTES` | 262144 | 单个 JSON 值回填字节上限 |

业务缓存故障时，在**会话认证已通过**的前提下继续从 SQLite 读取，短暂熔断避免持续等待 Redis。
Redis 同时承载验证码和会话：Redis 整体故障时不能保证登录或已登录请求可用，不得绕过会话验证。
应用启动仍需连接 Redis。Redis 恢复后自动重试并回填；重启使用新命名空间，旧键靠 TTL 清理。
写操作会使整组基础数据换代，可能导致集中冷读，由有界回源保护 SQLite，而不是返回旧值。
默认阈值是保护参数，不是实测容量结论；依据真实负载、SQLite 连接池和 503/缓存告警调整。
Redis 配置内存预算与淘汰策略时应考虑会话、验证码共用实例，避免仅为业务缓存挤出会话。

## 离线 IP 地名库

登录日志与操作日志的 `location` 列分两级解析：

1. **本地网段**：回环记「本机」，RFC1918 私网、链路本地与 IPv6 ULA 记「内网」，不查库。
2. **其余地址**：查 ip2region 的离线库（IPv4 与 IPv6 各一个库），命中时写入**库里的原始串**，如
   `中国|广东省|茂名市|电信|CN`、`United States|California|0|Google LLC|US`；
   未收录或落在保留地址段（库返回 `Reserved|...`）时记「外网」。

库文件不随仓库分发（v4 约 10.6 MiB、v6 约 35.6 MiB，且上游每周更新），部署时手动下载：

```bash
# 目录自定，与 .env 的 IP_LOCATION_DB_PATH / IP_LOCATION_V6_DB_PATH 保持一致
curl -fL -o /var/lib/guardian/ip2region_v4.xdb \
  https://raw.githubusercontent.com/lionsoul2014/ip2region/master/data/ip2region_v4.xdb
curl -fL -o /var/lib/guardian/ip2region_v6.xdb \
  https://raw.githubusercontent.com/lionsoul2014/ip2region/master/data/ip2region_v6.xdb
ls -l /var/lib/guardian/ip2region_v*.xdb
```

```env
IP_LOCATION_DB_PATH=/var/lib/guardian/ip2region_v4.xdb
IP_LOCATION_V6_DB_PATH=/var/lib/guardian/ip2region_v6.xdb
```

- 解析实现是 vendored 的官方 Rust 客户端（`backend/vendor/ip2region`，锁在提交 `1bf6cab`），
  IPv4 与 IPv6 各用一个库文件，两个路径互相独立。
- **路径留空**：该协议不查地名，地点只有本机 / 内网 / 外网。只配其中一个时另一个协议的来源
  按本地网段与「外网」判定，不会回落到另一个库（库头记录的 IP 版本不匹配时查询直接报错）。
- **配了却不可用**（文件不存在、不是合法 xdb、库的 IP 版本与配置的变量不符）：启动直接失败，
  不会静默降级。
- **更新库**：下载新文件覆盖后重启进程。查询按需读文件（只缓存索引），替换文件不重启会读到
  不一致的内容。库更新频繁，建议随版本发布或定期任务进行，并保留上一版以备回滚。
- `location` 存的是库里的完整原始串（含 ISP 与 ISO 码），不裁剪字段。

## 启动与进程管理

发布页提供预编译包，按「架构 × libc」分为四个，文件名形如
`guardian-backend-v0.5.1-x86_64-linux-musl.tar.gz`：

| 包名后缀 | 运行时 | 适用场景 |
| --- | --- | --- |
| `x86_64-linux-gnu` / `aarch64-linux-gnu` | 动态链接 glibc | 主流发行版（Debian / Ubuntu / RHEL），体积略小 |
| `x86_64-linux-musl` / `aarch64-linux-musl` | 静态链接，无 libc 依赖 | Alpine、distroless 等 musl 发行版，或不想操心 glibc 版本 |

解压后只有 `guardian-backend` 与 `.env-example` 两项。迁移 SQL 已由 `sqlx::migrate!` 在编译期
嵌入二进制，包内不需要 `migrations/` 目录。不确定该用哪个时用 musl 包：静态链接，
不会因目标机器的 glibc 版本过旧而启动失败（`ldd` 显示 `statically linked`，
`readelf -l` 无 `INTERP` 段）。

从源码自行构建 musl 包时，只给 C 代码指定 musl 编译器即可：

```bash
rustup target add x86_64-unknown-linux-musl
CC_x86_64_unknown_linux_musl=musl-gcc \
  cargo build --release --locked --target x86_64-unknown-linux-musl
```

**不要**再设 `CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc`：`musl-gcc` 是编译器驱动，
交给 rustc 当链接器时会额外写入动态解释器，产物变成 dynamic-pie，启动即段错误。
不设 `LINKER` 时 rustc 用自带的 rust-lld 直接产出 static-pie。

从源码构建：

```bash
cd backend
cargo build --release
APP_ENV=production ./target/release/guardian-backend    # 实际部署建议用 systemd / supervisor 托管
```

- **必须用 release 二进制**：`APP_ENV=production` 遇到 debug 构建会拒绝启动，
  因为 debug 构建注册了可直接指定身份的 `dev-login` / `dev-transfer` 调试路由。
  该检查在 `main` 与嵌入入口 `create_app` 两处生效。
- 首次启动会自动执行迁移并播种数据；库文件不存在时会创建，升级时结构变更自动补齐。
- 服务本身不 fork，交 systemd 等进程管理器托管，配置开机自启与失败重启即可。
- `GET /api/v1/system/health` 与 `/api/v1/system/ready` 可作存活 / 就绪探针。探针同样受额度保护，
  频率与失败阈值要考虑短时过载，不要把公网请求失败直接当成进程死亡。

## 前端构建与静态托管

```bash
cd frontend
npm ci
npm run build      # 产出 dist/
```

- 把 `dist/` 交给 Nginx / CDN 等静态服务器托管。Vue Router 使用 history 模式，
  静态服务器需要配置 SPA 回退（未命中的路径返回 `index.html`），并把 `/api/v1` 转发到后端。
- 后端与前端不同源时，构建前用 `VITE_API_BASE_URL` 指定接口前缀；
  同时在后端 `MIDDLEWARE_CORS_ALLOWED_ORIGINS` 列出前端来源。同源（同一域名下反代 `/api`）时两者都不用配。
- 顶栏的 GitHub 入口指向本仓库，不希望外露时把 `frontend/src/data/index.ts` 的 `APP.repo`
  置为空字符串即可不渲染，详见 [前端说明](../frontend/README.md)。

## 安全边界

- **TLS 由网关负责**：服务只提供明文 HTTP，`MIDDLEWARE_HSTS_ENABLED` 只是响应头，不提供 HTTPS。
  CDN / Nginx 侧配置证书与 HTTP → HTTPS 跳转。
- **连接层与 DDoS 由网关负责**：TCP 连接数、请求头大小与读取超时、空闲连接回收、
  慢速攻击防护、公网 DDoS 清洗都不在应用层实现。
- **文档与调试入口**：Swagger 由 `docs` feature 门控（默认不编译），
  若生产显式启用，必须在网关限制 `/swagger-ui` 与 `/api-doc/openapi.json` 的访问范围。
  生产构建不注册 `dev-login` / `dev-transfer`。
- **留痕来源**：操作日志与登录日志的来源 IP 取可信解析结果，因此必须正确配置可信代理，
  否则日志记录的是代理地址，也会导致所有用户共用同一份额度。

## 日志与磁盘

- 文件日志按时间轮转（默认按天，保留 `LOG_MAX_LOG_FILES` 份），日志目录按需清理；
  程序不负责磁盘配额与归档告警，由部署设施保证。
- 每条请求记录一条访问日志（`request completed`）；慢请求、超时（408）与业务 5xx 会附阶段汇总
  （排队耗时、`db.*` 阶段耗时、是否被中断），排查方法见
  [日志：低噪声请求诊断](../backend/docs/LOGGING.md#低噪声请求诊断)。
- 批量落盘参数（`LOG_BATCH_MAX_EVENTS` / `LOG_BATCH_FLUSH_INTERVAL_SECS`）为 0 时拒绝启动。

## 数据与备份

- 备份对象是 SQLite 库文件及独立保管的 `TWO_FACTOR_KEY`；Redis 存验证码、会话和可重建业务缓存，可丢弃但会使用户重新登录。
  停服或用 `sqlite3 .backup` 做一致性备份，不要直接拷贝正在写入的库文件。
- 上线前必须按 [生产环境注意事项](../backend/README.md#生产环境注意事项) 替换示例密钥与默认口令、
  清理演示数据；[backend/docs/CHANGELOG.md](../backend/docs/CHANGELOG.md) 与 `backend/AGENTS.md`
  记录了口令与种子数据的来源。
- 数据库文件不要入版本库；表结构与种子数据由启动流程自动维护。

## 上线检查清单

- [ ] `.env`：`APP_ENV=production`、替换 `JWT_SECRET`、`DATABASE_URL` 指向部署路径
- [ ] `.env`：替换 `SEED_ADMIN_PASSWORD`，清空或替换 `SEED_USERS`；首个账号登录后立即改密
- [ ] Redis 可达、已设密码；会话与验证码正常
- [ ] `TWO_FACTOR_KEY` 已配置并独立备份；完成绑定、恢复码保存和恢复演练
- [ ] 基础缓存只部署单实例；确认回源上限及故障时认证不绕过
- [ ] `MIDDLEWARE_TRUSTED_PROXIES` 按形态填对（有代理必填，且不能用 `*`）
- [ ] `SERVER_HOST` 绑本机或内网；直连入口已由防火墙 / 安全组收紧
- [ ] `IP_LOCATION_DB_PATH` / `IP_LOCATION_V6_DB_PATH` 已指向真实 xdb（或有意留空），
      分别用 IPv4 与 IPv6 登录一次确认 `location` 写的是地名 / 本机 / 内网
- [ ] 用「验证真实 IP 是否生效」的三条请求确认来源 IP 与留痕正确
- [ ] 前端 `dist/` 已托管，SPA 回退与 `/api/v1` 转发正确，跨域来源已按需配置
- [ ] 用 release 二进制启动，健康 / 就绪探针已接
- [ ] 演示数据已按清单清理，备份与日志轮转已就位
