<script setup lang="ts">
import Icon from '@/components/Icon.vue'

withDefaults(defineProps<{
  variant?: 'primary' | 'secondary' | 'ghost' | 'text' | 'danger'
  size?: 'md' | 'sm'
  icon?: string
  loading?: boolean
  disabled?: boolean
  danger?: boolean
  block?: boolean
  type?: 'button' | 'submit'
}>(), {
  variant: 'secondary',
  size: 'md',
  type: 'button',
})
</script>

<template>
  <button
    :type="type"
    :class="[
      'btn', `btn-${variant}`,
      size === 'sm' && 'btn-sm',
      block && 'btn-block',
      danger && 'is-danger',
      variant === 'secondary' && 'm-pill',
    ]"
    :disabled="disabled || loading"
    :aria-busy="loading || undefined"
  >
    <span v-if="loading" class="spinner" aria-hidden="true" />
    <Icon v-else-if="icon" :name="icon" :size="size === 'sm' ? 14 : 16" />
    <slot />
  </button>
</template>
