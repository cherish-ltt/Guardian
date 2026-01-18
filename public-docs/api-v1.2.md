# 
# Guardian Auth API v1.2

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

---

## 认证方式

对于需要认证的接口，使用 `Authorization` 请求头：

```
Authorization: Bearer <access_token>
```

**令牌说明**：
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
  "timestamp": 1700000000  // 时间戳（部分接口包含）
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

## 管理员接口

> ⚠️ **注意**：以下接口已定义 DTO 但尚未在 router.rs 中实现路由

### 创建管理员

**接口描述**: 创建新的管理员账号

**请求方式**: `POST`

**请求路径**: `/admins`

**认证**: 需要 JWT

**请求头**: ```
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
    "username": "new_admin",
    "password": "SecurePass123",
    "is_super_admin": false,
    "role_ids": ["role-uuid-1", "role-uuid-2"]
  }'
```

**响应示例**:
```json
{
  "code": 200,
  "msg": "创建成功",
  "data": {
    "id": "admin-uuid",
    "username": "new_admin",
    "is_super_admin": false,
    "status": 1,
    "created_at": "2024-01-01T12:00:00:00Z"
  }
}
```

---

### 查询管理员列表

**接口描述**: 分页查询管理员列表

**请求方式**: `GET`

**请求路径**: `/admins`

**认证**: 需要 JWT

**请求头**: ```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**查询参数**:

| 参数名 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| page | number | 否 | 页码（从 1 开始） |
| page_size | number | 否 | 每页数量（默认 20） |
| status | number | 否 | 状态筛选（1=正常，0=禁用） |
| keyword | string | 否 | 用户名关键字搜索 |

**请求示例**:
```bash
# 获取第 1 页，每页 20 条
curl -X GET "http://localhost:6123/guardian-auth/v1/admins?page=1&page_size=20" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" | jq .

# 使用 keyword 搜索
curl -X GET "http://localhost:6123/guardian-auth/v1/admins?keyword=admin&page=1&page_size=20 \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" | jq .

# 同时筛选状态
curl -X GET "http://localhost:6123/guardian-auth/v1/admins?status=1&page=1&page_size=20" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" | jq .
```

**响应示例**:
```json
{
  "code": 200,
  "data": {
    "total": 100,
    "page": 1,
    "page_size": 20,
    "list": [
      {
        "id": "admin-uuid-1",
        "username": "admin",
        "is_super_admin": true,
        "status": 1,
        "last_login_at": "2024-01-01T10:30:00Z",
        "created_at": "2024-01-01T08:00:00Z"
      },
      {
        "id": "admin-uuid-2",
        "username": "test_user",
        "is_super_admin": false,
        "status": 1,
        "last_login_at": "2024-01-01T11:20:15:00Z",
        "created_at": "2024-01-01T09:00:00Z"
      }
    ]
  }
}
```

---

### 获取管理员详情

**接口描述**: 获取指定管理员的详细信息

**请求方式**: `GET`

**请求路径**: `/admins/:id`

**认证**: 需要 JWT

**请求头**: ```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**路径参数**:
- `id`: 管理员 ID（UUID）

**请求示例**:
```bash
curl -X GET http://localhost:6123/guardian-auth/v1/admins/<admin-uuid> \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" | jq .
```

**响应示例**:
```json
{
  "code": 200,
  "msg": null,
  "data": {
    "id": "admin-uuid",
    "username": "admin",
    "is_super_admin": true,
    "status": 1,
    "login_attempts": 0,
    "locked_until": null,
    "last_login_at": "2024-01-01T10:30:00Z",
    "created_at": "2023-12-01T08:00:00Z",
    "updated_at": "2024-01-01T09:15:00Z"
  }
}
```

---

### 更新管理员

**接口描述**: 更新管理员信息

**请求方式**: `PUT`

**请求路径**: `/admins/:id`

**认证**: 需要 JWT

**请求头**: ```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**路径参数**:
- `id`: 管理员 ID（UUID）

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| password | string | 否 | 新密码（明文，将进行 Argon2 哈希） |
| status | number | 否 | 状态（1=正常，0=禁用） |
| role_ids | array | 否 | 关联的角色 ID 数组（全量替换） |

**请求示例**:
```bash
# 更新管理员密码和状态
curl -X PUT http://localhost:6123/guardian-auth/v1/admins/<admin-uuid> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "password": "NewSecurePass456",
    "status": 1
  }'
