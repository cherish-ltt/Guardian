import { ref, type Ref } from 'vue'
import { systemApi } from '@/api'

/* 字典缓存：同一字典类型只请求一次，DictSelect / DictTag 共用。 */
export interface DictOption {
  label: string
  value: string
  tone?: string
  isDefault?: boolean
}

const cache = new Map<string, Ref<DictOption[]>>()
const inflight = new Map<string, Promise<void>>()

export function useDict(type: string): Ref<DictOption[]> {
  let entry = cache.get(type)
  if (!entry) {
    entry = ref<DictOption[]>([])
    cache.set(type, entry)
  }
  if (!inflight.has(type)) {
    const target = entry
    const task = systemApi.dictData.options(type)
      .then(rows => {
        if (cache.get(type) !== target) return
        target.value = rows
          .filter(row => Number(row.status) === 1)
          .map(row => ({
            label: String(row.label),
            value: String(row.value),
            tone: row.tone ? String(row.tone) : undefined,
            isDefault: Boolean(row.def),
          }))
      })
      .catch(() => { if (cache.get(type) === target) inflight.delete(type) })
    inflight.set(type, task)
  }
  return entry
}

export function clearDictCache() {
  cache.forEach(entry => { entry.value = [] })
  cache.clear()
  inflight.clear()
}

/** 字典数据被修改后调用，下次读取时重新拉取 */
export function invalidateDict(type?: string) {
  if (type) {
    inflight.delete(type)
    if (cache.has(type)) useDict(type)
    return
  }
  inflight.clear()
  cache.forEach((_, key) => useDict(key))
}
