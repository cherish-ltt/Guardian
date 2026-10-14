import { reactive, watch } from 'vue'

/* 外观设置：主题（浅色 / 深色 / 跟随系统）、表格密度、导航轨折叠。持久化在本机浏览器。 */
export type ThemeMode = 'light' | 'dark' | 'auto'
export type Density = 'comfy' | 'regular' | 'compact'

export interface Appearance {
  theme: ThemeMode
  density: Density
  railCollapsed: boolean
}

const STORAGE_KEY = 'guardian.appearance'
const DEFAULTS: Appearance = { theme: 'light', density: 'regular', railCollapsed: false }

function load(): Appearance {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) {
      const saved = JSON.parse(raw) as Partial<Appearance>
      return {
        theme: (['light', 'dark', 'auto'] as const).includes(saved.theme as ThemeMode) ? saved.theme as ThemeMode : DEFAULTS.theme,
        density: (['comfy', 'regular', 'compact'] as const).includes(saved.density as Density) ? saved.density as Density : DEFAULTS.density,
        railCollapsed: Boolean(saved.railCollapsed),
      }
    }
  } catch { /* 存储不可用时使用默认值 */ }
  return { ...DEFAULTS }
}

export const appearance = reactive<Appearance>(load())

const darkQuery = window.matchMedia('(prefers-color-scheme: dark)')

function resolvedTheme() {
  if (appearance.theme === 'auto') return darkQuery.matches ? 'dark' : 'light'
  return appearance.theme
}

function apply() {
  const root = document.documentElement
  root.dataset.theme = resolvedTheme()
  root.dataset.density = appearance.density
}

watch(() => [appearance.theme, appearance.density], apply, { immediate: true })
darkQuery.addEventListener('change', () => { if (appearance.theme === 'auto') apply() })

watch(appearance, (value) => {
  try { localStorage.setItem(STORAGE_KEY, JSON.stringify(value)) } catch { /* 忽略 */ }
}, { deep: true })
