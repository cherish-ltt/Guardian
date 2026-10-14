<script setup lang="ts">
import { computed } from 'vue'
import Tag from '@/components/ui/Tag.vue'
import { useDict } from '@/composables/dict'

/* 字典值展示：按字典数据的标签文本与样式渲染，找不到时原样显示。 */
const props = defineProps<{
  type: string
  value: string | number | null | undefined
}>()

const dict = useDict(props.type)
const item = computed(() => dict.value.find(d => d.value === String(props.value ?? '')))
</script>

<template>
  <Tag :tone="item?.tone" dot>{{ item?.label ?? value ?? '—' }}</Tag>
</template>
