# 架构设计

> 本文档详解后端的分层架构与目录结构，概览见 [README](../README.md)。

## 为什么改成洋葱架构

后端现在采用 4 层加装配层的方式：

```text
api -> services -> domain
         |
         v
   infrastructure
```

对应目录：

```text
src
├─ api                # HTTP 入口层：路由、控制器、DTO、提取器
├─ services           # 用例实现层：组合领域规则与基础设施能力
├─ domain             # 领域内核：模型、错误、接口抽象、常量
├─ infrastructure     # 外部实现：配置、数据库、JWT、仓储适配器
├─ util               # 通用工具：如 Argon2 密码哈希
├─ container.rs       # 依赖装配中心
├─ create_app.rs      # Axum app factory
├─ error.rs           # 把领域错误映射成 HTTP 响应
├─ response.rs        # 统一 API 返回包装
├─ docs.rs            # Swagger 聚合定义
└─ main.rs            # 启动入口
```

### 对比 Java 常见的 `controller -> service -> dao`

很多 Java 项目里的 `controller/service/dao` 在小项目里上手快，但一旦业务变复杂，常见问题是：

- `service` 既管业务规则，又直接拼 ORM 查询，又顺手处理 HTTP DTO
- `dao` 很容易被上层直接拿去用，导致业务规则绕过 service
- controller 层本该只处理输入输出，最后却夹带鉴权、参数转换、异常映射
- 换数据库、换鉴权、接 MQ、接第三方 API 时，修改范围会一路渗透进业务代码

洋葱架构的优势在于依赖方向被强制收紧：

- `domain` 不依赖 `axum`、`sea-orm`、JWT 库这些外部框架
- `services` 只面向领域接口和模型写用例，不把 HTTP 层细节带进去
- `api` 只负责输入输出适配，不承载业务规则
- `infrastructure` 只是外部能力实现，未来可以替换而不改核心业务
- `container.rs` 集中装配依赖，避免到处散落“手动注入”

一句话概括：

- Java 风格三层更像“按职责分文件”
- 洋葱架构更强调“按依赖方向隔离变化”

对本项目来说，后者更适合长期扩展，因为一开始就把“业务核心”和“外部框架”拆开了。

## 当前目录结构

```text
src
├─ api
│  ├─ controllers
│  │  ├─ admin                   # 后台管理：7 个资源 + 模块汇总
│  │  │  ├─ users.rs  roles.rs  menus.rs  depts.rs  posts.rs
│  │  │  ├─ dicts.rs  configs.rs
│  │  │  └─ mod.rs
│  │  ├─ auth_controller.rs      # 登录、验证码、路由、改密
│  │  ├─ account_controller.rs   # 本人资料与 2FA
│  │  ├─ monitor_controller.rs   # 操作 / 登录日志
│  │  ├─ dashboard_controller.rs
│  │  ├─ generator_controller.rs
│  │  ├─ system_controller.rs    # 健康检查
│  │  ├─ example_controller.rs   # 骨架示例
│  │  ├─ transaction_controller.rs
│  │  └─ mod.rs
│  ├─ dto
│  │  ├─ admin.rs  auth.rs  example.rs  system.rs  transaction.rs
│  │  └─ mod.rs
│  ├─ extractors
│  │  ├─ admin_user.rs           # 后台令牌 + 权限码
│  │  ├─ current_user.rs         # 骨架令牌
│  │  ├─ api_json.rs             # 请求体反序列化失败转 400
│  │  ├─ client_source.rs        # 可信代理链下的真实来源
│  │  └─ mod.rs
│  └─ mod.rs                     # 合并 9 个 controller 路由
├─ domain
│  ├─ models                   # 13 个模型文件：identity、system、system_admin、
│  │                          # account、audit_log、monitor、dashboard、generator、
│  │                          # auth、example、transaction、pagination、mod
│  ├─ services
│  │  ├─ admin                 # 10 个后台用例：users/roles/menus/depts/posts/
│  │  │                        # dicts/configs/logs/graph/mod
│  │  ├─ admin_auth.rs  account.rs  audit.rs  auth.rs  dashboard.rs
│  │  ├─ generator.rs  monitor.rs  system.rs  example.rs  transaction.rs
│  │  └─ mod.rs
│  ├─ constants.rs
│  ├─ error.rs
│  ├─ mod.rs
│  └─ repositories
│     └─ mod.rs                 # 仅说明边界，仓储接口在 infrastructure
├─ entities                     # SeaORM 实体，每表一个文件（sys_* / gen_* / transfer_*）
│  └─ mod.rs
├─ infrastructure
│  ├─ databases
│  │  ├─ mod.rs                 # 连接、迁移、播种入口
│  │  ├─ schema.rs              # 老库补列/补索引/幂等种子（建表在 migrations/）
│  │  └─ seed.rs                # 后台演示数据
│  ├─ repositories              # 仓储：接口 + SeaORM 实现
│  │  └─ mod.rs
│  ├─ services
│  │  ├─ auth_store.rs          # 会话与验证码存储接口（Redis / 内存）
│  │  ├─ cache_store.rs  redis_store.rs  two_factor.rs  jwt.rs
│  │  └─ mod.rs
│  ├─ ip_location.rs            # 离线归属地查询
│  ├─ config.rs                 # AppConfig::from_env 与全部启动校验
│  └─ mod.rs
├─ middleware
│  ├─ stack.rs                  # 中间件栈装配顺序
│  ├─ trace.rs  diagnostics.rs  error_response.rs
│  ├─ audit.rs                  # 写操作留痕与 AUDIT_ROUTES
│  ├─ client_ip.rs  client_limits.rs
│  └─ mod.rs
├─ services                     # 用例实现层，事务边界所在层
│  ├─ account                  # login / profile / security
│  ├─ admin                    # 与 domain/services/admin 一一对应
│  ├─ admin_auth.rs  audit.rs  auth.rs  dashboard.rs  example.rs
│  ├─ generator.rs  monitor.rs  system.rs  transaction.rs
│  ├─ hot_cache.rs             # 基础数据缓存与写屏障
│  ├─ support.rs
│  └─ mod.rs
├─ util
│  ├─ csv.rs  password.rs  time.rs  user_agent.rs
│  └─ mod.rs
├─ container.rs
├─ create_app.rs
├─ docs.rs
├─ error.rs
├─ lib.rs
├─ logging.rs
├─ main.rs
└─ response.rs
```

