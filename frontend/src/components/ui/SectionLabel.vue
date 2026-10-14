<script setup lang="ts">
/* 区块标题：横带分层与仪器化面板的中文小标题（不再使用英文眉标与编号） */
defineProps<{
  title?: string
  as?: 'h2' | 'h3'
}>()
</script>

<template>
  <div class="section-mark">
    <component :is="as ?? 'h2'" v-if="title" class="section-mark-title">{{ title }}</component>
  </div>
</template>
