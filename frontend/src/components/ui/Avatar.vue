<script setup lang="ts">
import { computed } from 'vue'
import { AVATAR_TINTS } from '@/data'

const props = withDefaults(defineProps<{
  name?: string
  size?: number
}>(), {
  name: '',
  size: 32,
})

// 中文名取后两字，英文名取首字母
const text = computed(() => {
  const name = (props.name || '?').trim()
  return /^[\x00-\x7f]+$/.test(name) ? name.charAt(0).toUpperCase() : name.slice(-2)
})
const tint = computed(() => {
  let hash = 0
  for (const ch of props.name) hash = (hash * 31 + ch.charCodeAt(0)) >>> 0
  return AVATAR_TINTS[hash % AVATAR_TINTS.length]
})
</script>

<template>
  <span
    class="avatar"
    :style="{ width: `${size}px`, height: `${size}px`, fontSize: `${Math.round(size * 0.36)}px`, background: tint }"
    aria-hidden="true"
  >{{ text }}</span>
</template>
