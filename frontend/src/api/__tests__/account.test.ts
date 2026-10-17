import test from 'node:test'
import assert from 'node:assert/strict'
import { createAuthApi } from '../auth.ts'
import { createSystemApi } from '../system.ts'
import { createHttpClient } from '../http.ts'

test('profile and two factor operations use exact methods and proof bodies', async () => {
  const calls: Array<{ url: string; method: string; body: unknown }> = []
  const client = createHttpClient({ baseUrl: '/api/v1', tokenProvider: () => 'token', fetcher: async (url, init) => {
    calls.push({ url: String(url), method: init?.method ?? 'GET', body: init?.body ? JSON.parse(String(init.body)) : null })
    return new Response(JSON.stringify({ code: 200, message: '操作成功', data: {} }), { status: 200, headers: { 'content-type': 'application/json' } })
  } })
  const api = createAuthApi(client)
  const proof = { password: 'current-password', secondFactor: '123456' }
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
