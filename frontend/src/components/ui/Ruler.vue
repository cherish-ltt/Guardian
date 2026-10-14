<script setup lang="ts">
/* 刻度尺：细刻度 + 主刻度标签，marker 为 0–1 的当前位置（一根亮刻度） */
withDefaults(defineProps<{
  labels?: string[]
  marker?: number | null
  markerLabel?: string
  minor?: number
}>(), {
  labels: () => [],
  marker: null,
  minor: 4,
})
</script>

<template>
  <div class="ruler" :style="{ '--ruler-major': Math.max(labels.length - 1, 1), '--ruler-minor': minor }" aria-hidden="true">
    <div class="ruler-ticks" />
    <div class="ruler-labels">
      <span v-for="(l, i) in labels" :key="i" :style="{ left: `${(i / Math.max(labels.length - 1, 1)) * 100}%` }">{{ l }}</span>
    </div>
    <span v-if="marker !== null" class="ruler-marker" :style="{ left: `${Math.min(Math.max(marker, 0), 1) * 100}%` }">
      <span v-if="markerLabel" class="ruler-marker-label tnum">{{ markerLabel }}</span>
    </span>
  </div>
</template>
