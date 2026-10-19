<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import Icon from '@/components/Icon.vue'
import { TextInput } from '@/components/ui'
import { collectDeptIds, deptTree, ensureDepts, flattenDepts } from '@/composables/dept'
import type { Dept } from '@/data'

/* 部门多选：数据权限的「自定义」范围用它勾选可见部门，可整棵子树一次勾选。 */
const props = defineProps<{ modelValue: number[] }>()
const emit = defineEmits<{ (e: 'update:modelValue', value: number[]): void }>()

const keyword = ref('')

onMounted(() => ensureDepts())

const selected = computed(() => new Set(props.modelValue))
const rows = computed(() => {
  const key = keyword.value.trim()
  const flat = flattenDepts(deptTree.value)
  if (!key) return flat
  // 搜索时保留命中项及其祖先，避免层级断掉
  const keep = new Set<number>()
  const walk = (nodes: Dept[], trail: number[]) => nodes.forEach(node => {
    const next = [...trail, node.id]
    if (node.name.includes(key)) next.forEach(id => keep.add(id))
    walk(node.children ?? [], next)
  })
  walk(deptTree.value, [])
  return flat.filter(({ dept }) => keep.has(dept.id))
})

function toggle(id: number) {
  const next = new Set(selected.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  emit('update:modelValue', [...next].sort((a, b) => a - b))
}

function toggleBranch(node: Dept) {
  const ids = collectDeptIds(node)
  const allOn = ids.every(id => selected.value.has(id))
  const next = new Set(selected.value)
  ids.forEach(id => (allOn ? next.delete(id) : next.add(id)))
  emit('update:modelValue', [...next].sort((a, b) => a - b))
}

function clear() {
  emit('update:modelValue', [])
}
</script>

<template>
  <div class="dept-picker">
    <div class="dept-picker-head">
      <TextInput v-model="keyword" icon="search" placeholder="搜索部门" clearable />
      <span class="dept-picker-count">已选 {{ modelValue.length }}</span>
      <button v-if="modelValue.length" type="button" class="dept-picker-clear" @click="clear">清空</button>
    </div>
    <div class="dept-picker-tree" role="group" aria-label="可见部门">
      <div v-for="row in rows" :key="row.dept.id" class="dept-picker-row" :style="{ paddingLeft: `${4 + row.depth * 16}px` }">
        <label class="dept-picker-label">
          <input
            type="checkbox"
            :checked="selected.has(row.dept.id)"
            :disabled="row.dept.status !== 1"
            @change="toggle(row.dept.id)"
          />
          <span>{{ row.dept.name }}</span>
          <span v-if="row.dept.status !== 1" class="dept-picker-off">停用</span>
        </label>
        <button
          v-if="row.dept.children?.length"
          type="button"
          class="dept-picker-branch"
          :aria-label="`整棵勾选 ${row.dept.name}`"
          title="整棵勾选 / 取消"
          @click="toggleBranch(row.dept)"
        ><Icon name="chevronRight" :size="12" /></button>
      </div>
      <div v-if="!rows.length" class="dept-picker-empty">无匹配部门</div>
    </div>
  </div>
</template>
