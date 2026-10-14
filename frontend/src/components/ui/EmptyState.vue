<script setup lang="ts">
import Icon from '@/components/Icon.vue'

withDefaults(defineProps<{
  title?: string
  desc?: string
  icon?: string
}>(), {
  title: '暂无数据',
  icon: 'search',
})
</script>

<template>
  <div class="empty">
    <span class="empty-ico m-groove"><Icon :name="icon" :size="20" /></span>
    <div class="empty-title">{{ title }}</div>
    <div v-if="desc">{{ desc }}</div>
    <slot />
  </div>
</template>
