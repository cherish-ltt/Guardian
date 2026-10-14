<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import Icon from '@/components/Icon.vue'
import TextInput from '@/components/ui/TextInput.vue'
import { deptTree, ensureDepts } from '@/composables/dept'
import type { Dept } from '@/data'

/* 部门树：左栏筛选用。选中节点即 emit 部门 ID，再点一次取消。 */
const props = defineProps<{ modelValue: number | null; data?: Dept[] }>()
const emit = defineEmits<{ (e: 'update:modelValue', value: number | null): void }>()

const keyword = ref('')
const expanded = ref(new Set<number>())
const nodes = computed(() => props.data ?? deptTree.value)

onMounted(async () => {
  if (!props.data) await ensureDepts()
})
watch(nodes, rows => { expanded.value = new Set(rows.map(d => d.id)) }, { immediate: true })

function matches(node: Dept): boolean {
  const key = keyword.value.trim()
  if (!key) return true
  return node.name.includes(key) || (node.children ?? []).some(matches)
}

interface Row { node: Dept; depth: number; hasChildren: boolean; open: boolean }
const rows = computed<Row[]>(() => {
  const list: Row[] = []
  const searching = Boolean(keyword.value.trim())
  const walk = (nodes: Dept[], depth: number) => nodes.filter(matches).forEach(node => {
    const children = node.children ?? []
    const open = searching || expanded.value.has(node.id)
    list.push({ node, depth, hasChildren: children.length > 0, open })
    if (open) walk(children, depth + 1)
  })
  walk(nodes.value, 0)
  return list
})

function toggle(id: number) {
  const next = new Set(expanded.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  expanded.value = next
}

function select(id: number) {
  emit('update:modelValue', props.modelValue === id ? null : id)
}
</script>

<template>
  <div>
    <div class="split-side-head">
      <TextInput v-model="keyword" icon="search" placeholder="搜索部门" clearable />
    </div>
    <div class="tree" role="tree" aria-label="部门">
      <div
        v-for="r in rows"
        :key="r.node.id"
        role="treeitem"
        tabindex="0"
        :aria-selected="modelValue === r.node.id"
        :aria-expanded="r.hasChildren ? r.open : undefined"
        :class="['tree-row', modelValue === r.node.id && 'is-selected']"
        :style="{ paddingLeft: `${4 + r.depth * 16}px` }"
        @click="select(r.node.id)"
        @keydown.enter.prevent="select(r.node.id)"
        @keydown.space.prevent="select(r.node.id)"
      >
        <button
          type="button"
          :class="['tree-toggle', !r.hasChildren && 'is-leaf', r.open && 'is-open']"
          :aria-label="r.open ? '收起' : '展开'"
          tabindex="-1"
          @click.stop="toggle(r.node.id)"
        ><Icon name="chevronRight" :size="13" /></button>
        <span class="tree-label">{{ r.node.name }}</span>
        <span v-if="r.node.status !== 1" class="tree-count">停用</span>
      </div>
      <div v-if="!rows.length" class="empty" style="padding: 24px 8px">无匹配部门</div>
    </div>
  </div>
</template>
