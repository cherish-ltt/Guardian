<script setup lang="ts">
import { computed } from 'vue'
import { ICON_PATHS } from '@/data/icons'

const props = withDefaults(defineProps<{
  name: string
  size?: number
  stroke?: number
}>(), {
  size: 18,
  stroke: 1.6,
})

const d = computed(() => ICON_PATHS[props.name] ?? ICON_PATHS.file)
</script>

<template>
  <svg
    class="icon"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    :stroke-width="stroke"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    style="flex: none"
  >
    <path :d="d" />
  </svg>
</template>
