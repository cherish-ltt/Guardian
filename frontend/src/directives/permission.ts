import { ref, type App, type Directive } from 'vue'

/* 按钮级权限。权限码来自 /api/auth/me 的 permissions；'*' 表示全部。
   permissions 是响应式的，PermissionButton 与 v-permission 都会随登录态更新。 */
export const permissions = ref<Set<string>>(new Set())

export function setPermissions(perms: string[]) {
  permissions.value = new Set(perms)
}

export function hasPermission(permission?: string | string[]): boolean {
  if (!permission || (Array.isArray(permission) && permission.length === 0)) return true
  const set = permissions.value
  if (set.has('*')) return true
  const list = Array.isArray(permission) ? permission : [permission]
  return list.some(item => set.has(item))
}

function apply(el: HTMLElement, value?: string | string[]) {
  const allowed = hasPermission(value)
  el.style.display = allowed ? (el.dataset.permissionDisplay || '') : 'none'
  el.setAttribute('aria-hidden', String(!allowed))
}

const permissionDirective: Directive<HTMLElement, string | string[]> = {
  mounted(el, binding) {
    el.dataset.permissionDisplay = el.style.display || ''
    apply(el, binding.value)
  },
  updated(el, binding) {
    apply(el, binding.value)
  },
}

export function installPermissionDirective(app: App) {
  app.directive('permission', permissionDirective)
}
