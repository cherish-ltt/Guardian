import { ref, watch, type Ref } from 'vue'

/* 页面视图偏好（表格 / 画布 / 看板 / 矩阵）：按页面记在本机，窄屏首次进入用 narrow 默认值。
   只是个人便利设置，读写失败（隐私模式等）时回落默认值。 */
export function useViewMode<T extends string>(key: string, fallback: T, allowed: readonly T[], narrow?: T) {
  const storageKey = `guardian.view.${key}`
  let initial = fallback
  try {
    const saved = localStorage.getItem(storageKey) as T | null
    if (saved && allowed.includes(saved)) initial = saved
    else if (narrow && window.matchMedia('(max-width: 820px)').matches) initial = narrow
  } catch {
    // 存储不可用时使用默认视图
  }
  const mode = ref(initial) as Ref<T>
  watch(() => mode.value, (value) => {
    try { localStorage.setItem(storageKey, value) } catch { /* 忽略 */ }
  })
  return mode
}
