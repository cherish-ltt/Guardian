<script setup lang="ts" generic="T extends string | number">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { slideThumb } from '@/composables/motion'

const props = defineProps<{
  modelValue: T
  options: Array<{ label: string; value: T }>
  label?: string
}>()
const emit = defineEmits<{ (e: 'update:modelValue', value: T): void }>()

const root = ref<HTMLElement | null>(null)
const thumb = ref<HTMLElement | null>(null)

function activeButton() {
  const index = props.options.findIndex(o => o.value === props.modelValue)
  return root.value?.querySelectorAll<HTMLElement>('button')[index] ?? null
}

function sync(instant = false) {
  slideThumb(thumb.value, activeButton(), 'x', instant)
}

function onKey(event: KeyboardEvent) {
  const index = props.options.findIndex(o => o.value === props.modelValue)
  const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0
  if (!step) return
  event.preventDefault()
  const next = props.options[(index + step + props.options.length) % props.options.length]
  emit('update:modelValue', next.value)
  nextTick(() => activeButton()?.focus())
}

/* 容器或任一选项尺寸变化都重新对位：拉伸布局里网页字体加载完成只改选项宽度，不改容器宽度 */
let observer: ResizeObserver | null = null
function observeAll() {
  observer?.disconnect()
  if (!root.value || !observer) return
  observer.observe(root.value)
  root.value.querySelectorAll('button').forEach(b => observer!.observe(b))
}
onMounted(() => {
  nextTick(() => sync(true))
  observer = new ResizeObserver(() => sync(true))
  observeAll()
})
onBeforeUnmount(() => observer?.disconnect())
watch(() => props.modelValue, () => nextTick(() => sync()))
watch(() => props.options.map(o => o.label).join('\u0000'), () => nextTick(() => { observeAll(); sync(true) }))
</script>

<template>
  <div ref="root" class="segmented m-groove" role="radiogroup" :aria-label="label" @keydown="onKey">
    <span ref="thumb" class="seg-thumb m-pill" aria-hidden="true" />
    <button
      v-for="o in options"
      :key="String(o.value)"
      type="button"
      role="radio"
      :aria-checked="o.value === modelValue"
      :tabindex="o.value === modelValue ? 0 : -1"
      :class="{ 'is-active': o.value === modelValue }"
      @click="emit('update:modelValue', o.value)"
    >{{ o.label }}</button>
  </div>
</template>
