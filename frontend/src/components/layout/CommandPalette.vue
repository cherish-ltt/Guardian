<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import Icon from '@/components/Icon.vue'
import { Avatar } from '@/components/ui'
import { systemApi, type UserRow } from '@/api'
import { navGroups, pathOf } from '@/composables/auth'
import { appearance } from '@/composables/appearance'
import { overlayEnter, overlayLeave, revealCascade, slideThumb, typeOut } from '@/composables/motion'
import { createFocusTrap } from '@/composables/focus'
import { ask, presetQuestions, type AnswerCard, type AnswerChip } from '@/composables/ask'
import { hasPermission } from '@/directives/permission'

/* ============================================================
   命令面板：最近访问 / 页面 / 用户 / 快捷动作 四组结果，
   一块玻璃滑块在结果间移动表示键盘焦点。
   ============================================================ */
const props = defineProps<{ open: boolean }>()
const emit = defineEmits<{
  (e: 'close'): void
  (e: 'action', name: 'appearance' | 'password' | 'logout'): void
}>()

const router = useRouter()
const route = useRoute()
const panel = ref<HTMLElement | null>(null)
const list = ref<HTMLElement | null>(null)
const thumb = ref<HTMLElement | null>(null)
const input = ref<HTMLInputElement | null>(null)
const keyword = ref('')
const active = ref(0)
const users = ref<UserRow[]>([])
const trap = createFocusTrap()

interface Entry { key: string; group: string; label: string; sub?: string; icon?: string; avatar?: string; run: () => void }

/* ── 最近访问：本机记录最近 5 个页面 ── */
const RECENT_KEY = 'guardian.recent'
const recent = ref<string[]>(readRecent())
function readRecent(): string[] {
  try { return JSON.parse(localStorage.getItem(RECENT_KEY) || '[]') as string[] } catch { return [] }
}
watch(() => route.path, (path) => {
  if (!route.meta.title || route.name === 'not-found') return
  recent.value = [path, ...recent.value.filter(p => p !== path)].slice(0, 5)
  try { localStorage.setItem(RECENT_KEY, JSON.stringify(recent.value)) } catch { /* 忽略 */ }
}, { immediate: true })

const pages = computed(() => navGroups.value.flatMap(g => g.items.map(i => ({ ...i, group: g.single ? '' : g.label }))))
const match = (text: string, key: string) => text.toLowerCase().includes(key)

const entries = computed<Entry[]>(() => {
  const key = keyword.value.trim().toLowerCase()
  const out: Entry[] = []
  if (!key) {
    recent.value
      .map(path => pages.value.find(p => p.path === path))
      .filter((p): p is NonNullable<typeof p> => Boolean(p) && p!.path !== route.path)
      .slice(0, 4)
      .forEach(p => out.push({ key: `r-${p.key}`, group: '最近访问', label: p.label, sub: p.group, icon: p.icon, run: () => go(p.path) }))
  }
  pages.value
    .filter(p => !key || match(p.label, key) || match(p.group, key) || p.path.includes(key))
    .forEach(p => out.push({ key: `p-${p.key}`, group: '页面', label: p.label, sub: p.group, icon: p.icon, run: () => go(p.path) }))
  const usersPath = pathOf('system/user/index')
  if (key && usersPath) {
    users.value.forEach(u => out.push({
      key: `u-${u.id}`, group: '用户', label: u.nickname, sub: `@${u.username} · ${u.deptName || '未分配部门'}`, avatar: u.nickname,
      run: () => go(usersPath, { keyword: u.username }),
    }))
  }
  const dark = document.documentElement.dataset.theme === 'dark'
  const actions: Entry[] = [
    { key: 'a-theme', group: '动作', label: dark ? '切换到浅色外观' : '切换到深色外观', icon: dark ? 'sun' : 'moon', run: () => { appearance.theme = dark ? 'light' : 'dark'; emit('close') } },
    { key: 'a-appearance', group: '动作', label: '外观设置', sub: '主题、表格密度', icon: 'sliders', run: () => act('appearance') },
    { key: 'a-password', group: '动作', label: '修改密码', icon: 'key', run: () => act('password') },
    { key: 'a-logout', group: '动作', label: '退出登录', icon: 'logout', run: () => act('logout') },
  ]
  actions.filter(a => !key || match(a.label, key) || match(a.sub ?? '', key)).forEach(a => out.push(a))
  return out
})

