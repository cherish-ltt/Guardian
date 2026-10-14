<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { gsap, EASE, motionSkipped } from '@/composables/motion'
import { buildHeatGrid, type HeatCell, type HeatGridCell } from '@/utils/heatmap'

/* ============================================================
   星期 × 小时热力格：格子按分位分 5 级（青柠由浅到深），失败占比高的格子叠斜线（--hatch）。
   入场从左上角向右下对角线波纹点亮（导 Conduct）；悬停十字高亮整行整列并显示读数。
   键盘：格子可聚焦，方向键移动。
   ============================================================ */
const props = defineProps<{ cells: HeatCell[]; loading?: boolean }>()

const DAYS = ['周一', '周二', '周三', '周四', '周五', '周六', '周日']
const data = computed(() => buildHeatGrid(props.cells))
const hover = ref<HeatGridCell | null>(null)
const root = ref<HTMLElement | null>(null)

function play() {
  if (!root.value || motionSkipped()) return
  const cells = root.value.querySelectorAll('.heat-cell')
  gsap.fromTo(cells, { scale: 0.2, opacity: 0 }, {
    scale: 1, opacity: 1, duration: 0.5, ease: EASE.reveal,
    stagger: { grid: [7, 24], from: [0, 0], amount: 0.9 },
    clearProps: 'transform,opacity', overwrite: 'auto',
  })
}
watch(() => props.cells, () => nextTick(play), { immediate: true })

function onKey(event: KeyboardEvent, cell: HeatGridCell) {
  const move = { ArrowLeft: [0, -1], ArrowRight: [0, 1], ArrowUp: [-1, 0], ArrowDown: [1, 0] }[event.key]
  if (!move) return
  event.preventDefault()
  const dow = Math.min(6, Math.max(0, cell.dow + move[0]))
  const hour = Math.min(23, Math.max(0, cell.hour + move[1]))
  root.value?.querySelector<HTMLElement>(`[data-cell="${dow}-${hour}"]`)?.focus()
}
const label = (c: HeatGridCell) => `${DAYS[c.dow]} ${String(c.hour).padStart(2, '0')}:00，登录 ${c.total} 次${c.failed ? `，失败 ${c.failed} 次` : ''}`
</script>

<template>
  <div ref="root" class="heat" :class="{ 'is-loading': loading }" @pointerleave="hover = null">
    <div class="heat-hours" aria-hidden="true">
      <span v-for="h in 24" :key="h" :class="{ 'is-on': hover?.hour === h - 1 }">{{ (h - 1) % 3 === 0 ? String(h - 1).padStart(2, '0') : '' }}</span>
    </div>
    <div v-for="(row, dow) in data.grid" :key="dow" class="heat-row" role="row">
      <span :class="['heat-day', { 'is-on': hover?.dow === dow }]">{{ DAYS[dow] }}</span>
      <button
        v-for="cell in row"
        :key="cell.hour"
        type="button"
        :data-cell="`${cell.dow}-${cell.hour}`"
        :class="['heat-cell', `lv-${cell.level}`, { 'is-risky': cell.risky, 'is-cross': hover && (hover.dow === cell.dow || hover.hour === cell.hour), 'is-hover': hover === cell }]"
        :aria-label="label(cell)"
        @pointerenter="hover = cell"
        @focus="hover = cell"
        @keydown="onKey($event, cell)"
      />
    </div>
    <div class="heat-foot">
      <span class="heat-readout" aria-live="polite">
        <template v-if="hover">{{ label(hover) }}</template>
        <template v-else-if="data.peak">高峰：{{ DAYS[data.peak.dow] }} {{ String(data.peak.hour).padStart(2, '0') }}:00 · {{ data.peak.total }} 次</template>
        <template v-else>所选范围内暂无登录</template>
      </span>
      <span class="heat-legend" aria-hidden="true">少<i class="lv-1" /><i class="lv-2" /><i class="lv-3" /><i class="lv-4" />多<i class="is-risky" />失败集中</span>
    </div>
  </div>
</template>
