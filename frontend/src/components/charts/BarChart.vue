<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { gsap, motionSkipped, EASE } from '@/composables/motion'

/* 柱状图：柱子从基线生长（结 Crystallize），highlight 柱用斜线填充表示「当前」。
   失败部分叠在柱顶，红色细段。悬停显示准星与读数。 */
const props = withDefaults(defineProps<{
  values: number[]
  labels: string[]
  failed?: number[]
  highlight?: number | null
  height?: number
  unit?: string
}>(), {
  failed: () => [],
  highlight: null,
  height: 168,
  unit: '',
})

const bars = ref<HTMLElement | null>(null)
const hover = ref<number | null>(null)
const max = computed(() => Math.max(1, ...props.values))
const peakIndex = computed(() => props.values.indexOf(Math.max(...props.values)))

function grow() {
  const list = bars.value?.querySelectorAll<HTMLElement>('.bar-col')
  if (!list?.length || motionSkipped()) return
  gsap.fromTo(list, { scaleY: 0 }, { scaleY: 1, duration: 0.8, ease: EASE.settle, stagger: { each: 0.05, from: 'start' }, clearProps: 'transform' })
}

let grown = false
watch(() => props.values, (values) => {
  if (grown || !values.length) return
  grown = true
  nextTick(grow)
}, { immediate: true })
onBeforeUnmount(() => gsap.killTweensOf(bars.value?.querySelectorAll('.bar-col') ?? []))
</script>

<template>
  <div class="bars" :style="{ height: `${height}px` }" @pointerleave="hover = null">
    <div ref="bars" class="bars-plot" role="img" :aria-label="`柱状图，最高 ${Math.max(0, ...values)}${unit}`">
      <div
        v-for="(v, i) in values"
        :key="i"
        :class="['bar-slot', { 'is-hover': hover === i }]"
        @pointerenter="hover = i"
      >
        <span v-if="hover === i || (hover === null && i === peakIndex && v > 0)" class="bar-readout tnum">{{ v }}{{ unit }}</span>
        <div
          :class="['bar-col', { 'is-highlight': i === highlight }]"
          :style="{ height: `${Math.max(v ? 4 : 2, (v / max) * 100)}%` }"
        >
          <span v-if="failed[i]" class="bar-failed" :style="{ height: `${Math.min(100, (failed[i] / Math.max(v, 1)) * 100)}%` }" />
        </div>
      </div>
    </div>
    <div class="bars-labels">
      <span v-for="(l, i) in labels" :key="i" :class="{ 'is-highlight': i === highlight }">{{ l }}</span>
    </div>
  </div>
</template>
