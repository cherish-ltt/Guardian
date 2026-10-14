<script setup lang="ts">
import { ref, watch } from 'vue'
import Icon from '@/components/Icon.vue'
import { settlePop } from '@/composables/motion'

const props = defineProps<{
  modelValue: boolean
  indeterminate?: boolean
  disabled?: boolean
  label?: string
}>()
const emit = defineEmits<{ (e: 'update:modelValue', value: boolean): void }>()

const mark = ref<HTMLElement | null>(null)

// 勾上时一次落定弹入（结 Crystallize）
watch(() => props.modelValue, (on) => { if (on) settlePop(mark.value) })
</script>

<template>
  <label :class="['checkbox', modelValue && 'is-on', indeterminate && !modelValue && 'is-partial', disabled && 'is-disabled']" @click.stop>
    <input
      type="checkbox"
      :checked="modelValue"
      :disabled="disabled"
      :aria-checked="indeterminate && !modelValue ? 'mixed' : modelValue"
      :aria-label="label ? undefined : '选择'"
      @change="emit('update:modelValue', ($event.target as HTMLInputElement).checked)"
    />
    <span class="checkbox-box">
      <span ref="mark" style="display: inline-flex">
        <Icon v-if="modelValue" name="check" :size="12" :stroke="2.6" />
        <svg v-else-if="indeterminate" width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M2 5h6" stroke="currentColor" stroke-width="2" stroke-linecap="round" /></svg>
      </span>
    </span>
    <span v-if="label">{{ label }}</span>
    <slot />
  </label>
</template>
