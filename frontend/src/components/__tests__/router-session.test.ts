import test, { type TestContext } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { parse, compileScript, compileTemplate } from '@vue/compiler-sfc'
import { build } from 'esbuild'
import * as vue from 'vue'
import * as vueRouter from 'vue-router'

const root = fileURLToPath(new URL('../../../', import.meta.url))
const compiled = await build({
  stdin: {
    contents: `
      export { default as router } from './src/router/index.ts'
      export { default as Generator } from './src/views/tool/Generator.vue'
      export { default as NoAccess } from './src/views/NoAccess.vue'
      export { authSession } from './src/api/session.ts'
      export { clearSession, sessionVersion } from './src/composables/auth.ts'
      export { settleConfirm } from './src/composables/confirm.ts'
      export { notices } from './src/composables/toast.ts'
    `,
    resolveDir: root,
    loader: 'ts',
  },
  tsconfig: `${root}/tsconfig.json`,
  bundle: true,
  write: false,
  format: 'cjs',
  platform: 'node',
  external: ['vue', 'vue-router'],
  define: { APP_VERSION: '"test"' },
  logOverride: { 'empty-import-meta': 'silent' },
  plugins: [{
    name: 'browser-boundary',
    setup(builder) {
      builder.onLoad({ filter: /\.vue$/ }, ({ path }) => {
        if (!path.endsWith('/AppShell.vue') && !path.endsWith('/Generator.vue')) {
          return { contents: 'export default { setup() { return () => null } }' }
        }
        const { descriptor, errors } = parse(readFileSync(path, 'utf8'), { filename: path })
        assert.deepEqual(errors, [])
        const script = compileScript(descriptor, { id: path, genDefaultAs: 'component' })
        const template = path.endsWith('/AppShell.vue') ? compileTemplate({
          source: descriptor.template!.content,
          filename: path,
          id: path,
          compilerOptions: { bindingMetadata: script.bindings },
        }) : { code: 'function render() { return null }', errors: [] }
        assert.deepEqual(template.errors, [])
        return { contents: `${script.content}\n${template.code}\ncomponent.render = render\nexport default component`, loader: 'ts' }
      })
      builder.onLoad({ filter: /composables\/motion\.ts$/ }, () => ({ contents: `
        export function pageEnter(element, done) { done() }
        export function pageLeave(element, done) { queueMicrotask(done) }
        export function playShellEntrance() {}
      ` }))
      builder.onLoad({ filter: /composables\/appearance\.ts$/ }, () => ({ contents: `
        import { reactive } from 'vue'
        export const appearance = reactive({ railCollapsed: false })
      ` }))
      builder.onLoad({ filter: /composables\/toast\.ts$/ }, () => ({ contents: `
        export const notices = []
        export const pushToast = (...args) => notices.push(args)
        export const toastError = (...args) => notices.push(args)
      ` }))
    },
  }],
})

interface HostNode {
  parent?: HostNode
  children: HostNode[]
  text?: string
  scrollTo(): void
}
const node = (): HostNode => ({ children: [], scrollTo() {} })
const renderer = vue.createRenderer<HostNode, HostNode>({
  insert(child, parent, anchor) {
    if (child.parent) child.parent.children.splice(child.parent.children.indexOf(child), 1)
    child.parent = parent
    const index = anchor ? parent.children.indexOf(anchor) : -1
    parent.children.splice(index < 0 ? parent.children.length : index, 0, child)
  },
  remove(child) {
    if (child.parent) child.parent.children.splice(child.parent.children.indexOf(child), 1)
    child.parent = undefined
  },
  createElement: node,
  createText: text => ({ ...node(), text }),
  createComment: text => ({ ...node(), text }),
  setText(child, text) { child.text = text },
  setElementText(child, text) { child.text = text; child.children = [] },
  parentNode: child => child.parent ?? null,
  nextSibling(child) { return child.parent?.children[child.parent.children.indexOf(child) + 1] ?? null },
  patchProp() {},
})

interface RequestRecord { path: string; token: string; method: string }
interface PendingResponse { promise: Promise<Response>; resolve(response: Response): void }
function deferred(): PendingResponse {
  let resolve!: PendingResponse['resolve']
  const promise = new Promise<Response>(done => { resolve = done })
  return { promise, resolve }
}
function response(data: unknown, status = 200) {
  return new Response(JSON.stringify({ code: status, message: status === 200 ? 'ok' : '旧请求失败', data }), { status })
}
async function settle() {
  await new Promise<void>(resolve => setImmediate(resolve))
  await vue.nextTick()
}