迁移文件在 `migrations/`（`0001`–`0004`），不在 `src` 下。

## 每层负责什么

### 1. `api`

最外层，处理 HTTP 相关内容：

- `controllers`：接收请求、调用用例、返回统一响应
- `dto`：请求体和响应体
- `extractors`：例如 JWT 当前用户提取

这一层不做核心业务判断，只做协议适配。

### 2. `services`

应用服务层，也就是“用例实现层”。

这一层负责：

- 组织一次完整业务流程
- 调用领域接口
- 调用基础设施能力
- 保持 controller 足够薄
- **持有数据库事务边界**（`begin` → 读写 → `commit` / `rollback`）

这里可以理解成“真正写业务编排的地方”。

### 3. `domain`

最内层，定义系统核心抽象：

- 领域模型
- 领域错误
- service trait
- 常量

这里不直接依赖 `axum`、`sea-orm`、JWT 或数据库连接。

仓储接口需要引用 `sea_orm::ConnectionTrait`（否则无法在事务内复用），
因此它和实现一起放在 `infrastructure/repositories`，`domain/repositories` 只保留边界说明。

### 4. `infrastructure`

外部世界的实现层：

- 环境配置读取
- SeaORM 数据库连接、版本化迁移（`migrations/`）与老库补列/索引/种子数据
- JWT 签发与校验
- 仓储适配器（接口 + SeaORM 实现）

这层是最容易变化的地方，所以应该被隔离在外圈。

### 5. `container.rs`

整个项目的依赖装配中心。当前默认装配：

- 基础能力：`AppConfig`、`SeaORM DatabaseConnection`、`JwtService`、`SystemUseCase`、`AuthUseCase`、
  `ExampleUseCase`、`TransferUseCase`、`AuditStore`（Redis 或内存）
- 后台管理：`AdminAuthService`、`AdminService`、`MonitorService`、`DashboardService`、
  `GeneratorService`、`AuditService`（写操作留痕）

`bootstrap` 按配置连接 Redis 组装 `RedisAuthStore`；`bootstrap_with_auth_store(config, store)` 允许测试
换成 `Container::memory_auth_store()`，从而不依赖外部 Redis。后续接邮件、对象存储、MQ，也建议继续在这里统一装配。

## SeaORM 放在哪里

本项目没有改掉 SeaORM，而是把它放回更合理的位置：

- 实体统一放在 `src/entities`
- 数据库连接在 `src/infrastructure/databases`；建表在 `migrations/`（由 `sqlx::migrate!` 执行），
  老库补列、补索引与幂等种子在 `src/infrastructure/databases/schema.rs`
- 仓储接口与实现放在 `src/infrastructure/repositories`

> 迁移与补列逻辑都是 SQLite 语法，后端只支持 `sqlite://` 的连接串
> （非 SQLite 的 `DATABASE_URL` 会在启动时被 `AppConfig::from_env` 拒绝）。

事务也遵循同一套边界：

- **只有 `services` 层开启事务**，controller 和仓储都不管事务生命周期
- 仓储方法接收 `C: ConnectionTrait`，因此同一套方法既能用于事务内，也能用于普通查询
- 参考实现：`services/transaction.rs`（事务边界）+ `infrastructure/repositories/transaction.rs`（读写）

也就是说：

- 不把 ORM 直接塞进 controller
- 不让领域层直接依赖数据库细节
- 但也不牺牲 SeaORM 的使用体验
