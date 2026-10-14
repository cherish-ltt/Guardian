<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'

/* 逐位滚动读数（结 Crystallize）：每一位是一条 0–9 的竖带，只有变化的位会滚。
   位次按「从右往左」编号作为 key，位数变化时个位、十位不会错位重排。 */
const props = withDefaults(defineProps<{
  value: number | null | undefined
  decimals?: number
  /** 千分位分隔 */
  group?: boolean
}>(), {
  decimals: 0,
  group: true,
})

const mounted = ref(false)
// 首帧显示 0 再滚到目标值；后台标签页里 rAF 不触发，用定时器兜底
onMounted(() => {
  const settle = () => { mounted.value = true }
  if (document.hidden) return settle()
  requestAnimationFrame(settle)
  window.setTimeout(settle, 120)
})

const text = computed(() => {
  if (props.value === null || props.value === undefined || Number.isNaN(props.value)) return '—'
  return Number(props.value).toLocaleString('en-US', {
    minimumFractionDigits: props.decimals,
    maximumFractionDigits: props.decimals,
    useGrouping: props.group,
  })
})

const chars = computed(() => {
  const list = [...text.value]
  const dot = list.indexOf('.')
  return list.map((ch, i) => ({
    ch,
    key: list.length - i,
    digit: /\d/.test(ch) ? Number(ch) : null,
    fraction: dot !== -1 && i > dot,
  }))
})
</script>

<template>
  <span class="odometer tnum" :aria-label="text" role="img">
    <span
      v-for="c in chars"
      :key="c.key"
      :class="['odo-cell', { 'is-fraction': c.fraction, 'is-sep': c.digit === null }]"
      aria-hidden="true"
    >
      <span
        v-if="c.digit !== null"
        class="odo-strip"
        :style="{ transform: `translateY(${-(mounted ? c.digit : 0) * 10}%)`, transitionDelay: `${(c.key % 6) * 40}ms` }"
      ><span v-for="n in 10" :key="n">{{ n - 1 }}</span></span>
      <template v-else>{{ c.ch }}</template>
    </span>
  </span>
</template>
