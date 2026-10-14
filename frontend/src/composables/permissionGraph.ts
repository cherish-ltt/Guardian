import { shallowRef, watch } from 'vue'
import { systemApi } from '@/api'
import { sessionVersion } from '@/composables/auth'
import { indexPermissionGraph, type PermissionIndex } from '@/utils/permissionGraph'

/* 权限关系图：溯源视图、菜单星图、问答规则共用一份；账号切换或写操作后 invalidate 重新取。 */
export const permissionIndex = shallowRef<PermissionIndex | null>(null)
let pending: Promise<PermissionIndex> | null = null

export function loadPermissionGraph(force = false) {
  if (!force && permissionIndex.value) return Promise.resolve(permissionIndex.value)
  if (!force && pending) return pending
  const version = sessionVersion.value
  pending = systemApi.permissions.graph()
    .then((graph) => {
      const index = indexPermissionGraph(graph)
      if (version === sessionVersion.value) permissionIndex.value = index
      return index
    })
    .finally(() => { pending = null })
  return pending
}

export function invalidatePermissionGraph() {
  permissionIndex.value = null
}

watch(sessionVersion, invalidatePermissionGraph)
