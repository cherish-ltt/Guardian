<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import Ruler from '@/components/ui/Ruler.vue'
import { gsap, motionSkipped, EASE } from '@/composables/motion'
import type { TimelineBlock } from '@/api'

/* 时间轴泳道：泳道 × 时间轴。默认聚焦到 [首条 −1h, 现在 +1h]（至少 6 小时），可切回全天。
   - 时间块从起点向右生长（导 Conduct），按泳道阅读序依次接通
   - 失败块红描边，耗时超过 slowMs 的块加斜线
   - 「现在」竖线每分钟推进一格，页面在后台时不推进 */
export interface SwimlaneLane {
  module: string
  count: number
  blocks: TimelineBlock[]
}

const props = withDefaults(defineProps<{
  lanes: SwimlaneLane[]
  nowSec?: number | null
  compact?: boolean
  maxLanes?: number
  slowMs?: number
}>(), {
  nowSec: null,
  compact: false,
  maxLanes: 8,
  slowMs: 1000,
})
const emit = defineEmits<{ (e: 'select', payload: { lane: SwimlaneLane; block: TimelineBlock }): void }>()

const DAY = 86400
const MIN_WIDTH = 0.5 // 百分比：单次操作也要看得见
const plot = ref<HTMLElement | null>(null)
const tip = ref<HTMLElement | null>(null)
const hover = ref<{ lane: SwimlaneLane; block: TimelineBlock } | null>(null)
const tipPos = ref({ x: 0, y: 0 })
const now = ref<number | null>(props.nowSec)
const shownLanes = computed(() => props.lanes.slice(0, props.maxLanes))
const hiddenCount = computed(() => Math.max(0, props.lanes.length - props.maxLanes))
const fmt = (sec: number) => `${String(Math.floor(sec / 3600)).padStart(2, '0')}:${String(Math.floor((sec % 3600) / 60)).padStart(2, '0')}`
const full = ref(false)

/* 时间窗：整点对齐，区间长度为刻度步长的整数倍，刻度均匀铺满标尺 */
const domain = computed(() => {
  const H = 3600
  const maxCols = props.compact ? 4 : 8
  const blocks = props.lanes.flatMap(l => l.blocks)
  if (full.value || !blocks.length) return { start: 0, end: DAY, step: DAY / maxCols }
  const first = Math.min(...blocks.map(b => b.startSec))
  const last = Math.max(...blocks.map(b => b.endSec), now.value ?? 0)
  let start = Math.max(0, Math.floor((first - H) / H) * H)
  let end = Math.min(DAY, Math.ceil((last + H) / H) * H)
  if (end - start < 6 * H) {
    const pad = 6 * H - (end - start)
    start = Math.max(0, start - Math.ceil(pad / 2 / H) * H)
    end = Math.min(DAY, start + 6 * H)
    start = Math.max(0, end - 6 * H)
  }
  const step = Math.ceil((end - start) / H / maxCols) * H
  end = Math.min(DAY, start + step * Math.ceil((end - start) / step))
  if (end - start < step) start = Math.max(0, end - step)
  return { start, end, step }
})
const span = computed(() => domain.value.end - domain.value.start)
const cols = computed(() => Math.max(1, Math.round(span.value / domain.value.step)))
const labels = computed(() => Array.from({ length: cols.value + 1 }, (_, i) => {
  const sec = domain.value.start + i * domain.value.step
  return props.compact ? String(Math.floor(sec / 3600)).padStart(2, '0') : fmt(Math.min(sec, DAY))
}))
const frac = (sec: number) => (sec - domain.value.start) / span.value

const nowLabel = computed(() => now.value === null ? '' : fmt(now.value))

function blockStyle(block: TimelineBlock) {
  const left = frac(block.startSec) * 100
  const width = Math.max(MIN_WIDTH, ((block.endSec - block.startSec) / span.value) * 100)
  return { left: `${Math.min(left, 100 - width)}%`, width: `${width}%` }
}