```

# 更新管理员角色关联
curl -X PUT http://localhost:6123/guardian-auth/v1/admins/<admin-uuid> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "role_ids": ["role-uuid-1", "role-uuid-2"]
  }'
```

**响应示例**:
```json
{
  "code": 200,
  "msg": "更新成功",
  "data": {
    "id": "admin-uuid",
    "username": "admin",
    "is_super_admin": false,
    "status": 1,
    "updated_at": "2024-01-01T15:00:00Z"
  }
}
```

---

### 删除管理员

**接口描述**: 删除指定的管理员账号

**请求方式**: `DELETE`

**请求路径**: `/admins/:id`

**认证**: 需要 JWT

**请求头**: ```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**路径参数**:
- `id`: 管理员 ID（UUID）

**业务规则**：
- 超级管理员不能被删除
- 不能删除有活跃会话的账号
- 不能删除自己

**请求示例**:
```bash
curl -X DELETE http://localhost:6123/guardian-auth/v1/admins/<admin-uuid> \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" | jq .
```

**响应示例**:
```json
{
  "code": 200,
  "msg": "删除成功",
  "data": null
}
```

---

### 为管理员分配角色（**v1.2 新增**）

**接口描述**: 为指定管理员分配/替换角色

**请求方式**: `POST`

**请求路径**: `/admins/:id/roles`

**认证**: 需要 JWT

**请求头**: ```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**路径参数**:
- `id`: 管理员 ID（UUID）

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| role_ids | array | 是 | 要分配的角色 ID 数组（全量替换） |

**请求示例**:
```bash
curl -X POST http://localhost:6123/guardian-auth/v1/admins/<admin-uuid>/roles \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "role_ids": ["role-uuid-1", "role-uuid-2", "role-uuid-3"]
  }' | jq .

# 清空所有角色
curl -X POST http://localhost:6123/guardian-auth/v1/admins/<admin-uuid>/roles \
  -H "Content-Type: application/json" \
  -H "H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "role_ids": []
  }' | jq .
```

**响应示例**:
```json
{
  "code": 200,
  "msg": "角色分配成功"
}
```

**业务规则**:
- 超级管理员不会被分配角色
- 如果管理员已经是超级管理员，返回错误（17004）
- 新管理员会自动创建默认角色

---

## 角色接口

> ⚠️ **注意**：以下接口已定义 DTO 但尚未在 router.rs 中实现路由

### 创建角色

**接口描述**: 创建新的角色

**请求方式**: `POST`

**请求路径**: `/roles`

**认证**: 需要 JWT

**请求头**: ```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| code | string | 是 | 角色代码（唯一） |
| name | string | 是 | 角色名称 |
| description | string | 否 | 角色描述 |
| permission_ids | array | 否 | 关联的权限 ID 数组 |

**请求示例**:
```bash
# 创建角色并分配权限
curl -X POST http://localhost:6123/guardian-auth/v1/roles \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "code": "EDITOR_ROLE",
    "name": "编辑器角色",
    "description": "可以编辑内容权限",
    "permission_ids": ["perm-uuid-1", "perm-uuid-2"]
  }' | jq .

# 创建角色（不分配权限）
curl -X POST http://localhost:6123/guardian-auth/v1/roles \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "code": "VIEWER_ROLE",
    "name": "查看器角色",
    "description": "只能查看权限"
  }' | jq .
```

**响应示例**:
```json
{
  "code": 200,
  "msg": "创建成功",
  "data": {
    "id": "role-uuid",
    "code": "EDITOR_ROLE",
    "name": "编辑器角色",
    "description": "可以编辑内容权限",
    "is_system": false,
    "created_at": "2024-01-01T12:00:00Z"
  }
}
```

**权限类型说明**：
- `editor` - 编辑器角色：可以编辑内容和权限（但不能删除）
- `viewer` - 查看器角色：只能查看权限（不能编辑）

**业务规则**:
- 系统内置角色（`is_system = true`）不可删除
- 角色被管理员使用中不能删除
- 有管理员关联的角色不能删除

---

### 查询角色列表

**接口描述**: 分页查询角色列表

**请求方式**: `GET`

**请求路径**: `/roles`

**认证**: 需要 JWT

**请求头**: ```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**查询参数**:

| 参数名 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| page | number | 否 | 页码（从 1 开始） |
| page_size | number | 否 | 每页数量（默认 20） |
| keyword | string | 否 | 角色名或代码关键字搜索 |

**请求示例**:
```bash
# 获取第 1 页，每页 20 条
curl -X GET http://localhost:6123/guardian-auth/v1/roles?page=1&page_size=20 \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" | jq .

# 使用 keyword 搜索
curl -X GET "http://localhost:6123/guardian-auth/v1/roles?keyword=editor&page=1&page_size=20 \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" | jq .
```

**响应示例**:
```json
{
  "code": 200,
  "data": {
    "total": 50,
    "page": 1,
    "page_size": 20,
    "list": [
      {
        "id": "role-uuid-1",
        "code": "EDITOR_ROLE",
        "name": "编辑器角色",
        "description": "可以编辑内容和权限",
        "is_system": false,
        "created_at": "2024-01-01T12:00:00Z"
      },
      {
        "id": "role-uuid-2",
        "code": "VIEWER_ROLE",
        "name": "查看器角色",
        "description": "只能查看权限",
        "is_system": false,
        "created_at": "2024-01-01T09:15:00Z"
      }
    ]
  }
}
```

---

### 获取角色详情

**接口描述**: 获取角色的详细信息，包括关联的权限列表

**请求方式**: `GET`

**请求路径**: `/roles/:id`

**认证**: 需要 JWT

**请求头**: ```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**路径参数**:
- `id`: 角色 ID（UUID）

**请求示例**:
```bash
curl -X GET http://localhost:6123/guardian-auth/v1/roles/<role-uuid> \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" | jq .
```

**响应示例**:
```json
{
  "code": 200,
  "msg": null,
  "data": {
    "id": "role-uuid",
    "code": "EDITOR_ROLE",
    "name": "编辑器角色",
    "description": "可以编辑内容和权限",
    "is_system": false,
    "permissions": [
      {
        "id": "perm-uuid-1",
        "code": "ADMIN_READ",
        "name": "管理员列表",
        "resource_type": "api",
        "http_method": "GET",
        "resource_path": "/guardian-auth/v1/admins",
        "sort_order": 1
      },
      {
        "id": "perm-uuid-2",
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

---

### 更新角色

**接口描述**: 更新角色信息

**请求方式**: `PUT`

**请求路径**: `/roles/:id`

**认证**: 需要 JWT

**请求头**: ```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**路径参数**:
- `id`: 角色 ID（UUID）

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| name | string | 否 | 角色名称 |
| description | string | 否 | 角色描述 |
| permission_ids | array | 否 | 关联的权限 ID 数组（全量替换） |

**请求示例**:
```bash
# 更新角色名称
curl -X PUT http://localhost:6123/guardian-auth/v1/roles/<role-uuid> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "name": "编辑器角色（已更新）"
  }' | jq .

# 更新角色描述
curl -X PUT http://localhost:6123/guardian-v1/roles/<role-uuid> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "description": "更新了角色描述"
  }' | jq .

# 更新角色权限（替换）
curl -X PUT http://localhost:6123/guardian-auth/v1/roles/<role-uuid> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "permission_ids": ["perm-uuid-1", "perm-uuid-2", "perm-uuid-3"]
  }' | jq .

# 转换为查看器角色
curl -X PUT http://localhost:6123/guardian-v1/roles/<role-uuid> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "code": "VIEWER_ROLE",
    "name": "查看器角色"
  }' | jq .
```

**响应示例**:
```json
{
  "code": 200,
  "msg": "更新成功",
  "data": {
    "id": "role-uuid",
    "code": "VIEWER_ROLE",
    "name": "查看器角色",
    "is_system": false,
    "created_at": "2024-01-01T12:00:00Z"
  }
}
```

**业务规则**:
- 编辑器角色不能直接转换为系统内置角色
- 系统内置角色不能被修改为查看器角色

---

### 删除角色

**接口描述**: 删除指定的角色

**请求方式**: `DELETE`

**请求路径**: `/roles/:id`

**认证**: 需要 JWT

**请求头**: ```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**路径参数**:
- `id`: 角色 ID（UUID）

