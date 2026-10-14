import { reactive, ref, shallowRef, type Ref } from 'vue'
import type { PageResult, QueryParams } from '@/api'
import { downloadBlob } from '@/utils/download'
import { pushToast, toastError } from './toast'

/* ============================================================
   列表页通用逻辑：查询、重置、服务端分页、导出、勾选。
   页面只声明「怎么取数据」「怎么导出」和查询字段的初始值。
   ============================================================ */

export interface ListPageOptions<Row, Query extends Record<string, unknown>> {
  /** 查询字段初始值，重置时回到这里 */
  query: Query
  fetch: (params: QueryParams) => Promise<PageResult<Row>>
  /** 导出：返回后端生成的文件；不提供则不显示导出 */
  exporter?: (params: QueryParams) => Promise<Blob>
  exportName?: string
  pageSize?: number
  /** 查询参数加工：空字符串转 undefined 已内置 */
  toParams?: (query: Query) => QueryParams
  immediate?: boolean
}

export function useListPage<Row extends Record<string, any>, Query extends Record<string, unknown>>(options: ListPageOptions<Row, Query>) {
  const initial = { ...options.query }
  const query = reactive({ ...options.query }) as Query
  const rows = shallowRef<Row[]>([])
  const total = ref(0)
  const page = ref(1)
  const pageSize = ref(options.pageSize ?? 10)
  const loading = ref(false)
  const exporting = ref(false)
  const selected = ref<(string | number)[]>([])
  let requestId = 0

  function params(): QueryParams {
    const source = options.toParams ? options.toParams(query) : (query as QueryParams)
    const result: QueryParams = {}
    Object.entries(source).forEach(([key, value]) => {
      if (value !== '' && value !== null && value !== undefined) result[key] = value as QueryParams[string]
    })
    return result
  }

  async function load() {
    const current = ++requestId
    loading.value = true
    try {
      const data = await options.fetch({ ...params(), pageNum: page.value, pageSize: pageSize.value })
      if (current !== requestId) return
      rows.value = data.rows
      total.value = data.total
      selected.value = []
      // 删除最后一页的最后一条后回退一页
      if (data.rows.length === 0 && page.value > 1 && data.total > 0) {
        page.value = Math.max(1, Math.ceil(data.total / pageSize.value))
        await load()
      }
    } catch (error) {
      if (current === requestId) toastError(error, '数据加载失败')
    } finally {
      if (current === requestId) loading.value = false
    }
  }

  function search() {
    page.value = 1
    return load()
  }

  function reset() {
    Object.assign(query, initial)
    return search()
  }

  function setPage(value: number) {
    page.value = value
    return load()
  }

  function setPageSize(value: number) {
    pageSize.value = value
    page.value = 1
    return load()
  }

  async function exportData() {
    if (!options.exporter) return
    exporting.value = true
    try {
      const blob = await options.exporter(params())
      downloadBlob(blob, options.exportName ?? 'export.csv')
      pushToast('已导出当前查询结果', 'ok')
    } catch (error) {
      toastError(error, '导出失败')
    } finally {
      exporting.value = false
    }
  }

  if (options.immediate !== false) load()

  return {
    query, rows, total, page, pageSize, loading, exporting, selected,
    load, search, reset, setPage, setPageSize, exportData,
    canExport: Boolean(options.exporter),
  }
}

/** ListPage 组件接收的列表状态（与具体行类型解耦） */
export interface ListPageState {
  query: Record<string, any>
  rows: Ref<any[]>
  total: Ref<number>
  page: Ref<number>
  pageSize: Ref<number>
  loading: Ref<boolean>
  exporting: Ref<boolean>
  selected: Ref<(string | number)[]>
  load: () => Promise<void>
  search: () => Promise<void>
  reset: () => Promise<void>
  setPage: (value: number) => Promise<void>
  setPageSize: (value: number) => Promise<void>
  exportData: () => Promise<void>
  canExport: boolean
}

/* ── 树形列表（菜单、部门）：不分页，其余能力与 useListPage 一致 ── */
export interface TreeListOptions<Row, Query extends Record<string, unknown>> {
  query: Query
  fetch: (params: QueryParams) => Promise<Row[]>
  exporter?: (params: QueryParams) => Promise<Blob>
  exportName?: string
}

export function useTreeListPage<Row extends Record<string, any>, Query extends Record<string, unknown>>(options: TreeListOptions<Row, Query>): ListPageState & { tree: true } {
  const state = useListPage<Row, Query>({
    query: options.query,
    exporter: options.exporter,
    exportName: options.exportName,
    pageSize: Number.MAX_SAFE_INTEGER,
    fetch: async params => {
      const { pageNum: _pageNum, pageSize: _pageSize, ...rest } = params
      const rows = await options.fetch(rest)
      const count = (nodes: Row[]): number => nodes.reduce((n, node) => n + 1 + count((node.children ?? []) as Row[]), 0)
      return { rows, total: count(rows), pageNum: 1, pageSize: rows.length }
    },
  })
  return { ...state, tree: true }
}
