<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref } from 'vue'
import { useRoute } from 'vue-router'
import Icon from '@/components/Icon.vue'
import IconButton from '@/components/ui/IconButton.vue'
import Avatar from '@/components/ui/Avatar.vue'
import { currentUser } from '@/composables/auth'
import { overlayEnter, overlayLeave } from '@/composables/motion'

const emit = defineEmits<{
  (e: 'toggle-rail'): void
  (e: 'command'): void
  (e: 'appearance'): void
  (e: 'password'): void
  (e: 'logout'): void
}>()

const route = useRoute()
const chip = ref<HTMLElement | null>(null)
const pop = ref<HTMLElement | null>(null)
const menuOpen = ref(false)
const pos = ref({ top: 0, right: 0 })

const group = computed(() => route.meta.group as string | undefined)
const title = computed(() => route.meta.title as string | undefined)
const isMac = /Mac|iPhone|iPad/.test(navigator.platform)

async function openMenu() {
  const rect = chip.value?.getBoundingClientRect()
  if (!rect) return
  pos.value = { top: rect.bottom + 8, right: window.innerWidth - rect.right }
  menuOpen.value = true
  await nextTick()
  pop.value?.querySelector<HTMLElement>('.menu-item')?.focus()
  document.addEventListener('pointerdown', onOutside, true)
  document.addEventListener('keydown', onKey)
}

function closeMenu(refocus = false) {
  menuOpen.value = false
  document.removeEventListener('pointerdown', onOutside, true)
  document.removeEventListener('keydown', onKey)
  if (refocus) chip.value?.focus()
}

function onOutside(event: PointerEvent) {
  const target = event.target as Node
  if (chip.value?.contains(target) || pop.value?.contains(target)) return
  closeMenu()
}

function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape') closeMenu(true)
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    event.preventDefault()
    const items = Array.from(pop.value?.querySelectorAll<HTMLElement>('.menu-item') ?? [])
    const index = items.indexOf(document.activeElement as HTMLElement)
    const next = items[(index + (event.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length]
    next?.focus()
  }
}

function pick(action: 'appearance' | 'password' | 'logout') {
  closeMenu()
  if (action === 'appearance') emit('appearance')
  else if (action === 'password') emit('password')
  else emit('logout')
}

onBeforeUnmount(() => closeMenu())
</script>

<template>
  <header class="topbar">
    <IconButton name="sidebar" label="切换导航" @click="emit('toggle-rail')" />
    <div class="crumbs" aria-label="当前位置">
      <template v-if="group"><span>{{ group }}</span><Icon class="crumb-sep" name="chevronRight" :size="12" /></template>
      <span class="crumb-current">{{ title }}</span>
    </div>
    <div class="spacer" />
    <button type="button" class="cmd-trigger m-pill" aria-label="搜索菜单" @click="emit('command')">
      <Icon name="search" :size="15" />
      <span class="cmd-text">搜索菜单</span>
      <span class="kbd">{{ isMac ? '⌘' : 'Ctrl' }} K</span>
    </button>
    <IconButton name="contrast" label="外观设置" @click="emit('appearance')" />
    <button
      ref="chip"
      type="button"
      class="user-chip"
      aria-haspopup="menu"
      :aria-expanded="menuOpen"
      @click="menuOpen ? closeMenu() : openMenu()"
    >
      <Avatar :name="currentUser?.nickname" :size="32" />
      <span class="user-meta">
        <span class="user-name" style="display: block">{{ currentUser?.nickname }}</span>
        <span class="user-role" style="display: block">{{ currentUser?.deptName || currentUser?.username }}</span>
      </span>
      <Icon name="chevronDown" :size="14" style="color: var(--ink-soft)" />
    </button>

    <Teleport to="body">
      <Transition :css="false" @enter="(el, done) => overlayEnter(el, done, 'pop')" @leave="(el, done) => overlayLeave(el, done, 'pop')">
        <div v-if="menuOpen" ref="pop" class="menu-pop m-float" role="menu" :style="{ top: `${pos.top}px`, right: `${pos.right}px` }">
          <div class="menu-head">
            <div style="font-weight: 680">{{ currentUser?.nickname }}</div>
            <div class="soft" style="font-size: 12px">@{{ currentUser?.username }}<template v-if="currentUser?.deptName"> · {{ currentUser.deptName }}</template></div>
          </div>
          <button type="button" class="menu-item" role="menuitem" @click="pick('password')"><Icon name="key" :size="16" />修改密码</button>
          <button type="button" class="menu-item" role="menuitem" @click="pick('appearance')"><Icon name="contrast" :size="16" />外观设置</button>
          <button type="button" class="menu-item is-danger" role="menuitem" @click="pick('logout')"><Icon name="logout" :size="16" />退出登录</button>
        </div>
      </Transition>
    </Teleport>
  </header>
</template>
