import { execFileSync } from 'node:child_process'
import { randomBytes } from 'node:crypto'
import { mkdir } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'

const { chromium, expect, request } = await import(process.env.GUARDIAN_PLAYWRIGHT_MODULE || '@playwright/test')
const base = (process.env.GUARDIAN_TEST_URL || 'http://127.0.0.1:5173').replace(/\/$/, '')
const credential = process.env.GUARDIAN_TEST_PASSWORD
const database = process.env.GUARDIAN_TEST_DATABASE
if (!credential || !database) throw new Error('请提供隔离实例的 GUARDIAN_TEST_PASSWORD 与 GUARDIAN_TEST_DATABASE')
const prefix = `remove_ui_${Date.now()}`
const tables = Array.from({ length: 11 }, (_, index) => `${prefix}_${index}`)
const artifacts = new URL('../.test-artifacts/', import.meta.url)
await mkdir(artifacts, { recursive: true })
const api = await request.newContext({ baseURL: `${base}/api/v1/` })
let browser
let headers
let otherUser
let generatorUser
let generatorRole
const otherPassword = randomBytes(24).toString('hex')
function sql(statement) { return execFileSync('sqlite3', [database, statement], { encoding: 'utf8' }).trim() }
function solve(image) { return execFileSync('python3', [fileURLToPath(new URL('./captcha_solver.py', import.meta.url))], { input: image, encoding: 'utf8' }).trim() }
async function json(response) { expect(response.status()).toBe(200); return (await response.json()).data }
async function listed() { return (await json(await api.get('tool/generator/tables', { headers, params: { keyword: prefix, pageSize: '100' } }))).rows }
async function signIn(page, username, secret, captcha) {
  if (!captcha) {
    const ready = page.waitForResponse(response => response.url().endsWith('/auth/captcha')).then(response => response.json())
    await page.goto(`${base}/login`)
    captcha = (await ready).data
  }
  await page.getByPlaceholder('请输入账号', { exact: true }).fill(username)
  await page.getByPlaceholder('请输入密码', { exact: true }).fill(secret)
  if (captcha.enabled) await page.getByPlaceholder('请输入右侧字符', { exact: true }).fill(solve(captcha.image))
  await page.getByRole('button', { name: '登录', exact: true }).click()
  await page.waitForURL(url => !url.pathname.startsWith('/login'))
  await expect(page.locator('.user-chip')).toBeVisible()
}
async function signOut(page) {
  await page.locator('.user-chip').click()
  await page.getByRole('menuitem', { name: '退出登录', exact: true }).click()
  const ready = page.waitForResponse(response => response.url().endsWith('/auth/captcha')).then(response => response.json())
  await page.getByRole('dialog').getByRole('button', { name: '退出', exact: true }).click()
  await page.waitForURL('**/login**')
  return (await ready).data
}
async function lastConfiguration(page) {
  await page.goto(`${base}/tool/generator`)
  await page.getByPlaceholder('表名 / 表说明', { exact: true }).fill(prefix)
  const filtered = page.waitForResponse(response => {
    const url = new URL(response.url())
    return url.pathname === '/api/v1/tool/generator/tables' && url.searchParams.get('keyword') === prefix && url.searchParams.get('pageNum') === '1'
  })
  await page.getByRole('button', { name: '查询', exact: true }).click()
  expect((await filtered).status()).toBe(200)
  await expect(page.locator('tr.data-row')).toHaveCount(10)
  await expect(page.locator('table')).toHaveAttribute('aria-busy', 'false')
  await page.getByRole('button', { name: '下一页', exact: true }).click()
  await expect(page.locator('tr.data-row')).toHaveCount(1)
  const name = (await page.locator('tr.data-row .cell-title').innerText()).trim()
  const target = (await listed()).find(row => row.tableName === name)
  expect(target).toBeTruthy()
  return target
}

