<script setup lang="ts">
/* 页面头：大标题 + 说明；右侧放规格铭牌（#spec）与主操作（默认插槽）。位置信息由顶栏面包屑承担，不再放英文眉标 */
defineProps<{
  title: string
  desc?: string
}>()
</script>

<template>
  <header class="page-head reveal">
    <div class="page-head-main">
      <h1 class="page-title">{{ title }}</h1>
      <p v-if="desc" class="page-desc">{{ desc }}</p>
    </div>
    <div v-if="$slots.spec || $slots.default" class="page-head-side">
      <slot name="spec" />
      <div v-if="$slots.default" class="page-actions"><slot /></div>
    </div>
  </header>
</template>