const grouped = computed(() => {
  const groups: Array<{ name: string; items: Array<Entry & { index: number }> }> = []
  entries.value.forEach((entry, index) => {
    let g = groups.find(x => x.name === entry.group)
    if (!g) groups.push(g = { name: entry.group, items: [] })
    g.items.push({ ...entry, index })
  })
  return groups
})

/* ── 问系统：输入 ? 进入问答模式，预设芯片打字机输入，答案卡级联，芯片可跳转 ── */
const askMode = computed(() => keyword.value.startsWith('?') || keyword.value.startsWith('？'))
const askQuestion = computed(() => keyword.value.replace(/^[?？]/, '').trim())
const askPresets = computed(() => presetQuestions())
const askCards = ref<AnswerCard[] | null>(null)
const askLoading = ref(false)
const askedText = ref('')
const askResults = ref<HTMLElement | null>(null)
let cancelTyping: () => void = () => {}

async function runAsk(text: string) {
  const q = text.trim()
  if (!q || askLoading.value) return
  askLoading.value = true
  askedText.value = q
  try {
    askCards.value = await ask(q)
    await nextTick()
    revealCascade(askResults.value, '.ask-card', 0.09)
  } finally {
    askLoading.value = false
  }
}

function typeAsk(question: string) {
  cancelTyping()
  cancelTyping = typeOut(`?${question}`, (v) => { keyword.value = v }, () => runAsk(question))
}

function openAnswerChip(chip: AnswerChip) {
  if (!chip.to) return
  emit('close')
  router.push(chip.to)
}

/* ── 用户搜索：有用户列表权限时防抖查询 ── */
let timer = 0
let requestId = 0
watch(keyword, (value) => {
  active.value = 0
  window.clearTimeout(timer)
  const key = value.trim()
  if (!key || !hasPermission('system:user:list') || !pathOf('system/user/index')) { users.value = []; return }
  timer = window.setTimeout(async () => {
    const current = ++requestId
    try {
      const page = await systemApi.users.list({ keyword: key, pageNum: 1, pageSize: 5 })
      if (current === requestId) users.value = page.rows
    } catch {
      if (current === requestId) users.value = []
    }
  }, 220)
})

function go(path: string, query?: Record<string, string>) {
  emit('close')
  router.push({ path, query })
}
function act(name: 'appearance' | 'password' | 'logout') {
  emit('close')
  emit('action', name)
}

function syncThumb(instant = false) {
  nextTick(() => {
    const el = list.value?.querySelector<HTMLElement>(`[data-index="${active.value}"]`)
    if (!el || !thumb.value) return
    slideThumb(thumb.value, el, 'y', instant)
    el.scrollIntoView({ block: 'nearest' })
  })
}
watch(active, () => syncThumb())
watch(entries, () => syncThumb(true))

watch(() => props.open, async (open) => {
  if (open) {
    keyword.value = ''
    active.value = 0
    users.value = []
    askCards.value = null
    askedText.value = ''
    cancelTyping()
    await nextTick()
    trap.activate(panel.value)
    input.value?.focus()
    syncThumb(true)
  } else {
    trap.deactivate()
  }
})

function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape') { emit('close'); return }
  if (askMode.value) {
    if (event.key === 'Enter') { event.preventDefault(); runAsk(askQuestion.value) }
    return
  }
  const n = entries.value.length
  if (!n) return
  if (event.key === 'ArrowDown') { event.preventDefault(); active.value = (active.value + 1) % n }
  else if (event.key === 'ArrowUp') { event.preventDefault(); active.value = (active.value - 1 + n) % n }
  else if (event.key === 'Enter') { event.preventDefault(); entries.value[active.value]?.run() }
}
</script>

