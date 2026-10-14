<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import Select from '@/components/ui/Select.vue'
import { systemApi } from '@/api'

/* 用户下拉：可搜索。valueKey 决定绑定的是用户 ID 还是昵称（部门负责人按昵称存储）。 */
const props = withDefaults(defineProps<{
  modelValue: string | number | null | undefined
  valueKey?: 'id' | 'nickname'
  placeholder?: string
  clearable?: boolean
}>(), {
  valueKey: 'id',
  placeholder: '请选择用户',
  clearable: true,
})
const emit = defineEmits<{ (e: 'update:modelValue', value: string | number | null): void }>()

const users = ref<Array<{ id: number; nickname: string; username: string }>>([])

onMounted(async () => {
  try {
    const page = await systemApi.users.list({ pageNum: 1, pageSize: 100, status: 1 })
    users.value = page.rows.map(u => ({ id: u.id, nickname: u.nickname, username: u.username }))
  } catch {
    users.value = []
  }
})

const options = computed(() => {
  const list = users.value.map(u => ({ label: u.nickname, value: props.valueKey === 'id' ? u.id : u.nickname, hint: u.username }))
  // 当前值不在列表中（例如历史数据）时仍然显示出来
  if (props.modelValue && !list.some(o => o.value === props.modelValue)) {
    list.unshift({ label: String(props.modelValue), value: props.modelValue, hint: '' })
  }
  return list
})
</script>

<template>
  <Select
    :model-value="modelValue"
    :options="options"
    :placeholder="placeholder"
    :clearable="clearable"
    filterable
    @update:model-value="emit('update:modelValue', $event === '' ? null : ($event ?? null))"
  />
</template>
