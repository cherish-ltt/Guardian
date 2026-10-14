import { ref } from 'vue'
import { systemApi } from '@/api'
import type { Dept } from '@/data'
import { toastError } from './toast'

/* 部门树缓存：DeptTree / DeptSelect 共用，部门变更后调用 reloadDepts()。 */
export const deptTree = ref<Dept[]>([])
let loaded: Promise<void> | null = null
let generation = 0

export function ensureDepts() {
  if (!loaded) {
    const version = generation
    loaded = systemApi.depts.tree()
      .then(rows => { if (version === generation) deptTree.value = rows as unknown as Dept[] })
      .catch(error => {
        if (version !== generation) return
        loaded = null
        toastError(error, '部门数据加载失败')
      })
  }
  return loaded
}

export function reloadDepts() {
  generation += 1
  loaded = null
  return ensureDepts()
}

export function clearDeptCache() {
  generation += 1
  loaded = null
  deptTree.value = []
}

/** 展平为带层级的选项，供下拉使用 */
export function flattenDepts(nodes: Dept[], depth = 0, acc: Array<{ dept: Dept; depth: number }> = []) {
  nodes.forEach(node => {
    acc.push({ dept: node, depth })
    if (node.children?.length) flattenDepts(node.children, depth + 1, acc)
  })
  return acc
}

/** 收集某部门及全部下级的 ID */
export function collectDeptIds(node: Dept): number[] {
  return [node.id, ...(node.children ?? []).flatMap(collectDeptIds)]
}
