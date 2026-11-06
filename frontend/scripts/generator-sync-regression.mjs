import { randomBytes } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { access, mkdir } from 'node:fs/promises'
import { DatabaseSync } from 'node:sqlite'
import { fileURLToPath } from 'node:url'

const { chromium, expect, request } = await import(process.env.GUARDIAN_PLAYWRIGHT_MODULE || '@playwright/test')
const base = (process.env.GUARDIAN_TEST_URL || 'http://127.0.0.1:5173').replace(/\/$/, '')
const password = process.env.GUARDIAN_TEST_PASSWORD
const databasePath = process.env.GUARDIAN_TEST_DB
if (!password || !databasePath) throw new Error('请指定 GUARDIAN_TEST_PASSWORD 和已启动后端使用的独立 SQLite 测试库 GUARDIAN_TEST_DB')
await access(databasePath)
const database = new DatabaseSync(databasePath)
database.exec('PRAGMA busy_timeout = 5000')
const artifacts = new URL('../.test-artifacts/', import.meta.url)
await mkdir(artifacts, { recursive: true })
const api = await request.newContext({ baseURL: `${base}/api/v1/` })
const table = `sync_ui_${randomBytes(6).toString('hex')}`
const tokens = new Set()
const errors = []
let browser
let headers
let tableId

function solveCaptcha(image) {
  return execFileSync('python3', [fileURLToPath(new URL('./captcha_solver.py', import.meta.url))], { input: image, encoding: 'utf8' }).trim()
}

async function signIn(page, captcha) {
  if (!captcha) {
    const captchaReady = page.waitForResponse(response => response.url().endsWith('/auth/captcha'))
    await page.goto(`${base}/login?redirect=/tool/generator`)
    captcha = (await (await captchaReady).json()).data
  }
  await page.getByPlaceholder('请输入账号', { exact: true }).fill('admin')
  await page.getByPlaceholder('请输入密码', { exact: true }).fill(password)
  if (captcha.enabled) await page.getByPlaceholder('请输入右侧字符', { exact: true }).fill(solveCaptcha(captcha.image))
  await page.getByRole('button', { name: '登录', exact: true }).click()
  await page.waitForURL('**/tool/generator')
  await expect(page.getByRole('heading', { name: '代码生成', exact: true })).toBeVisible()
  tokens.add(await page.evaluate(() => localStorage.getItem('guardian.token') || sessionStorage.getItem('guardian.token')))
  await page.getByPlaceholder('表名 / 表说明', { exact: true }).fill(table)
  await page.getByRole('button', { name: '查询', exact: true }).click()
  await expect(page.locator('tr.data-row')).toHaveCount(1)
}

function configuration() {
  return {
    table: database.prepare('SELECT * FROM gen_table WHERE id = ?').get(tableId),
    columns: database.prepare('SELECT * FROM gen_table_column WHERE table_id = ? ORDER BY id').all(tableId),
  }
}

async function open(page) {
  await page.locator('tr.data-row').getByRole('button', { name: '同步结构', exact: true }).click()
  const dialog = page.getByRole('dialog', { name: '同步表结构', exact: true })
  await expect(dialog).toBeVisible()
  return dialog
}

async function hold(page, url, fetchFirst) {
  let release
  let entered
  let delivered
  let count = 0
  const waiting = new Promise(resolve => { release = resolve })
  const started = new Promise(resolve => { entered = resolve })
  const done = new Promise(resolve => { delivered = resolve })
  await page.route(url, async route => {
    count++
    if (count > 1) return route.continue()
    const response = fetchFirst ? await route.fetch() : null
    entered()
    await waiting
    if (response) await route.fulfill({ response })
    else await route.continue()
    delivered()
  })
  return { started, done, release, count: () => count }
}

