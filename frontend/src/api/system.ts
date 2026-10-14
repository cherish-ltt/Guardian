import type { HttpClient, QueryParams } from './http'

export interface PageResult<T> {
  pageNum: number
  pageSize: number
  total: number
  rows: T[]
}

export interface IdResponse {
  id: number
}

export interface UserRow {
  id: number
  username: string
  nickname: string
  phone: string
  email: string
  deptName: string
  postName: string
  roleName: string
  status: number
  createTime: string
  lastLogin: string
}

export interface UserWriteRequest {
  username?: string
  password?: string
  nickname?: string
  phone?: string
  email?: string
  deptId?: number
  status?: number
  roleIds?: number[]
  postIds?: number[]
}

export interface UserOptions {
  depts: Record<string, unknown>[]
  roles: Array<{ id: number; name: string }>
  posts: Array<{ id: number; name: string }>
  initPassword: string
}

export interface RoleWriteRequest {
  name?: string
  code?: string
  dataScope?: string
  sort?: number
  status?: number
  remark?: string
  menuIds?: number[]
}

export interface MenuWriteRequest {
  parentId?: number
  name?: string
  type?: string
  icon?: string
  path?: string
  component?: string
  permission?: string
  sort?: number
  visible?: number
  status?: number
}

export interface DeptWriteRequest {
  parentId?: number
  name?: string
  leader?: string
  phone?: string
  email?: string
  sort?: number
  status?: number
}

export interface PostWriteRequest {
  postCode?: string
  postName?: string
  sort?: number
  status?: number
  remark?: string
}

export interface DictTypeWriteRequest {
  dictName?: string
  dictType?: string
  status?: number
  remark?: string
}

export interface DictDataWriteRequest {
  dictType?: string
  label?: string
  value?: string
  sort?: number
  status?: number
  tone?: string
  def?: boolean
  remark?: string
}

export interface ConfigWriteRequest {
  name?: string
  key?: string
  value?: string
  type?: 'Y' | 'N' | string
  builtin?: boolean
  remark?: string
}

/** 统计时间档：24h 按小时、7d / 30d 按天 */
export type StatsRange = '24h' | '7d' | '30d'

/** 趋势时间桶：bucket 桶标签（HH:00 或 MM-dd）、total 总次数、failed 失败次数 */
export interface StatsBucket {
  bucket: string
  total: number
  failed: number
}

/** 操作日志概览：successRate 成功率 %、avgCost 平均耗时 ms、p95Cost 95 分位耗时 ms、activeUsers 活跃账号数 */
export interface OperLogStats {
  range: StatsRange
  total: number
  failed: number
  successRate: number
  avgCost: number
  p95Cost: number
  activeUsers: number
  series: StatsBucket[]
}

/** 登录日志概览：users 登录账号数、ips 来源 IP 数 */
export interface LoginLogStats {
  range: StatsRange
  total: number
  failed: number
  successRate: number
  users: number
  ips: number
  series: StatsBucket[]
}

/** 泳道时间块：startSec / endSec 当日秒数，count 次数，failed 失败数，maxCost 最大耗时 ms，ids 日志 ID（最多 50 个） */
export interface TimelineBlock {
  startSec: number
  endSec: number
  count: number
  failed: number
  maxCost: number
  ids: number[]
}

/** 当日操作泳道：nowSec 当前秒数（非当天为 null），lanes 按模块分道 */
export interface OperTimeline {
  date: string
  nowSec: number | null
  lanes: Array<{ module: string; count: number; blocks: TimelineBlock[] }>
}

/** 登录热力：cells 星期（0 周一 … 6 周日）× 小时的登录 / 失败次数；anomalies 失败次数达到 threshold 的 IP（kind=ip，spread 尝试账号数）或账号（kind=user，spread 来源 IP 数） */
export interface LoginHeatmap {
  days: number
  since: string
  threshold: number
  cells: Array<{ dow: number; hour: number; total: number; failed: number }>
  anomalies: Array<{ kind: 'ip' | 'user'; key: string; failed: number; total: number; spread: number; firstTime: string; lastTime: string }>
}

/** 权限关系图：users 用户节点（无用户查询权限时为空），roles 角色（menuIds 授予的菜单），menus 菜单，userRoles [用户ID, 角色ID] 关联 */
export interface PermissionGraph {
  users: Array<{ id: number; username: string; nickname: string; deptName: string | null; status: number }>
  roles: Array<{ id: number; name: string; code: string; dataScope: string; status: number; builtin: boolean; userCount: number; menuIds: number[] }>
  menus: Array<{ id: number; parentId: number; name: string; type: 'dir' | 'menu' | 'btn'; icon: string | null; path: string | null; permission: string | null; sort: number; visible: number; status: number }>
  userRoles: Array<[number, number]>
  usersVisible: boolean
}

