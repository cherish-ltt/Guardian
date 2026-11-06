import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router'
import { ApiError, authSession, setPasswordChangeHandler, setUnauthorizedHandler } from '@/api'
import { authed, clearSession, currentUser, dynamicRoutes, loadRoutes, navGroups, refreshUser, requirePasswordChange, sessionVersion, setSessionRouteReset, synchronizeSession } from '@/composables/auth'
import { pushToast } from '@/composables/toast'
import { APP } from '@/data'
import AppShell from '@/components/layout/AppShell.vue'

const staticRoutes: RouteRecordRaw[] = [
  { path: '/login/2fa', name: 'login-2fa', component: () => import('@/views/LoginTwoFactor.vue'), meta: { public: true, title: '二次验证' } },
  { path: '/login', name: 'login', component: () => import('@/views/Login.vue'), meta: { public: true, title: '登录' } },
  { path: '/password-change', name: 'password-change', component: () => import('@/views/PasswordChangeRequired.vue'), meta: { title: '请先修改密码' } },
  {
    path: '/',
    name: 'root',
    component: AppShell,
    children: [
      { path: 'profile', name: 'profile', component: () => import('@/views/Profile.vue'), meta: { title: '个人资料' } },
      { path: 'no-access', name: 'no-access', component: () => import('@/views/NoAccess.vue'), meta: { title: '暂未分配菜单' } },
      { path: ':pathMatch(.*)*', name: 'not-found', component: () => import('@/views/NotFound.vue'), meta: { title: '页面不存在' } },
    ],
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes: staticRoutes,
})

let routesReady = false
let routeLoad: Promise<boolean> | null = null
let cancelRouteLoad: (() => void) | null = null
const removeRoutes: Array<() => void> = []

/** 登录后的落点：有工作台权限去工作台，否则去第一个可见菜单 */
export function homePath() {
  if (currentUser.value?.passwordChangeRequired) return '/password-change'
  return navGroups.value[0]?.items[0]?.path ?? '/no-access'
}

export function resetRoutes() {
  removeRoutes.splice(0).forEach(remove => remove())
  routesReady = false
  cancelRouteLoad?.()
  cancelRouteLoad = null
  routeLoad = null
}
setSessionRouteReset(resetRoutes)

async function ensureRoutes() {
  if (routesReady) return true
  if (routeLoad) return routeLoad
  const version = sessionVersion.value
  const cancelled = new Promise<boolean>(resolve => { cancelRouteLoad = () => resolve(false) })
  const task = Promise.race([cancelled, (async () => {
    const user = await refreshUser()
    if (!user || version !== sessionVersion.value) return false
    if (user.passwordChangeRequired) {
      routesReady = true
      return true
    }
    await loadRoutes()
    if (version !== sessionVersion.value || !authed.value) return false
    dynamicRoutes.value.forEach(route => removeRoutes.push(router.addRoute('root', route)))
    routesReady = true
    return true
  })()])
  routeLoad = task
  try {
    return await task
  } finally {
    if (routeLoad === task) {
      routeLoad = null
      cancelRouteLoad = null
    }
  }
}

router.beforeEach(async (to) => {
  synchronizeSession()
  if (to.meta.public) {
    return authed.value && (to.name === 'login' || to.name === 'login-2fa') ? { path: '/' } : true
  }
  if (!authed.value) {
    return { name: 'login', query: to.fullPath !== '/' ? { redirect: to.fullPath } : undefined }
  }
  if (!routesReady) {
    const version = sessionVersion.value
    try {
      if (!await ensureRoutes()) return false
    } catch (error) {
      if (version !== sessionVersion.value) return false
      if (error instanceof ApiError && error.passwordChangeRequired) return { name: 'password-change', replace: true }
      clearSession()
      pushToast('菜单加载失败，请重新登录', 'danger')
      return { name: 'login' }
    }
    if (currentUser.value?.passwordChangeRequired) return to.name === 'password-change' || { name: 'password-change', replace: true }
    // 动态路由刚注册：按原地址重新解析，保留查询参数（深链如 ?trace=user:1）
    return to.path === '/' ? { path: homePath(), replace: true } : { path: to.path, query: to.query, hash: to.hash, replace: true }
  }
  if (currentUser.value?.passwordChangeRequired) return to.name === 'password-change' || { name: 'password-change', replace: true }
  if (to.name === 'password-change') return { path: homePath(), replace: true }
  if (to.path === '/') return { path: homePath(), replace: true }
  return true
})

router.afterEach((to) => {
  const title = to.meta.title as string | undefined
  document.title = title ? `${title} · ${APP.name}` : APP.name
})

// 任一接口返回 401：清理登录态并回到登录页
let redirecting = false
setUnauthorizedHandler(() => {
  if (redirecting || !authed.value) return
  redirecting = true
  clearSession()
  pushToast('登录已过期，请重新登录', 'info')
  router.replace({ name: 'login', query: { redirect: router.currentRoute.value.fullPath } }).finally(() => { redirecting = false })
})

setPasswordChangeHandler(() => {
  if (!authSession.isAuthenticated()) return
  requirePasswordChange()
  if (router.currentRoute.value.name !== 'password-change') void router.replace({ name: 'password-change' })
})

window.addEventListener('storage', (event) => {
  if (event.key !== null && !['guardian.token', 'guardian.user', 'guardian.expiresAt'].includes(event.key)) return
  if (synchronizeSession()) void router.replace(authed.value ? '/' : '/login')
})

export default router
