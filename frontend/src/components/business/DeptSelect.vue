<script setup lang="ts">
import { computed, onMounted } from 'vue'
import Select from '@/components/ui/Select.vue'
import { deptTree, ensureDepts, flattenDepts } from '@/composables/dept'
import type { Dept } from '@/data'

/* 部门下拉：按组织层级缩进，可搜索。exclude 用于编辑部门时排除自身及下级。 */
const props = withDefaults(defineProps<{
  modelValue: number | null | undefined
  data?: Dept[]
  placeholder?: string
  /** 顶层选项（如「无上级部门」），值为 0 */
  rootLabel?: string
  excludeIds?: number[]
  invalid?: boolean
  clearable?: boolean
}>(), {
  placeholder: '请选择部门',
})
const emit = defineEmits<{ (e: 'update:modelValue', value: number | null): void }>()

onMounted(() => { if (!props.data) return ensureDepts() })

const options = computed(() => [
  ...(props.rootLabel ? [{ label: props.rootLabel, value: 0 }] : []),
  ...flattenDepts(props.data ?? deptTree.value).map(({ dept, depth }) => ({
    label: dept.name,
    value: dept.id,
    depth: depth + (props.rootLabel ? 1 : 0),
    disabled: props.excludeIds?.includes(dept.id) || dept.status !== 1,
  })),
])
</script>

<template>
  <Select
    :model-value="modelValue"
    :options="options"
    :placeholder="placeholder"
    :invalid="invalid"
    :clearable="clearable"
    filterable
    @update:model-value="emit('update:modelValue', $event === '' || $event === null || $event === undefined ? null : Number($event))"
  />
</template>
