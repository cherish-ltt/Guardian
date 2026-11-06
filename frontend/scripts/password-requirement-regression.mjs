import { randomBytes } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { mkdir } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'

const { chromium, expect, request } = await import(process.env.GUARDIAN_PLAYWRIGHT_MODULE || '@playwright/test')
const base = (process.env.GUARDIAN_TEST_URL || 'http://127.0.0.1:5173').replace(/\/$/, '')
const adminPassword = process.env.GUARDIAN_TEST_PASSWORD
if (!adminPassword) throw new Error('请用 GUARDIAN_TEST_PASSWORD 指定测试管理员口令')
const artifacts = new URL('../.test-artifacts/', import.meta.url)
await mkdir(artifacts, { recursive: true })
const api = await request.newContext({ baseURL: `${base}/api/v1/` })
const users = []
const contexts = []
const errors = []
const initialPassword = randomBytes(24).toString('hex')
const newPassword = randomBytes(24).toString('hex')
const resetPassword = randomBytes(24).toString('hex')
const requiredPath = '/password-change'
let browser
let headers

function solveCaptcha(image) {
  return execFileSync('python3', [fileURLToPath(new URL('./captcha_solver.py', import.meta.url))], { input: image, encoding: 'utf8' }).trim()
}
async function apiLogin(username, password) {
  const captcha = (await (await api.get('auth/captcha')).json()).data
  const response = await api.post('auth/login', { data: { username, password, ...(captcha.enabled ? { captcha: solveCaptcha(captcha.image), captchaUuid: captcha.uuid } : {}) } })
  expect(response.status()).toBe(200)
  return (await response.json()).data
}
async function newPage(context) {
  const page = await context.newPage()
  page.on('pageerror', error => errors.push(error.message))
  return page
}
async function signIn(page, username, password, redirect = '/', navigate = true) {
  if (navigate) await page.goto(`${base}/login?redirect=${encodeURIComponent(redirect)}`)
  await page.getByPlaceholder('请输入账号', { exact: true }).fill(username)
  await page.getByPlaceholder('请输入密码', { exact: true }).fill(password)
  await expect(page.locator('.captcha-box[disabled]')).toHaveCount(0)
  if (await page.getByPlaceholder('请输入右侧字符', { exact: true }).isVisible()) {
    await expect(page.locator('.captcha-image')).toBeVisible()
    const image = await page.locator('.captcha-image').getAttribute('src')
    await page.getByPlaceholder('请输入右侧字符', { exact: true }).fill(solveCaptcha(image))
  }
  const submitted = page.waitForResponse(response => response.url().endsWith('/auth/login'))
  await page.getByRole('button', { name: '登录', exact: true }).click()
  const response = await submitted
  expect(response.status()).toBe(200)
  const result = (await response.json()).data
  expect(result.token).toBeTruthy()
  await page.waitForURL(url => url.pathname !== '/login')
  return result
}
async function expectRestricted(page) {
  await expect(page).toHaveURL(`${base}${requiredPath}`)
  await expect(page.getByRole('heading', { name: '请先修改密码', exact: true })).toBeVisible()
  await expect(page.locator('.nav-item, .user-chip')).toHaveCount(0)
}
async function fillPassword(page, oldPassword, nextPassword) {
  const dialog = page.getByRole('dialog', { name: '修改密码', exact: true })
  await dialog.locator('input[autocomplete="current-password"] ').fill(oldPassword)
  await dialog.locator('input[autocomplete="new-password"] ').nth(0).fill(nextPassword)
  await dialog.locator('input[autocomplete="new-password"] ').nth(1).fill(nextPassword)
  return dialog
}
async function expectRejectedChange(page, oldPassword, nextPassword, message) {
  const dialog = await fillPassword(page, oldPassword, nextPassword)
  const result = page.waitForResponse(response => response.url().endsWith('/auth/password'))
  await dialog.getByRole('button', { name: '确认修改', exact: true }).click()
  expect((await result).status()).toBe(400)
  await expect(page.getByText(message, { exact: true })).toBeVisible()
  await expectRestricted(page)
}