**业务规则**：
- 不能删除系统内置角色（`is_system = true`）
- 有管理员关联的角色不能删除
- 删除角色前需要解除所有管理员关联

**请求示例**:
```bash
curl -X DELETE http://localhost:6123/guardian-auth/v1/roles/<role-uuid> \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" | jq .
```

**响应示例**:
```json
{
  "code": 200,
  "msg": "删除成功",
  "data": null
}
```

---

### 为角色分配权限（**v1.2 新增**）

**接口描述**: 为角色分配/替换权限

**请求方式**: `POST`

**请求路径**: `/roles/:id/permissions`

**认证**: 需要 JWT

**请求头**: ```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**路径参数**:
- `id`: 角色 ID（UUID）

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|--------|------|
| permission_ids | array | 是 | 要分配的权限 ID 数组（全量替换） |

**请求示例**:
```bash
# 为角色分配 3 个权限
curl -X POST http://localhost:6123/guardian-auth/v1/roles/<role-uuid>/permissions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "permission_ids": ["perm-uuid-1", "perm-uuid-2", "perm-uuid-3"]
  }' | jq .

# 清空所有权限
curl -X POST http://localhost:6123/guardian-auth/v1/roles/<role-uuid>/permissions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "permission_ids": []
  }' | jq .
```

**响应示例**:
```json
{
  "code": 200,
  "msg": "权限分配成功"
}
```

**业务规则**:
- 编辑器角色只能关联 view 权限
- 系统内置角色不能被修改权限
- 权限被角色使用中不能删除

---

## 权限接口

> ⚠️ **注意**：以下接口已定义 DTO 但尚未在 router.rs 中实现路由

### 获取权限树

**接口描述**: 获取权限的树形结构

**请求方式**: `GET`

**请求路径**: `/permissions/tree`

**认证**: 需要 JWT

**请求头**: ```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**响应示例**:
```json
{
  "code": 200,
  "msg": null,
  "data": [
    {
      "id": "perm-uuid-1",
      "code": "USER_MANAGE",
      "name": "用户管理",
      "resource_type": "menu",
      "sort_order": 1,
      "children": [
        {
          "id": "perm-uuid-2",
          "code": "USER_CREATE",
          "name": "创建用户",
          "resource_type": "api",
          "http_method": "POST",
          "resource_path": "/guardian-auth/v1/admins",
          "sort_order": 2
        },
        {
          "id": "perm-uuid-3",
          "code": "USER_DELETE",
          "name": "删除用户",
          "resource_type": "api",
          "http_method": "DELETE",
          "resource_path": "/guardian-auth/v1/admins/:id",
          "sort_order": 3
        }
      ]
    }
  ]
}
```

---

### 获取权限列表

**接口描述**: 分页查询权限列表

**请求方式**: `GET`

**请求路径**: `/permissions`

**认证**: 需要 JWT

**请求头**: ```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**查询参数**:

| 参数名 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| page | number | 否 | 页码（从 1 开始） |
| page_size | number | 否 | 每页数量（默认 20） |
| resource_type | string | 否 | 资源类型筛选（api/menu/button） |
| keyword | string | 否 | 权限名或代码关键字搜索 |

**请求示例**:
```bash
# 获取 API 类型权限
curl -X GET "http://localhost:6123/guardian-auth/v1/permissions?resource_type=api&page=1&page_size=20 \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" | jq .

# 获取 Menu 类型权限
curl -X GET "http://localhost:6123/guardian-auth/v1/permissions?resource_type=menu&page=1&page_size=20 \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" | jq .

# 使用 keyword 搜索
curl -X GET "http://localhost:6123/guardian-auth/v1/permissions?keyword=user&page=1&page_size=20 \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" | jq .
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
        "id": "perm-uuid-1",
        "code": "ADMIN_LIST",
        "name": "管理员列表",
        "resource_type": "api",
        "http_method": "GET",
        "resource_path": "/guardian-auth/v1/admins",
        "sort_order": 1
      },
      {
        "id": "perm-uuid-2",
        "code": "ADMIN_READ",
        "name": "管理员详情",
        "resource_type": "api",
        "http_method": "GET",
        "resource_path": "/guardian-auth/v1/admins/:id",
        "sort_order": 2
      }
    ]
  }
}
```

---

### 创建权限

**接口描述**: 创建新的权限

**请求方式**: `POST`

**请求路径**: `/permissions`

**认证**: 需要 JWT

**请求头**: ```
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
| parent_id | string(UUID) | 否 | 父权限 ID（用于树形结构） |
| sort_order | number | 否 | 排序字段 |

