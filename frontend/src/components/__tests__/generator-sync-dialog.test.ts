import test, { type TestContext } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { parse, compileScript } from '@vue/compiler-sfc'
import { transformSync } from 'esbuild'
import * as vue from 'vue'
import { createAuthSession } from '../../api/session.ts'
import { createHttpClient } from '../../api/http.ts'
import { createPortalApi, type GeneratorSyncPreview } from '../../api/portal.ts'

const filename = new URL('../business/GeneratorSyncDialog.vue', import.meta.url).pathname
const { descriptor, errors } = parse(readFileSync(filename, 'utf8'), { filename })
assert.deepEqual(errors, [])
const script = transformSync(compileScript(descriptor, { id: filename }).content, {
  loader: 'ts', format: 'cjs', target: 'es2022',
}).code

interface HostNode { parent?: HostNode; children?: HostNode[]; text?: string }
const renderer = vue.createRenderer<HostNode, HostNode>({
  insert(node, parent) { node.parent = parent; (parent.children ??= []).push(node) },
  remove(node) {
    if (node.parent) node.parent.children = node.parent.children?.filter(child => child !== node)
  },
  createElement: () => ({ children: [] }),
  createText: text => ({ text }),
  createComment: text => ({ text }),
  setText(node, text) { node.text = text },
  setElementText(node, text) { node.text = text },
  parentNode: node => node.parent ?? null,
  nextSibling: () => null,
  patchProp() {},
})

interface DialogState {
  preview: GeneratorSyncPreview | null
  loading: boolean
  saving: boolean
  error: string
  load(): Promise<void>
  synchronize(): Promise<void>
  close(): void
}

interface PendingRequest {
  url: string
  init: RequestInit
  resolve(response: Response): void
}

const preview: GeneratorSyncPreview = {
  tableName: 'sync_review', revision: 'revision-a',
  changes: [{
    columnName: 'new_column', kind: 'added', before: null,
    after: {
      col: 'new_column', columnType: 'TEXT', nullable: true, primaryKey: false,
      sort: 1, rustType: 'String', formType: 'input', required: false,
      insert: true, edit: true, list: true,
    },
    changedAttributes: [],
  }],
}

function response(data: unknown, status = 200) {
  return new Response(JSON.stringify({ code: status, message: status === 200 ? 'ok' : '旧请求失败', data }), { status })
}

async function settle() {
  await new Promise<void>(resolve => setImmediate(resolve))
  await vue.nextTick()
}

function mountDialog(t: TestContext) {
  const authSession = createAuthSession()
  const sessionVersion = vue.ref(0)
  const notices: string[] = []
  const events: string[] = []
  const requests: PendingRequest[] = []
  const portalApi = createPortalApi(createHttpClient({
    tokenProvider: authSession.token,
    fetcher: (url, init = {}) => new Promise<Response>(resolve => {
      requests.push({ url: String(url), init, resolve })
      if (String(url).endsWith('/tables')) resolve(response({ records: [], total: 0 }))
    }),
  }))
  const dependencies: Record<string, unknown> = {
    vue,
    '@/components/ui': { Button: {}, Tag: {} },
    '@/components/business': { FormDialog: {} },
    '@/api': { portalApi, authSession },
    '@/composables/auth': { sessionVersion },
    '@/composables/toast': { pushToast: (message: string) => notices.push(message) },
  }
  const module = { exports: {} as { default: vue.ComponentOptions } }
  new Function('require', 'module', 'exports', script)((name: string) => {
    assert.ok(name in dependencies, `未知依赖：${name}`)
    return dependencies[name]
  }, module, module.exports)
  const component = { ...module.exports.default, render: () => null }
  const table = vue.ref<{ id: number; tableName: string } | null>({ id: 17, tableName: 'sync_review' })
  function login(token: string) {
    authSession.save({ token, expiresIn: 3600 }, { username: token, nickname: token }, true)
  }
  login('sync-session-a')
  const app = renderer.createApp({
    render: () => vue.h(component, {
      table: table.value,
      onClose() { events.push('close'); table.value = null },
      onSaved() { events.push('saved'); void portalApi.generator.tables.list() },
    }),
  })
  app.mount({ children: [] })
  const state = (app._instance!.subTree.component as unknown as { setupState: DialogState }).setupState
  let mounted = true
  function unmount() {
    if (mounted) { app.unmount(); mounted = false }
  }
  t.after(async () => {
    unmount()
    requests.forEach(request => request.resolve(response(null, 500)))
    await settle()
    authSession.clear()
  })
  return { state, requests, notices, events, login, authSession, sessionVersion, table, unmount }
}

