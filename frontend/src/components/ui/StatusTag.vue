<script setup lang="ts">
import Tag from './Tag.vue'

withDefaults(defineProps<{
  status: number | boolean
  on?: string
  off?: string
}>(), {
  on: '正常',
  off: '停用',
})
</script>

<template>
  <Tag :tone="status === 1 || status === true ? 'ok' : 'neutral'" dot>{{ status === 1 || status === true ? on : off }}</Tag>
</template>
