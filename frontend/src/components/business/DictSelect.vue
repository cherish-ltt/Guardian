<script setup lang="ts">
import { computed } from 'vue'
import Select from '@/components/ui/Select.vue'
import { useDict } from '@/composables/dict'

/* 字典下拉：选项来自「字典管理」中的字典数据，值统一为字符串。 */
const props = withDefaults(defineProps<{
  type: string
  modelValue: string | number | null | undefined
  placeholder?: string
  /** 查询区使用：在首项加一个「全部」 */
  allLabel?: string
  invalid?: boolean
  disabled?: boolean
}>(), {
  placeholder: '请选择',
})
const emit = defineEmits<{ (e: 'update:modelValue', value: string): void }>()

const dict = useDict(props.type)
const options = computed(() => [
  ...(props.allLabel ? [{ label: props.allLabel, value: '' }] : []),
  ...dict.value.map(item => ({ label: item.label, value: item.value })),
])
</script>

<template>
  <Select
    :model-value="modelValue === null || modelValue === undefined ? '' : String(modelValue)"
    :options="options"
    :placeholder="placeholder"
    :invalid="invalid"
    :disabled="disabled"
    @update:model-value="emit('update:modelValue', String($event ?? ''))"
  />
</template>
