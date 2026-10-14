<script setup lang="ts">
import { computed, ref } from 'vue'
import Icon from '@/components/Icon.vue'
import Checkbox from '@/components/ui/Checkbox.vue'
import type { Menu } from '@/data'

/* 菜单权限树：目录 / 菜单为树节点（勾选含全部下级），按钮权限渲染为节点下方的标签。 */
const props = defineProps<{
  menus: Menu[]
  modelValue: number[]
}>()
const emit = defineEmits<{ (e: 'update:modelValue', value: number[]): void }>()

const collapsed = ref(new Set<number>())
const checked = computed(() => new Set(props.modelValue))

// 子 → 父映射：勾选页面或按钮时连带勾上所在目录，否则导航里会出现脱离目录的孤立页面
const parentOf = computed(() => {
  const map = new Map<number, number>()
  const walk = (nodes: Menu[], parent: number) => nodes.forEach(node => {
    if (parent) map.set(node.id, parent)
    walk(node.children ?? [], node.id)
  })
  walk(props.menus, 0)
  return map
})

function addAncestors(set: Set<number>, id: number) {
  let parent = parentOf.value.get(id)
  while (parent) {
    set.add(parent)
    parent = parentOf.value.get(parent)
  }
}

function descendants(node: Menu): number[] {
  return (node.children ?? []).flatMap(child => [child.id, ...descendants(child)])
}

interface Row { node: Menu; depth: number; buttons: Menu[]; hasSubs: boolean }
const rows = computed<Row[]>(() => {
  const list: Row[] = []
  const walk = (nodes: Menu[], depth: number) => nodes.forEach(node => {
    if (node.type === 'btn') return
    const children = node.children ?? []
    const subs = children.filter(c => c.type !== 'btn')
    list.push({ node, depth, buttons: children.filter(c => c.type === 'btn'), hasSubs: subs.length > 0 })
    if (!collapsed.value.has(node.id)) walk(subs, depth + 1)
  })
  walk(props.menus, 0)
  return list
})

function state(node: Menu) {
  const ids = [node.id, ...descendants(node)]
  const on = ids.filter(id => checked.value.has(id)).length
  return { all: on === ids.length, some: on > 0 && on < ids.length }
}

function toggleNode(node: Menu) {
  const ids = [node.id, ...descendants(node)]
  const next = new Set(props.modelValue)
  if (state(node).all) ids.forEach(id => next.delete(id))
  else {
    ids.forEach(id => next.add(id))
    addAncestors(next, node.id)
  }
  emit('update:modelValue', [...next])
}

function toggleButton(btn: Menu, parent: Menu) {
  const next = new Set(props.modelValue)
  if (next.has(btn.id)) next.delete(btn.id)
  else {
    next.add(btn.id)
    next.add(parent.id) // 有按钮权限必须能进入页面
    addAncestors(next, parent.id)
  }
  emit('update:modelValue', [...next])
}

function toggleCollapse(id: number) {
  const next = new Set(collapsed.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  collapsed.value = next
}
</script>

<template>
  <div class="tree" role="tree" aria-label="菜单权限">
    <div v-for="r in rows" :key="r.node.id" role="treeitem" :aria-expanded="r.hasSubs ? !collapsed.has(r.node.id) : undefined">
      <div class="tree-row" :style="{ paddingLeft: `${4 + r.depth * 20}px`, cursor: 'default' }">
        <button
          type="button"
          :class="['tree-toggle', !r.hasSubs && 'is-leaf', !collapsed.has(r.node.id) && 'is-open']"
          :aria-label="collapsed.has(r.node.id) ? '展开' : '收起'"
          @click="toggleCollapse(r.node.id)"
        ><Icon name="chevronRight" :size="13" /></button>
        <Checkbox :model-value="state(r.node).all" :indeterminate="state(r.node).some" @update:model-value="toggleNode(r.node)">
          <Icon v-if="r.node.icon" :name="r.node.icon" :size="15" style="color: var(--ink-soft)" />
          <span :style="{ fontWeight: r.node.type === 'dir' ? 650 : 500 }">{{ r.node.name }}</span>
        </Checkbox>
      </div>
      <div v-if="r.buttons.length" class="perm-chips" :style="{ paddingLeft: `${54 + r.depth * 20}px` }">
        <button
          v-for="b in r.buttons"
          :key="b.id"
          type="button"
          :class="['perm-chip', checked.has(b.id) && 'is-on']"
          :aria-pressed="checked.has(b.id)"
          @click="toggleButton(b, r.node)"
        >
          <Icon v-if="checked.has(b.id)" name="check" :size="12" :stroke="2.4" />{{ b.name }}
        </button>
      </div>
    </div>
  </div>
</template>
