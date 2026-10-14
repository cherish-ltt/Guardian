<script setup lang="ts">
import Icon from '@/components/Icon.vue'

withDefaults(defineProps<{
  name: string
  label: string
  size?: number
  bordered?: boolean
  danger?: boolean
  small?: boolean
  disabled?: boolean
}>(), {
  size: 17,
})
</script>

<template>
  <button
    type="button"
    :class="['icon-btn', bordered && 'is-bordered', danger && 'is-danger', small && 'icon-btn-sm']"
    :title="label"
    :aria-label="label"
    :disabled="disabled"
  >
    <Icon :name="name" :size="size" />
  </button>
</template>
