import type { HttpClient, QueryParams } from './http'
import type { PageResult } from './system'

export interface DashboardSummary {
  stats: Array<Record<string, unknown>>
  trend: number[]
  activities: Array<Record<string, unknown>>
  roleDist: Array<{ label: string; value: number }>
  trendTotal: number
  successRate: number
  /** 上一等长周期的登录次数，用于环比 */
  trendPrev: number
  /** 近 7 天按日操作量：date 日期、total 操作数、failed 失败数 */
  dailyOps: Array<{ date: string; total: number; failed: number }>
}

export interface GeneratorFile {
  name: string
  path: string
  code: string
}

export interface GeneratorPreview {
  files: GeneratorFile[]
  activeFile: string
  code: string
}

export interface GeneratorTableUpdate {
  tableComment?: string
  synced?: boolean
}

export interface GeneratorImportRequest {
  tableNames: string[]
}

/** 服务健康：status 运行状态（UP 正常）、time 服务端时间 */
export interface HealthStatus {
  status: string
  time: string
}

export function createPortalApi(client: HttpClient) {
  return {
    health: () => client.get<HealthStatus>('/health'),
    dashboard: {
      summary: (query?: QueryParams) => client.get<DashboardSummary>('/dashboard/summary', query),
    },
    generator: {
      tables: {
        list: (query?: QueryParams) => client.get<PageResult<Record<string, unknown>>>('/tool/generator/tables', query),
        importable: (query?: QueryParams) => client.get<Record<string, unknown>[]>('/tool/generator/importable-tables', query),
        import: (body: GeneratorImportRequest) => client.post<Record<string, unknown>>('/tool/generator/import', body),
        columns: (id: number) => client.get<Record<string, unknown>[]>(`/tool/generator/tables/${id}/columns`),
        preview: (id: number) => client.get<GeneratorPreview>(`/tool/generator/tables/${id}/preview`),
        update: (id: number, body: GeneratorTableUpdate) => client.put<void>(`/tool/generator/tables/${id}`, body),
        generate: (id: number) => client.post<Record<string, unknown>>(`/tool/generator/tables/${id}/generate`),
        download: (id: number) => client.download(`/tool/generator/tables/${id}/download`),
      },
    },
  }
}
