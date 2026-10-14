<script setup lang="ts">
import { computed } from 'vue'

/* 环比胶囊：上升青柠、下降红、持平中性；上一周期为 0（fresh）时显示「新增」中性态，不给出误导的百分比 */
const props = withDefaults(defineProps<{
  value: number | null | undefined
  /** 显式方向；不传则按 value 正负判断 */
  up?: boolean | null
  fresh?: boolean
  suffix?: string
  label?: string
}>(), {
  up: null,
  fresh: false,
  suffix: '%',
})

const dir = computed(() => {
  if (props.fresh) return 'new'
  const v = Number(props.value ?? 0)
  if (!v) return 'flat'
  if (props.up === null) return v > 0 ? 'up' : 'down'
  return props.up ? 'up' : 'down'
})
const text = computed(() => {
  if (props.fresh) return '新增'
  const v = Math.abs(Number(props.value ?? 0))
  const sign = dir.value === 'up' ? '+' : dir.value === 'down' ? '−' : '±'
  return `${sign}${Number.isInteger(v) ? v : v.toFixed(1)}${props.suffix}`
})
</script>

<template>
  <span :class="['delta-pill', `is-${dir}`]" :title="label">
    <svg width="8" height="8" viewBox="0 0 8 8" aria-hidden="true">
      <path v-if="dir === 'up'" d="M4 1.2 7 6H1z" fill="currentColor" />
      <path v-else-if="dir === 'down'" d="M4 6.8 1 2h6z" fill="currentColor" />
      <circle v-else-if="dir === 'new'" cx="4" cy="4" r="2.2" fill="currentColor" />
      <rect v-else x="1" y="3.3" width="6" height="1.4" rx=".7" fill="currentColor" />
    </svg>
    <span class="tnum">{{ text }}</span>
    <span v-if="label" class="sr-only">{{ label }}</span>
  </span>
</template>
