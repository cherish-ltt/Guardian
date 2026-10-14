<script setup lang="ts">
import { ref } from 'vue'
import Icon from '@/components/Icon.vue'

const props = withDefaults(defineProps<{
  modelValue?: string | number | null
  placeholder?: string
  type?: 'text' | 'password' | 'number' | 'email' | 'tel'
  icon?: string
  invalid?: boolean
  disabled?: boolean
  readonly?: boolean
  clearable?: boolean
  multiline?: boolean
  rows?: number
  maxlength?: number
  autocomplete?: string
}>(), {
  type: 'text',
  rows: 3,
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string | number | null): void
  (e: 'enter'): void
}>()

const el = ref<HTMLInputElement | HTMLTextAreaElement | null>(null)

function onInput(event: Event) {
  const raw = (event.target as HTMLInputElement).value
  if (props.type === 'number') emit('update:modelValue', raw === '' ? null : Number(raw))
  else emit('update:modelValue', raw)
}

function clear() {
  emit('update:modelValue', props.type === 'number' ? null : '')
  el.value?.focus()
}

defineExpose({ focus: () => el.value?.focus() })
</script>

<template>
  <span :class="['input-shell', invalid && 'is-invalid', disabled && 'is-disabled', multiline && 'is-multiline']">
    <Icon v-if="icon" :name="icon" :size="15" />
    <textarea
      v-if="multiline"
      ref="el"
      :value="modelValue ?? ''"
      :placeholder="placeholder"
      :disabled="disabled"
      :readonly="readonly"
      :rows="rows"
      :maxlength="maxlength"
      :aria-invalid="invalid || undefined"
      @input="onInput"
    />
    <input
      v-else
      ref="el"
      :type="type"
      :value="modelValue ?? ''"
      :placeholder="placeholder"
      :disabled="disabled"
      :readonly="readonly"
      :maxlength="maxlength"
      :autocomplete="autocomplete"
      :aria-invalid="invalid || undefined"
      @input="onInput"
      @keydown.enter="emit('enter')"
    />
    <button
      v-if="clearable && modelValue !== '' && modelValue !== null && modelValue !== undefined && !disabled"
      type="button"
      class="input-clear"
      aria-label="清空"
      @click="clear"
    >
      <Icon name="x" :size="14" />
    </button>
  </span>
</template>
