import test from 'node:test'
import assert from 'node:assert/strict'
import { createHttpClient } from '../http.ts'
import { createAuthApi } from '../auth.ts'

test('auth api maps login, current user, routes and logout endpoints', async () => {
  const calls: Array<{ url: string; method: string; body?: string }> = []
  const fetcher = async (url: string | URL | Request, init?: RequestInit) => {
    calls.push({ url: String(url), method: init?.method ?? 'GET', body: init?.body as string | undefined })
    const data = url.toString().endsWith('/login')
      ? { token: 'jwt-token', expiresIn: 7200 }
      : url.toString().endsWith('/me')
        ? { username: 'admin', nickname: '陈立诚', roles: ['super_admin'], permissions: ['system:user:list'] }
        : url.toString().endsWith('/routes')
          ? { routes: [{ name: '工作台', path: '/dashboard' }] }
          : null
    return new Response(JSON.stringify({ code: 200, message: 'success', data, traceId: 't' }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    })
  }
  const api = createAuthApi(createHttpClient({ baseUrl: '/api/v1', tokenProvider: () => 'token', fetcher }))

  const login = await api.login({ username: 'admin', password: 'admin123', captcha: '1234', captchaUuid: 'captcha-id' })
  const me = await api.me()
  const routes = await api.routes()
  await api.changePassword({ oldPassword: 'oldPwd123', newPassword: 'newPwd123' })
  await api.logout()

  assert.ok('token' in login)
  assert.equal(login.token, 'jwt-token')
  assert.equal(me.username, 'admin')
  assert.deepEqual(routes.routes, [{ name: '工作台', path: '/dashboard' }])
  assert.deepEqual(calls.map(call => `${call.method} ${call.url}`), [
    'POST /api/v1/auth/login',
    'GET /api/v1/auth/me',
    'GET /api/v1/auth/routes',
    'PUT /api/v1/auth/password',
    'POST /api/v1/auth/logout',
  ])
  assert.deepEqual(JSON.parse(calls[0].body ?? '{}'), { username: 'admin', password: 'admin123', captcha: '1234', captchaUuid: 'captcha-id' })
  assert.deepEqual(JSON.parse(calls[3].body ?? '{}'), { oldPassword: 'oldPwd123', newPassword: 'newPwd123' })
})

test('captcha api exposes the enable switch and login omits captcha when disabled', async () => {
  const calls: Array<{ url: string; method: string; body?: string }> = []
  const fetcher = async (url: string | URL | Request, init?: RequestInit) => {
    calls.push({ url: String(url), method: init?.method ?? 'GET', body: init?.body as string | undefined })
    const data = url.toString().endsWith('/captcha')
      ? { enabled: false, uuid: '', code: '' }
      : { token: 'jwt-token', expiresIn: 7200 }
    return new Response(JSON.stringify({ code: 200, message: 'success', data, traceId: 't' }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    })
  }
  const api = createAuthApi(createHttpClient({ baseUrl: '/api/v1', tokenProvider: () => '', fetcher }))

  const captcha = await api.captcha()
  // 开关关闭时后端不生成验证码，uuid / code 都是空串
  assert.deepEqual(captcha, { enabled: false, uuid: '', code: '' })

  // 关闭时登录请求不带验证码字段，而不是传空串
  await api.login({ username: 'admin', password: 'admin123' })
  assert.deepEqual(JSON.parse(calls[1].body ?? '{}'), { username: 'admin', password: 'admin123' })
})