**请求示例**:
```bash
# 创建管理员列表 API 权限
curl -X POST http://localhost:6123/guardian-auth/v1/permissions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "code": "ADMIN_READ",
    "name": "管理员列表",
    "resource_type": "api",
    "http_method": "GET",
    "resource_path": "/guardian-auth/v1/admins"
  }' | jq .

# 创建菜单权限
curl -X POST http://localhost:6123/guardian-auth/v1/permissions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "code": "MENU_VIEW",
    "name": "菜单查看",
    "resource_type": "menu",
    "resource_path": "/guardian-auth/v1/menu/view"
    "sort_order": 1
  }' | jq .

# 创建按钮权限
curl -X POST http://localhost:6123/guardian-auth/v1/permissions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  - -d '{
    "code": "BUTTON_CLICK",
    "name": "按钮点击",
    "resource_type": "button",
    "resource_path": "/guardian-auth/v1/button/click",
    "http_method": "POST",
    "sort_order": 1
  }' | jq .
```

**权限类型说明**：
- `menu` - 菜单权限：用于前端菜单显示
- `button` - 按钮权限：用于前端按钮显示
- `api` - API 权限：用于后端 API 访问控制

**资源路径示例**:
- `/guardian-auth/v1/admins` - 管理员列表访问
- `/guardian-auth/v1/admins/:id` - 管理员详情访问
- `/guardian-auth/v1/admins` - 创建管理员
- `/guardian-auth/v1/admins/:id` - 更新管理员
- `/guardian-auth/v1/admins/:id` - 删除管理员

**响应示例**:
```json
{
  "code": 200,
  "msg": null,
  "data": {
    "id": "perm-uuid",
    "code": "ADMIN_READ",
    "name": "管理员列表",
    "resource_type": "api",
    "http_method": "GET",
    "resource_path": "/guardian-auth/v1/admins"
  }
}
```

---

### 更新权限

**接口描述**: 更新权限信息

**请求方式**: `PUT`

**请求路径**: `/permissions/:id`

**认证**: 需要 JWT

**请求头**: ```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**路径参数**:
- `id`: 权限 ID（UUID）

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
curl -X PUT http://localhost:6123/guardian-auth/v1/permissions/<perm-uuid> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "name": "管理员列表（已更新）",
    "description": "更新了描述"
  }' | jq .

# 更新权限类型和路径
curl -X PUT http://localhost:6123/guardian-auth/v1/permissions/<perm-uuid> \
  -H "Content-Type: application/json" \
  - -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "resource_type": "button",
    "resource_path": "/guardian-auth/v1/button/click",
    "sort_order": 1
  }' | jq .
```

# 更新权限路径（支持通配符）
curl -X PUT http://localhost:6123/guardian-auth/v1/permissions/<perm-uuid> \
  -H "Content-Type: application/json" \
  -H "Authorization": Bearer $GUARDIAN_TOKEN" \
  -d '{
    "resource_path": "/guardian-auth/v1/admins/{id}",
    "sort_order": 1
  }' | jq .
```

# 转换父权限
curl -X PUT http://localhost:6123/guardian-auth/v1/permissions/<perm-uuid> \
  -H "Content-Type: application/json" \
  -d '{
    "code": "VIEWER_ROLE",
    "name": "查看器角色"
  }' | jq .
```

**响应示例**:
```json
{
  "code": 200,
  "msg": "更新成功",
  "data": {
    "id": "perm-uuid",
    "code": "VIEWER_ROLE",
    "name": "查看器角色",
    "is_system": false,
    "created_at": "2024-01-01T12:00:00Z"
  }
}
```

---

### 删除权限

**接口描述**: 删除指定的权限

**请求方式**: `DELETE`

**请求路径**: `/permissions/:id`

**认证**: 需要 JWT

