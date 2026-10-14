<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import Odometer from '@/components/ui/Odometer.vue'
import { gsap, motionSkipped, EASE } from '@/composables/motion'

/* 刻度环：60 根径向刻度按顺时针逐根点亮（导 Conduct），
   点亮到位后末端刻度一次 back.out 过冲（结 Crystallize），中心读数逐位滚动。 */
const props = withDefaults(defineProps<{
  value: number | null | undefined
  display?: number
  unit?: string
  label?: string
  size?: number
  ticks?: number
  decimals?: number
}>(), {
  size: 168,
  ticks: 60,
  decimals: 1,
  unit: '%',
})

const root = ref<SVGSVGElement | null>(null)
const pct = computed(() => Math.min(100, Math.max(0, Number(props.value ?? 0))))
const lit = computed(() => Math.round((pct.value / 100) * props.ticks))
const R_OUT = 0.5
const R_IN = 0.4
const R_MAJOR = 0.37

const marks = computed(() => Array.from({ length: props.ticks }, (_, i) => {
  const a = (i / props.ticks) * Math.PI * 2 - Math.PI / 2
  const major = i % 5 === 0
  const inner = major ? R_MAJOR : R_IN
  return {
    x1: 0.5 + Math.cos(a) * inner, y1: 0.5 + Math.sin(a) * inner,
    x2: 0.5 + Math.cos(a) * (R_OUT - 0.02), y2: 0.5 + Math.sin(a) * (R_OUT - 0.02),
    on: i < lit.value,
    major,
  }
}))
const cap = computed(() => {
  if (!lit.value) return null
  const a = ((lit.value - 0.5) / props.ticks) * Math.PI * 2 - Math.PI / 2
  return { x: 0.5 + Math.cos(a) * 0.53, y: 0.5 + Math.sin(a) * 0.53 }
})

function play() {
  if (!root.value || motionSkipped()) return
  const on = root.value.querySelectorAll('.ring-tick.is-on')
  const capEl = root.value.querySelector('.ring-cap')
  const tl = gsap.timeline()
  tl.fromTo(on, { opacity: 0.15 }, { opacity: 1, duration: 0.28, ease: 'power1.out', stagger: { each: 0.014, from: 'start' }, clearProps: 'opacity' })
  if (capEl) tl.fromTo(capEl, { scale: 0.2, transformOrigin: '50% 50%' }, { scale: 1, duration: 0.5, ease: EASE.settle, clearProps: 'transform' }, '-=0.1')
}

let played = false
watch(() => props.value, (value) => {
  if (played || value === null || value === undefined) return
  played = true
  nextTick(play)
}, { immediate: true })
onBeforeUnmount(() => gsap.killTweensOf(root.value?.querySelectorAll('*') ?? []))
</script>

<template>
  <div class="tick-ring" :style="{ width: `${size}px`, height: `${size}px` }">
    <svg ref="root" viewBox="0 0 1 1" :width="size" :height="size" role="img" :aria-label="`${label ?? ''} ${pct}%`">
      <line
        v-for="(m, i) in marks"
        :key="i"
        :class="['ring-tick', { 'is-on': m.on, 'is-major': m.major }]"
        :x1="m.x1" :y1="m.y1" :x2="m.x2" :y2="m.y2"
      />
      <circle v-if="cap" class="ring-cap" :cx="cap.x" :cy="cap.y" r=".022" />
    </svg>
    <div class="tick-ring-center">
      <span class="tick-ring-value"><Odometer :value="display ?? value" :decimals="decimals" /><sup v-if="unit">{{ unit }}</sup></span>
      <span v-if="label" class="tick-ring-label">{{ label }}</span>
    </div>
  </div>
</template>
