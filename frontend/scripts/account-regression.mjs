import { chromium, expect, request } from '@playwright/test'
import { createHmac, randomBytes } from 'node:crypto'

const base = process.env.GUARDIAN_TEST_URL || 'http://127.0.0.1:5173'
const password = process.env.GUARDIAN_TEST_PASSWORD || 'admin123456'
const username = `security_${Date.now()}`
const userPassword = randomBytes(20).toString('hex')
const api = await request.newContext({ baseURL: `${base}/api/v1/` })
const captcha = (await (await api.get('auth/captcha')).json()).data
const signed = await api.post('auth/login', { data: { username: 'admin', password, captcha: captcha.code, captchaUuid: captcha.uuid } })
expect(signed.ok()).toBeTruthy()
const token = (await signed.json()).data.token
const headers = { Authorization: `Bearer ${token}` }
const created = await api.post('system/users', { headers, data: { username, nickname: '安全回归', password: userPassword, status: 1, deptId: 5, roleIds: [] } })
expect(created.ok()).toBeTruthy()
const id = (await created.json()).data.id
let browser
function totp(secret) {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'
  const bits = [...secret].map(c => alphabet.indexOf(c).toString(2).padStart(5, '0')).join('')
  const key = Buffer.from(bits.match(/.{8}/g).map(b => parseInt(b, 2)))
  const counter = Buffer.alloc(8)
  counter.writeBigUInt64BE(BigInt(Math.floor(Date.now() / 30000)))
  const hash = createHmac('sha1', key).update(counter).digest()
  return ((hash.readUInt32BE(hash[19] & 15) & 0x7fffffff) % 1000000).toString().padStart(6, '0')
}
async function login(page, factor = '') {
  await page.goto(`${base}/login?redirect=/profile`)
  await page.getByPlaceholder('请输入账号', { exact: true }).fill(username)
  await page.getByPlaceholder('请输入密码', { exact: true }).fill(userPassword)
  await expect(page.locator('.captcha-code')).not.toBeEmpty()
  await page.getByPlaceholder('请输入右侧字符', { exact: true }).fill((await page.locator('.captcha-code').innerText()).replace(/\s/g, ''))
  expect(await page.locator('input[autocomplete="one-time-code"]').count()).toBe(0)
  await page.getByRole('button', { name: '登录', exact: true }).click()
  if (factor) {
    await page.waitForURL('**/login/2fa**')
    await page.getByPlaceholder('请输入动态码或恢复码').fill(factor)
    await page.getByRole('button', { name: '验证并登录', exact: true }).click()
  }
}
async function enable(page) {
  await page.getByRole('button', { name: '启用2FA', exact: true }).click()
  const dialog = page.getByRole('dialog')
  await dialog.locator('input[type="password"]').fill(userPassword)
  await dialog.getByRole('button', { name: '生成绑定信息', exact: true }).click()
  await expect(dialog.getByAltText('认证器绑定二维码')).toBeVisible()
  const secret = await dialog.locator('code').innerText()
  await dialog.locator('input[autocomplete="one-time-code"]').fill(totp(secret))
  await dialog.getByRole('button', { name: '确认', exact: true }).click()
  await expect(page.locator('.profile-codes')).toBeVisible()
  const codes = (await page.locator('.profile-codes').innerText()).trim().split('\n')
  expect(codes).toHaveLength(10)
  const storage = await page.evaluate(() => JSON.stringify([localStorage, sessionStorage]))
  expect(storage).not.toContain(secret)
  for (const code of codes) expect(storage).not.toContain(code)
  await page.getByRole('button', { name: '已保存恢复码，重新登录', exact: true }).click()
  return codes
}
try {
  browser = await chromium.launch({ headless: true })
  const page = await browser.newPage()
  const errors = []
  page.on('pageerror', e => errors.push(e.message))
  await login(page)
  await expect(page.getByRole('heading', { name: '个人资料', exact: true })).toBeVisible()
  await page.getByRole('button', { name: '编辑资料', exact: true }).click()
  const edit = page.getByRole('dialog')
  await edit.locator('input').nth(0).fill('更新后的昵称')
  await edit.locator('input[type="email"]').fill('profile@example.com')
  await edit.getByRole('button', { name: '保存', exact: true }).click()
  await expect(page.locator('.profile-details')).toContainText('更新后的昵称')
  await page.reload()
  await expect(page.locator('.profile-details')).toContainText('profile@example.com')
  let codes = await enable(page)
  await login(page)
  await page.waitForURL('**/login/2fa**')
  expect(await page.evaluate(() => localStorage.getItem('guardian.token') || sessionStorage.getItem('guardian.token'))).toBeNull()
  await page.reload()
  await expect(page.getByRole('alert')).toContainText('验证已失效')
  await login(page, codes[0])
  await expect(page.getByRole('button', { name: '关闭2FA', exact: true })).toBeVisible()
  await page.getByRole('button', { name: '重新生成恢复码', exact: true }).click()
  let dialog = page.getByRole('dialog')
  await dialog.locator('input[type="password"]').fill(userPassword)
  await dialog.locator('input[autocomplete="one-time-code"]').fill(codes[1])
  await dialog.getByRole('button', { name: '确认', exact: true }).click()
  await expect(page.locator('.profile-codes')).toBeVisible()
  const old = codes[2]
  codes = (await page.locator('.profile-codes').innerText()).trim().split('\n')
  await page.getByRole('button', { name: '已保存恢复码，重新登录', exact: true }).click()
  await login(page, old)
  await expect(page.locator('.login-error')).toContainText('动态码或恢复码无效')
  await login(page, codes[0])
  await page.getByRole('button', { name: '关闭2FA', exact: true }).click()
  dialog = page.getByRole('dialog')
  await dialog.locator('input[type="password"]').fill(userPassword)
  await dialog.locator('input[autocomplete="one-time-code"]').fill(codes[1])
  await dialog.getByRole('button', { name: '确认', exact: true }).click()
  await page.getByRole('button', { name: '重新登录', exact: true }).click()
  await login(page)
  await expect(page.getByRole('button', { name: '启用2FA', exact: true })).toBeVisible()
  await page.setViewportSize({ width: 390, height: 844 })
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy()
  expect(errors).toEqual([])
  console.log('PASS: 无菜单账号资料编辑/刷新、二维码绑定、缺少2FA拒绝、恢复码登录/更新/旧码失效、关闭及移动端布局')
} finally {
  await browser?.close()
  const removed = await api.delete(`system/users/${id}`, { headers })
  expect(removed.ok()).toBeTruthy()
  await api.dispose()
}
