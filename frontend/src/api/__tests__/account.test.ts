import test from 'node:test'
import assert from 'node:assert/strict'
import { randomUUID } from 'node:crypto'
import { createAuthApi } from '../auth.ts'
import { createSystemApi } from '../system.ts'
import { ApiError, createHttpClient, setUnauthorizedHandler } from '../http.ts'

test('profile and two factor operations use exact methods and proof bodies', async () => {
  const calls: Array<{ url: string; method: string; body: unknown }> = []
  const client = createHttpClient({ baseUrl: '/api/v1', tokenProvider: () => 'token', fetcher: async (url, init) => {
    calls.push({ url: String(url), method: init?.method ?? 'GET', body: init?.body ? JSON.parse(String(init.body)) : null })
    return new Response(JSON.stringify({ code: 200, message: '操作成功', data: {} }), { status: 200, headers: { 'content-type': 'application/json' } })
  } })
  const api = createAuthApi(client)
  const proof = { password: randomUUID(), secondFactor: '123456' }
  await api.verifyLogin({ challenge: 'opaque-challenge', secondFactor: '123456' })
  assert.deepEqual(calls.pop(), { url: '/api/v1/auth/login/2fa', method: 'POST', body: { challenge: 'opaque-challenge', secondFactor: '123456' } })
  await api.profile()
  await api.updateProfile({ nickname: '昵称', phone: null, email: null })
  await api.setupTwoFactor({ password: proof.password })
  await api.enableTwoFactor(proof)
  await api.disableTwoFactor(proof)
  await api.regenerateRecoveryCodes(proof)
  await createSystemApi(client).users.resetTwoFactor(7, proof.password)
  assert.deepEqual(calls.map(c => c.method + ' ' + c.url), [
    'GET /api/v1/auth/profile', 'PUT /api/v1/auth/profile', 'POST /api/v1/auth/2fa/setup',
    'POST /api/v1/auth/2fa/enable', 'POST /api/v1/auth/2fa/disable', 'POST /api/v1/auth/2fa/recovery-codes',
    'POST /api/v1/system/users/7/2fa/reset',
  ])
  assert.deepEqual(calls[1].body, { nickname: '昵称', phone: null, email: null })
  assert.deepEqual(calls[3].body, proof)
  assert.deepEqual(calls[6].body, { password: proof.password })
})

test('session listing preserves opaque cursors and revocation sends the security proof', async () => {
  const calls: Array<{ url: string; method: string; body: unknown }> = []
  const session = { id: 'other-session', current: false, issuedAt: 1_700_000_000, expiresAt: 1_700_003_600, browser: '', os: '' }
  const client = createHttpClient({ baseUrl: '/api/v1', tokenProvider: () => 'current-token', fetcher: async (url, init) => {
    calls.push({ url: String(url), method: init?.method ?? 'GET', body: init?.body ? JSON.parse(String(init.body)) : null })
    const data = init?.method === 'POST' ? { revoked: true } : { items: [session], nextCursor: '18446744073709551615' }
    return new Response(JSON.stringify({ code: 200, message: '操作成功', data }), { status: 200 })
  } })
  const api = createAuthApi(client)
  const first = await api.sessions()
  assert.deepEqual(first.items, [session])
  await api.sessions(first.nextCursor!)
  const proof = { password: randomUUID(), secondFactor: '123456' }
  assert.deepEqual(await api.revokeSession(session.id, proof), { revoked: true })
  assert.deepEqual(calls, [
    { url: '/api/v1/auth/sessions', method: 'GET', body: null },
    { url: '/api/v1/auth/sessions?cursor=18446744073709551615', method: 'GET', body: null },
    { url: '/api/v1/auth/sessions/other-session/revoke', method: 'POST', body: proof },
  ])
})

test('an already invalid session is distinct from a failed revocation and keeps the caller signed in', async () => {
  let status = 200
  let redirects = 0
  setUnauthorizedHandler(() => { redirects += 1 })
  const api = createAuthApi(createHttpClient({ tokenProvider: () => 'current-token', fetcher: async () => {
    return new Response(JSON.stringify({ code: status, message: status === 500 ? '会话撤销失败' : '操作成功', data: { revoked: false } }), { status })
  } }))
  try {
    assert.deepEqual(await api.revokeSession('expired-session', { password: randomUUID() }), { revoked: false })
    status = 500
    await assert.rejects(api.revokeSession('other-session', { password: randomUUID() }), (error: unknown) => error instanceof ApiError && error.message === '会话撤销失败')
    assert.equal(redirects, 0)
  } finally {
    setUnauthorizedHandler(null)
  }
})

test('a late revocation response cannot expire a subsequently established session', async () => {
  let token = 'old-session'
  let resolveResponse!: (response: Response) => void
  let redirects = 0
  setUnauthorizedHandler(() => { redirects += 1 })
  const api = createAuthApi(createHttpClient({ tokenProvider: () => token, fetcher: () => new Promise(resolve => { resolveResponse = resolve }) }))
  try {
    const pending = api.revokeSession('other-session', { password: randomUUID() })
    token = 'new-session'
    resolveResponse(new Response(JSON.stringify({ code: 401, message: '登录已过期' }), { status: 401 }))
    await assert.rejects(pending, ApiError)
    assert.equal(redirects, 0)
  } finally {
    setUnauthorizedHandler(null)
  }
})
