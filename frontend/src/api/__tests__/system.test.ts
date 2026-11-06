import test from 'node:test'
import assert from 'node:assert/strict'
import { randomBytes } from 'node:crypto'
import { createHttpClient } from '../http.ts'
import { createSystemApi } from '../system.ts'

test('log lists and exports preserve server-local time windows without timezone conversion', async () => {
  const calls: URL[] = []
  const api = createSystemApi(createHttpClient({
    baseUrl: 'http://localhost/api/v1',
    fetcher: async url => {
      calls.push(new URL(String(url)))
      return String(url).includes('/export')
        ? new Response('fixture')
        : new Response(JSON.stringify({ code: 200, data: { rows: [], total: 0 } }))
    },
  }))
  for (const logs of [api.operLogs, api.loginLogs]) {
    for (const bounds of [
      { startTime: '2026-01-02 10:00:00', endTime: '2026-01-02 11:00:00' },
      { startTime: '2026-01-02 10:00:00' },
      { endTime: '2026-01-02 11:00:00' },
      {},
    ]) {
      const query = { keyword: '日志', ...bounds }
      await logs.list({ ...query, pageNum: 2, pageSize: 10 })
      await logs.export(query)
      const [list, exported] = calls.slice(-2)
      assert.equal(list!.searchParams.get('pageNum'), '2')
      assert.equal(exported!.searchParams.has('pageNum'), false)
      for (const key of ['startTime', 'endTime', 'keyword'] as const) {
        assert.equal(list!.searchParams.get(key), query[key] ?? null)
        assert.equal(exported!.searchParams.get(key), query[key] ?? null)
      }
    }
  }
})

test('system api maps user list and CRUD calls to backend endpoints', async () => {
  const calls: Array<{ url: string; method: string; body?: string }> = []
  const fetcher = async (url: string | URL | Request, init?: RequestInit) => {
    calls.push({ url: String(url), method: init?.method ?? 'GET', body: init?.body as string | undefined })
    if (String(url).includes('/export')) {
      return new Response('登录账号,用户昵称\nadmin,管理员\n', {
        status: 200,
        headers: { 'content-type': 'text/csv;charset=UTF-8' },
      })
    }
    return new Response(JSON.stringify({ code: 200, message: 'success', data: { id: 99, rows: [], total: 0 }, traceId: 't' }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    })
  }
  const api = createSystemApi(createHttpClient({ baseUrl: '/api/v1', tokenProvider: () => 'token', fetcher }))

  await api.users.list({ pageNum: 2, pageSize: 20, keyword: 'admin', status: 1, deptId: 6 })
  await api.users.create({ username: 'it_user', nickname: '测试用户', password: 'admin123', deptId: 6, status: 1, roleIds: [4], postIds: [6] })
  await api.users.update(99, { nickname: '测试用户已更新', status: 0, roleIds: [5], postIds: [7] })
  await api.users.updateStatus(99, 0)
  const resetValue = randomBytes(16).toString('hex')
  await api.users.resetPassword(99, resetValue)
  const exportBlob = await api.users.export({ keyword: 'admin', status: 1, deptId: 6 })
  await api.users.remove(99)

  assert.equal(await exportBlob.text(), '登录账号,用户昵称\nadmin,管理员\n')
  assert.equal(calls[0].url, '/api/v1/system/users?pageNum=2&pageSize=20&keyword=admin&status=1&deptId=6')
  assert.equal(calls[1].method, 'POST')
  assert.equal(calls[1].url, '/api/v1/system/users')
  assert.deepEqual(JSON.parse(calls[1].body ?? '{}'), { username: 'it_user', nickname: '测试用户', password: 'admin123', deptId: 6, status: 1, roleIds: [4], postIds: [6] })
  assert.equal(calls[2].method, 'PUT')
  assert.equal(calls[2].url, '/api/v1/system/users/99')
  assert.equal(calls[3].method, 'PUT')
  assert.equal(calls[3].url, '/api/v1/system/users/99/status')
  assert.deepEqual(JSON.parse(calls[3].body ?? '{}'), { status: 0 })
  assert.equal(calls[4].method, 'PUT')
  assert.equal(calls[4].url, '/api/v1/system/users/99/password')
  assert.deepEqual(JSON.parse(calls[4].body ?? '{}'), { password: resetValue, passwordChangeRequired: false })
  assert.equal(calls[5].method, 'GET')
  assert.equal(calls[5].url, '/api/v1/system/users/export?keyword=admin&status=1&deptId=6')
  assert.equal(calls[6].method, 'DELETE')
  assert.equal(calls[6].url, '/api/v1/system/users/99')
})