function user(token: string, required = false) {
  return { username: token, nickname: token, roles: [], permissions: ['tool:gen:list'], passwordChangeRequired: required }
}
const menu = { routes: [{ name: '代码生成', path: '/tool/generator', component: 'tool/gen/index', icon: 'code' }] }
const row = { id: 17, tableName: 'session_fixture', tableComment: '会话夹具', className: 'SessionFixture', module: 'fixture', businessName: 'session', synced: true, createTime: '' }

async function boot(t: TestContext, noMenu = false) {
  const requests: RequestRecord[] = []
  const held = new Map<string, PendingResponse>()
  const storageWindow = Object.assign(new EventTarget(), {
    matchMedia: () => ({ matches: false, addEventListener() {}, removeEventListener() {} }),
  })
  const documentStub = Object.assign(new EventTarget(), { title: '' })
  const fetcher = async (url: string, init: RequestInit = {}) => {
    const path = new URL(url, 'http://guardian.test').pathname
    const token = new Headers(init.headers).get('Authorization')?.replace('Bearer ', '') ?? ''
    const method = init.method ?? 'GET'
    requests.push({ path, token, method })
    const pending = held.get(`${token} ${method} ${path}`)
    if (pending) return pending.promise
    if (path === '/api/v1/auth/me') return response(user(token))
    if (path === '/api/v1/auth/routes') return response(noMenu ? { routes: [] } : menu)
    if (path === '/api/v1/tool/generator/tables') return response({ rows: [{ ...row, tableComment: token }], total: 1, pageNum: 1, pageSize: 10 })
    throw new Error(`未定义的请求：${token} ${method} ${path}`)
  }
  const module = { exports: {} as any }
  new Function('require', 'module', 'exports', 'window', 'document', 'fetch', compiled.outputFiles[0].text)(
    (name: string) => {
      if (name === 'vue') return vue
      if (name === 'vue-router') return { ...vueRouter, createWebHistory: vueRouter.createMemoryHistory }
      throw new Error(`未定义的依赖：${name}`)
    }, module, module.exports, storageWindow, documentStub, fetcher,
  )
  const subject = module.exports
  const instances: Array<{ state: any; token: string; unmounted: boolean }> = []
  const staticInstances: Array<{ token: string; unmounted: boolean }> = []
  for (const [component, list] of [[subject.Generator, instances], [subject.NoAccess, staticInstances]] as const) {
    const setup = component.setup
    component.setup = (props: unknown, context: unknown) => {
      const state = setup(props, context)
      const item = { state, token: subject.authSession.token(), unmounted: false }
      list.push(item)
      vue.onBeforeUnmount(() => { item.unmounted = true })
      return state
    }
  }
  function save(token: string) {
    subject.authSession.save({ token, expiresIn: 3600 }, user(token), true)
  }
  function hold(token: string, path: string, method = 'GET') {
    const pending = deferred()
    held.set(`${token} ${method} /api/v1${path}`, pending)
    return pending
  }
  function switchSession(token: string) {
    const completed = new Promise<{ to: vueRouter.RouteLocationNormalized; failure: unknown }>(resolve => {
      const remove = subject.router.afterEach((to: vueRouter.RouteLocationNormalized, _from: unknown, failure: unknown) => {
        remove()
        resolve({ to, failure })
      })
    })
    save(token)
    storageWindow.dispatchEvent(Object.assign(new Event('storage'), { key: 'guardian.token' }))
    return completed
  }
  save('account-a')
  const app = renderer.createApp({ render: () => vue.h(vueRouter.RouterView) })
  app.use(subject.router)
  await subject.router.push(noMenu ? '/no-access' : '/tool/generator')
  app.mount(node())
  await settle()
  t.after(async () => {
    app.unmount()
    subject.clearSession()
    for (const pending of held.values()) pending.resolve(response(null, 500))
    await settle()
  })
  return { subject, requests, instances, staticInstances, hold, switchSession }
}

