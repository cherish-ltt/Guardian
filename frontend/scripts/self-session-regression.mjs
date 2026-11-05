import { randomBytes } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { mkdir } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'

const { chromium, expect, request } = await import(process.env.GUARDIAN_PLAYWRIGHT_MODULE || '@playwright/test')
const base = (process.env.GUARDIAN_TEST_URL || 'http://127.0.0.1:5173').replace(/\/$/, '')
const password = process.env.GUARDIAN_TEST_PASSWORD
if (!password) throw new Error('请用 GUARDIAN_TEST_PASSWORD 指定测试管理员口令')
const artifacts = new URL('../.test-artifacts/', import.meta.url)
await mkdir(artifacts, { recursive: true })
const api = await request.newContext({ baseURL: `${base}/api/v1/` })
const users = []
const contexts = []
const errors = []
const fixturePassword = randomBytes(24).toString('hex')
let browser
let headers

function solveCaptcha(image) {
  return execFileSync('python3', [fileURLToPath(new URL('./captcha_solver.py', import.meta.url))], { input: image, encoding: 'utf8' }).trim()
}
async function signIn(page, username, captcha) {
  const existingPage = Boolean(captcha)
  if (!captcha) {
    const captchaReady = page.waitForResponse(response => response.url().endsWith('/auth/captcha')).then(response => response.json())
    await page.goto(`${base}/login?redirect=/profile`)
    captcha = (await captchaReady).data
  }
  await page.getByPlaceholder('请输入账号', { exact: true }).fill(username)
  await page.getByPlaceholder('请输入密码', { exact: true }).fill(fixturePassword)
  if (captcha.enabled) await page.getByPlaceholder('请输入右侧字符', { exact: true }).fill(solveCaptcha(captcha.image))
  const listed = page.waitForResponse(response => response.url().endsWith('/auth/sessions'))
  await page.getByRole('button', { name: '登录', exact: true }).click()
  if (existingPage) {
    await page.locator('.user-chip').click()
    await page.getByRole('menuitem', { name: '个人资料', exact: true }).click()
  }
  const response = await listed
  expect(response.status()).toBe(200)
  await expect(page.getByRole('heading', { name: '个人资料', exact: true })).toBeVisible()
  await expect(page.locator('.nav-item')).toHaveCount(0)
  return (await response.json()).data
}
async function refresh(page) {
  const ready = page.waitForResponse(response => response.url().endsWith('/auth/sessions'))
  await page.locator('.profile-sessions').getByRole('button', { name: '刷新', exact: true }).click()
  const response = await ready
  expect(response.status()).toBe(200)
  await expect(page.locator('.profile-sessions table')).toHaveAttribute('aria-busy', 'false')
  return (await response.json()).data
}
async function snapshot(page, size, theme) {
  await page.setViewportSize(size === 'desktop' ? { width: 1440, height: 1080 } : { width: 390, height: 844 })
  await page.getByRole('button', { name: '外观设置', exact: true }).click()
  const dialog = page.getByRole('dialog', { name: '外观设置' })
  await dialog.getByRole('radio', { name: theme === 'light' ? '浅色' : '深色', exact: true }).click()
  await dialog.getByRole('button', { name: '完成', exact: true }).click()
  await expect(dialog).toHaveCount(0)
  await expect(page.locator('html')).toHaveAttribute('data-theme', theme)
  await page.locator('.profile-sessions').scrollIntoViewIfNeeded()
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy()
  await page.screenshot({ path: fileURLToPath(new URL(`sessions-${size}-${theme}.png`, artifacts)), fullPage: true, animations: 'disabled' })
}
async function openTarget(page, client) {
  await page.locator('.profile-sessions tr.data-row').filter({ hasText: client }).getByRole('button', { name: '撤销会话', exact: true }).click()
  const dialog = page.getByRole('dialog', { name: '撤销登录会话' })
  await dialog.locator('input[type="password"] ').fill(fixturePassword)
  return dialog
}

