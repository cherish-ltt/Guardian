import test, { after, beforeEach, type TestContext } from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { build } from 'esbuild'
import { effectScope, type EffectScope } from 'vue'
import type { ListPageOptions } from '../../composables/list.ts'
import type { PageResult } from '../../api/system.ts'

const root = fileURLToPath(new URL('../../../', import.meta.url))
const output = new URL(`../../.sfc-build/list-session-${process.pid}.mjs`, import.meta.url)
// 打包真实 composable，只替换浏览器提示与文件下载副作用。
const compiled = await build({
  stdin: {
    contents: `
      export { useListPage } from './src/composables/list.ts'
      export { captureSession, clearSession, sessionVersion } from './src/composables/auth.ts'
      export { authSession } from './src/api/session.ts'
      export { confirm, pendingConfirm, settleConfirm } from './src/composables/confirm.ts'
      export { notices } from './src/composables/toast.ts'
      export { downloads } from './src/utils/download.ts'
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
  plugins: [{
    name: 'browser-effects',
    setup(builder) {
      builder.onLoad({ filter: /composables\/toast\.ts$/ }, () => ({ contents: `
        export const notices = []
        export const pushToast = (...args) => notices.push(args)
        export const toastError = (...args) => notices.push(args)
      ` }))
      builder.onLoad({ filter: /utils\/download\.ts$/ }, () => ({ contents: `
        export const downloads = []
        export const downloadBlob = (...args) => downloads.push(args)
      ` }))
    },
  }],
})
mkdirSync(new URL('./', output), { recursive: true })
writeFileSync(output, compiled.outputFiles[0].text)
const subject = await import(output.href)
after(() => { subject.clearSession(); rmSync(output, { force: true }) })

function saveSession(token = 'session-a') {
  subject.authSession.save({ token, expiresIn: 3600 }, { username: token, nickname: token }, true)
}
beforeEach(() => {
  subject.clearSession()
  saveSession()
  subject.notices.length = 0
  subject.downloads.length = 0
})

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (error: Error) => void
  const promise = new Promise<T>((success, failure) => { resolve = success; reject = failure })
  return { promise, resolve, reject }
}
function makeList(t: TestContext, options: Partial<ListPageOptions<{ id: number }, { keyword: string }>> = {}) {
  const scope = effectScope()
  t.after(() => scope.stop())
  const list = scope.run(() => subject.useListPage({
    query: { keyword: '' },
    immediate: false,
    fetch: async () => ({ rows: [], total: 0, pageNum: 1, pageSize: 10 }),
    ...options,
  }))
  return { list, scope }
}

test('会话快照同时校验令牌和版本，不暴露令牌', () => {
  const original = subject.captureSession()
  assert.equal(original(), true)
  saveSession('session-b')
  assert.equal(original(), false)
  const replacement = subject.captureSession()
  assert.equal(replacement(), true)
  subject.sessionVersion.value += 1
  assert.equal(replacement(), false)
  subject.clearSession()
  assert.equal(subject.captureSession()(), false)
})

test('统一会话清理取消待决确认，之后的新确认可正常完成', async () => {
  const old = subject.confirm({ title: '移除配置', message: '旧会话' })
  subject.clearSession()
  assert.equal(await old, false)
  assert.equal(subject.pendingConfirm.value, null)
  saveSession('session-b')
  const next = subject.confirm({ title: '移除配置', message: '新会话' })
  subject.settleConfirm(true)
  assert.equal(await next, true)
})

test('有效会话删除末页最后一项后仍回退并加载上一页', async t => {
  const pages: number[] = []
  const { list } = makeList(t, { fetch: async params => {
    const pageNum = Number(params.pageNum)
    pages.push(pageNum)
    return { rows: pageNum === 2 ? [] : [{ id: 1 }], total: 10, pageNum, pageSize: 10 }
  } })
  list.page.value = 2
  await list.load()
  assert.deepEqual(pages, [2, 1])
  assert.deepEqual(list.rows.value, [{ id: 1 }])
  assert.equal(list.page.value, 1)
  assert.equal(list.loading.value, false)
})

test('同一会话的较旧列表响应不能覆盖最新查询', async t => {
  const first = deferred<PageResult<{ id: number }>>()
  let calls = 0
  const { list } = makeList(t, { fetch: () => ++calls === 1 ? first.promise : Promise.resolve({ rows: [{ id: 2 }], total: 1, pageNum: 1, pageSize: 10 }) })
  const pending = list.load()
  await list.load()
  first.resolve({ rows: [{ id: 1 }], total: 1, pageNum: 1, pageSize: 10 })
  await pending
  assert.deepEqual(list.rows.value, [{ id: 2 }])
  assert.equal(list.loading.value, false)
})

const invalidations: Array<[string, (scope: EffectScope) => void]> = [
  ['令牌切换但版本尚未同步', () => saveSession('session-b')],
  ['会话版本变化', () => { subject.sessionVersion.value += 1 }],
  ['组件已卸载', scope => scope.stop()],
]
for (const [label, invalidate] of invalidations) {
  for (const failed of [false, true]) {
    test(`${label}后迟到的列表${failed ? '错误不提示' : '结果不回写或递归翻页'}`, async t => {
      const response = deferred<PageResult<{ id: number }>>()
      let requests = 0
      const { list, scope } = makeList(t, { fetch: () => { requests += 1; return response.promise } })
      list.rows.value = [{ id: 9 }]
      list.total.value = 11
      list.selected.value = [9]
      list.page.value = 2
      const pending = list.load()
      invalidate(scope)
      if (failed) response.reject(new Error('旧列表失败'))
      else response.resolve({ rows: [], total: 10, pageNum: 2, pageSize: 10 })
      await pending
      assert.equal(requests, 1)
      assert.deepEqual(list.rows.value, [{ id: 9 }])
      assert.deepEqual(list.selected.value, [9])
      assert.equal(list.total.value, 11)
      assert.equal(list.page.value, 2)
      assert.equal(list.loading.value, true)
      assert.deepEqual(subject.notices, [])
    })

    test(`${label}后迟到的导出${failed ? '错误不提示' : '结果不下载'}`, async t => {
      const response = deferred<Blob>()
      const { list, scope } = makeList(t, { exporter: () => response.promise })
      const pending = list.exportData()
      invalidate(scope)
      if (failed) response.reject(new Error('旧导出失败'))
      else response.resolve(new Blob(['private']))
      await pending
      assert.deepEqual(subject.downloads, [])
      assert.deepEqual(subject.notices, [])
      assert.equal(list.exporting.value, true)
    })
  }
}

test('有效会话的导出仍下载文件并报告成功，失败仍明确提示', async t => {
  const blob = new Blob(['csv'])
  let failed = false
  const { list } = makeList(t, { exporter: async () => {
    if (failed) throw new Error('导出失败')
    return blob
  } })
  await list.exportData()
  assert.deepEqual(subject.downloads, [[blob, 'export.csv']])
  assert.equal(subject.notices.length, 1)
  assert.equal(list.exporting.value, false)
  failed = true
  await list.exportData()
  assert.equal(subject.notices.length, 2)
  assert.equal(subject.downloads.length, 1)
  assert.equal(list.exporting.value, false)
})
