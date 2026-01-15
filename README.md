

<div align="center">
<h1>Guardian</h1>
<p>基于 Rust 的生产级后台管理认证系统</p>
  <p>
  <a href="https://github.com/cherish-ltt/Guardian/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="license"/>
  </a>
  <a href="https://www.rust-lang.org">
    <img src="https://img.shields.io/badge/rust-1.92.0+-orange.svg" alt="license"/>
  </a>
  <a href="https://img.shields.io/badge/version-v1.0-blue.svg">
    <img src="https://img.shields.io/badge/version-v0.1-blue.svg" alt="license"/>
  </a>
  <a href="https://img.shields.io/badge/status-stable-green.svg">
    <img src="https://img.shields.io/badge/status-stable-green.svg" alt="license"/>
  </a>
  </p>
</div>

## 📖 项目简介

  Guardian 是一个功能完整、高性能的认证授权系统，专为现代 Web 应用程序设计。它提供了完整的用户认证、权限管理和操作审计功能，采用模块化架构设计，确保高可用性和易维护性。

推荐前端页面：[Guardian-Website](https://github.com/cherish-ltt/Guardian-Website) 

## ✨ 核心特性

### 🔐 安全认证
- **Argon2 密码加密** - 业界最安全的密码哈希算法
- **双因素认证 (2FA)** - 支持 TOTP 协议（预留接口）
- **账户锁定机制** - 5次登录失败锁定15分钟
- **JWT 令牌管理**
  - Access Token: 15分钟有效期
  - Refresh Token: 7天有效期
  - 令牌黑名单机制防止重放攻击

### 🎯 权限控制 (RBAC)
- **细粒度权限** - API/菜单/按钮三级权限
- **角色-权限体系** - 灵活的权限分配
- **超级管理员** - 跳过权限检查，拥有所有权限
- **系统内置角色/权限** - 不可删除，保证系统安全

### 📊 审计日志
- **异步批量写入** - 每3秒批量写入，或队列满10条立即触发
- **完整的操作记录** - 用户信息、请求参数、响应结果
- **性能监控** - 记录请求耗时
- **IP和User-Agent** - 完整的客户端信息

### 🚀 高性能
- **异步非阻塞IO** - 基于 Tokio 运行时
- **SeaORM 数据库抽象** - 类型安全，编译期检查
- **连接池优化** - 高效的数据库连接管理
- **内存优化** - 日志缓冲池大小限制

## 🛠 技术栈

| 技术 | 版本 | 说明 |
|------|--------|------|
| **Rust** | 2024 Edition | 系统语言 |
| **Axum** | 0.8 | Web 框架 |
| **SeaORM** | 0.12 | ORM 框架 |
| **PostgreSQL** | 17 | 数据库 |
| **Tokio** | 1.x | 异步运行时 |
| **JWT** | 9.0 | 认证令牌 |
| **Chrono** | 0.4 | 时间处理 |
| **Serde** | 1.x | 序列化/反序列化 |

## 📦 数据库设计

### 核心表结构

| 表名 | 说明 | 主要字段 |
|--------|------|----------|
| `guardian_admins` | 管理员表 | id(UUID-V7), username, password_hash, two_fa_secret, is_super_admin, status, last_login_at, login_attempts, locked_until |
| `guardian_roles` | 角色表 | id(UUID-V7), code, name, description, is_system |
| `guardian_permissions` | 权限表 | id(UUID-V7), code, name, description, resource_type, http_method, resource_path, parent_id(UUID-V7), sort_order, is_system |
| `guardian_admin_roles` | 管理员-角色关联 | admin_id(UUID-V7), role_id(UUID-V7) |
| `guardian_role_permissions` | 角色-权限关联 | role_id(UUID-V7), permission_id(UUID-V7) |
| `guardian_token_blacklist` | 令牌黑名单 | id(UUID-V7), token_id, expires_at |
| `guardian_audit_logs` | 审计日志表 | id(UUID-V7), trace_id, admin_id(UUID-V7), username, action, resource, method, params, result, status_code, ip_address, user_agent, duration_ms |

### 设计特点
- ✅ 无外键约束 - 只通过逻辑关联，提高灵活性
- ✅ 完整的索引设计 - 优化查询性能
- ✅ UUIDv7 主键 - 全局唯一且有序，避免 ID 碰撞
- ✅ 自动触发器 - 数据库自动维护 created_at 和 updated_at
- ✅ 时间戳字段 - TIMESTAMPTZ 类型，支持时区
- ✅ 状态字段 - 软删除、禁用等功能

## 🚀 快速开始

### 环境要求

#### 必需软件
- **Rust 1.92+** - 使用 rustup 安装：
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source $HOME/.cargo/env
  ```
  验证安装：`rustc --version` 和 `cargo --version`

- **PostgreSQL 17+** - 数据库服务：
  - **macOS** (使用 Homebrew):
    ```bash
    brew install postgresql@17
    brew services start postgresql@17
    ```
  - **Linux (RHEL/CentOS)**:
    ```bash
    # 添加 PostgreSQL 17 仓库
    sudo dnf install https://download.postgresql.org/pub/repos/yum/reporpms/EL-8-x86_64/pgdg-redhat-repo-latest.noarch.rpm
    sudo dnf install postgresql17-server
    sudo /usr/pgsql-17/bin/postgresql-17-setup initdb
    sudo systemctl start postgresql-17
    ```
  - **Ubuntu/Debian**:
    ```bash
    sudo apt update
    sudo apt install postgresql-17
    sudo systemctl start postgresql
    ```
  - **验证安装**：`psql --version` 应显示 `psql (PostgreSQL) 17.x`

- **Python 3.8+** - 用于运行脚本工具（system_monitor.py, init_db.py）

#### Python 依赖（用于脚本）
```bash
pip install asyncpg psutil croniter psycopg2
```

> 📦 Python 包说明：
> - `asyncpg` - 异步 PostgreSQL 客户端（system_monitor.py 使用）
> - `psutil` - 系统信息收集（CPU、内存、磁盘、网络）
> - `croniter` - Cron 表达式解析（定时任务调度）
> - `psycopg2` - 同步 PostgreSQL 客户端（init_db.py 使用）

### 1. 克隆项目
```bash
git clone https://github.com/cherry-llt/Guardian.git
cd Guardian
```

### 2. 配置数据库

#### 方法 A：使用初始化脚本（无默认账号）
创建 PostgreSQL 数据库并运行初始化脚本：

```bash
# 编辑 scripts/init_db.py 中的数据库配置（如需要）
python3 scripts/init_db.py
```

这会创建数据库、所有表和初始数据。

#### 方法 B：导入 public.sql（推荐，有默认账号）
如果已有 PostgreSQL 数据库，直接导入 SQL 文件：

```bash
# 确保数据库已创建（如未创建，请先运行 init_db.py 或手动创建）
psql -h 127.0.0.1 -p 5432 -U postgres -d guardian_auth -f scripts/public.sql
```

#### 默认管理员账户

使用 `public.sql` 导入后，系统包含一个默认超级管理员：

- **用户名**：`guardian`
- **密码**：`123456`
- **角色**：超级管理员（SuperAdmin）
- **权限**：拥有所有系统权限

> ⚠️ **安全提醒**：生产环境请立即登录并修改默认密码！

> 💡 **提示**：`public.sql` 文件包含完整的数据库架构和初始数据，包括系统内置角色、权限和默认管理员。

### 3. 配置环境变量

使用 `.env-public` 作为模板创建 `.env` 文件：

```bash
cp .env-public .env
```

编辑 `.env` 文件，配置以下环境变量：

#### 数据库配置
```env
DATABASE_URL=postgresql://postgres:123456@127.0.0.1:5432/guardian_auth
```
- `DATABASE_URL`：PostgreSQL 连接字符串
- 格式：`postgresql://用户名:密码@主机:端口/数据库名`

#### JWT 配置
```env
JWT_SECRET=your-super-secret-jwt-key-min-32-chars
```
- `JWT_SECRET`：JWT 令牌签名密钥
- **⚠️ 生产环境必须使用强随机密钥（至少 32 字符）**
- 用于签名 Access Token 和 Refresh Token

#### 加密配置
```env
ENCRYPTION_KEY=32-byte-encryption-key-for-chacha20
```
- `ENCRYPTION_KEY`：ChaCha20 加密密钥（用于 2FA secret 存储）
- **⚠️ 生产环境必须使用 32 字节强密钥**
- 用于加密/解密 TOTP secret

#### 日志缓冲配置
```env
LOG_BUFFER_SIZE=1000
LOG_BATCH_SIZE=10
LOG_FLUSH_INTERVAL_SECS=3
```
- `LOG_BUFFER_SIZE`：审计日志缓冲池大小
- `LOG_BATCH_SIZE`：每次批量写入的日志数量
- `LOG_FLUSH_INTERVAL_SECS`：批量写入间隔（秒）

#### 速率限制
```env
RATE_LIMIT_MAX_REQUESTS=100
RATE_LIMIT_WINDOW_SECS=60
```
- `RATE_LIMIT_MAX_REQUESTS`：时间窗口内最大请求数
- `RATE_LIMIT_WINDOW_SECS`：速率限制时间窗口（秒）
- 防止暴力攻击和 API 滥用

#### 服务器配置
```env
SERVER_HOST=0.0.0.0
SERVER_PORT=6123
```
- `SERVER_HOST`：服务器监听地址（`0.0.0.0` 监听所有接口）
- `SERVER_PORT`：服务器端口（默认 6123）

#### Python 脚本环境变量（system_monitor.py）

```bash
DB_HOST=127.0.0.1
DB_PORT=5432
DB_USER=postgres
DB_PASSWORD=123456
DB_NAME=guardian_auth
```

> 💡 **提示**：`.env-public` 文件包含所有环境变量的示例配置和详细注释。生产环境请复制为 `.env` 并修改敏感信息。

### 3.5 系统监控脚本（可选）

`scripts/system_monitor.py` 是一个系统性能监控脚本，自动收集系统指标并存储到数据库。

#### 功能特性
- **系统指标收集**：CPU、内存、磁盘、网络使用情况
- **Cron 定时调度**：默认每 5 分钟运行一次（可配置）
- **异步写入**：使用 asyncpg 高效写入数据库
- **优雅关闭**：支持 SIGINT/SIGTERM 信号

#### 运行方式

```bash
# 使用默认配置（每 5 分钟）
python3 scripts/system_monitor.py

# 或自定义 cron 表达式（例如每 10 分钟）
CRON="*/10 * * * *" python3 scripts/system_monitor.py
```

#### 环境变量配置

```bash
export DB_HOST=127.0.0.1
export DB_PORT=5432
export DB_USER=postgres
export DB_PASSWORD=123456
export DB_NAME=guardian_auth
```

#### Cron 表达式示例

- `*/5 * * * *` - 每 5 分钟运行（默认）
- `*/10 * * * *` - 每 10 分钟运行
- `0 */2 * * *` - 每 2 小时运行
- `0 0 * * *` - 每天午夜运行

#### 监控数据

监控数据存储在 `guardian_systeminfo` 表中，可通过 API 端点查询：

```
GET /guardian-auth/v1/systeminfo?page=1&limit=10
```

返回最近的系统监控记录，包括：
- CPU 核心数和使用率
- 内存使用量/总量
- 磁盘使用量/总量
- 网络上传/下载量
- 记录时间戳

> 💡 **提示**：system_monitor.py 是可选的，但推荐在生产环境运行以监控系统健康状况。

### 4. 创建超级管理员（可选）

如未使用 `public.sql` 导入，可通过 API 或直接插入数据库创建新的超级管理员账户。

参考 `public.sql` 中的默认管理员配置作为示例。

### 5. 构建并运行

#### 开发模式
```bash
cargo run
```
服务器将在 `http://localhost:6123` 启动。

#### 生产模式
```bash
# 编译发布版本
cargo build --release

# 运行
./target/release/Guardian
```

#### 验证服务
```bash
# 检查服务器状态
curl http://localhost:6123/health

# 测试默认管理员登录
curl -X POST http://localhost:6123/guardian-auth/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"guardian","password":"123456"}'
```

成功登录应返回：
```json
{
  "code": 200,
  "msg": "登录成功",
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_in": 900
  }
}
```

#### 后台运行（生产环境推荐）

使用 systemd、supervisor 或 pm2 管理服务进程：

**systemd 示例** (`/etc/systemd/system/guardian.service`):

```ini
[Unit]
Description=Guardian Auth Service
After=network.target

[Service]
Type=simple
User=guardian
WorkingDirectory=/opt/guardian
ExecStart=/opt/guardian/target/release/Guardian
Restart=on-failure
RestartSec=10s

[Install]
WantedBy=multi-user.target
```

启动服务：
```bash
sudo systemctl daemon-reload
sudo systemctl enable guardian
sudo systemctl start guardian
sudo systemctl status guardian
```

### 6.[Guardian-Website](https://github.com/cherish-ltt/Guardian-Website)（可选）

Guardian-Website 是基于 **Next.js 16** + **React 19** + **Tailwind CSS 4** + **shadcn/ui** 构建的现代化前端界面，为 Guardian 后端提供完整的 Web 管理界面。

#### 前端核心特性

- 🔐 **完整的认证流程** - 登录、2FA、JWT 令牌管理
- 🎨 **现代化 UI 设计** - shadcn/ui 组件库，支持深色/浅色主题
- 📱 **响应式布局** - 完美适配桌面和移动设备
- 🔄 **完善的 API 集成** - 自动 Token 刷新和错误处理
- 🚀 **高性能架构** - React Server Components，Turbopack 构建优化
- 🌐 **类型安全** - 完整的 TypeScript 类型定义

#### 快速启动前端

```bash
# 克隆前端仓库
git clone https://github.com/cherish-ltt/Guardian-Website.git
cd Guardian-Website

# 安装依赖（需要 pnpm）
pnpm install

# 配置 API 地址
echo 'NEXT_PUBLIC_API_BASE_URL=http://localhost:6123/guardian-auth/v1' > .env.local

# 启动开发服务器
pnpm dev
```

前端将在 `http://localhost:3000` 启动。

#### 前后端集成

- **API 连接**：通过 `NEXT_PUBLIC_API_BASE_URL` 配置
- **认证方式**：JWT Bearer Token 存储
- **错误处理**：统一的 code/msg/data 响应格式
- **完整文档**：前端包含完整的 API v1.0 文档

> 💡 **提示**：Guardian-Website 是**可选的**，Guardian 后端可独立作为 REST API 使用。前端可作为参考实现或直接使用。

## 📚 API 文档

完整的 API 文档请参考：

~~[public-docs/api-v1.0.md](https://github.com/cherish-ltt/Guardian/blob/main/public-docs/api-v1.0.md)~~

[public-docs/api-v1.1.md](https://github.com/cherish-ltt/Guardian/blob/main/public-docs/api-v1.1.md)

### API 概览

#### 认证接口
- `POST /guardian-auth/v1/auth/login` - 登录
- `POST /guardian-auth/v1/auth/refresh` - 刷新令牌
- `POST /guardian-auth/v1/auth/logout` - 登出（需认证）
- `POST /guardian-auth/v1/auth/2fa/setup` - 设置2FA（需认证）
- `POST /guardian-auth/v1/auth/2fa/verify` - 验证2FA（需认证）

#### 管理员接口
- `POST /guardian-auth/v1/admins` - 创建管理员（需认证）
- `GET /guardian-auth/v1/admins` - 查询管理员列表（需认证）
- `GET /guardian-auth/v1/admins/:id` - 获取管理员详情（需认证）
- `PUT /guardian-auth/v1/admins/:id` - 更新管理员（需认证）
- `DELETE /guardian-auth/v1/admins/:id` - 删除管理员（需认证）
- `POST /guardian-auth/v1/admins/:id/change-password` - 修改密码（需认证）

#### 角色接口
- `POST /guardian-auth/v1/roles` - 创建角色（需认证）
- `GET /guardian-auth/v1/roles` - 查询角色列表（需认证）
- `GET /guardian-auth/v1/roles/:id` - 获取角色详情（需认证）
- `PUT /guardian-auth/v1/roles/:id` - 更新角色（需认证）
- `DELETE /guardian-auth/v1/roles/:id` - 删除角色（需认证）
- `POST /guardian-auth/v1/roles/:id/permissions` - 分配权限（需认证）

#### 权限接口
- `GET /guardian-auth/v1/permissions/tree` - 获取权限树（需认证）
- `GET /guardian-auth/v1/permissions` - 查询权限列表（需认证）
- `POST /guardian-auth/v1/permissions` - 创建权限（需认证）
- `PUT /guardian-auth/v1/permissions/:id` - 更新权限（需认证）
- `DELETE /guardian-auth/v1/permissions/:id` - 删除权限（需认证）

## 📖 统一响应格式

所有 API 响应遵循统一格式：

```json
{
  "code": 200,           // 业务状态码，200表示成功
  "msg": "操作成功",     // 消息描述，可为null
  "data": { ... },       // 响应数据，成功时包含
  "timestamp": 1700000000000  // 时间戳（某些响应包含）
}
```

### 状态码说明

| 状态码 | 说明 |
|--------|------|
| 200 | 成功 |
| 1000 | 通用请求失败 |
| 1001 | 未知错误 |
| 17000 | 系统内部错误 |
| 17001 | 参数验证失败 |
| 17002 | 认证失败 |
| 17003 | 令牌过期 |
| 17004 | 权限不足 |
| 17005 | 资源不存在 |
| 17006 | 请求频率过高 |
| 17007 | 2FA验证失败 |
| 17008 | 无效的2FA验证码 |
| 17009 | 未启用2FA |
| 17010 | 已启用2FA |

## 🔒 安全特性

### 密码安全
- ✅ Argon2 哈希加密存储
- ✅ 密码强度要求
- ✅ 登录失败次数限制（5次）
- ✅ 自动锁定机制（15分钟）
- ✅ 密码修改需验证旧密码

### 令牌安全
- ✅ JWT 签名验证
- ✅ Access Token 短期有效（15分钟）
- ✅ Refresh Token 长期有效（7天）
- ✅ 令牌黑名单防止重放攻击
- ✅ 令牌自动刷新机制

### 访问控制
- ✅ 基于 RBAC 的权限控制
- ✅ 超级管理员跳过权限检查
- ✅ 细粒度权限（API/菜单/按钮）
- ✅ IP 级限记录（审计日志）
- ✅ 操作时间戳记录

## 📊 审计日志

审计日志记录以下信息：
- **追踪ID** - 关联同一请求的多个操作
- **管理员信息** - ID、用户名
- **操作类型** - login/logout/create/update/delete
- **资源信息** - 操作的资源路径
- **HTTP 方法** - GET/POST/PUT/DELETE
- **请求参数** - JSONB 格式存储
- **响应结果** - JSONB 格式存储
- **状态码** - HTTP 状态码
- **IP 地址** - 客户端 IP
- **User-Agent** - 客户端标识
- **耗时** - 请求处理时长（毫秒）

## 🏗 项目架构

```
Guardian/
├── src/
│   ├── controller/        # 控制器层 - 处理HTTP请求
│   ├── service/           # 业务逻辑层 - 核心业务逻辑
│   ├── entities/          # 数据模型层 - SeaORM实体
│   ├── dto/               # 数据传输对象 - 请求/响应结构
│   ├── middleware/        # 中间件层 - 认证、审计等
│   ├── utils/             # 工具类 - 加密、JWT等
│   ├── response/          # 响应封装 - 统一响应格式
│   ├── router.rs          # 路由配置
│   ├── error.rs           # 错误定义
│   └── main.rs            # 程序入口
├── scripts/               # 脚本工具
│   ├── init_db.py         # 数据库初始化脚本
│   ├── system_monitor.py  # 系统监控脚本（可选）
│   └── public.sql         # 数据库导入文件（包含默认管理员）
├── design-docs/           # 设计文档
│   └── public-docs/       # 公开文档
│       └── api-v1.0.md    # API详细文档
├── Cargo.toml             # 项目配置
└── README.md              # 项目说明
```

## 🧪 测试

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test auth_service

# 带输出显示测试
cargo test -- --nocapture
```

## 🤝 贡献指南

欢迎贡献！请遵循以下步骤：

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

### 代码规范

- 遵循 Rust 官方风格指南
- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查代码
- 添加单元测试
- 更新相关文档

## 📜 License

本项目采用 MIT 许可证 - 详见 [LICENSE](https://github.com/cherish-ltt/Guardian/blob/main/LICENSE) 文件。

## 👥 作者

Guardian Team - @opencode - <opencode@opencode.ai>

## 🙏 致谢

感谢本项目中所使用的所有开源项目和crates.io库

如果这个项目对你有帮助，请给一个Star⭐️

------

<div align="center">  
  <p>Built with ❤️ by the Guardian team</p>

