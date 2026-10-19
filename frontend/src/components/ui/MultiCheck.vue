<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue'
import Icon from '@/components/Icon.vue'
import Checkbox from '@/components/ui/Checkbox.vue'

/* 多选下拉：用于用户的多角色 / 多岗位，选项不多时比穿梭框更轻。 */
const props = withDefaults(defineProps<{
  modelValue: number[]
  options: Array<{ label: string; value: number }>
  placeholder?: string
  invalid?: boolean
}>(), {
  placeholder: '请选择',
})

const emit = defineEmits<{ (e: 'update:modelValue', value: number[]): void }>()

const open = ref(false)
const root = ref<HTMLElement | null>(null)

const selected = computed(() => new Set(props.modelValue))
const summary = computed(() => {
  const labels = props.options.filter(option => selected.value.has(option.value)).map(option => option.label)
  return labels.length ? labels.join('、') : ''
})

function toggle(value: number) {
  const next = new Set(selected.value)
  if (next.has(value)) next.delete(value)
  else next.add(value)
  emit('update:modelValue', [...next])
}

function onDocumentPointerDown(event: PointerEvent) {
  if (!open.value) return
  if (root.value && !root.value.contains(event.target as Node)) open.value = false
}

function onDocumentKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') open.value = false
}

document.addEventListener('pointerdown', onDocumentPointerDown)
document.addEventListener('keydown', onDocumentKeydown)
onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', onDocumentPointerDown)
  document.removeEventListener('keydown', onDocumentKeydown)
})
</script>

<template>
  <div ref="root" :class="['multi-check', open && 'is-open']">
    <button
      type="button"
      :class="['input-shell', 'select-trigger', invalid && 'is-invalid', open && 'is-open']"
      :aria-expanded="open"
      aria-haspopup="listbox"
      @click="open = !open"
    >
      <span v-if="summary" class="multi-check-summary">{{ summary }}</span>
      <span v-else class="multi-check-placeholder">{{ placeholder }}</span>
      <Icon name="chevronRight" :size="14" class="select-caret multi-check-caret" />
    </button>
    <div v-if="open" class="multi-check-panel m-pill" role="listbox" aria-multiselectable="true">
      <label v-for="option in options" :key="option.value" class="multi-check-option">
        <Checkbox :model-value="selected.has(option.value)" @update:model-value="toggle(option.value)" />
        <span>{{ option.label }}</span>
      </label>
      <p v-if="!options.length" class="multi-check-empty">无可选项</p>
    </div>
  </div>
</template>
