export interface ApiResponse<T> {
  code: number
  message: string
  data: T
  traceId?: string
}

export interface HttpClientOptions {
  baseUrl?: string
  tokenProvider?: () => string | null
  fetcher?: typeof fetch
}

export interface HttpClient {
  get<T>(path: string, query?: QueryParams): Promise<T>
  post<T>(path: string, body?: unknown): Promise<T>
  put<T>(path: string, body?: unknown): Promise<T>
  delete<T>(path: string): Promise<T>
  download(path: string, query?: QueryParams): Promise<Blob>
}

export type QueryParams = Record<string, string | number | boolean | null | undefined>

export class ApiError extends Error {
  code: number
  status: number
  traceId?: string

  passwordChangeRequired: boolean

  constructor(message: string, code: number, status: number, traceId?: string, passwordChangeRequired = false) {
    super(message)
    this.name = 'ApiError'
    this.code = code
    this.status = status
    this.traceId = traceId
    this.passwordChangeRequired = passwordChangeRequired
  }
}

let unauthorizedHandler: (() => void) | null = null
let passwordChangeHandler: (() => void) | null = null

/** 登录失效（401）时的统一处理，由路由层注册 */
export function setUnauthorizedHandler(handler: (() => void) | null) {
  unauthorizedHandler = handler
}

export function setPasswordChangeHandler(handler: (() => void) | null) {
  passwordChangeHandler = handler
}

export function createHttpClient(options: HttpClientOptions = {}): HttpClient {
  const baseUrl = normalizeBaseUrl(options.baseUrl ?? defaultBaseUrl())
  const tokenProvider = options.tokenProvider ?? authSession.token
  const fetcher = options.fetcher ?? fetch

  function responseError(payload: ApiResponse<unknown>, response: Response, token: string | null) {
    const required = response.status === 403 && payload.code === 403
      && typeof payload.data === 'object' && payload.data !== null
      && 'passwordChangeRequired' in payload.data && payload.data.passwordChangeRequired === true
    if (token && token === tokenProvider()) {
      if (payload.code === 401 || response.status === 401) unauthorizedHandler?.()
      else if (required) passwordChangeHandler?.()
    }
    return new ApiError(payload.message || '请求失败', payload.code || response.status, response.status, payload.traceId, required)
  }

  async function request<T>(method: string, path: string, body?: unknown, query?: QueryParams): Promise<T> {
    const headers: Record<string, string> = { Accept: 'application/json' }
    const token = tokenProvider()
    if (token) headers.Authorization = `Bearer ${token}`
    const init: RequestInit = { method, headers }
    if (body !== undefined) {
      headers['Content-Type'] = 'application/json'
      init.body = JSON.stringify(body)
    }

    const response = await fetcher(buildUrl(baseUrl, path, query), init)
    const payload = await parseJson<ApiResponse<T>>(response)
    if (!response.ok || payload.code !== 200) {
      throw responseError(payload, response, token)
    }
    return payload.data
  }

  async function download(path: string, query?: QueryParams): Promise<Blob> {
    const headers: Record<string, string> = { Accept: 'text/csv,application/octet-stream,*/*' }
    const token = tokenProvider()
    if (token) headers.Authorization = `Bearer ${token}`

    const response = await fetcher(buildUrl(baseUrl, path, query), { method: 'GET', headers })
    if (!response.ok) {
      const payload = await parseJson<ApiResponse<unknown>>(response)
      throw responseError(payload, response, token)
    }
    return response.blob()
  }

  return {
    get: (path, query) => request('GET', path, undefined, query),
    post: (path, body) => request('POST', path, body),
    put: (path, body) => request('PUT', path, body),
    delete: (path) => request('DELETE', path),
    download,
  }
}

function normalizeBaseUrl(value: string) {
  return value.endsWith('/') ? value.slice(0, -1) : value
}

function defaultBaseUrl() {
  return ((import.meta as ImportMeta & { env?: { VITE_API_BASE_URL?: string } }).env?.VITE_API_BASE_URL) || '/api/v1'
}

function buildUrl(baseUrl: string, path: string, query?: QueryParams) {
  const normalizedPath = path.startsWith('/') ? path : `/${path}`
  const queryString = query ? toQueryString(query) : ''
  return `${baseUrl}${normalizedPath}${queryString ? `?${queryString}` : ''}`
}

function toQueryString(query: QueryParams) {
  const params = new URLSearchParams()
  Object.entries(query).forEach(([key, value]) => {
    if (value !== undefined && value !== null && value !== '') {
      params.set(key, String(value))
    }
  })
  return params.toString()
}

/**
 * 解析统一响应体。
 *
 * 网关返回的 HTML 错误页、反向代理的纯文本提示都不是 JSON：
 * 直接 `JSON.parse` 会抛 `SyntaxError`，调用方只认 `ApiError`，
 * 用户最终看到的是通用兜底文案，真实状态码丢失。这里统一收敛成同形状对象。
 */
async function parseJson<T>(response: Response): Promise<T> {
  const text = await response.text()
  if (!text) {
    return { code: response.status, message: response.statusText, data: null } as T
  }
  try {
    return JSON.parse(text) as T
  } catch {
    return {
      code: response.status,
      message: response.statusText || `HTTP ${response.status}`,
      data: null,
    } as T
  }
}

import { authSession } from './session.ts'

export const httpClient = createHttpClient()