try {
  const captcha = (await (await api.get('auth/captcha')).json()).data
  const login = await api.post('auth/login', { data: { username: 'admin', password, ...(captcha.enabled ? { captcha: solveCaptcha(captcha.image), captchaUuid: captcha.uuid } : {}) } })
  expect(login.status()).toBe(200)
  const token = (await login.json()).data.token
  tokens.add(token)
  headers = { Authorization: `Bearer ${token}` }
  database.exec(`CREATE TABLE "${table}" (record_id INTEGER PRIMARY KEY, name TEXT NOT NULL); INSERT INTO "${table}" VALUES (9, 'preserved business row')`)
  const imported = await api.post('tool/generator/import', { headers, data: { tableNames: [table] } })
  expect(imported.status()).toBe(200)
  tableId = (await imported.json()).data.ids[0]
  database.prepare("UPDATE gen_table SET table_comment = '同步浏览器回归' WHERE id = ?").run(tableId)
  database.prepare("UPDATE gen_table_column SET column_comment = '自定义说明', is_query = 0 WHERE table_id = ? AND column_name = 'name'").run(tableId)
  database.exec(`ALTER TABLE "${table}" ADD COLUMN note TEXT`)
  const before = configuration()
  const syncUrl = `${base}/api/v1/tool/generator/tables/${tableId}/sync`
  const previewUrl = `${syncUrl}-preview`
  browser = await chromium.launch({ headless: true, channel: process.env.GUARDIAN_BROWSER_CHANNEL })
  const page = await browser.newPage({ viewport: { width: 1440, height: 1080 }, reducedMotion: 'reduce' })
  page.on('pageerror', error => errors.push(error.message))
  await signIn(page)
  let dialog = await open(page)
  await expect(dialog.locator('.sync-change')).toHaveCount(1)
  await expect(dialog.locator('.sync-change')).toContainText('note')
  for (const width of [1440, 390, 320]) {
    await page.setViewportSize({ width, height: width > 1000 ? 1080 : 844 })
    const bounds = await dialog.boundingBox()
    expect(bounds.x).toBeGreaterThanOrEqual(0)
    expect(bounds.x + bounds.width).toBeLessThanOrEqual(width)
    expect(await dialog.evaluate(element => element.scrollWidth <= element.clientWidth)).toBeTruthy()
    await page.screenshot({ path: fileURLToPath(new URL(`generator-sync-${width}.png`, artifacts)), fullPage: true, animations: 'disabled' })
  }
  await page.setViewportSize({ width: 1440, height: 1080 })
  await dialog.getByRole('button', { name: '取消', exact: true }).click()
  await expect(dialog).toHaveCount(0)
  expect(configuration()).toEqual(before)
  console.log('PASS: 只读预览、取消保留配置、1440/390/320px 弹窗边界和截图')

  dialog = await open(page)
  await expect(dialog.locator('.sync-change')).toHaveCount(1)
  database.exec(`ALTER TABLE "${table}" ADD COLUMN later INTEGER`)
  const conflicted = page.waitForResponse(syncUrl)
  await dialog.getByRole('button', { name: '确认同步', exact: true }).click()
  expect((await conflicted).status()).toBe(409)
  await expect(dialog.getByRole('alert')).toContainText('重新预览')
  await expect(dialog.getByRole('button', { name: '确认同步', exact: true })).toBeDisabled()
  expect(configuration()).toEqual(before)
  await dialog.getByRole('button', { name: '重新预览', exact: true }).click()
  await expect(dialog.locator('.sync-change')).toHaveCount(2)
  const pending = await hold(page, syncUrl, false)
  const synchronized = page.waitForResponse(syncUrl)
  await dialog.getByRole('button', { name: '确认同步', exact: true }).click()
  await pending.started
  await expect(dialog.getByRole('button', { name: '确认同步', exact: true })).toBeDisabled()
  await dialog.locator('form').evaluate(form => { form.requestSubmit(); form.requestSubmit() })
  await dialog.getByRole('button', { name: '取消', exact: true }).click()
  await expect(dialog).toBeVisible()
  expect(pending.count()).toBe(1)
  pending.release()
  expect((await synchronized).status()).toBe(200)
  await pending.done
  await expect(dialog).toHaveCount(0)
  await expect(page.locator('tr.data-row')).toContainText('待确认')
  await page.unroute(syncUrl)
  const after = configuration()
  expect(after.table.id).toBe(before.table.id)
  expect(after.table.table_comment).toBe(before.table.table_comment)
  expect(after.columns.slice(0, 2)).toEqual(before.columns)
  expect(after.columns.map(column => column.column_name)).toEqual(['record_id', 'name', 'note', 'later'])
  expect(database.prepare(`SELECT * FROM "${table}"`).all()).toEqual([{ record_id: 9, name: 'preserved business row', note: null, later: null }])
  dialog = await open(page)
  await expect(dialog.getByRole('status')).toHaveText('表结构没有变化，无需同步。')
  await expect(dialog.getByRole('button', { name: '确认同步', exact: true })).toBeDisabled()
  await dialog.getByRole('button', { name: '取消', exact: true }).click()
  console.log('PASS: 真实409、重新预览、慢确认防重复和关闭、配置身份/自定义保留、无差异禁止确认')

  const previewed = page.waitForResponse(`${base}/api/v1/tool/generator/tables/${tableId}/preview`)
  await page.locator('tr.data-row').getByRole('button', { name: '预览', exact: true }).click()
  const files = (await (await previewed).json()).data.files
  const pipeline = page.getByRole('dialog', { name: '代码生成流水线', exact: true })
  await pipeline.locator('.preview-file').first().click()
  await expect(pipeline.locator('.code-block')).toContainText('pub note: Option<String>')
  const downloaded = page.waitForEvent('download')
  await pipeline.getByRole('button', { name: '生成并下载', exact: true }).click()
  const zip = await (await downloaded).path()
  const contents = JSON.parse(execFileSync('python3', ['-c', 'import json,sys,zipfile; z=zipfile.ZipFile(sys.argv[1]); print(json.dumps({n:z.read(n).decode() for n in z.namelist()}))', zip], { encoding: 'utf8' }))
  expect(contents).toEqual(Object.fromEntries(files.map(file => [file.path, file.code])))
  await pipeline.getByRole('button', { name: '关闭', exact: true }).click()
  console.log('PASS: 同步后的实际页面代码预览及下载ZIP全部文件一致')

  database.exec(`ALTER TABLE "${table}" ADD COLUMN retry_note TEXT`)
  dialog = await open(page)
  await expect(dialog.locator('.sync-change')).toHaveCount(1)
  await page.route(syncUrl, route => route.fulfill({ status: 500, json: { code: 500, message: '模拟同步失败', data: null } }))
  await dialog.getByRole('button', { name: '确认同步', exact: true }).click()
  await expect(dialog.getByRole('alert')).toContainText('模拟同步失败')
  await expect(dialog.getByRole('button', { name: '确认同步', exact: true })).toBeDisabled()
  expect(configuration()).toEqual(after)
  await dialog.getByRole('button', { name: '取消', exact: true }).click()
  await page.unroute(syncUrl)

  const oldPreview = await hold(page, previewUrl, true)
  dialog = await open(page)
  await oldPreview.started
  await dialog.getByRole('button', { name: '取消', exact: true }).click()
  database.exec(`ALTER TABLE "${table}" ADD COLUMN newest_note TEXT`)
  dialog = await open(page)
  await expect(dialog.locator('.sync-change')).toHaveCount(2)
  oldPreview.release()
  await oldPreview.done
  await expect(dialog.locator('.sync-change')).toHaveCount(2)
  await expect(dialog).toContainText('newest_note')
  await dialog.getByRole('button', { name: '取消', exact: true }).click()
  await page.unroute(previewUrl)
  console.log('PASS: 模拟500要求重新预览、取消后旧预览晚到不覆盖新差异')

  const previousSession = await hold(page, previewUrl, true)
  dialog = await open(page)
  await previousSession.started
  const healthUrl = `${base}/api/v1/health`
  await page.route(healthUrl, route => route.fulfill({ status: 401, json: { code: 401, message: '登录已过期', data: null } }))
  const nextCaptcha = page.waitForResponse(response => response.url().endsWith('/auth/captcha'))
  const status = await page.evaluate(async () => {
    const source = performance.getEntriesByType('resource').find(entry => entry.name.includes('/src/api/http.ts'))
    const { httpClient } = await import(source.name)
    return httpClient.get('/health').catch(error => error.status)
  })
  expect(status).toBe(401)
  await page.waitForURL('**/login**')
  await expect(dialog).toHaveCount(0)
  await page.unroute(healthUrl)
  database.exec(`ALTER TABLE "${table}" ADD COLUMN session_note TEXT`)
  await signIn(page, (await (await nextCaptcha).json()).data)
  dialog = await open(page)
  await expect(dialog.locator('.sync-change')).toHaveCount(3)
  previousSession.release()
  await previousSession.done
  await expect(dialog.locator('.sync-change')).toHaveCount(3)
  await expect(dialog).toContainText('session_note')
  await dialog.getByRole('button', { name: '取消', exact: true }).click()
  await page.unroute(previewUrl)
  console.log('PASS: 401关闭预览、重新登录后旧会话预览不污染新差异')
  expect(errors).toEqual([])
} finally {
  await browser?.close()
  for (const token of tokens) {
    const loggedOut = await api.post('auth/logout', { headers: { Authorization: `Bearer ${token}` } })
    expect([200, 401]).toContain(loggedOut.status())
  }
  database.prepare('DELETE FROM gen_table_column WHERE table_id IN (SELECT id FROM gen_table WHERE table_name = ?)').run(table)
  database.prepare('DELETE FROM gen_table WHERE table_name = ?').run(table)
  database.exec(`DROP TABLE IF EXISTS "${table}"`)
  database.close()
  await api.dispose()
}