function onEnter(event: PointerEvent, lane: SwimlaneLane, block: TimelineBlock) {
  hover.value = { lane, block }
  const box = plot.value?.getBoundingClientRect()
  const target = (event.currentTarget as HTMLElement).getBoundingClientRect()
  if (!box) return
  tipPos.value = { x: target.left - box.left + target.width / 2, y: target.top - box.top }
}

/* 现在线：对齐到整分钟后每 60s 推进 */
let timer = 0
function tick() {
  if (now.value === null || document.hidden) return
  now.value = Math.min(DAY, now.value + 60)
}
watch(() => props.nowSec, (value) => { now.value = value })

function grow() {
  const blocks = plot.value?.querySelectorAll('.lane-block')
  if (!blocks?.length || motionSkipped()) return
  gsap.fromTo(blocks, { scaleX: 0, transformOrigin: '0% 50%' }, { scaleX: 1, duration: 0.6, ease: EASE.glide, stagger: { each: 0.018, from: 'start' }, clearProps: 'transform' })
}
let grown = false
watch(() => props.lanes.length, (n) => {
  if (grown || !n) return
  grown = true
  nextTick(grow)
}, { immediate: true })

onMounted(() => { timer = window.setInterval(tick, 60_000) })
onBeforeUnmount(() => window.clearInterval(timer))
</script>

<template>
  <div :class="['swimlane', { 'is-compact': compact }]">
    <div class="swim-head">
      <button type="button" class="swim-corner swim-zoom" :aria-pressed="full" :title="full ? '聚焦到有操作的时段' : '查看全天 00:00–24:00'" @click="full = !full">{{ full ? '全天' : '聚焦' }}</button>
      <Ruler :labels="labels" :marker="now === null ? null : frac(now)" :marker-label="compact ? undefined : nowLabel" :minor="compact ? 6 : 3" />
    </div>
    <div ref="plot" class="swim-body" @pointerleave="hover = null">
      <div class="swim-grid" :style="{ '--swim-cols': cols }" aria-hidden="true" />
      <div v-for="(lane, li) in shownLanes" :key="lane.module" :class="['swim-lane', `cat-${(li % 5) + 1}`]">
        <div class="swim-label">
          <span class="swim-name">{{ lane.module }}</span>
          <span class="swim-count tnum">{{ lane.count }} 次</span>
        </div>
        <div class="swim-track">
          <button
            v-for="(b, bi) in lane.blocks"
            :key="bi"
            type="button"
            :class="['lane-block', { 'is-failed': b.failed > 0, 'is-slow': b.maxCost >= slowMs }]"
            :style="blockStyle(b)"
            :aria-label="`${lane.module} ${fmt(b.startSec)}–${fmt(b.endSec)}，${b.count} 次${b.failed ? `，失败 ${b.failed}` : ''}`"
            @pointerenter="onEnter($event, lane, b)"
            @focus="onEnter($event as unknown as PointerEvent, lane, b)"
            @blur="hover = null"
            @click="emit('select', { lane, block: b })"
          ><span v-if="!compact && b.count > 1" class="lane-block-count tnum">{{ b.count }}</span></button>
        </div>
      </div>
      <div v-if="now !== null" class="swim-now" :style="{ left: `calc(var(--swim-label-w) + (100% - var(--swim-label-w)) * ${frac(now)})` }" aria-hidden="true" />
      <div v-if="hover" ref="tip" class="swim-tip m-float" :style="{ left: `${tipPos.x}px`, top: `${tipPos.y}px` }">
        <span class="eyebrow">{{ hover.lane.module }}</span>
        <b class="tnum">{{ fmt(hover.block.startSec) }} – {{ fmt(hover.block.endSec) }}</b>
        <span class="swim-tip-row"><span>操作</span><b class="tnum">{{ hover.block.count }} 次</b></span>
        <span class="swim-tip-row"><span>失败</span><b :class="['tnum', { 'is-red': hover.block.failed }]">{{ hover.block.failed }}</b></span>
        <span class="swim-tip-row"><span>最大耗时</span><b class="tnum">{{ hover.block.maxCost }} ms</b></span>
      </div>
      <p v-if="hiddenCount" class="swim-more faint">另有 {{ hiddenCount }} 个模块未显示</p>
    </div>
  </div>
</template>
