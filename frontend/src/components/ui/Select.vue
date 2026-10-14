<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import Icon from '@/components/Icon.vue'
import { overlayEnter, overlayLeave } from '@/composables/motion'
import { floatingLayerFor } from '@/composables/focus'

export interface SelectOption {
  label: string
  value: string | number
  /** 树形选项的缩进层级 */
  depth?: number
  disabled?: boolean
  hint?: string
  icon?: string
}
type Value = string | number | null | undefined

const props = withDefaults(defineProps<{
  modelValue: Value
  options: Array<SelectOption | string | number>
  placeholder?: string
  filterable?: boolean
  clearable?: boolean
  disabled?: boolean
  invalid?: boolean
  emptyText?: string
}>(), {
  placeholder: '请选择',
  emptyText: '无匹配项',
})
const emit = defineEmits<{ (e: 'update:modelValue', value: Value): void }>()

const trigger = ref<HTMLElement | null>(null)
const menu = ref<HTMLElement | null>(null)
const searchInput = ref<HTMLInputElement | null>(null)
const open = ref(false)
const keyword = ref('')
const activeIndex = ref(-1)
const pos = ref({ top: 0, left: 0, width: 0 })
const listId = `sel-${Math.random().toString(36).slice(2, 8)}`

const normalized = computed<SelectOption[]>(() => props.options.map(o =>
  typeof o === 'object' ? o : { label: String(o), value: o }))
const current = computed(() => normalized.value.find(o => o.value === props.modelValue))
const visible = computed(() => {
  const key = keyword.value.trim().toLowerCase()
  return key ? normalized.value.filter(o => o.label.toLowerCase().includes(key)) : normalized.value
})

function place() {
  const rect = trigger.value?.getBoundingClientRect()
  if (!rect) return
  const menuHeight = menu.value?.offsetHeight ?? 260
  const below = window.innerHeight - rect.bottom
  const top = below < menuHeight + 12 && rect.top > menuHeight + 12 ? rect.top - menuHeight - 6 : rect.bottom + 6
  const width = Math.max(rect.width, 160)
  const left = Math.min(rect.left, window.innerWidth - width - 8)
  pos.value = { top, left, width }
}

async function show() {
  if (props.disabled || open.value) return
  keyword.value = ''
  open.value = true
  activeIndex.value = Math.max(0, visible.value.findIndex(o => o.value === props.modelValue))
  place()
  await nextTick()
  place()
  if (props.filterable) searchInput.value?.focus()
  scrollActiveIntoView()
  document.addEventListener('pointerdown', onOutside, true)
  window.addEventListener('scroll', place, true)
  window.addEventListener('resize', place)
}

function hide(refocus = false) {
  if (!open.value) return
  open.value = false
  document.removeEventListener('pointerdown', onOutside, true)
  window.removeEventListener('scroll', place, true)
  window.removeEventListener('resize', place)
  if (refocus) trigger.value?.focus()
}

function pick(option: SelectOption) {
  if (option.disabled) return
  emit('update:modelValue', option.value)
  hide(true)
}

function onOutside(event: PointerEvent) {
  const target = event.target as Node
  if (trigger.value?.contains(target) || menu.value?.contains(target)) return
  hide()
}

function move(step: number) {
  const list = visible.value
  if (!list.length) return
  let index = activeIndex.value
  for (let i = 0; i < list.length; i += 1) {
    index = (index + step + list.length) % list.length
    if (!list[index].disabled) break
  }
  activeIndex.value = index
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
    if (option) pick(option)
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
    :class="['input-shell', 'select-trigger', invalid && 'is-invalid', disabled && 'is-disabled']"
    :disabled="disabled"
    role="combobox"
    aria-haspopup="listbox"
    :aria-expanded="open"
    :aria-controls="listId"
    @click="open ? hide() : show()"
    @keydown="onKey"
  >
    <Icon v-if="current?.icon" :name="current.icon" :size="15" />
    <span :class="['select-value', !current && 'is-placeholder']">{{ current ? current.label : placeholder }}</span>
    <span
      v-if="clearable && current && !disabled"
      class="input-clear"
      role="button"
      aria-label="清空"
      @click.stop="emit('update:modelValue', typeof modelValue === 'number' ? null : '')"
    ><Icon name="x" :size="14" /></span>
    <Icon v-else class="select-caret" name="chevronDown" :size="15" />
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
        <div v-if="filterable" class="select-search">
          <span class="input-shell">
            <Icon name="search" :size="14" />
            <input ref="searchInput" v-model="keyword" placeholder="搜索" aria-label="搜索选项" />
          </span>
        </div>
        <div :id="listId" class="select-options" role="listbox">
          <div
            v-for="(o, i) in visible"
            :key="String(o.value)"
            role="option"
            :aria-selected="o.value === modelValue"
            :aria-disabled="o.disabled || undefined"
            :class="['select-option', i === activeIndex && 'is-active', o.value === modelValue && 'is-selected', o.disabled && 'is-disabled']"
            :style="o.depth ? { paddingLeft: `${10 + o.depth * 16}px` } : undefined"
            @pointerenter="activeIndex = i"
            @click="pick(o)"
          >
            <Icon v-if="o.icon" :name="o.icon" :size="15" style="color: var(--ink-soft)" />
            <span class="opt-label">{{ o.label }}</span>
            <span v-if="o.hint" class="faint" style="font-size: 11px">{{ o.hint }}</span>
            <Icon v-if="o.value === modelValue" name="check" :size="14" />
          </div>
          <div v-if="!visible.length" class="select-empty">{{ emptyText }}</div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