**请求头**: ```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**路径参数**:
- `id`: 权限 ID（UUID）

**业务规则**:
- 不能删除系统内置权限（`is_system = true`）
- 有角色/管理员关联的权限不能删除
- 删除权限前需要解除所有角色关联

**请求示例**:
```bash
curl -X DELETE http://localhost:6123/guardian-auth/v1/permissions/<perm-uuid> \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" | jq .
```

**响应示例**:
```json
{
  "code": 200,
  "msg": "删除成功",
  "data": null
}
```

---

## 权限验证机制

### 权限中间件工作原理

权限验证中间件在所有受保护路由上的执行顺序：

```
用户请求
    ↓
auth_middleware (认证中间件)
    ↓
permission_middleware (权限中间件)
    ↓
业务处理函数
    ↓
返回响应
```

### 权限检查流程

1. **提取认证上下文**：从请求中获取用户 ID、用户名、是否超级管理员
2. **超级管理员判断**：如果 `is_super_admin = true`，直接通过
3. **权限查询**：
   - 查询用户关联的所有角色
   - 查询角色关联的所有权限
   - 检查是否有匹配的 API 权限
4. **路径匹配**：
   - 比较 HTTP 方法（GET/POST/PUT/DELETE）
   - 比较资源路径（支持通配符 `*` 和路径参数 `{id}`）
5. **结果判断**：
   - 有权限 → 放行请求
   - 无权限 → 返回 403 Forbidden

### 权限匹配规则

| 检查项 | 匹配规则 |
|--------|------|--------|------|
| resource_type | 必须 | 为 `"api"`（当前实现） |
| http_method | 必须 | 完全匹配（不区分大小写） |
| resource_path | 必须 | 完全匹配（支持通配符和路径参数） |

### 支持的通配符

| `*`：匹配任何路径
| `{id}`：匹配 `/guardian-auth/v1/admins/{id}`

### 超级管理员

- guardian 超级管理员（`is_super_admin = true`）→ 跳过所有权限检查
- 有任何角色和权限

### 业务规则

- 超级管理员不会被分配角色
- 新管理员自动创建默认角色
- 超级管理员拥有所有权限（但数据库中可能没有实际分配）

### 权限示例配置

```json
{
  "code": "ADMIN_LIST",
  "name": "管理员列表",
  "resource_type": "api",
  "http_method": "GET",
  "resource_path": "/guardian-auth/v1/admins"
}
```

---

## 错误码

### 业务错误（17000-17009）
| 业务错误（17000-17001）
| 认证错误（17002）...
| 令牌错误（17003）...

详细错误码请参考 v1.1 文档中的错误码表
```

---

## v1.2 版本变更日志

### 新增功能
- ✅ 管理员-角色绑定：`POST /admins/:id/roles`
- ✅ 权限验证中间件：API 类型权限检查
- ✅ 权限通配符和路径参数匹配
- ✅ 所有受保护路由已应用权限检查
- ✅ 超级管理员权限跳过验证功能正常

### 已修复问题
- ✅ 超级管理员权限检查逻辑正确工作
- ✅ 数据库数据已配置（test_admin 超级管理员）

---

## 使用说明

### 基础认证流程

1. **登录获取 Access Token**
```bash
curl -X POST http://localhost:6123/guardian-auth/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "guardian",
    "password": "123456"
  }' | jq .

# 保存 token
export GUARDIAN_TOKEN="<返回的access_token>"
```

### 2. 使用 Access Token 访问受保护接口
```bash
curl -X GET http://localhost:6123/guardian-auth/v1/admins \
  -H "Authorization: Bearer $GUARDIAN_TOKEN>" | jq .
```

### 3. 刷新 Access Token
```bash
curl -X POST http://localhost:6123/guardian-auth/v1/auth/refresh \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <refresh_token>" \
  -d '{
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  }' | jq .
```

### 4. 登出
```bash
curl -X POST http://localhost:6123/guardian-auth/v1/auth/logout \
  -H "Authorization: Bearer <access_token>" | jq .
```

---

### 权限测试

#### 测试超级管理员跳过权限检查

```bash
# 1. 登录 guardian 超级管理员
curl -X POST http://localhost:6123/guardian-auth/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"guardian","password":"123456"}' | jq .

# 保存 token
export GUARDIAN_TOKEN="<返回的access_token>"

# 2. 验证超级管理员可以访问受保护端点（应该成功）
curl -X GET http://localhost:6123/guardian-auth/v1/admins \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" | jq .
```