for (const failed of [false, true]) {
  test(`同路径换号在身份与菜单确认前卸载旧页，旧移除${failed ? '失败' : '成功'}不让新页永久忙碌`, async t => {
    const app = await boot(t)
    assert.equal(app.instances.length, 1)
    const old = app.instances[0]
    const pendingDelete = app.hold('account-a', '/tool/generator/tables/17', 'DELETE')
    const removing = old.state.removeConfiguration(row)
    app.subject.settleConfirm(true)
    await settle()
    assert.equal(old.state.removing.value, 17)
    const me = app.hold('account-b', '/auth/me')
    const routes = app.hold('account-b', '/auth/routes')
    const completed = app.switchSession('account-b')
    await settle()
    assert.equal(old.unmounted, true, '换号时必须销毁旧业务实例')
    assert.equal(app.instances.length, 1, '身份未知时不能用新 token 重挂载旧业务页')
    assert.deepEqual(app.requests.filter(request => request.token === 'account-b').map(request => request.path), ['/api/v1/auth/me'])
    me.resolve(response(user('account-b')))
    await settle()
    assert.equal(app.instances.length, 1, '菜单未验证时不能挂载业务页')
    assert.deepEqual(app.requests.filter(request => request.token === 'account-b').map(request => request.path), ['/api/v1/auth/me', '/api/v1/auth/routes'])
    routes.resolve(response(menu))
    const navigation = await completed
    await settle()
    assert.equal(navigation.failure, undefined)
    assert.equal(navigation.to.path, '/tool/generator')
    assert.equal(app.instances.length, 2)
    const current = app.instances[1]
    assert.equal(current.token, 'account-b')
    assert.equal(current.state.removing.value, null)
    assert.notEqual(current.state.list, old.state.list)
    assert.equal(current.state.list.loading.value, false)
    assert.equal(current.state.list.rows.value[0].tableComment, 'account-b')
    const requestCount = app.requests.length
    pendingDelete.resolve(response(null, failed ? 500 : 200))
    await removing
    assert.equal(current.state.removing.value, null)
    assert.equal(app.requests.length, requestCount, '旧移除不能触发新账号的列表或候选刷新')
    assert.deepEqual(app.subject.notices, [])
  })
}

test('换号后服务端要求改密时只进入静态改密页，不挂载旧业务或请求菜单', async t => {
  const app = await boot(t)
  const me = app.hold('account-b', '/auth/me')
  const completed = app.switchSession('account-b')
  await settle()
  assert.equal(app.instances[0].unmounted, true)
  assert.equal(app.instances.length, 1)
  me.resolve(response(user('account-b', true)))
  const navigation = await completed
  await settle()
  assert.equal(navigation.failure, undefined)
  assert.equal(navigation.to.name, 'password-change')
  assert.equal(app.instances.length, 1)
  assert.deepEqual(app.requests.filter(request => request.token === 'account-b').map(request => request.path), ['/api/v1/auth/me'])
})

test('换号后相同静态落点也完成新会话导航并创建新实例', async t => {
  const app = await boot(t, true)
  assert.equal(app.staticInstances.length, 1)
  const me = app.hold('account-b', '/auth/me')
  const completed = app.switchSession('account-b')
  await settle()
  assert.equal(app.staticInstances[0].unmounted, true)
  assert.equal(app.staticInstances.length, 1)
  me.resolve(response(user('account-b')))
  const navigation = await completed
  await settle()
  assert.equal(navigation.failure, undefined, '相同静态路径也需要提交本次会话的授权结果')
  assert.equal(navigation.to.path, '/no-access')
  assert.equal(app.staticInstances.length, 2)
  assert.equal(app.staticInstances[1].token, 'account-b')
})


test('同路径快速换号后重新创建空闲页面，不复用旧移除的忙碌状态', async t => {
  const app = await boot(t)
  const pending = app.hold('account-a', '/tool/generator/tables/17', 'DELETE')
  const removing = app.instances[0].state.removeConfiguration(row)
  app.subject.settleConfirm(true)
  await settle()
  const navigation = await app.switchSession('account-b')
  await settle()
  assert.equal(navigation.to.path, '/tool/generator')
  pending.resolve(response(null))
  await removing
  const current = app.instances.find(instance => !instance.unmounted)!
  assert.equal(current.state.removing.value, null, '新会话不能继承旧实例无法清除的 removing')
  assert.equal(current.token, 'account-b')
  assert.equal(app.instances.length, 2)
})