try {
  const captcha = (await (await api.get('auth/captcha')).json()).data
  const login = await api.post('auth/login', { data: { username: 'admin', password, ...(captcha.enabled ? { captcha: solveCaptcha(captcha.image), captchaUuid: captcha.uuid } : {}) } })
  expect(login.status()).toBe(200)
  headers = { Authorization: `Bearer ${(await login.json()).data.token}` }
  for (const label of ['a', 'b']) {
    const username = `sessions_${label}_${Date.now()}`
    const created = await api.post('system/users', { headers, data: { username, nickname: `会话回归 ${label.toUpperCase()}`, password: fixturePassword, status: 1, deptId: 5, roleIds: [] } })
    expect(created.status()).toBe(200)
    users.push({ id: (await created.json()).data.id, username })
  }
  browser = await chromium.launch({ headless: true, channel: process.env.GUARDIAN_BROWSER_CHANNEL })
  const agents = [
    'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:132.0) Gecko/20100101 Firefox/132.0',
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 14_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15',
    'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
  ]
  const pages = []
  const currentIds = []
  for (const [index, userAgent] of agents.entries()) {
    const context = await browser.newContext({ userAgent, viewport: { width: 1440, height: 1080 } })
    contexts.push(context)
    const page = await context.newPage()
    page.on('pageerror', error => errors.push(error.message))
    pages.push(page)
    const list = await signIn(page, users[index === 3 ? 1 : 0].username)
    expect(list.items.filter(item => item.current)).toHaveLength(1)
    currentIds.push(list.items.find(item => item.current).id)
  }
  const [a1, a2, a3, b] = pages
  const list = await refresh(a1)
  expect(list.items.map(item => item.id).sort()).toEqual(currentIds.slice(0, 3).sort())
  expect(list.items.find(item => item.current).id).toBe(currentIds[0])
  const tokens = await Promise.all(pages.map(page => page.evaluate(() => localStorage.getItem('guardian.token') || sessionStorage.getItem('guardian.token'))))
  const serialized = JSON.stringify(list)
  expect(tokens.every(token => token && !serialized.includes(token))).toBeTruthy()
  expect(list.items.every(item => !('ip' in item) && !('token' in item))).toBeTruthy()
  await expect(a1.locator('.profile-sessions tr.data-row')).toHaveCount(3)
  await expect(a1.locator('.profile-sessions').getByText('当前会话', { exact: true })).toHaveCount(1)
  await expect(a1.locator('.profile-sessions').getByRole('button', { name: '撤销会话', exact: true })).toHaveCount(2)
  expect((await refresh(b)).items.map(item => item.id)).toEqual([currentIds[3]])
  console.log('PASS: 四个独立浏览器上下文、无菜单账号访问、本人列表隔离、当前标记及不返回令牌')

  for (const size of ['desktop', 'mobile']) for (const theme of ['light', 'dark']) await snapshot(a1, size, theme)
  await a1.setViewportSize({ width: 1440, height: 1080 })
  console.log('PASS: 桌面/移动端浅深主题截图与页面无横向溢出')

  const revokeUrl = `${base}/api/v1/auth/sessions/${currentIds[1]}/revoke`
  await a1.route(revokeUrl, route => route.fulfill({ status: 500, json: { code: 500, message: '模拟会话撤销失败', data: null } }))
  let dialog = await openTarget(a1, 'Firefox')
  await expect(dialog.locator('input[autocomplete="one-time-code"]')).toHaveCount(0)
  await dialog.getByRole('button', { name: '确认撤销', exact: true }).click()
  await expect(dialog.getByRole('alert')).toHaveText('模拟会话撤销失败')
  await expect(a1.locator('.profile-sessions tr.data-row')).toHaveCount(3)
  await expect(a1.locator('.profile-sessions [role="status"]')).toHaveCount(0)
  await dialog.getByRole('button', { name: '取消', exact: true }).click()
  await a1.unroute(revokeUrl)
  expect((await refresh(a2)).items.some(item => item.current)).toBeTruthy()
  console.log('PASS: 撤销失败明确显示错误、列表不删行、目标会话保持有效')

  let release
  let started
  const held = new Promise(resolve => { release = resolve })
  const entered = new Promise(resolve => { started = resolve })
  let submissions = 0
  await a1.route(revokeUrl, async route => { submissions += 1; started(); await held; await route.continue() })
  dialog = await openTarget(a1, 'Firefox')
  await dialog.getByRole('button', { name: '确认撤销', exact: true }).click()
  await entered
  await expect(dialog.getByRole('button', { name: '确认撤销', exact: true })).toBeDisabled()
  await dialog.locator('form').evaluate(form => { form.requestSubmit(); form.requestSubmit() })
  expect(submissions).toBe(1)
  await expect(a1.locator('.profile-sessions tr.data-row')).toHaveCount(3)
  release()
  await expect(dialog).toHaveCount(0)
  await expect(a1.locator('.profile-sessions [role="status"]')).toContainText('该会话已撤销')
  await expect(a1.locator('.profile-sessions tr.data-row')).toHaveCount(2)
  expect(submissions).toBe(1)
  await a1.unroute(revokeUrl)
  const unauthorized = a2.waitForResponse(response => response.url().endsWith('/auth/sessions'))
  await a2.locator('.profile-sessions').getByRole('button', { name: '刷新', exact: true }).click()
  expect((await unauthorized).status()).toBe(401)
  await a2.waitForURL('**/login**')
  for (const page of [a1, a3, b]) {
    const active = await refresh(page)
    expect(active.items.filter(item => item.current)).toHaveLength(1)
    await expect(page).toHaveURL(/\/profile$/)
  }
  console.log('PASS: 慢撤销只提交一次、A2 后续请求401跳转登录、A1/A3/B保持登录')

  let releaseList
  let listStarted
  const heldList = new Promise(resolve => { releaseList = resolve })
  const listEntered = new Promise(resolve => { listStarted = resolve })
  let oldResponse
  const delivered = new Promise(resolve => { oldResponse = resolve })
  let first = true
  const listingUrl = `${base}/api/v1/auth/sessions`
  await a1.route(listingUrl, async route => {
    if (!first) return route.continue()
    first = false
    const response = await route.fetch()
    listStarted()
    await heldList
    await route.fulfill({ response })
    oldResponse()
  })
  await a1.locator('.profile-sessions').getByRole('button', { name: '刷新', exact: true }).click()
  await listEntered
  await a1.locator('.user-chip').click()
  await a1.getByRole('menuitem', { name: '退出登录', exact: true }).click()
  const nextCaptcha = a1.waitForResponse(response => response.url().endsWith('/auth/captcha')).then(response => response.json())
  await a1.getByRole('dialog').getByRole('button', { name: '退出', exact: true }).click()
  await a1.waitForURL('**/login**')
  await signIn(a1, users[1].username, (await nextCaptcha).data)
  releaseList()
  await delivered
  await a1.unroute(listingUrl)
  await expect(a1.locator('.profile-sessions tr.data-row')).toHaveCount(2)
  const switched = await refresh(a1)
  expect(switched.items.some(item => currentIds.slice(0, 3).includes(item.id))).toBeFalsy()
  expect(switched.items.some(item => item.id === currentIds[3])).toBeTruthy()
  console.log('PASS: 旧账号列表慢响应不写回新登录账号')
  expect(errors).toEqual([])
} finally {
  await Promise.all(contexts.map(context => context.close()))
  await browser?.close()
  for (const user of users) {
    const removed = await api.delete(`system/users/${user.id}`, { headers })
    expect(removed.status()).toBe(200)
  }
  await api.dispose()
}