try {
  const admin = await apiLogin('admin', adminPassword)
  expect(admin.passwordChangeRequired).toBe(false)
  headers = { Authorization: `Bearer ${admin.token}` }
  for (const required of [true, false]) {
    const username = `password_${required ? 'required' : 'normal'}_${Date.now()}`
    const created = await api.post('system/users', { headers, data: { username, nickname: `改密回归 ${required ? 'A' : 'B'}`, password: initialPassword, status: 1, deptId: 5, roleIds: [], passwordChangeRequired: required } })
    expect(created.status()).toBe(200)
    users.push({ id: (await created.json()).data.id, username })
  }
  browser = await chromium.launch({ headless: true, channel: process.env.GUARDIAN_BROWSER_CHANNEL })
  const context = await browser.newContext({ viewport: { width: 1440, height: 1080 }, reducedMotion: 'reduce' })
  contexts.push(context)
  const page = await newPage(context)
  const requests = []
  page.on('request', req => { if (req.url().includes('/api/v1/')) requests.push(new URL(req.url()).pathname) })
  const first = await signIn(page, users[0].username, initialPassword, '/system/user')
  expect(first.passwordChangeRequired).toBe(true)
  await expectRestricted(page)
  for (const path of ['/profile', '/system/user', requiredPath]) {
    await page.goto(`${base}${path}`)
    await expectRestricted(page)
  }
  expect(requests.some(path => path === '/api/v1/auth/routes' || path.startsWith('/api/v1/system/users') || path === '/api/v1/auth/profile')).toBe(false)
  const secondTab = await newPage(context)
  await secondTab.goto(`${base}/profile`)
  await expectRestricted(secondTab)
  expect((await api.get('auth/routes', { headers: { Authorization: `Bearer ${first.token}` } })).status()).toBe(403)
  expect((await api.get('transactions', { headers: { Authorization: `Bearer ${first.token}` } })).status()).toBe(403)
  console.log('PASS: 无菜单账号登录、业务深链及刷新均进入独立改密页，不加载菜单；后端真实403')

  for (const size of ['desktop', 'mobile']) for (const theme of ['light', 'dark']) {
    await page.setViewportSize(size === 'desktop' ? { width: 1440, height: 1080 } : { width: 390, height: 844 })
    await page.evaluate(value => localStorage.setItem('guardian.appearance', JSON.stringify({ theme: value, density: 'regular', railCollapsed: false })), theme)
    await page.reload()
    await expectRestricted(page)
    await expect(page.locator('html')).toHaveAttribute('data-theme', theme)
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy()
    await page.screenshot({ path: fileURLToPath(new URL(`password-required-${size}-${theme}.png`, artifacts)), fullPage: true, animations: 'disabled' })
    const changeDialog = page.getByRole('dialog', { name: '修改密码', exact: true })
    await changeDialog.getByRole('button', { name: '取消', exact: true }).click()
    await expect(changeDialog).toBeHidden()
    await expectRestricted(page)
    await page.screenshot({ path: fileURLToPath(new URL(`password-required-${size}-${theme}-page.png`, artifacts)), fullPage: true, animations: 'disabled' })
    await page.getByRole('button', { name: '修改密码', exact: true }).click()
    await expect(changeDialog).toBeVisible()
  }
  await page.setViewportSize({ width: 1440, height: 1080 })
  await expectRejectedChange(page, 'wrong-current-password', newPassword, '旧密码不正确')
  await expectRejectedChange(page, initialPassword, initialPassword, '新密码不能与旧密码一致')
  let dialog = await fillPassword(page, initialPassword, '12345')
  await dialog.getByRole('button', { name: '确认修改', exact: true }).click()
  await expect(dialog.getByText('密码长度必须为 6 到 64 个字符', { exact: true })).toBeVisible()
  console.log('PASS: 桌面/移动浅深主题无溢出，错误旧密码、相同密码及短密码保持受限')

  let releaseChange
  let changeStarted
  const held = new Promise(resolve => { releaseChange = resolve })
  const entered = new Promise(resolve => { changeStarted = resolve })
  let submissions = 0
  const changeUrl = `${base}/api/v1/auth/password`
  await page.route(changeUrl, async route => { submissions += 1; changeStarted(); await held; await route.continue() })
  dialog = await fillPassword(page, initialPassword, newPassword)
  await dialog.getByRole('button', { name: '确认修改', exact: true }).click()
  await entered
  await expect(dialog.getByRole('button', { name: '确认修改', exact: true })).toBeDisabled()
  await dialog.locator('form').evaluate(form => { form.requestSubmit(); form.requestSubmit() })
  expect(submissions).toBe(1)
  const changeDone = page.waitForResponse(response => response.url() === changeUrl)
  releaseChange()
  const changeResponse = await changeDone
  expect(changeResponse.status()).toBe(200)
  try {
    await page.waitForURL('**/login**')
  } catch (error) {
    console.log('Change navigation state:', await page.evaluate(async () => {
      const auth = await import('/src/composables/auth.ts')
      const { authSession } = await import('/src/api/session.ts')
      return { path: location.pathname, authed: auth.authed.value, version: auth.sessionVersion.value,
        tokenPresent: Boolean(authSession.token()), passwordChangeRequired: auth.currentUser.value?.passwordChangeRequired,
        dialogs: [...document.querySelectorAll('[role="dialog"]')].map(el => el.textContent),
        toasts: [...document.querySelectorAll('.toast')].map(el => el.textContent) }
    }))
    console.log('Page errors:', errors)
    throw error
  }
  await secondTab.waitForURL('**/login**')
  expect(submissions).toBe(1)
  await page.unroute(changeUrl)
  for (const path of ['auth/me', 'transactions']) expect((await api.get(path, { headers: { Authorization: `Bearer ${first.token}` } })).status()).toBe(401)
  const changed = await signIn(page, users[0].username, newPassword)
  expect(changed.passwordChangeRequired).toBe(false)
  await expect(page.getByText('暂未分配可用菜单', { exact: true })).toBeVisible()
  await expect(secondTab).toHaveURL(`${base}/no-access`)
  console.log('PASS: 慢改密只提交一次，成功使旧令牌401、共享标签页退出，新密码进入无菜单提示')

  const adminContext = await browser.newContext({ viewport: { width: 1440, height: 1080 }, reducedMotion: 'reduce' })
  contexts.push(adminContext)
  const adminPage = await newPage(adminContext)
  await signIn(adminPage, 'admin', adminPassword, `/system/user?keyword=${users[0].username}`)
  await adminPage.getByRole('button', { name: '新增用户', exact: true }).click()
  const createDialog = adminPage.getByRole('dialog', { name: '新增用户', exact: true })
  const createSwitch = createDialog.getByRole('switch', { name: '要求用户登录后先修改密码', exact: true })
  await expect(createSwitch).not.toBeChecked()
  await createDialog.getByText('要求用户登录后先修改密码', { exact: true }).click()
  await expect(createSwitch).toBeChecked()
  await createDialog.getByRole('button', { name: '取消', exact: true }).click()
  await expect(createDialog).toBeHidden()
  await adminPage.getByRole('button', { name: '新增用户', exact: true }).click()
  await expect(createSwitch).not.toBeChecked()
  await createDialog.getByRole('button', { name: '取消', exact: true }).click()
  const row = adminPage.locator('tr.data-row').filter({ hasText: users[0].username })
  await row.getByRole('button', { name: '重置密码', exact: true }).click()
  const resetDialog = adminPage.getByRole('dialog', { name: '重置密码', exact: true })
  const resetSwitch = resetDialog.getByRole('switch', { name: '要求用户登录后先修改密码', exact: true })
  await expect(resetSwitch).not.toBeChecked()
  await resetDialog.locator('input[autocomplete="new-password"] ').fill(resetPassword)
  await resetDialog.getByText('要求用户登录后先修改密码', { exact: true }).click()
  await expect(resetSwitch).toBeChecked()
  const resetDone = adminPage.waitForResponse(response => response.url().endsWith(`/system/users/${users[0].id}/password`))
  await resetDialog.getByRole('button', { name: '确认重置', exact: true }).click()
  const resetResponse = await resetDone
  expect(resetResponse.status()).toBe(200)
  expect(resetResponse.request().postDataJSON().passwordChangeRequired).toBe(true)
  expect((await api.get('auth/me', { headers: { Authorization: `Bearer ${changed.token}` } })).status()).toBe(401)
  await page.reload()
  await page.waitForURL('**/login**')
  const resetLogin = await signIn(page, users[0].username, resetPassword)
  expect(resetLogin.passwordChangeRequired).toBe(true)
  await expectRestricted(page)
  console.log('PASS: 新建/重置开关默认关闭，重置UI真实写入要求并撤销旧会话')

  let releaseOld
  let oldStarted
  const oldHeld = new Promise(resolve => { releaseOld = resolve })
  const oldEntered = new Promise(resolve => { oldStarted = resolve })
  const routeUrl = `${base}/api/v1/auth/routes`
  let firstRoute = true
  await page.route(routeUrl, async route => {
    if (!firstRoute) return route.continue()
    firstRoute = false
    const response = await route.fetch()
    expect(response.status()).toBe(403)
    oldStarted()
    await oldHeld
    await route.fulfill({ response })
  })
  await page.evaluate(async () => {
    const { httpClient } = await import('/src/api/http.ts')
    window.oldPasswordRequirement = httpClient.get('/auth/routes').then(() => 200, error => error.status)
  })
  await oldEntered
  await page.getByRole('dialog', { name: '修改密码', exact: true }).getByRole('button', { name: '取消', exact: true }).click()
  await page.getByRole('button', { name: '退出登录', exact: true }).click()
  await page.waitForURL('**/login**')
  await signIn(page, users[1].username, initialPassword, '/', false)
  await expect(page.getByText('暂未分配可用菜单', { exact: true })).toBeVisible()
  releaseOld()
  expect(await page.evaluate(() => window.oldPasswordRequirement)).toBe(403)
  await page.unroute(routeUrl)
  await expect(page).toHaveURL(`${base}/no-access`)
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem('guardian.user')).username)).toBe(users[1].username)
  console.log('PASS: 待改密账号的迟到403不限制随后登录的新账号')
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