try {
  for (const table of tables) sql(`CREATE TABLE ${table} (record_id INTEGER PRIMARY KEY, name TEXT NOT NULL); INSERT INTO ${table} VALUES (7, 'keep');`)
  const captcha = await json(await api.get('auth/captcha'))
  const login = await json(await api.post('auth/login', { data: { username: 'admin', 'password': credential,
    ...(captcha.enabled ? { captcha: solve(captcha.image), captchaUuid: captcha.uuid } : {}) } }))
  headers = { Authorization: `Bearer ${login.token}` }
  await json(await api.post('tool/generator/import', { headers, data: { tableNames: tables } }))
  const username = `${prefix}_reader`
  const created = await json(await api.post('system/users', { headers, data: { username, nickname: '移除会话回归', 'password': otherPassword, deptId: 5, status: 1, roleIds: [], passwordChangeRequired: false } }))
  otherUser = { id: created.id, username }
  generatorRole = await json(await api.post('system/roles', { headers, data: {
    name: '同路径会话回归', code: `${prefix}_generator`, dataScope: 'SELF', sort: 99, status: 1, menuIds: [300, 301],
  } }))
  const generatorName = `${prefix}_generator`
  const generatorAccount = await json(await api.post('system/users', { headers, data: {
    username: generatorName, nickname: '生成器会话回归', 'password': otherPassword, deptId: 5, status: 1, roleIds: [generatorRole.id], passwordChangeRequired: false,
  } }))
  generatorUser = { id: generatorAccount.id, username: generatorName }
  browser = await chromium.launch({ headless: true, channel: process.env.GUARDIAN_BROWSER_CHANNEL || 'chrome' })
  const context = await browser.newContext({ viewport: { width: 1440, height: 1080 }, reducedMotion: 'reduce' })
  const page = await context.newPage()
  const errors = []
  page.on('pageerror', error => errors.push(error.message))
  const generatorRequests = []
  page.on('request', req => { if (new URL(req.url()).pathname.startsWith('/api/v1/tool/generator/')) generatorRequests.push(`${req.method()} ${req.url()}`) })
  await signIn(page, 'admin', credential)
  const target = await lastConfiguration(page)
  const name = target.tableName
  const endpoint = `${base}/api/v1/tool/generator/tables/${target.id}`
  let deletes = 0
  page.on('request', req => { if (req.method() === 'DELETE') deletes += 1 })
  await page.getByRole('button', { name: '移除配置', exact: true }).click()
  const dialog = page.getByRole('dialog', { name: '移除生成配置' })
  await expect(dialog).toContainText(name)
  await expect(dialog).toContainText('永久丢失')
  await expect(dialog).toContainText('业务表、业务数据及已生成文件都会保留')
  await dialog.getByRole('button', { name: '取消', exact: true }).click()
  await expect(dialog).toHaveCount(0)
  expect(deletes).toBe(0)
  expect((await listed()).length).toBe(11)
  console.log('PASS: 取消不请求，确认文案说明配置丢失与保留范围')

  await page.route(endpoint, route => route.fulfill({ status: 500, json: { code: 500, message: '模拟移除失败', data: null } }))
  await page.getByRole('button', { name: '移除配置', exact: true }).click()
  await dialog.getByRole('button', { name: '移除配置', exact: true }).click()
  const failureToast = page.locator('.toast').filter({ hasText: '模拟移除失败' })
  await expect(failureToast).toBeVisible()
  await failureToast.getByRole('button', { name: '关闭提示', exact: true }).click()
  await expect(failureToast).toHaveCount(0)
  await expect(dialog).toHaveCount(0)
  await expect(page.locator('tr.data-row')).toHaveCount(1)
  expect((await listed()).length).toBe(11)
  expect(await page.getByText(`已移除「${name}」的生成配置`, { exact: true }).count()).toBe(0)
  await page.unroute(endpoint)
  console.log('PASS: 删除失败保留行且不报告成功')

  for (const width of [1440, 390, 320]) {
    await page.setViewportSize({ width, height: width > 1000 ? 1080 : 844 })
    const rowActions = page.locator('tr.data-row .row-actions')
    for (const label of ['预览', '配置', '同步结构', '生成', '移除配置']) {
      const action = rowActions.getByRole('button', { name: label, exact: true })
      await action.scrollIntoViewIfNeeded()
      await action.click({ trial: true })
    }
    await rowActions.getByRole('button', { name: '移除配置', exact: true }).click()
    await expect(dialog).toBeVisible()
    await expect(dialog).toHaveCSS('opacity', '1')
    const bounds = await dialog.boundingBox()
    expect(bounds.x).toBeGreaterThanOrEqual(0)
    expect(bounds.x + bounds.width).toBeLessThanOrEqual(width)
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy()
    await expect(dialog.getByRole('button', { name: '移除配置', exact: true })).toBeInViewport()
    const image = width === 1440 ? 'generator-remove-desktop.png' : width === 390 ? 'generator-remove-mobile.png' : 'generator-remove-320.png'
    await page.screenshot({ path: fileURLToPath(new URL(image, artifacts)), animations: 'disabled' })
    await dialog.getByRole('button', { name: '取消', exact: true }).click()
    await expect(dialog).toHaveCount(0)
  }
  console.log('PASS: 1440/390/320px 行操作均可点击，确认框和按钮位于视口内')
  await page.setViewportSize({ width: 1440, height: 1080 })
  await page.getByRole('button', { name: '移除配置', exact: true }).click()
  await dialog.getByRole('button', { name: '移除配置', exact: true }).click()
  await expect(page.locator('tr.data-row')).toHaveCount(10)
  await expect(page.locator('.pg-btn[aria-current="page"]')).toHaveText('1')
  expect((await listed()).length).toBe(10)
  expect(sql(`SELECT name FROM ${name} WHERE record_id=7`)).toBe('keep')
  await expect(dialog).toHaveCount(0)
  await page.getByRole('button', { name: '导入数据表', exact: true }).click()
  const importer = page.getByRole('dialog', { name: '导入数据表' })
  const candidate = importer.locator('.import-row').filter({ has: page.getByText(name, { exact: true }) })
  await expect(candidate).not.toHaveClass(/is-disabled/)
  await candidate.click()
  await importer.getByRole('button', { name: '导入', exact: true }).click()
  await expect(importer).toHaveCount(0)
  const replacement = (await listed()).find(row => row.tableName === name)
  expect(replacement.id).not.toBe(target.id)
  const preview = await json(await api.get(`tool/generator/tables/${replacement.id}/preview`, { headers }))
  expect(preview.files).toHaveLength(10)
  console.log('PASS: 末页回退、候选刷新、新 ID 重导、业务数据保留、桌面与窄屏确认可用')

  await lastConfiguration(page)
  const beforePending = deletes
  await page.getByRole('button', { name: '移除配置', exact: true }).click()
  await expect(dialog).toBeVisible()
  await page.route('**/api/v1/health', route => route.fulfill({ status: 401, json: { code: 401, message: '登录已过期', data: null } }))
  const expiredCaptcha = page.waitForResponse(response => response.url().endsWith('/auth/captcha')).then(response => response.json())
  await page.evaluate(async () => {
    const source = performance.getEntriesByType('resource').find(entry => entry.name.includes('/src/api/http.ts'))
    if (!source) throw new Error('此会话回归需要 Vite 开发服务的实际 HTTP 模块')
    const { httpClient } = await import(source.name)
    await httpClient.get('/health').catch(() => {})
  })
  await page.waitForURL('**/login**')
  await expect(dialog).toHaveCount(0)
  await page.unroute('**/api/v1/health')
  await signIn(page, otherUser.username, otherPassword, (await expiredCaptcha).data)
  await expect(page.getByText('页面不存在或没有访问权限', { exact: true })).toBeVisible()
  await page.getByRole('button', { name: '返回首页', exact: true }).click()
  await expect(page.getByText('暂未分配可用菜单', { exact: true })).toBeVisible()
  expect(deletes).toBe(beforePending)
  expect((await listed()).length).toBe(11)
  console.log('PASS: 待决移除确认遇401取消，换号后不发送旧DELETE')

  await signIn(page, 'admin', credential, await signOut(page))
  await lastConfiguration(page)
  const beforeStorage = deletes
  await page.getByRole('button', { name: '移除配置', exact: true }).click()
  await expect(dialog).toBeVisible()
  const second = await context.newPage()
  second.on('pageerror', error => errors.push(error.message))
  try {
    await second.goto(`${base}/profile`)
    const nextCaptcha = await signOut(second)
    await page.waitForURL('**/login**')
    await expect(dialog).toHaveCount(0)
    await signIn(second, otherUser.username, otherPassword, nextCaptcha)
    await expect(page.getByText('暂未分配可用菜单', { exact: true })).toBeVisible()
    expect(deletes).toBe(beforeStorage)
    expect((await listed()).length).toBe(11)
  } finally {
    await second.close()
  }
  console.log('PASS: 另一标签页更换记住登录的账号后取消旧确认，不发送跨账号DELETE')

  for (const stage of ['delete', 'refresh']) for (const failed of [false, true]) {
    await signIn(page, 'admin', credential, await signOut(page))
    const old = await lastConfiguration(page)
    const removePath = `/api/v1/tool/generator/tables/${old.id}`
    const match = url => stage === 'delete' ? url.pathname === removePath
      : ['/api/v1/tool/generator/tables', '/api/v1/tool/generator/importable-tables'].includes(url.pathname)
    let release
    const held = new Promise(resolve => { release = resolve })
    let started = 0
    let delivered = 0
    const expected = stage === 'delete' ? 1 : 2
    const lateMessage = `旧${stage === 'delete' ? '删除' : '刷新'}失败`
    const handler = async route => {
      const response = stage === 'delete' && failed ? null : await route.fetch()
      started += 1
      await held
      if (failed) await route.fulfill({ status: 500, json: { code: 500, message: lateMessage, data: null } })
      else await route.fulfill({ response })
      delivered += 1
    }
    await page.route(match, handler)
    try {
      await page.getByRole('button', { name: '移除配置', exact: true }).click()
      await dialog.getByRole('button', { name: '移除配置', exact: true }).click()
      await expect.poll(() => started).toBe(expected)
      await expect(dialog).toHaveCount(0)
      await signIn(page, otherUser.username, otherPassword, await signOut(page))
      await expect(page.getByText('暂未分配可用菜单', { exact: true })).toBeVisible()
      const beforeLateResponse = generatorRequests.length
      release()
      await expect.poll(() => delivered).toBe(expected)
      await page.waitForLoadState('networkidle')
      expect(generatorRequests.length).toBe(beforeLateResponse)
      await expect(page.getByText(lateMessage, { exact: true })).toHaveCount(0)
      if (stage === 'delete') await expect(page.getByText(`已移除「${old.tableName}」的生成配置`, { exact: true })).toHaveCount(0)
      expect(sql(`SELECT name FROM ${old.tableName} WHERE record_id=7`)).toBe('keep')
      console.log(`PASS: ${stage === 'delete' ? 'DELETE' : '末页列表及候选刷新'}迟到${failed ? '失败' : '成功'}不影响新会话、不继续刷新或提示`)
    } finally {
      release()
      await page.unroute(match, handler)
    }
    if (!(await listed()).some(row => row.id === old.id)) await json(await api.post('tool/generator/import', { headers, data: { tableNames: [old.tableName] } }))
    expect((await listed()).length).toBe(11)
  }
  await signIn(page, 'admin', credential, await signOut(page))
  const previous = await lastConfiguration(page)
  const previousUrl = `${base}/api/v1/tool/generator/tables/${previous.id}`
  let releaseDelete
  let deleteStarted = false
  let deleteDelivered = false
  const oldDelete = new Promise(resolve => { releaseDelete = resolve })
  await page.route(previousUrl, async route => {
    const response = await route.fetch()
    deleteStarted = true
    await oldDelete
    await route.fulfill({ response })
    deleteDelivered = true
  })
  await page.getByRole('button', { name: '移除配置', exact: true }).click()
  await dialog.getByRole('button', { name: '移除配置', exact: true }).click()
  await expect.poll(() => deleteStarted).toBe(true)
  const switcher = await context.newPage()
  let releaseIdentity
  let identityStarted = false
  const identity = new Promise(resolve => { releaseIdentity = resolve })
  const identityPath = `${base}/api/v1/auth/me`
  const switchedRequests = []
  const changedCaptcha = await json(await api.get('auth/captcha'))
  const replacementSession = await json(await api.post('auth/login', { data: {
    username: generatorUser.username, 'password': otherPassword,
    ...(changedCaptcha.enabled ? { captcha: solve(changedCaptcha.image), captchaUuid: changedCaptcha.uuid } : {}),
  } }))
  page.on('request', req => {
    if (req.headers().authorization === `Bearer ${replacementSession.token}` && new URL(req.url()).pathname.startsWith('/api/v1/tool/generator/')) switchedRequests.push(req.url())
  })
  await page.route(identityPath, async route => {
    const response = await route.fetch()
    identityStarted = true
    await identity
    await route.fulfill({ response })
  })
  try {
    await switcher.goto(`${base}/profile`)
    await switcher.waitForLoadState('networkidle')
    await switcher.evaluate(async ({ login, username }) => {
      const source = performance.getEntriesByType('resource').find(entry => new URL(entry.name).pathname === '/src/api/session.ts')
      if (!source) throw new Error('未找到实际会话存储模块')
      const { authSession } = await import(source.name)
      authSession.save(login, { username, nickname: username }, true)
    }, { login: replacementSession, username: generatorUser.username })
    await expect.poll(() => identityStarted).toBe(true)
    expect(switchedRequests).toEqual([])
    releaseIdentity()
    await expect(page.locator('.user-chip .user-name')).toHaveText('生成器会话回归')
    await expect(page.getByPlaceholder('表名 / 表说明', { exact: true })).toHaveValue('')
    await expect(page.locator('tr.data-row').getByRole('button', { name: '移除配置', exact: true }).first()).toBeEnabled()
    expect(new URL(page.url()).pathname).toBe('/tool/generator')
    await expect(page.locator('table')).toHaveAttribute('aria-busy', 'false')
    const newRequests = generatorRequests.length
    releaseDelete()
    await expect.poll(() => deleteDelivered).toBe(true)
    await page.waitForLoadState('networkidle')
    expect(generatorRequests.length).toBe(newRequests)
    await expect(page.getByText(`已移除「${previous.tableName}」的生成配置`, { exact: true })).toHaveCount(0)
    await page.locator('tr.data-row').getByRole('button', { name: '移除配置', exact: true }).first().click()
    await expect(dialog).toBeVisible()
    await dialog.getByRole('button', { name: '取消', exact: true }).click()
    console.log('PASS: 同路径换号先验证身份再重建页面，旧删除不遗留忙碌状态或污染新会话')
  } finally {
    releaseIdentity()
    releaseDelete()
    await page.unroute(identityPath)
    await page.unroute(previousUrl)
    await switcher.close()
  }
  expect(errors).toEqual([])
} finally {
  await browser?.close()
  try {
    if (headers) for (const row of await listed()) await json(await api.delete(`tool/generator/tables/${row.id}`, { headers }))
  } finally {
    try {
      for (const table of tables) sql(`DROP TABLE IF EXISTS ${table};`)
    } finally {
      try {
        if (otherUser) await json(await api.delete(`system/users/${otherUser.id}`, { headers }))
        if (generatorUser) await json(await api.delete(`system/users/${generatorUser.id}`, { headers }))
        if (generatorRole) await json(await api.delete(`system/roles/${generatorRole.id}`, { headers }))
      } finally {
        await api.dispose()
      }
    }
  }
}