#### 测试普通管理员无权限被拒绝

```bash
# 1. 创建普通管理员并分配角色
curl -X POST http://localhost:6123/guardian-auth/v1/admins \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "username": "test_user",
    "password": "Test@123",
    "role_ids": ["role-uuid"]
  }' | jq .

# 2. 登录 test_admin（没有权限）
curl -X POST http://localhost:6123/guardian-auth/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"test_admin","password":"Test@123"}' | jq .

# 3. 测试访问（应该被拒绝 403）
curl -X GET http://localhost:6123/guardian-auth/v1/admins \
  -H "Authorization: Bearer <test_admin_token>" | jq .
```

---

## 测试数据准备

### 创建测试角色和权限

使用 Python 脚本自动准备测试数据

```bash
# 连接数据库
PGPASSWORD=123456 psql -h 127.0.0.1 -p 5432 -U postgres -d guardian_auth

# 1. 创建 API 权限
curl -X POST http://localhost:6123/guardian-auth/v1/permissions \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -H '{
    "code": "ADMIN_READ",
    "name": "管理员列表",
    "resource_type": "api",
    "http_method": "GET",
    "resource_path": "/guardian-auth/v1/admins"
  }' | jq .

# 保存权限 ID
export PERMISSION_ID="<返回的perm-uuid>"
```

# 2. 创建测试角色
curl -X POST http://localhost:6123/guardian-auth/v1/roles \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "code": "EDITOR_ROLE",
    "name": "测试角色",
    "description": "用于测试的角色",
    "permission_ids": ["PERMISSION_ID"]
  }' | jq .

# 保存角色 ID
export ROLE_ID="<返回的role-uuid>"
```

# 3. 为角色分配权限
curl -X POST http://localhost:6123/guardian-auth/v1/roles/$ROLE_ID/permissions \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -H '{
    "permission_ids": ["PERMISSION_ID"]
  }' | jq .
```

# 4. 创建测试管理员
curl -X POST http://localhost:6123/guardian-auth/v1/admins \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "username": "test_admin",
    "password": "Test@123",
    "role_ids": ["ROLE_ID"]
  }' | jq .

# 保存管理员 ID
export ADMIN_ID="<返回的admin-uuid>"
```

# 5. 为管理员分配角色
curl -X POST http://localhost:6123/guardian-auth/v1/admins/$ADMIN_ID/roles \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GUARDIAN_TOKEN" \
  -d '{
    "role_ids": ["ROLE_ID"]
  }' | jq .
```
```

---

## 注意事项

### 1. API 前缀
- v1.0: `/guardian-auth/v1`
- v1.2: `/guardian-auth/v1`（与 v1.0 共存）

### 2. JWT Token
- 从登录接口获取，15 分钟有效期
- Access Token 用于访问受保护接口
- Refresh Token 从刷新接口获取，7 天有效期

### 3. 超级管理员
- guardian 用户名：`guardian`
- is_super_admin：`true`
- JWT 令牌中包含 `is_super_admin: true`

### 4. 测试账户
- test_admin 用户名：`test_admin`
- is_super_admin：`true`（故意设置为测试权限检查）
- 密码：`Test@123`
- 无 2FA

### 5. 权限类型
- **api**（当前中间件验证）
  - **menu** - 菜单权限（数据库已支持，中间件暂不验证）
  - **button** - 按钮权限（数据库已支持，中间件暂不验证）

### 6. 密码安全
- 所有密码使用 Argon2 哈希
- 密码强度建议：至少 8 位，包含字母和数字

### 7. 测试环境
- 本地数据库：PostgreSQL 17
- 后端服务：Rust + Axum
- API 地址：`http://localhost:6123/guardian-auth/v1`
- 测试脚本：`scripts/test_full_permissions.py`

---

## 更新日志

### v1.2 (2026-01-19)
- ✅ 新增：管理员-角色绑定功能
- ✅ 新增：权限验证中间件
- ✅ 完善权限检查逻辑
- ✅ 测试脚本
- ✅ API 文档更新
- 📝 修复超级管理员权限检查 bug

### v1.2 (2026-01-19) - 发布
- ✅ Guardian Auth API v1.2 完整版
