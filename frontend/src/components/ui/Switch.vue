<script setup lang="ts">
defineProps<{
  modelValue: boolean
  disabled?: boolean
  label?: string
}>()
const emit = defineEmits<{ (e: 'update:modelValue', value: boolean): void }>()
</script>

<template>
  <label :class="['switch', modelValue && 'is-on', disabled && 'is-disabled']">
    <input
      type="checkbox"
      role="switch"
      :checked="modelValue"
      :disabled="disabled"
      :aria-checked="modelValue"
      :aria-label="label ? undefined : '开关'"
      @change="emit('update:modelValue', ($event.target as HTMLInputElement).checked)"
    />
    <span class="switch-track"><span class="switch-knob" /></span>
    <span v-if="label">{{ label }}</span>
    <slot />
  </label>
</template>
