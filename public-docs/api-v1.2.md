

<div align="center">
  <h1>Guardian Auth API v1.2</h1>
  <p>
  <a href="https://img.shields.io/badge/version-v1.2-blue.svg">
    <img src="https://img.shields.io/badge/version-v1.2-blue.svg" alt="license"/>
  </a>
  <a href="https://img.shields.io/badge/status-stable-green.svg">
    <img src="https://img.shields.io/badge/status-stable-green.svg" alt="license"/>
  </a>
  </p>
</div>


完整的 Guardian 认证系统 API 文档。

## 📋 目录

- [概述](#概述)
- [认证方式](#认证方式)
- [通用响应格式](#通用响应格式)
- [认证接口](#认证接口)
- [管理员接口](#管理员接口)
- [角色接口](#角色接口)
- [权限接口](#权限接口)
- [系统信息接口](#系统信息接口)
- [错误码](#错误码)

---

## 概述

Guardian API v1.2 提供了完整的用户认证、权限管理、操作审计和系统监控功能。

**Base URL**: `http://localhost:6123/guardian-auth/v1`

**Content-Type**: `application/json`

**字符编码**: `UTF-8`

**v1.2 新增功能**：
- ✅ **管理员-角色绑定**：为指定管理员分配/替换角色
- ✅ **权限验证中间件**：API 类型权限验证，支持通配符和路径参数匹配
- ✅ **完整的 CRUD 接口**：管理员、角色、权限的全套增删改查功能

---

## 认证方式

### JWT 认证

对于需要认证的接口，使用 `Authorization` 请求头：

```
Authorization: Bearer <access_token>
```

**令牌说明**:
- **Access Token**: 有效期 15 分钟，用于访问受保护接口
- **Refresh Token**: 有效期 7 天，用于刷新 Access Token

---

## 通用响应格式

所有接口返回统一格式的 JSON：

```json
{
  "code": 200,           // 业务状态码，200 表示成功
  "msg": "操作成功",      // 消息描述，可为 null
  "data": { ... },         // 响应数据，成功时包含
  "timestamp": 1700000000000  // 时间戳（部分接口包含）
}
```

### 成功响应示例

```json
{
  "code": 200,
  "msg": null,
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_in": 900
  }
}
```

### 失败响应示例

```json
{
  "code": 17002,
  "msg": "用户名或密码错误",
  "data": null
}
```

---

## 认证接口

### 用户登录

**接口描述**: 使用用户名和密码登录系统，获取访问令牌

**请求方式**: `POST`

**请求路径**: `/auth/login`

**请求头**:
```
Content-Type: application/json
```

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| username | string | 是 | 用户名 |
| password | string | 是 | 密码 |
| two_fa_code | string | 否 | 2FA验证码（如果账户启用了2FA，此参数必填） |

**请求示例**:

```bash
curl -X POST http://localhost:6123/guardian-auth/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "123456"
  }'
```

**响应示例**:

```json
{
  "code": 200,
  "msg": null,
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_in": 900
  }
}
```

**响应字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| access_token | string | 访问令牌 |
| refresh_token | string | 刷新令牌 |
| expires_in | number | 访问令牌有效期（秒） |

**错误响应示例**:

```json
{
  "code": 17002,
  "msg": "用户名或密码错误",
  "data": null
}
```

---

### 刷新令牌

**接口描述**: 使用 refresh token 获取新的 access token

**请求方式**: `POST`

**请求路径**: `/auth/refresh`

**请求头**:
```
Content-Type: application/json
```

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| refresh_token | string | 是 | 刷新令牌 |

**请求示例**:

```bash
curl -X POST http://localhost:6123/guardian-auth/v1/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  }'
```

**响应示例**:

```json
{
  "code": 200,
  "msg": null,
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_in": 900
  }
}
```

**响应字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| access_token | string | 新的访问令牌 |
| expires_in | number | 访问令牌有效期（秒） |

---

### 用户登出

**接口描述**: 登出系统，使 refresh token 失效

**请求方式**: `POST`

**请求路径**: `/auth/logout`

**认证**: 需要 JWT

**请求头**:
```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| refresh_token | string | 是 | 要失效的刷新令牌 |

**请求示例**:

```bash
curl -X POST http://localhost:6123/guardian-auth/v1/auth/logout \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  }'
```

**响应示例**:

```json
{
  "code": 200,
  "msg": null,
  "data": null
}
```

---

### 设置2FA

**接口描述**: 为当前账户启用双因素认证（2FA）

**请求方式**: `POST`

**请求路径**: `/auth/2fa/setup`

**认证**: 需要 JWT

**请求头**:
```
Authorization: Bearer <access_token>
```

**请求参数**: 无（使用 JWT 中的用户信息）

**请求示例**:

```bash
curl -X POST http://localhost:6123/guardian-auth/v1/auth/2fa/setup \
  -H "Authorization: Bearer <access_token>"
```

**响应示例**:

```json
{
  "code": 200,
  "msg": null,
  "data": {
    "secret": "JBSWY3DPEHPK3PXP",
    "qr_code_url": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAA...",
    "backup_codes": [
      "12345678",
      "23456789",
      "34567890",
      "45678901",
      "56789012",
      "67890123",
      "78901234",
      "89012345",
      "90123456",
      "01234567"
    ]
  }
}
```

**响应字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| secret | string | TOTP 密钥 |
| qr_code_url | string | 二维码图片（Base64 编码） |
| backup_codes | array | 10个备用验证码 |

**业务规则**:
- 每个账户只能设置一次 2FA
- 如果已经启用 2FA，会返回错误（17010: 已启用2FA）
- 响应包含 TOTP 密钥、二维码和10个备用验证码
- 请妥善保存备用验证码，每个验证码只能使用一次

**错误响应示例**:

```json
{
  "code": 17010,
  "msg": "已启用2FA",
  "data": null
}
```

---

### 验证2FA

**接口描述**: 验证用户输入的2FA验证码是否正确

**请求方式**: `POST`

**请求路径**: `/auth/2fa/verify`

**认证**: 需要 JWT

**请求头**:
```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| code | string | 是 | 6位数字的 TOTP 验证码 |

**请求示例**:

```bash
curl -X POST http://localhost:6123/guardian-auth/v1/auth/2fa/verify \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "code": "123456"
  }'
```

**响应示例**:

```json
{
  "code": 200,
  "msg": null,
  "data": {
    "verified": true
  }
}
```

**响应字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| verified | boolean | 验证是否成功 |

**错误响应示例**:

```json
{
  "code": 17008,
  "msg": "无效的2FA验证码",
  "data": null
}
```

**业务规则**:
- 如果未启用 2FA，会返回错误（17009: 未启用2FA）
- 验证码有效期为 30 秒
- 验证失败不会锁定账户

---

### 禁用2FA

**接口描述**: 禁用当前账户的双因素认证

**请求方式**: `POST`

**请求路径**: `/auth/2fa/disable`

**认证**: 需要 JWT

**请求头**:
```
Authorization: Bearer <access_token>
```

**请求参数**: 无（使用 JWT 中的用户信息）

**请求示例**:

```bash
curl -X POST http://localhost:6123/guardian-auth/v1/auth/2fa/disable \
  -H "Authorization: Bearer <access_token>"
```

**响应示例**:

```json
{
  "code": 200,
  "msg": null,
  "data": {
    "disabled": true
  }
}
```

**响应字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| disabled | boolean | 是否成功禁用 |

---

### 重置密码（通过 2FA）

**接口描述**: 使用用户名和 2FA 验证码重置密码（无需登录）

**请求方式**: `POST`

**请求路径**: `/auth/reset-password`

**认证**: 无需认证（公开接口）

**请求头**:
```
Content-Type: application/json
```

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| username | string | 是 | 用户名 |
| two_fa_code | string | 是 | 6位数字的 TOTP 验证码 |
| new_password | string | 是 | 新密码 |

**请求示例**:

```bash
curl -X POST http://localhost:6123/guardian-auth/v1/auth/reset-password \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "two_fa_code": "123456",
    "new_password": "NewSecurePassword123"
  }'
```

**响应示例**:

```json
{
  "code": 200,
  "msg": "密码重置成功",
  "data": null
}
```

**错误响应示例**:

用户不存在：
```json
{
  "code": 17005,
  "msg": "用户不存在",
  "data": null
}
```

未启用 2FA：
```json
{
  "code": 17009,
  "msg": "未启用2FA，无法通过此方式重置密码",
  "data": null
}
```

2FA 验证码错误：
```json
{
  "code": 17008,
  "msg": "无效的2FA验证码",
  "data": null
}
```

**业务规则**:
- 账户必须启用 2FA 才能使用此接口
- 2FA 验证码有效期为 30 秒
- 新密码会使用 Argon2 算法哈希后存储
- 密码重置后会自动更新 `updated_at` 时间戳
- 此接口无需 JWT token，适用于忘记密码场景

---

### 修改密码（需要登录）

**接口描述**: 登录后修改自己的密码（需要 JWT 认证）

**请求方式**: `POST`

**请求路径**: `/auth/change-password`

**认证**: 需要 JWT

**请求头**:
```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| new_password | string | 是 | 新密码（将进行 Argon2 哈希） |

**请求示例**:

```bash
curl -X POST http://localhost:6123/guardian-auth/v1/auth/change-password \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "new_password": "NewSecurePassword456"
  }'
```

**响应示例**:

```json
{
  "code": 200,
  "msg": "密码修改成功",
  "data": null
}
```

**错误响应示例**:

```json
{
  "code": 17000,
  "msg": "管理员不存在",
  "data": null
}
```

**业务规则**:
- 需要先登录获取 JWT token
- 从 JWT token 中自动提取用户 ID
- 新密码会使用 Argon2 算法哈希后存储
- 密码修改后会自动更新 `updated_at` 时间戳
- 此接口不需要验证旧密码（适用于已认证用户主动修改密码）

---

## 管理员接口

### 创建管理员

**接口描述**: 创建新的管理员账号

**请求方式**: `POST`

**请求路径**: `/admins`

**认证**: 需要 JWT

**请求头**:
```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| username | string | 是 | 用户名（唯一） |
| password | string | 是 | 密码（明文，将进行 Argon2 哈希） |
| is_super_admin | boolean | 否 | 是否为超级管理员（默认 false） |
| role_ids | array | 否 | 关联的角色 ID 数组（UUID） |

**请求示例**:

```bash
curl -X POST http://localhost:6123/guardian-auth/v1/admins \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "username": "newadmin",
    "password": "SecurePass123",
    "is_super_admin": false,
    "role_ids": ["0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d"]
  }'
```

**响应示例**:

```json
{
  "code": 200,
  "msg": "创建成功",
  "data": {
    "id": "0190b2f9-8c4f-8b4g-9d2b-0f3g4b5c6d7e",
    "username": "newadmin",
    "is_super_admin": false,
    "status": 1,
    "created_at": "2024-01-01T12:00:00Z"
  }
}
```

**响应字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| id | UUID | 管理员 ID |
| username | string | 用户名 |
| is_super_admin | boolean | 是否为超级管理员 |
| status | number | 状态（1-正常，0-禁用） |
| created_at | datetime | 创建时间（ISO 8601 格式） |

---

### 查询管理员列表

**接口描述**: 分页查询管理员列表

**请求方式**: `GET`

**请求路径**: `/admins`

**认证**: 需要 JWT

**请求头**:
```
Authorization: Bearer <access_token>
```

**查询参数**:

| 参数名 | 类型 | 必填 | 默认值 | 说明 |
|--------|------|--------|--------|------|
| page | number | 否 | 1 | 页码 |
| page_size | number | 否 | 20 | 每页数量 |
| status | number | 否 | - | 状态筛选（1-正常，0-禁用） |
| keyword | string | 否 | - | 用户名关键字搜索 |

**请求示例**:

```bash
# 获取第 1 页，每页 20 条
curl -X GET "http://localhost:6123/guardian-auth/v1/admins?page=1&page_size=20" \
  -H "Authorization: Bearer <access_token>"

# 使用 keyword 搜索
curl -X GET "http://localhost:6123/guardian-auth/v1/admins?keyword=admin&page=1&page_size=20" \
  -H "Authorization: Bearer <access_token>"

# 同时筛选状态
curl -X GET "http://localhost:6123/guardian-auth/v1/admins?status=1&page=1&page_size=20" \
  -H "Authorization: Bearer <access_token>"
```

**响应示例**:

```json
{
  "code": 200,
  "msg": null,
  "data": {
    "total": 100,
    "page": 1,
    "page_size": 20,
    "list": [
      {
        "id": "0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d",
        "username": "admin",
        "is_super_admin": true,
        "status": 1,
        "last_login_at": "2024-01-01T10:30:00Z",
        "created_at": "2023-12-01T00:00:00Z"
      }
    ]
  }
}
```

**响应字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| total | number | 总记录数 |
| page | number | 当前页码 |
| page_size | number | 每页数量 |
| list | array | 管理员列表 |

**list 项字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| id | UUID | 管理员 ID |
| username | string | 用户名 |
| is_super_admin | boolean | 是否为超级管理员 |
| status | number | 状态（1-正常，0-禁用） |
| last_login_at | datetime | 最后登录时间（ISO 8601 格式） |
| created_at | datetime | 创建时间（ISO 8601 格式） |

---

### 获取管理员详情

**接口描述**: 获取指定管理员的详细信息

**请求方式**: `GET`

**请求路径**: `/admins/:id`

**认证**: 需要 JWT

**请求头**:
```
Authorization: Bearer <access_token>
```

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| id | string(UUID) | 是 | 管理员 ID |

**请求示例**:

```bash
curl -X GET http://localhost:6123/guardian-auth/v1/admins/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d \
  -H "Authorization: Bearer <access_token>"
```

**响应示例**:

```json
{
  "code": 200,
  "msg": null,
  "data": {
    "id": "0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d",
    "username": "admin",
    "is_super_admin": true,
    "status": 1,
    "last_login_at": "2024-01-01T10:30:00Z",
    "login_attempts": 0,
    "locked_until": null,
    "created_at": "2023-12-01T00:00:00Z",
    "updated_at": "2024-01-01T12:00:00Z",
    "roles": [
      {
        "id": "0190b2f9-8c4f-8b4g-9d2b-0f3g4b5c6d7e",
        "code": "ADMIN",
        "name": "管理员角色"
      },
      {
        "id": "0190c3g0-9d5g-9c5h-0e3c-1g4h5c6d7e8f",
        "code": "EDITOR",
        "name": "编辑器角色"
      }
    ]
  }
}
```

**响应字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| id | UUID | 管理员 ID |
| username | string | 用户名 |
| is_super_admin | boolean | 是否为超级管理员 |
| status | number | 状态（1-正常，0-禁用） |
| last_login_at | datetime | 最后登录时间（ISO 8601 格式） |
| login_attempts | number | 登录失败次数 |
| locked_until | datetime | 锁定到期时间（ISO 8601 格式，未锁定则为 null） |
| created_at | datetime | 创建时间（ISO 8601 格式） |
| updated_at | datetime | 更新时间（ISO 8601 格式） |
| roles | array | 关联的角色列表 |

**roles 项字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| id | UUID | 角色 ID |
| code | string | 角色代码 |
| name | string | 角色名称 |

---

### 更新管理员

**接口描述**: 更新管理员信息

**请求方式**: `PUT`

**请求路径**: `/admins/:id`

**认证**: 需要 JWT

**请求头**:
```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| id | string(UUID) | 是 | 管理员 ID |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| password | string | 否 | 新密码（将进行 Argon2 哈希） |
| status | number | 否 | 状态（1-正常，0-禁用） |
| role_ids | array | 否 | 关联的角色 ID 数组（UUID，全量替换） |

**请求示例**:

```bash
# 更新管理员密码和状态
curl -X PUT http://localhost:6123/guardian-auth/v1/admins/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "password": "NewSecurePass456",
    "status": 1
  }'

# 更新管理员角色关联
curl -X PUT http://localhost:6123/guardian-auth/v1/admins/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "role_ids": ["0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d", "0190b2f9-8c4f-8b4g-9d2b-0f3g4b5c6d7e"]
  }'
```

**响应示例**:

```json
{
  "code": 200,
  "msg": "更新成功",
  "data": {
    "id": "0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d",
    "username": "admin",
    "is_super_admin": false,
    "status": 1,
    "updated_at": "2024-01-01T12:00:00Z"
  }
}
```

---

### 删除管理员

**接口描述**: 删除指定的管理员账号

**请求方式**: `DELETE`

**请求路径**: `/admins/:id`

**认证**: 需要 JWT

**请求头**:
```
Authorization: Bearer <access_token>
```

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| id | string(UUID) | 是 | 要删除的管理员 ID |

**请求示例**:

```bash
curl -X DELETE http://localhost:6123/guardian-auth/v1/admins/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d \
  -H "Authorization: Bearer <access_token>"
```

**响应示例**:

```json
{
  "code": 200,
  "msg": "删除成功",
  "data": null
}
```

**业务规则**:
- 超级管理员不能被删除
- 不能删除自己

---

### 为管理员分配角色（**v1.2 新增**）

**接口描述**: 为指定管理员分配/替换角色

**请求方式**: `POST`

**请求路径**: `/admins/:id/roles`

**认证**: 需要 JWT

**请求头**:
```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| id | string(UUID) | 是 | 管理员 ID |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| role_ids | array | 是 | 要分配的角色 ID 数组（UUID，全量替换） |

**请求示例**:

```bash
# 分配多个角色
curl -X POST http://localhost:6123/guardian-auth/v1/admins/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d/roles \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "role_ids": ["0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d", "0190b2f9-8c4f-8b4g-9d2b-0f3g4b5c6d7e", "0190c3g0-9d5g-9c5h-0e3c-1g4h5c6d7e8f"]
  }'

# 清空所有角色
curl -X POST http://localhost:6123/guardian-auth/v1/admins/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d/roles \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "role_ids": []
  }'
```

**响应示例**:

```json
{
  "code": 200,
  "msg": "角色分配成功",
  "data": null
}
```

**业务规则**:
- 超级管理员不会被分配角色
- 如果管理员已经是超级管理员，返回错误（17004）
- 此接口会全量替换角色关联

---

## 角色接口

### 创建角色

**接口描述**: 创建新的角色

**请求方式**: `POST`

**请求路径**: `/roles`

**认证**: 需要 JWT

**请求头**:
```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| code | string | 是 | 角色代码（唯一，如 "admin", "editor"） |
| name | string | 是 | 角色名称 |
| description | string | 否 | 角色描述 |
| permission_ids | array | 否 | 关联的权限 ID 数组（UUID） |

**请求示例**:

```bash
# 创建角色并分配权限
curl -X POST http://localhost:6123/guardian-auth/v1/roles \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "code": "EDITOR_ROLE",
    "name": "编辑器角色",
    "description": "可以编辑内容权限",
    "permission_ids": ["0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d", "0190b2f9-8c4f-8b4g-9d2b-0f3g4b5c6d7e"]
  }'

# 创建角色（不分配权限）
curl -X POST http://localhost:6123/guardian-auth/v1/roles \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "code": "VIEWER_ROLE",
    "name": "查看器角色",
    "description": "只能查看权限"
  }'
```

**响应示例**:

```json
{
  "code": 200,
  "msg": "创建成功",
  "data": {
    "id": "0190b2f9-8c4f-8b4g-9d2b-0f3g4b5c6d7e",
    "code": "EDITOR_ROLE",
    "name": "编辑器角色",
    "description": "可以编辑内容权限",
    "is_system": false,
    "created_at": "2024-01-01T12:00:00Z"
  }
}
```

**响应字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| id | UUID | 角色 ID |
| code | string | 角色代码 |
| name | string | 角色名称 |
| description | string | 角色描述 |
| is_system | boolean | 是否为系统内置角色 |
| created_at | datetime | 创建时间（ISO 8601 格式） |

---

### 查询角色列表

**接口描述**: 分页查询角色列表

**请求方式**: `GET`

**请求路径**: `/roles`

**认证**: 需要 JWT

**请求头**:
```
Authorization: Bearer <access_token>
```

**查询参数**:

| 参数名 | 类型 | 必填 | 默认值 | 说明 |
|--------|------|--------|--------|------|
| page | number | 否 | 1 | 页码 |
| page_size | number | 否 | 20 | 每页数量 |
| keyword | string | 否 | - | 角色名或代码关键字搜索 |

**请求示例**:

```bash
# 获取第 1 页，每页 20 条
curl -X GET http://localhost:6123/guardian-auth/v1/roles?page=1&page_size=20 \
  -H "Authorization: Bearer <access_token>"

# 使用 keyword 搜索
curl -X GET "http://localhost:6123/guardian-auth/v1/roles?keyword=editor&page=1&page_size=20" \
  -H "Authorization: Bearer <access_token>"
```

**响应示例**:

```json
{
  "code": 200,
  "msg": null,
  "data": {
    "total": 50,
    "page": 1,
    "page_size": 20,
    "list": [
      {
        "id": "0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d",
        "code": "EDITOR_ROLE",
        "name": "编辑器角色",
        "description": "可以编辑内容和权限",
        "is_system": false,
        "created_at": "2024-01-01T12:00:00Z"
      }
    ]
  }
}
```

**响应字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| total | number | 总记录数 |
| page | number | 当前页码 |
| page_size | number | 每页数量 |
| list | array | 角色列表 |

---

### 获取角色详情

**接口描述**: 获取角色的详细信息，包括关联的权限列表

**请求方式**: `GET`

**请求路径**: `/roles/:id`

**认证**: 需要 JWT

**请求头**:
```
Authorization: Bearer <access_token>
```

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| id | string(UUID) | 是 | 角色 ID |

**请求示例**:

```bash
curl -X GET http://localhost:6123/guardian-auth/v1/roles/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d \
  -H "Authorization: Bearer <access_token>"
```

**响应示例**:

```json
{
  "code": 200,
  "msg": null,
  "data": {
    "id": "0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d",
    "code": "EDITOR_ROLE",
    "name": "编辑器角色",
    "description": "可以编辑内容和权限",
    "is_system": false,
    "permissions": [
      {
        "id": "0190b2f9-8c4f-8b4g-9d2b-0f3g4b5c6d7e",
        "code": "ADMIN_READ",
        "name": "管理员列表",
        "resource_type": "api",
        "http_method": "GET",
        "resource_path": "/guardian-auth/v1/admins",
        "sort_order": 1
      },
      {
        "id": "0190c3g0-9d5g-9c5h-0e3c-1g4h5c6d7e8f",
        "code": "ADMIN_CREATE",
        "name": "创建管理员",
        "resource_type": "api",
        "http_method": "POST",
        "resource_path": "/guardian-auth/v1/admins",
        "sort_order": 2
      }
    ],
    "created_at": "2024-01-01T12:00:00Z",
    "updated_at": "2024-01-01T09:15:00Z"
  }
}
```

**响应字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| id | UUID | 角色 ID |
| code | string | 角色代码 |
| name | string | 角色名称 |
| description | string | 角色描述 |
| is_system | boolean | 是否为系统内置角色 |
| permissions | array | 关联的权限列表 |
| created_at | datetime | 创建时间（ISO 8601 格式） |
| updated_at | datetime | 更新时间（ISO 8601 格式） |

**permissions 项字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| id | UUID | 权限 ID |
| code | string | 权限代码 |
| name | string | 权限名称 |
| resource_type | string | 资源类型（api/menu/button） |
| http_method | string | HTTP 方法 |
| resource_path | string | 资源路径 |
| sort_order | number | 排序字段 |

---

### 更新角色

**请求方式**: `PUT`

**请求路径**: `/roles/:id`

**认证**: 需要 JWT

**请求头**:
```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| id | string(UUID) | 是 | 角色 ID |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| name | string | 否 | 角色名称 |
| description | string | 否 | 角色描述 |
| permission_ids | array | 否 | 关联的权限 ID 数组（UUID，全量替换） |

**请求示例**:

```bash
# 更新角色名称
curl -X PUT http://localhost:6123/guardian-auth/v1/roles/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "name": "编辑器角色（已更新）"
  }'

# 更新角色描述
curl -X PUT http://localhost:6123/guardian-auth/v1/roles/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "description": "更新了角色描述"
  }'

# 更新角色权限（替换）
curl -X PUT http://localhost:6123/guardian-auth/v1/roles/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "permission_ids": ["0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d", "0190b2f9-8c4f-8b4g-9d2b-0f3g4b5c6d7e", "0190c3g0-9d5g-9c5h-0e3c-1g4h5c6d7e8f"]
  }'
```

**响应示例**:

```json
{
  "code": 200,
  "msg": "更新成功",
  "data": {
    "id": "0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d",
    "code": "EDITOR_ROLE",
    "name": "编辑器角色（已更新）",
    "is_system": false,
    "created_at": "2024-01-01T12:00:00Z"
  }
}
```

**业务规则**:
- 不能修改系统内置角色（`is_system = true`）的 code 字段

---

### 删除角色

**接口描述**: 删除指定的角色

**请求方式**: `DELETE`

**请求路径**: `/roles/:id`

**认证**: 需要 JWT

**请求头**:
```
Authorization: Bearer <access_token>
```

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| id | string(UUID) | 是 | 角色 ID |

**请求示例**:

```bash
curl -X DELETE http://localhost:6123/guardian-auth/v1/roles/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d \
  -H "Authorization: Bearer <access_token>"
```

**响应示例**:

```json
{
  "code": 200,
  "msg": "删除成功",
  "data": null
}
```

**业务规则**:
- 不能删除系统内置角色（`is_system = true`）
- 有管理员关联的角色不能删除
- 删除角色前需要解除所有管理员关联

---

### 为角色分配权限（**v1.2 新增**）

**接口描述**: 为角色分配/替换权限

**请求方式**: `POST`

**请求路径**: `/roles/:id/permissions`

**认证**: 需要 JWT

**请求头**:
```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| id | string(UUID) | 是 | 角色 ID |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| permission_ids | array | 是 | 要分配的权限 ID 数组（UUID，全量替换） |

**请求示例**:

```bash
# 为角色分配 3 个权限
curl -X POST http://localhost:6123/guardian-auth/v1/roles/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d/permissions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "permission_ids": ["0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d", "0190b2f9-8c4f-8b4g-9d2b-0f3g4b5c6d7e", "0190c3g0-9d5g-9c5h-0e3c-1g4h5c6d7e8f"]
  }'

# 清空所有权限
curl -X POST http://localhost:6123/guardian-auth/v1/roles/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d/permissions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "permission_ids": []
  }'
```

**响应示例**:

```json
{
  "code": 200,
  "msg": "权限分配成功",
  "data": null
}
```

**业务规则**:
- 此接口会全量替换角色关联的权限
- 不能为系统内置角色分配权限

---

## 权限接口

### 获取权限树

**接口描述**: 获取权限的树形结构

**请求方式**: `GET`

**请求路径**: `/permissions/tree`

**认证**: 需要 JWT

**请求头**:
```
Authorization: Bearer <access_token>
```

**请求示例**:

```bash
curl -X GET http://localhost:6123/guardian-auth/v1/permissions/tree \
  -H "Authorization: Bearer <access_token>"
```

**响应示例**:

```json
{
  "code": 200,
  "msg": null,
  "data": [
    {
      "id": "0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d",
      "code": "USER_MANAGE",
      "name": "用户管理",
      "description": "用户管理相关权限",
      "resource_type": "menu",
      "sort_order": 1,
      "is_system": false,
      "children": [
        {
          "id": "0190b2f9-8c4f-8b4g-9d2b-0f3g4b5c6d7e",
          "code": "USER_CREATE",
          "name": "创建用户",
          "description": "创建新用户",
          "resource_type": "api",
          "http_method": "POST",
          "resource_path": "/guardian-auth/v1/admins",
          "sort_order": 1,
          "is_system": false,
          "children": []
        },
        {
          "id": "0190c3g0-9d5g-9c5h-0e3c-1g4h5c6d7e8f",
          "code": "USER_READ",
          "name": "查看用户",
          "description": "查看用户列表",
          "resource_type": "api",
          "http_method": "GET",
          "resource_path": "/guardian-auth/v1/admins",
          "sort_order": 2,
          "is_system": false,
          "children": []
        }
      ]
    }
  ]
}
```

**响应字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| id | UUID | 权限 ID |
| code | string | 权限代码 |
| name | string | 权限名称 |
| description | string | 权限描述 |
| resource_type | string | 资源类型（api/menu/button） |
| http_method | string | HTTP 方法 |
| resource_path | string | 资源路径 |
| sort_order | number | 排序字段 |
| is_system | boolean | 是否为系统内置权限 |
| children | array | 子权限列表 |

---

### 查询权限列表

**接口描述**: 分页查询权限列表

**请求方式**: `GET`

**请求路径**: `/permissions`

**认证**: 需要 JWT

**请求头**:
```
Authorization: Bearer <access_token>
```

**查询参数**:

| 参数名 | 类型 | 必填 | 默认值 | 说明 |
|--------|------|--------|--------|------|
| page | number | 否 | 1 | 页码 |
| page_size | number | 否 | 20 | 每页数量 |
| resource_type | string | 否 | - | 资源类型筛选（api/menu/button） |
| keyword | string | 否 | - | 权限名或代码关键字搜索 |

**请求示例**:

```bash
# 获取 API 类型权限
curl -X GET "http://localhost:6123/guardian-auth/v1/permissions?resource_type=api&page=1&page_size=20" \
  -H "Authorization: Bearer <access_token>"

# 获取 Menu 类型权限
curl -X GET "http://localhost:6123/guardian-auth/v1/permissions?resource_type=menu&page=1&page_size=20" \
  -H "Authorization: Bearer <access_token>"

# 使用 keyword 搜索
curl -X GET "http://localhost:6123/guardian-auth/v1/permissions?keyword=user&page=1&page_size=20" \
  -H "Authorization: Bearer <access_token>"
```

**响应示例**:

```json
{
  "code": 200,
  "msg": null,
  "data": {
    "total": 50,
    "page": 1,
    "page_size": 20,
    "list": [
      {
        "id": "0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d",
        "code": "ADMIN_READ",
        "name": "管理员列表",
        "description": "查看管理员列表",
        "resource_type": "api",
        "http_method": "GET",
        "resource_path": "/guardian-auth/v1/admins",
        "sort_order": 1,
        "is_system": false,
        "created_at": "2024-01-01T00:00:00Z"
      },
      {
        "id": "0190b2f9-8c4f-8b4g-9d2b-0f3g4b5c6d7e",
        "code": "ADMIN_CREATE",
        "name": "创建管理员",
        "description": "创建新管理员",
        "resource_type": "api",
        "http_method": "POST",
        "resource_path": "/guardian-auth/v1/admins",
        "sort_order": 2,
        "is_system": false,
        "created_at": "2024-01-01T00:00:00Z"
      }
    ]
  }
}
```

**响应字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| total | number | 总记录数 |
| page | number | 当前页码 |
| page_size | number | 每页数量 |
| list | array | 权限列表 |

**list 项字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| id | UUID | 权限 ID |
| code | string | 权限代码 |
| name | string | 权限名称 |
| description | string | 权限描述 |
| resource_type | string | 资源类型（api/menu/button） |
| http_method | string | HTTP 方法 |
| resource_path | string | 资源路径 |
| parent_id | UUID | 父权限 ID |
| sort_order | number | 排序字段 |
| is_system | boolean | 是否为系统内置权限 |
| created_at | datetime | 创建时间（ISO 8601 格式） |

---

### 创建权限

**接口描述**: 创建新的权限

**请求方式**: `POST`

**请求路径**: `/permissions`

**认证**: 需要 JWT

**请求头**:
```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| code | string | 是 | 权限代码（唯一） |
| name | string | 是 | 权限名称 |
| description | string | 否 | 权限描述 |
| resource_type | string | 是 | 资源类型（api/menu/button） |
| http_method | string | 否 | HTTP 方法（GET/POST/PUT/DELETE） |
| resource_path | string | 否 | 资源路径（支持通配符 * 和路径参数 {id}） |
| parent_id | string(UUID) | 否 | 父权限 ID |
| sort_order | number | 否 | 排序字段 |

**请求示例**:

```bash
# 创建管理员列表 API 权限
curl -X POST http://localhost:6123/guardian-auth/v1/permissions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "code": "ADMIN_READ",
    "name": "管理员列表",
    "resource_type": "api",
    "http_method": "GET",
    "resource_path": "/guardian-auth/v1/admins"
  }'

# 创建菜单权限
curl -X POST http://localhost:6123/guardian-auth/v1/permissions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "code": "MENU_VIEW",
    "name": "菜单查看",
    "resource_type": "menu",
    "resource_path": "/guardian-auth/v1/menu/view",
    "sort_order": 1
  }'

# 创建按钮权限
curl -X POST http://localhost:6123/guardian-auth/v1/permissions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "code": "BUTTON_CLICK",
    "name": "按钮点击",
    "resource_type": "button",
    "resource_path": "/guardian-auth/v1/button/click",
    "http_method": "POST",
    "sort_order": 1
  }'
```

**权限类型说明**：
- `api` - API 权限：用于后端 API 访问控制
- `menu` - 菜单权限：用于前端菜单显示
- `button` - 按钮权限：用于前端按钮显示

**响应示例**:

```json
{
  "code": 200,
  "msg": null,
  "data": {
    "id": "0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d",
    "code": "ADMIN_READ",
    "name": "管理员列表",
    "description": "查看管理员列表",
    "resource_type": "api",
    "http_method": "GET",
    "resource_path": "/guardian-auth/v1/admins",
    "sort_order": 1,
    "is_system": false,
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

---

### 获取权限详情

**接口描述**: 获取指定权限的详细信息

**请求方式**: `GET`

**请求路径**: `/permissions/:id`

**认证**: 需要 JWT

**请求头**:
```
Authorization: Bearer <access_token>
```

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| id | string(UUID) | 是 | 权限 ID |

**请求示例**:

```bash
curl -X GET http://localhost:6123/guardian-auth/v1/permissions/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d \
  -H "Authorization: Bearer <access_token>"
```

**响应示例**:

```json
{
  "code": 200,
  "msg": null,
  "data": {
    "id": "0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d",
    "code": "ADMIN_READ",
    "name": "管理员列表",
    "description": "查看管理员列表",
    "resource_type": "api",
    "http_method": "GET",
    "resource_path": "/guardian-auth/v1/admins",
    "parent_id": null,
    "sort_order": 1,
    "is_system": false,
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

---

### 更新权限

**接口描述**: 更新权限信息

**请求方式**: `PUT`

**请求路径**: `/permissions/:id`

**认证**: 需要 JWT

**请求头**:
```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| id | string(UUID) | 是 | 权限 ID |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| name | string | 否 | 权限名称 |
| description | string | 否 | 权限描述 |
| resource_type | string | 否 | 资源类型 |
| http_method | string | 否 | HTTP 方法 |
| resource_path | string | 否 | 资源路径 |
| parent_id | string(UUID) | 否 | 父权限 ID |
| sort_order | number | 否 | 排序字段 |

**请求示例**:

```bash
# 更新权限名称和描述
curl -X PUT http://localhost:6123/guardian-auth/v1/permissions/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "name": "管理员列表（已更新）",
    "description": "更新了描述"
  }'

# 更新权限类型和路径
curl -X PUT http://localhost:6123/guardian-auth/v1/permissions/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -d '{
    "resource_type": "button",
    "resource_path": "/guardian-auth/v1/button/click",
    "sort_order": 1
  }'
```

**响应示例**:

```json
{
  "code": 200,
  "msg": "更新成功",
  "data": {
    "id": "0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d",
    "code": "ADMIN_READ",
    "name": "管理员列表（已更新）",
    "description": "更新了描述",
    "resource_type": "button",
    "resource_path": "/guardian-auth/v1/button/click",
    "sort_order": 1,
    "is_system": false,
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

---

### 删除权限

**接口描述**: 删除指定的权限

**请求方式**: `DELETE`

**请求路径**: `/permissions/:id`

**认证**: 需要 JWT

**请求头**:
```
Authorization: Bearer <access_token>
```

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| id | string(UUID) | 是 | 权限 ID |

**请求示例**:

```bash
curl -X DELETE http://localhost:6123/guardian-auth/v1/permissions/0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d \
  -H "Authorization: Bearer <access_token>"
```

**响应示例**:

```json
{
  "code": 200,
  "msg": "删除成功",
  "data": null
}
```

**业务规则**:
- 不能删除系统内置权限（`is_system = true`）
- 有角色关联的权限不能删除
- 删除权限前需要从角色中移除关联

---

## 系统信息接口

### 查询系统信息列表

**接口描述**: 获取系统监控信息列表，包括 CPU、内存、磁盘和网络使用情况

**请求方式**: `GET`

**请求路径**: `/systeminfo`

**认证**: 需要 JWT

**请求头**:
```
Authorization: Bearer <access_token>
```

**查询参数**:

| 参数名 | 类型 | 必填 | 默认值 | 说明 |
|--------|------|--------|--------|------|
| limit | number | 否 | 6 | 返回记录数量限制 |

**请求示例**:

```bash
# 返回最近 10 条记录
curl -X GET "http://localhost:6123/guardian-auth/v1/systeminfo?limit=10" \
  -H "Authorization: Bearer <access_token>"
```

**响应示例**:

```json
{
  "code": 200,
  "msg": null,
  "data": [
    {
      "id": "0190a1e8-7b3e-7a3f-8c1a-9e2f3a4b5c6d",
      "cpu_count": 8,
      "cpu_total_load": 45.50,
      "memory_used": 8589934592,
      "memory_total": 17179869184,
      "disk_used": 549755813888,
      "disk_total": 1099511627776,
      "network_upload": 104857600,
      "network_download": 524288000,
      "created_at": "2024-01-15T17:30:00Z"
    },
    {
      "id": "0190b2f9-8c4f-8b4g-9d2b-0f3g4b5c6d7e",
      "cpu_count": 8,
      "cpu_total_load": 42.30,
      "memory_used": 8388608000,
      "memory_total": 17179869184,
      "disk_used": 549755813888,
      "disk_total": 1099511627776,
      "network_upload": 104857600,
      "network_download": 524288000,
      "created_at": "2024-01-15T17:25:00Z"
    }
  ]
}
```

**响应字段说明**:

| 字段名 | 类型 | 说明 |
|--------|------|------|
| id | UUID | 系统信息记录 ID |
| cpu_count | integer | CPU 核心数 |
| cpu_total_load | decimal | CPU 总负载率（0-100） |
| memory_used | integer | 已使用内存（字节） |
| memory_total | integer | 总内存（字节） |
| disk_used | integer | 已使用磁盘空间（字节） |
| disk_total | integer | 总磁盘空间（字节） |
| network_upload | integer | 网络上传流量（字节） |
| network_download | integer | 网络下载流量（字节） |
| created_at | datetime | 记录创建时间（ISO 8601 格式） |

**业务规则**:
- 数据按创建时间倒序排列（最新的在最前）
- 默认返回最近 6 条记录
- 系统信息由后台定时任务自动采集和存储

---

## 错误码

| 状态码 | 说明 |
|--------|------|
| 200 | 操作成功 |
| 1000 | 通用业务错误 |
| 1001 | 通用系统错误 |
| 17000 | 系统内部错误 |
| 17001 | 参数验证失败 |
| 17002 | 认证失败 |
| 17003 | 令牌过期或无效 |
| 17004 | 权限不足 |
| 17005 | 资源不存在 |
| 17006 | 请求频率过高 |
| 17007 | 2FA 验证失败 |
| 17008 | 无效的2FA验证码 |
| 17009 | 未启用2FA |
| 17010 | 已启用2FA |

---

## 安全说明

### 密码安全
- 密码使用 Argon2 哈希存储（业界最安全的密码哈希算法）
- 密码强度建议：至少 8 位，包含字母和数字
- 首次登录后建议修改默认密码

### 令牌安全
- Access Token 有效期短（15 分钟）
- Refresh Token 有效期长（7 天）
- 令牌泄露后可以通过登出接口使其失效
- 建议使用 HTTPS 传输令牌

### 账户安全
- 5 次登录失败后锁定账户 15 分钟
- 支持禁用和启用账户
- 记录登录失败次数
- 记录最后登录时间

### 速率限制
- 登录接口有频率限制
- 建议合理的重试间隔
- 过于频繁的请求会被拒绝（17006 错误）

---

## 示例代码

### 使用 cURL

#### 登录获取令牌

```bash
curl -X POST http://localhost:6123/guardian-auth/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "123456"
  }'
```

#### 使用令牌访问受保护接口

```bash
curl -X POST http://localhost:6123/guardian-auth/v1/auth/logout \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -d '{
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  }'
```

#### 查询管理员列表

```bash
curl -X GET http://localhost:6123/guardian-auth/v1/admins?page=1&page_size=20 \
  -H "Authorization: Bearer <your_access_token>"
```

#### 查询系统信息

```bash
curl -X GET http://localhost:6123/guardian-auth/v1/systeminfo?limit=10 \
  -H "Authorization: Bearer <your_access_token>"
```

---

## 附录

### A. 默认账号

系统初始化后的默认管理员账号：

```
用户名: guardian
密码: 123456
权限: 超级管理员（所有权限）
```

**⚠️ 重要提示**: 首次登录后请立即修改默认密码！

---

## 更新日志

### v1.2.0 (2026-01-19)
- ✅ 新增管理员-角色绑定接口（`POST /admins/:id/roles`）
- ✅ 新增角色-权限绑定接口（`POST /roles/:id/permissions`）
- ✅ 完善管理员 CRUD 接口文档和示例
- ✅ 完善角色 CRUD 接口文档和示例
- ✅ 完善权限 CRUD 接口文档和示例
- ✅ 新增权限详情查询接口（`GET /permissions/:id`）
- ✅ 新增 2FA 禁用接口（`POST /auth/2fa/disable`）
- ✅ 统一文档格式，移除冗余内容
- ✅ 补充所有接口的完整请求/响应示例
- ✅ 添加详细的字段说明

### v1.1.1 (2026-01-15)
- ✅ 新增重置密码接口（`/auth/reset-password`）- 通过 2FA 重置密码
- ✅ 新增修改密码接口（`/auth/change-password`）- 登录后直接修改密码
- ✅ 重置密码接口为公开接口（无需 JWT 认证）
- ✅ 修改密码接口为受保护接口（需要 JWT 认证）
- ✅ 两个接口都使用 Argon2 密码哈希加密

### v1.1.0 (2026-01-15)
- ✅ 新增系统信息接口（`/systeminfo`）
- ✅ 新增 guardian_systeminfo 数据表
- ✅ 支持查询系统 CPU、内存、磁盘、网络监控数据
- ✅ 系统信息按时间倒序排列
- ✅ 支持自定义返回记录数量（limit 参数）

### v1.0.0 (2026-01-14)
- ✅ 实现基础认证功能（登录、登出、刷新令牌）
- ✅ 实现 2FA 接口（setup、verify）
- ✅ 实现 Argon2 密码加密
- ✅ 实现 UUIDv7 主键（全局唯一且有序）
- ✅ 实现 JWT 令牌管理
- ✅ 实现账户锁定机制
- ✅ 实现令牌黑名单机制
- ✅ 数据库自动维护 created_at 和 updated_at 字段
- ✅ 所有表名添加 guardian_ 前缀
- ✅ 添加完整的认证接口文档（refresh、logout、2fa）
- ✅ 修正所有 ID 类型为 UUID
- ✅ 新增错误码 17008、17009、17010

---
