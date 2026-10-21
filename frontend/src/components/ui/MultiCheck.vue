<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import Icon from '@/components/Icon.vue'
import { overlayEnter, overlayLeave } from '@/composables/motion'
import { floatingLayerFor } from '@/composables/focus'
import { placePopover, trackPopover } from '@/composables/popover'
import { searchEnabled } from './select-shared'

/* 多选下拉：用于用户的多角色 / 多岗位。浮层与单选下拉同源（Teleport 到 body + 跟随触发层），选项多时可搜索。 */
const props = withDefaults(defineProps<{
  modelValue: number[]
  options: Array<{ label: string; value: number }>
  placeholder?: string
  /** 搜索框：true 强制开，false 强制关，auto 时选项达到阈值才开 */
  filterable?: boolean | 'auto'
  invalid?: boolean
}>(), {
  placeholder: '请选择',
  filterable: 'auto',
})

const emit = defineEmits<{ (e: 'update:modelValue', value: number[]): void }>()

const trigger = ref<HTMLElement | null>(null)
const menu = ref<HTMLElement | null>(null)
const searchInput = ref<HTMLInputElement | null>(null)
const open = ref(false)
const keyword = ref('')
const activeIndex = ref(-1)
const pos = ref({ top: 0, left: 0, width: 0 })
const listId = `mc-${Math.random().toString(36).slice(2, 8)}`
let untrack: (() => void) | null = null

const selected = computed(() => new Set(props.modelValue))
const summary = computed(() => props.options
  .filter(option => selected.value.has(option.value))
  .map(option => option.label)
  .join('、'))
const visible = computed(() => {
  const key = keyword.value.trim().toLowerCase()
  return key ? props.options.filter(o => o.label.toLowerCase().includes(key)) : props.options
})
const searchable = computed(() => searchEnabled(props.filterable, props.options.length))

function place() {
  const next = placePopover(trigger.value, menu.value)
  if (next) pos.value = next
}

async function show() {
  if (open.value) return
  keyword.value = ''
  open.value = true
  activeIndex.value = visible.value.length ? 0 : -1
  place()
  await nextTick()
  place()
  if (searchable.value) searchInput.value?.focus()
  scrollActiveIntoView()
  document.addEventListener('pointerdown', onOutside, true)
  untrack = trackPopover(place)
}

function hide(refocus = false) {
  if (!open.value) return
  open.value = false
  document.removeEventListener('pointerdown', onOutside, true)
  untrack?.()
  untrack = null
  if (refocus) trigger.value?.focus()
}

function toggle(value: number) {
  const next = new Set(selected.value)
  if (next.has(value)) next.delete(value)
  else next.add(value)
  emit('update:modelValue', [...next])
}

function onOutside(event: PointerEvent) {
  const target = event.target as Node
  if (trigger.value?.contains(target) || menu.value?.contains(target)) return
  hide()
}

function move(step: number) {
  const list = visible.value
  if (!list.length) return
  activeIndex.value = (activeIndex.value + step + list.length) % list.length
  scrollActiveIntoView()
}

function scrollActiveIntoView() {
  nextTick(() => menu.value?.querySelector('.select-option.is-active')?.scrollIntoView({ block: 'nearest' }))
}

function onKey(event: KeyboardEvent) {
  if (!open.value) {
    if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(event.key)) {
      event.preventDefault()
      show()
    }
    return
  }
  if (event.key === 'ArrowDown') { event.preventDefault(); move(1) }
  else if (event.key === 'ArrowUp') { event.preventDefault(); move(-1) }
  else if (event.key === 'Enter') {
    event.preventDefault()
    const option = visible.value[activeIndex.value]
    if (option) toggle(option.value)
  } else if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); hide(true) }
  else if (event.key === 'Tab') hide(true)
}

watch(keyword, () => { activeIndex.value = visible.value.length ? 0 : -1 })
onBeforeUnmount(() => hide())
</script>

<template>
  <button
    ref="trigger"
    type="button"
    :class="['input-shell', 'select-trigger', invalid && 'is-invalid']"
    role="combobox"
    aria-haspopup="listbox"
    :aria-expanded="open"
    :aria-controls="listId"
    @click="open ? hide() : show()"
    @keydown="onKey"
  >
    <span v-if="summary" class="multi-check-summary">{{ summary }}</span>
    <span v-else class="multi-check-placeholder">{{ placeholder }}</span>
    <Icon class="select-caret" name="chevronDown" :size="15" />
  </button>
  <Teleport to="body">
    <Transition :css="false" @enter="(el, done) => overlayEnter(el, done, 'pop')" @leave="(el, done) => overlayLeave(el, done, 'pop')">
      <div
        v-if="open"
        ref="menu"
        class="select-menu m-float"
        :style="{ top: `${pos.top}px`, left: `${pos.left}px`, width: `${pos.width}px`, zIndex: floatingLayerFor(trigger) }"
        @keydown="onKey"
      >
        <div v-if="searchable" class="select-search">
          <span class="input-shell">
            <Icon name="search" :size="14" />
            <input ref="searchInput" v-model="keyword" placeholder="搜索" aria-label="搜索选项" />
          </span>
        </div>
        <div :id="listId" class="select-options" role="listbox" aria-multiselectable="true">
          <div
            v-for="(o, i) in visible"
            :key="o.value"
            role="option"
            :aria-selected="selected.has(o.value)"
            :class="['select-option', i === activeIndex && 'is-active', selected.has(o.value) && 'is-selected']"
            @pointerenter="activeIndex = i"
            @click="toggle(o.value)"
          >
            <span class="opt-label">{{ o.label }}</span>
            <Icon v-if="selected.has(o.value)" name="check" :size="14" />
          </div>
          <div v-if="!visible.length" class="select-empty">无匹配项</div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
