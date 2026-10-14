<script setup lang="ts">
/* 规格铭牌：深色玻璃小板，等宽键值行。只读读数，不带可点的样子 */
defineProps<{
  rows: Array<{ label: string; value: string | number | null | undefined; tone?: 'lime' | 'red' }>
  title?: string
}>()
</script>

<template>
  <dl class="spec-plate" :aria-label="title">
    <div v-if="title" class="spec-title">{{ title }}</div>
    <div v-for="r in rows" :key="r.label" class="spec-row">
      <dt>{{ r.label }}</dt>
      <dd :class="['tnum', r.tone && `is-${r.tone}`]">{{ r.value ?? '—' }}</dd>
    </div>
  </dl>
</template>