async function showPreview(dialog: ReturnType<typeof mountDialog>) {
  assert.equal(dialog.requests.length, 1)
  assert.equal(dialog.requests[0].url, '/api/v1/tool/generator/tables/17/sync-preview')
  dialog.requests[0].resolve(response(preview))
  await settle()
  assert.deepEqual(dialog.state.preview, preview)
  assert.equal(dialog.state.loading, false)
}

test('同步弹窗在有效会话中提交显示的 revision，成功后提示并刷新列表', async t => {
  const dialog = mountDialog(t)
  await showPreview(dialog)
  const task = dialog.state.synchronize()
  await dialog.state.synchronize()
  assert.equal(dialog.requests.length, 2, '同步过程中不能重复提交')
  assert.equal(dialog.requests[1].url, '/api/v1/tool/generator/tables/17/sync')
  assert.equal(dialog.requests[1].init.method, 'POST')
  assert.equal(new Headers(dialog.requests[1].init.headers).get('Authorization'), 'Bearer sync-session-a')
  assert.deepEqual(JSON.parse(String(dialog.requests[1].init.body)), { revision: preview.revision })
  dialog.requests[1].resolve(response({ tableName: preview.tableName, changed: 1 }))
  await task
  assert.deepEqual(dialog.notices, ['已同步 1 个字段，请重新确认生成配置'])
  assert.deepEqual(dialog.events, ['saved', 'close'])
  assert.equal(dialog.requests[2].url, '/api/v1/tool/generator/tables')
})

for (const status of [200, 500]) {
  test(`弹窗卸载后忽略迟到的同步响应（${status}）`, async t => {
    const dialog = mountDialog(t)
    await showPreview(dialog)
    const task = dialog.state.synchronize()
    dialog.unmount()
    dialog.requests[1].resolve(response({ tableName: preview.tableName, changed: 1 }, status))
    await task
    assert.deepEqual(dialog.notices, [])
    assert.deepEqual(dialog.events, [])
    assert.equal(dialog.requests.length, 2)
    assert.equal(dialog.state.error, '')
    assert.deepEqual(dialog.state.preview, preview)
    assert.equal(dialog.state.saving, true, '卸载后的 finally 不能更新旧组件')
  })

  test(`token 已切换但会话版本未更新时忽略迟到的预览（${status}）`, async t => {
    const dialog = mountDialog(t)
    dialog.login('sync-session-b')
    dialog.requests[0].resolve(response(preview, status))
    await settle()
    assert.equal(dialog.sessionVersion.value, 0)
    assert.equal(dialog.state.preview, null)
    assert.equal(dialog.state.error, '')
    assert.equal(dialog.state.loading, true)
    assert.deepEqual(dialog.notices, [])
    assert.deepEqual(dialog.events, [])
  })

  test(`token 已切换但会话版本未更新时忽略迟到的同步响应（${status}）`, async t => {
    const dialog = mountDialog(t)
    await showPreview(dialog)
    const task = dialog.state.synchronize()
    dialog.login('sync-session-b')
    dialog.requests[1].resolve(response({ tableName: preview.tableName, changed: 1 }, status))
    await task
    assert.equal(dialog.sessionVersion.value, 0)
    assert.deepEqual(dialog.notices, [])
    assert.deepEqual(dialog.events, [])
    assert.equal(dialog.requests.length, 2, '不能用新会话刷新列表')
    assert.equal(dialog.state.error, '')
    assert.deepEqual(dialog.state.preview, preview)
    assert.equal(dialog.state.saving, true)
  })
}