test('user creation and password reset explicitly carry the required-change option', async () => {
  const bodies: unknown[] = []
  const api = createSystemApi(createHttpClient({
    fetcher: async (_url, init) => {
      bodies.push(JSON.parse(String(init?.body)))
      return new Response(JSON.stringify({ code: 200, data: { id: 1 } }))
    },
  }))
  await api.users.create({ username: 'handoff', passwordChangeRequired: true })
  const resetValue = randomBytes(16).toString('hex')
  await api.users.resetPassword(1, resetValue, true)
  assert.deepEqual(bodies, [
    { username: 'handoff', passwordChangeRequired: true },
    { password: resetValue, passwordChangeRequired: true },
  ])
})

test('system api exposes catalog endpoints used by management pages', async () => {
  const calls: Array<{ url: string; method: string }> = []
  const fetcher = async (url: string | URL | Request, init?: RequestInit) => {
    calls.push({ url: String(url), method: init?.method ?? 'GET' })
    if (String(url).includes('/export')) {
      return new Response('模块,类型\n用户管理,查询\n', {
        status: 200,
        headers: { 'content-type': 'text/csv;charset=UTF-8' },
      })
    }
    return new Response(JSON.stringify({ code: 200, message: 'success', data: { id: 1, rows: [] }, traceId: 't' }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    })
  }
  const api = createSystemApi(createHttpClient({ baseUrl: '/api/v1', tokenProvider: () => 'token', fetcher }))

  await api.roles.create({ name: '角色', code: 'role_code', dataScope: 'SELF', status: 1 })
  await api.menus.update(101, { name: '用户管理', visible: 1, status: 1 })
  await api.depts.remove(6)
  await api.posts.list({ pageNum: 1, pageSize: 10 })
  await api.dictData.create({ dictType: 'sys_normal_disable', label: '正常', value: '1', status: 1, def: true })
  await api.configs.update(1, { name: '参数', value: 'enabled' })
  await api.operLogs.remove(12)
  await api.operLogs.clear()
  const operExport = await api.operLogs.export({ keyword: '用户', result: 1 })
  await api.loginLogs.remove(13)
  await api.loginLogs.clear()
  const loginExport = await api.loginLogs.export({ keyword: 'admin', status: 1 })
  await api.operLogs.stats('7d')
  await api.operLogs.timeline()
  await api.operLogs.timeline('2026-10-10')
  await api.loginLogs.stats()
  await api.permissions.graph()
  await api.loginLogs.heatmap(30)

  assert.equal(await operExport.text(), '模块,类型\n用户管理,查询\n')
  assert.equal(await loginExport.text(), '模块,类型\n用户管理,查询\n')
  assert.deepEqual(calls.map(call => `${call.method} ${call.url}`), [
    'POST /api/v1/system/roles',
    'PUT /api/v1/system/menus/101',
    'DELETE /api/v1/system/depts/6',
    'GET /api/v1/system/posts?pageNum=1&pageSize=10',
    'POST /api/v1/system/dict/data',
    'PUT /api/v1/system/configs/1',
    'DELETE /api/v1/monitor/oper-logs/12',
    'DELETE /api/v1/monitor/oper-logs',
    'GET /api/v1/monitor/oper-logs/export?keyword=%E7%94%A8%E6%88%B7&result=1',
    'DELETE /api/v1/monitor/login-logs/13',
    'DELETE /api/v1/monitor/login-logs',
    'GET /api/v1/monitor/login-logs/export?keyword=admin&status=1',
    'GET /api/v1/monitor/oper-logs/stats?range=7d',
    'GET /api/v1/monitor/oper-logs/timeline',
    'GET /api/v1/monitor/oper-logs/timeline?date=2026-10-10',
    'GET /api/v1/monitor/login-logs/stats?range=24h',
    'GET /api/v1/system/permissions/graph',
    'GET /api/v1/monitor/login-logs/heatmap?days=30',
  ])
})