<template>
  <Teleport to="body">
    <Transition :css="false" @enter="(el, done) => overlayEnter(el, done, 'modal')" @leave="(el, done) => overlayLeave(el, done, 'modal')">
      <div v-if="open" class="overlay cmd-overlay" @mousedown.self="emit('close')">
        <div ref="panel" class="modal cmd-panel m-float" role="dialog" aria-modal="true" aria-label="命令面板" @keydown="onKey">
          <div class="cmd-input">
            <Icon name="search" :size="18" />
            <input
              ref="input"
              v-model="keyword"
              placeholder="搜索页面、用户或动作"
              role="combobox"
              aria-expanded="true"
              aria-controls="cmd-results"
              :aria-activedescendant="entries.length ? `cmd-${active}` : undefined"
            />
            <span class="kbd">Esc</span>
          </div>
          <div id="cmd-results" ref="list" class="cmd-results" :role="askMode ? 'region' : 'listbox'">
            <div v-if="askMode" class="cmd-ask">
              <div v-if="!askCards" class="ask-presets">
                <button v-for="p in askPresets" :key="p" type="button" @click="typeAsk(p)">{{ p }}</button>
              </div>
              <p v-if="!askCards" class="cmd-ask-hint faint">输入问题后回车，例如「谁能删除用户」</p>
              <div v-else ref="askResults" class="cmd-ask-results" aria-live="polite">
                <div class="ask-results-head">
                  <span><Icon name="activity" :size="13" />{{ askedText }}</span>
                  <small>本地规则分析</small>
                </div>
                <article v-for="c in askCards" :key="c.title" :class="['ask-card', `tone-${c.tone}`]">
                  <h4>{{ c.title }}</h4>
                  <p>{{ c.text }}</p>
                  <div v-if="c.chips?.length" class="insight-chips">
                    <button v-for="chip in c.chips" :key="chip.key" type="button" :class="['insight-chip', chip.tone ? `tone-${chip.tone}` : '']" @click="openAnswerChip(chip)">
                      {{ chip.label }}<small v-if="chip.sub">{{ chip.sub }}</small>
                    </button>
                  </div>
                </article>
              </div>
              <p v-if="askLoading" class="cmd-ask-hint faint">正在分析…</p>
            </div>
            <template v-else>
            <span v-show="entries.length" ref="thumb" class="cmd-thumb m-pill" aria-hidden="true" />
            <template v-for="g in grouped" :key="g.name">
              <div class="cmd-group eyebrow" role="presentation">{{ g.name }}</div>
              <button
                v-for="item in g.items"
                :id="`cmd-${item.index}`"
                :key="item.key"
                :data-index="item.index"
                type="button"
                role="option"
                :aria-selected="item.index === active"
                :class="['cmd-item', item.index === active && 'is-active']"
                tabindex="-1"
                @pointerenter="active = item.index"
                @click="item.run()"
              >
                <Avatar v-if="item.avatar" :name="item.avatar" :size="30" />
                <span v-else class="cmd-ico m-groove"><Icon :name="item.icon ?? 'file'" :size="16" /></span>
                <span class="cmd-label">{{ item.label }}</span>
                <span v-if="item.sub" class="cmd-sub">{{ item.sub }}</span>
                <Icon name="arrowRight" :size="14" class="cmd-go" />
              </button>
            </template>
            <div v-if="!entries.length" class="select-empty" style="padding: 28px">没有找到匹配的结果</div>
            </template>
          </div>
          <div class="cmd-foot">
            <template v-if="askMode">
              <span><span class="kbd">↵</span> 提问</span>
              <span class="faint">删去 ? 返回搜索</span>
            </template>
            <template v-else>
              <span><span class="kbd">↑</span><span class="kbd">↓</span> 选择</span>
              <span><span class="kbd">↵</span> 打开</span>
              <span><span class="kbd">?</span> 问系统</span>
            </template>
            <span class="spacer" />
            <span class="eyebrow">命令</span>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