for (const change of ['token', 'logout', 'version'] as const) {
  test(`旧会话预览不能在 ${change} 变化后发起同步或重新预览`, async t => {
    const dialog = mountDialog(t)
    await showPreview(dialog)
    if (change === 'token') dialog.login('sync-session-b')
    if (change === 'logout') dialog.authSession.clear()
    if (change === 'version') dialog.sessionVersion.value++
    const task = dialog.state.synchronize()
    dialog.requests[1]?.resolve(response({ tableName: preview.tableName, changed: 1 }))
    await task
    const retry = dialog.state.load()
    dialog.requests[dialog.requests.length - 1]?.resolve(response(preview))
    await retry
    assert.equal(dialog.requests.length, 1, '失效弹窗不能把旧表或旧 revision 配对到当前会话')
    assert.deepEqual(dialog.notices, [])
    assert.equal(dialog.events.includes('saved'), false)
  })
}

test('弹窗卸载后忽略迟到的预览', async t => {
  const dialog = mountDialog(t)
  dialog.unmount()
  dialog.requests[0].resolve(response(preview))
  await settle()
  assert.equal(dialog.state.preview, null)
  assert.equal(dialog.state.loading, true)
  assert.deepEqual(dialog.events, [])
})

test('关闭后重新打开绑定新会话，旧预览不能覆盖新预览', async t => {
  const dialog = mountDialog(t)
  dialog.state.close()
  await vue.nextTick()
  dialog.login('sync-session-b')
  dialog.table.value = { id: 17, tableName: preview.tableName }
  await vue.nextTick()
  assert.equal(dialog.requests.length, 2)
  assert.equal(new Headers(dialog.requests[1].init.headers).get('Authorization'), 'Bearer sync-session-b')
  const newPreview = { ...preview, revision: 'revision-b' }
  dialog.requests[1].resolve(response(newPreview))
  await settle()
  dialog.requests[0].resolve(response(preview))
  await settle()
  assert.deepEqual(dialog.state.preview, newPreview)
  const task = dialog.state.synchronize()
  assert.deepEqual(JSON.parse(String(dialog.requests[2].init.body)), { revision: newPreview.revision })
  assert.equal(new Headers(dialog.requests[2].init.headers).get('Authorization'), 'Bearer sync-session-b')
  dialog.requests[2].resolve(response({ tableName: preview.tableName, changed: 1 }))
  await task
  assert.deepEqual(dialog.events, ['close', 'saved', 'close'])
})

test('有效会话中预览和同步失败可重新预览，再提交新 revision', async t => {
  const dialog = mountDialog(t)
  dialog.requests[0].resolve(response(null, 500))
  await settle()
  assert.equal(dialog.state.error, '旧请求失败')
  assert.equal(dialog.state.loading, false)
  const retry = dialog.state.load()
  dialog.requests[1].resolve(response(preview))
  await retry
  assert.equal(dialog.state.error, '')
  assert.deepEqual(dialog.state.preview, preview)
  const sync = dialog.state.synchronize()
  dialog.requests[2].resolve(response(null, 409))
  await sync
  assert.equal(dialog.state.error, '旧请求失败')
  assert.equal(dialog.state.saving, false)
  assert.equal(dialog.state.preview, null)
  assert.deepEqual(dialog.events, [])
  assert.deepEqual(dialog.notices, [])
  const reload = dialog.state.load()
  const newPreview = { ...preview, revision: 'revision-after-conflict' }
  dialog.requests[3].resolve(response(newPreview))
  await reload
  const confirm = dialog.state.synchronize()
  assert.deepEqual(JSON.parse(String(dialog.requests[4].init.body)), { revision: newPreview.revision })
  dialog.requests[4].resolve(response({ tableName: preview.tableName, changed: 1 }))
  await confirm
  assert.deepEqual(dialog.events, ['saved', 'close'])
  assert.equal(dialog.notices.length, 1)
})