export function createSystemApi(client: HttpClient) {
  return {
    users: {
      ...crudPage<UserRow, UserWriteRequest>(client, '/system/users'),
      options: () => client.get<UserOptions>('/system/users/options'),
      export: (query?: QueryParams) => client.download('/system/users/export', query),
      updateStatus: (id: number, status: number) => client.put<void>(`/system/users/${id}/status`, { status }),
      resetPassword: (id: number, password: string) => client.put<void>(`/system/users/${id}/password`, { password }),
    },
    roles: {
      ...crudPage<Record<string, unknown>, RoleWriteRequest>(client, '/system/roles'),
      menus: (id: number) => client.get<{ menuIds: number[] }>(`/system/roles/${id}/menus`),
      export: (query?: QueryParams) => client.download('/system/roles/export', query),
    },
    permissions: {
      graph: () => client.get<PermissionGraph>('/system/permissions/graph'),
    },
    menus: {
      tree: (query?: QueryParams) => client.get<Record<string, unknown>[]>('/system/menus/tree', query),
      create: (body: MenuWriteRequest) => client.post<IdResponse>('/system/menus', body),
      update: (id: number, body: MenuWriteRequest) => client.put<void>(`/system/menus/${id}`, body),
      remove: (id: number) => client.delete<void>(`/system/menus/${id}`),
      export: (query?: QueryParams) => client.download('/system/menus/export', query),
    },
    depts: {
      tree: (query?: QueryParams) => client.get<Record<string, unknown>[]>('/system/depts/tree', query),
      create: (body: DeptWriteRequest) => client.post<IdResponse>('/system/depts', body),
      update: (id: number, body: DeptWriteRequest) => client.put<void>(`/system/depts/${id}`, body),
      remove: (id: number) => client.delete<void>(`/system/depts/${id}`),
      export: (query?: QueryParams) => client.download('/system/depts/export', query),
    },
    posts: {
      ...crudPage<Record<string, unknown>, PostWriteRequest>(client, '/system/posts'),
      export: (query?: QueryParams) => client.download('/system/posts/export', query),
    },
    dictTypes: {
      ...crudPage<Record<string, unknown>, DictTypeWriteRequest>(client, '/system/dict/types'),
      export: (query?: QueryParams) => client.download('/system/dict/types/export', query),
    },
    dictData: {
      list: (dictType: string) => client.get<Record<string, unknown>[]>(`/system/dict/data/${dictType}`),
      options: (dictType: string) => client.get<Record<string, unknown>[]>(`/system/dict/options/${dictType}`),
      create: (body: DictDataWriteRequest) => client.post<IdResponse>('/system/dict/data', body),
      update: (id: number, body: DictDataWriteRequest) => client.put<void>(`/system/dict/data/${id}`, body),
      remove: (id: number) => client.delete<void>(`/system/dict/data/${id}`),
    },
    configs: {
      ...crudPage<Record<string, unknown>, ConfigWriteRequest>(client, '/system/configs'),
      export: (query?: QueryParams) => client.download('/system/configs/export', query),
    },
    operLogs: {
      list: (query?: QueryParams) => client.get<PageResult<Record<string, unknown>>>('/monitor/oper-logs', query),
      stats: (range: StatsRange = '24h') => client.get<OperLogStats>('/monitor/oper-logs/stats', { range }),
      timeline: (date?: string) => client.get<OperTimeline>('/monitor/oper-logs/timeline', date ? { date } : undefined),
      export: (query?: QueryParams) => client.download('/monitor/oper-logs/export', query),
      remove: (id: number | string) => client.delete<void>(`/monitor/oper-logs/${id}`),
      clear: () => client.delete<void>('/monitor/oper-logs'),
    },
    loginLogs: {
      list: (query?: QueryParams) => client.get<PageResult<Record<string, unknown>>>('/monitor/login-logs', query),
      stats: (range: StatsRange = '24h') => client.get<LoginLogStats>('/monitor/login-logs/stats', { range }),
      heatmap: (days: 7 | 30 = 7) => client.get<LoginHeatmap>('/monitor/login-logs/heatmap', { days }),
      export: (query?: QueryParams) => client.download('/monitor/login-logs/export', query),
      remove: (id: number | string) => client.delete<void>(`/monitor/login-logs/${id}`),
      clear: () => client.delete<void>('/monitor/login-logs'),
    },
  }
}

function crudPage<Row, WriteBody>(client: HttpClient, path: string) {
  return {
    list: (query?: QueryParams) => client.get<PageResult<Row>>(path, query),
    create: (body: WriteBody) => client.post<IdResponse>(path, body),
    update: (id: number, body: Partial<WriteBody>) => client.put<void>(`${path}/${id}`, body),
    remove: (id: number) => client.delete<void>(`${path}/${id}`),
  }
}
