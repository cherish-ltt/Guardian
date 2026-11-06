import test, { after, beforeEach } from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { build } from 'esbuild'

const root = fileURLToPath(new URL('../../../', import.meta.url))
const output = new URL(`../../.sfc-build/auth-state-${process.pid}.mjs`, import.meta.url)
const compiled = await build({
  stdin: {
    contents: `
      export * from './src/composables/auth.ts'
      export { authSession } from './src/api/session.ts'
      export { hasPermission, setPermissions } from './src/directives/permission.ts'
      export { confirm, pendingConfirm, settleConfirm } from './src/composables/confirm.ts'
    `,
    resolveDir: root,
    loader: 'ts',
  },
  tsconfig: `${root}/tsconfig.json`,
  bundle: true,
  write: false,
  format: 'esm',
  platform: 'node',
  external: ['vue', 'vue-router', '*.vue'],
})
mkdirSync(new URL('./', output), { recursive: true })
writeFileSync(output, compiled.outputFiles[0].text)
const subject = await import(output.href)
after(() => { subject.clearSession(); rmSync(output, { force: true }) })

let routeResets = 0
subject.setSessionRouteReset(() => { routeResets += 1 })

function saveSession(token: string) {
  subject.authSession.save({ token, expiresIn: 3600 }, {
    username: token, nickname: token, roles: [], permissions: ['system:user:query'], passwordChangeRequired: false,
  }, true)
}

function grantMenu() {
  subject.setPermissions(['system:user:query'])
  subject.navGroups.value = [{ key: 'system', label: '系统', icon: 'user', items: [{ key: 'users', label: '用户', path: '/system/user' }] }]
  subject.dynamicRoutes.value = [{ path: 'system/user', name: 'users', component: {} }]
  subject.componentPaths.value = { 'system/user/index': '/system/user' }
}

function assertNoAccess() {
  assert.equal(subject.hasPermission('system:user:query'), false)
  assert.deepEqual(subject.navGroups.value, [])
  assert.deepEqual(subject.dynamicRoutes.value, [])
  assert.deepEqual(subject.componentPaths.value, {})
}

beforeEach(() => {
  subject.clearSession()
  saveSession('account-a')
  subject.synchronizeSession()
  grantMenu()
  routeResets = 0
})

test('首次要求改密清理旧权限和路由，但保留当前会话', () => {
  const version = subject.sessionVersion.value
  subject.requirePasswordChange()
  assert.equal(subject.sessionVersion.value, version + 1)
  assert.equal(routeResets, 1)
  assertNoAccess()
  assert.equal(subject.currentUser.value.passwordChangeRequired, true)
  assert.equal(subject.authed.value, true)
  assert.equal(subject.authSession.token(), 'account-a')
})

test('同一会话重复要求改密不再使待完成请求的版本失效', () => {
  subject.requirePasswordChange()
  const version = subject.sessionVersion.value
  subject.requirePasswordChange()
  subject.requirePasswordChange()
  assert.equal(subject.sessionVersion.value, version)
  assert.equal(routeResets, 1)
  assertNoAccess()
  assert.equal(subject.authSession.token(), 'account-a')
})

test('换账号清理旧状态后，新账号首次要求改密仍独立生效', () => {
  subject.requirePasswordChange()
  const version = subject.sessionVersion.value
  saveSession('account-b')
  assert.equal(subject.synchronizeSession(), true)
  assert.equal(subject.sessionVersion.value, version + 1)
  assert.equal(routeResets, 2)
  assertNoAccess()
  assert.equal(subject.currentUser.value.username, 'account-b')
  assert.equal(subject.currentUser.value.passwordChangeRequired, false)
  assert.equal(subject.authed.value, true)
  assert.equal(subject.synchronizeSession(), false)
  assert.equal(subject.sessionVersion.value, version + 1)
  grantMenu()
  subject.requirePasswordChange()
  assert.equal(subject.sessionVersion.value, version + 2)
  assert.equal(routeResets, 3)
  assertNoAccess()
  assert.equal(subject.currentUser.value.passwordChangeRequired, true)
  assert.equal(subject.authSession.token(), 'account-b')
})

test('跨标签页换号同步取消旧确认，并保留新账号会话', async () => {
  const old = subject.confirm({ title: '移除配置', message: '旧账号配置' })
  const version = subject.sessionVersion.value
  saveSession('account-b')
  assert.equal(subject.synchronizeSession(), true)
  assert.equal(subject.pendingConfirm.value, null)
  assert.equal(await old, false)
  assert.equal(subject.sessionVersion.value, version + 1)
  assert.equal(subject.currentUser.value.username, 'account-b')
  assert.equal(subject.authSession.token(), 'account-b')
  assert.equal(subject.authed.value, true)
  assertNoAccess()
  const next = subject.confirm({ title: '移除配置', message: '新账号配置' })
  subject.settleConfirm(true)
  assert.equal(await next, true)
})

test('首次要求改密取消旧确认，重复响应不取消之后的新确认', async () => {
  const old = subject.confirm({ title: '移除配置', message: '业务配置' })
  const version = subject.sessionVersion.value
  subject.requirePasswordChange()
  assert.equal(subject.pendingConfirm.value, null)
  assert.equal(await old, false)
  assert.equal(subject.sessionVersion.value, version + 1)
  assert.equal(subject.authSession.token(), 'account-a')
  const next = subject.confirm({ title: '退出登录', message: '确认退出' })
  const pending = subject.pendingConfirm.value
  subject.requirePasswordChange()
  assert.equal(subject.sessionVersion.value, version + 1)
  assert.equal(subject.pendingConfirm.value, pending)
  subject.settleConfirm(true)
  assert.equal(await next, true)
})
