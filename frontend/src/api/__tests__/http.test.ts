import test from 'node:test'
import assert from 'node:assert/strict'
import { createHttpClient, ApiError, setPasswordChangeHandler, setUnauthorizedHandler } from '../http.ts'

test('http client sends bearer token and unwraps unified response data', async () => {
  const calls: Array<{ url: string; init: RequestInit }> = []
  const fetcher = async (url: string | URL | Request, init?: RequestInit) => {
    calls.push({ url: String(url), init: init ?? {} })
    return new Response(JSON.stringify({ code: 200, message: 'success', data: { id: 1 }, traceId: 't1' }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    })
  }
  const client = createHttpClient({ baseUrl: 'http://api.test/api/v1', tokenProvider: () => 'token-1', fetcher })

  const data = await client.get<{ id: number }>('/system/users', { pageNum: 1, keyword: '陈立诚' })

  assert.deepEqual(data, { id: 1 })
  assert.equal(calls[0].url, 'http://api.test/api/v1/system/users?pageNum=1&keyword=%E9%99%88%E7%AB%8B%E8%AF%9A')
  assert.equal((calls[0].init.headers as Record<string, string>).Authorization, 'Bearer token-1')
})

test('http client throws ApiError for business errors', async () => {
  const fetcher = async () => new Response(JSON.stringify({ code: 403, message: '没有访问权限', data: null, traceId: 't2' }), {
    status: 403,
    headers: { 'content-type': 'application/json' },
  })
  const client = createHttpClient({ baseUrl: '/api/v1', tokenProvider: () => null, fetcher })

  await assert.rejects(
    () => client.post('/system/users', { username: 'blocked' }),
    (error: unknown) => error instanceof ApiError && error.code === 403 && error.message === '没有访问权限',
  )
})

test('a late unauthorized response from an old account does not clear the new session', async () => {
  let token = 'old-session'
  let resolveResponse!: (response: Response) => void
  let redirects = 0
  setUnauthorizedHandler(() => { redirects += 1 })
  const client = createHttpClient({
    tokenProvider: () => token,
    fetcher: () => new Promise<Response>(resolve => { resolveResponse = resolve }),
  })
  try {
    const pending = client.get('/auth/me')
    token = 'new-session'
    resolveResponse(new Response(JSON.stringify({ code: 401, message: '登录已过期' }), { status: 401 }))
    await assert.rejects(pending, ApiError)
    assert.equal(redirects, 0)
  } finally {
    setUnauthorizedHandler(null)
  }
})

test('an unauthorized download for the current session uses the same login-expiry handler', async () => {
  let redirects = 0
  setUnauthorizedHandler(() => { redirects += 1 })
  const client = createHttpClient({
    tokenProvider: () => 'current-session',
    fetcher: async () => new Response(JSON.stringify({ code: 401, message: '登录已过期' }), { status: 401 }),
  })
  try {
    await assert.rejects(client.download('/system/users/export'), ApiError)
    assert.equal(redirects, 1)
  } finally {
    setUnauthorizedHandler(null)
  }
})

test('non-JSON error page becomes ApiError with the real status instead of SyntaxError', async () => {
  const fetcher = async () => new Response('<html><body>502 Bad Gateway</body></html>', {
    status: 502,
    statusText: 'Bad Gateway',
    headers: { 'content-type': 'text/html' },
  })
  const client = createHttpClient({ baseUrl: '/api/v1', tokenProvider: () => 'token-1', fetcher })

  await assert.rejects(
    () => client.get('/system/users'),
    (error: unknown) => error instanceof ApiError && error.status === 502 && error.message === 'Bad Gateway',
  )
})

test('required-password 403 redirects requests and downloads without ending the session', async () => {
  let requirements = 0
  let expiries = 0
  setPasswordChangeHandler(() => { requirements += 1 })
  setUnauthorizedHandler(() => { expiries += 1 })
  const client = createHttpClient({
    tokenProvider: () => 'restricted-session',
    fetcher: async () => new Response(JSON.stringify({ code: 403, message: '请先修改密码', data: { passwordChangeRequired: true } }), { status: 403 }),
  })
  try {
    for (const operation of [() => client.get('/auth/routes'), () => client.download('/system/users/export')]) {
      await assert.rejects(operation, (error: unknown) => error instanceof ApiError && error.passwordChangeRequired && error.status === 403)
    }
    assert.equal(requirements, 2)
    assert.equal(expiries, 0)
  } finally {
    setPasswordChangeHandler(null)
    setUnauthorizedHandler(null)
  }
})

test('ordinary forbidden and late required-password responses do not restrict a new session', async () => {
  let requirements = 0
  let token = 'old-session'
  let complete!: (response: Response) => void
  setPasswordChangeHandler(() => { requirements += 1 })
  try {
    const ordinary = createHttpClient({
      tokenProvider: () => token,
      fetcher: async () => new Response(JSON.stringify({ code: 403, message: '没有访问权限' }), { status: 403 }),
    })
    await assert.rejects(ordinary.get('/system/users'), (error: unknown) => error instanceof ApiError && !error.passwordChangeRequired)
    for (const download of [false, true]) {
      token = 'old-session'
      const client = createHttpClient({ tokenProvider: () => token, fetcher: () => new Promise<Response>(resolve => { complete = resolve }) })
      const pending = download ? client.download('/system/users/export') : client.get('/auth/routes')
      token = 'new-session'
      complete(new Response(JSON.stringify({ code: 403, message: '请先修改密码', data: { passwordChangeRequired: true } }), { status: 403 }))
      await assert.rejects(pending, ApiError)
    }
    assert.equal(requirements, 0)
  } finally {
    setPasswordChangeHandler(null)
  }
})
