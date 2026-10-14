<script setup lang="ts">
import { computed } from 'vue'
import Icon from '@/components/Icon.vue'
import Select from './Select.vue'

const props = withDefaults(defineProps<{
  total: number
  page: number
  pageSize: number
  sizes?: number[]
}>(), {
  sizes: () => [10, 20, 50, 100],
})

const emit = defineEmits<{
  (e: 'update:page', value: number): void
  (e: 'update:pageSize', value: number): void
}>()

const pages = computed(() => Math.max(1, Math.ceil(props.total / props.pageSize)))
const start = computed(() => (props.total === 0 ? 0 : (props.page - 1) * props.pageSize + 1))
const end = computed(() => Math.min(props.total, props.page * props.pageSize))
const sizeOptions = computed(() => props.sizes.map(size => ({ label: `${size} 条/页`, value: size })))

const nums = computed<(number | '…')[]>(() => {
  const list: (number | '…')[] = []
  for (let i = 1; i <= pages.value; i += 1) {
    if (i === 1 || i === pages.value || Math.abs(i - props.page) <= 1) list.push(i)
    else if (list[list.length - 1] !== '…') list.push('…')
  }
  return list
})

function go(value: number) {
  const next = Math.min(pages.value, Math.max(1, value))
  if (next !== props.page) emit('update:page', next)
}
</script>

<template>
  <nav class="pager" aria-label="分页">
    <div class="pager-info">共 <b class="tnum">{{ total }}</b> 条<template v-if="total">，当前 <b class="tnum">{{ start }}–{{ end }}</b></template></div>
    <div class="pager-ctrl">
      <div style="width: 112px; margin-right: 6px">
        <Select :model-value="pageSize" :options="sizeOptions" @update:model-value="emit('update:pageSize', Number($event))" />
      </div>
      <button type="button" class="pg-btn" :disabled="page <= 1" aria-label="上一页" @click="go(page - 1)">
        <Icon name="chevronLeft" :size="15" />
      </button>
      <template v-for="(n, i) in nums" :key="n === '…' ? `d${i}` : n">
        <span v-if="n === '…'" class="pg-dots">…</span>
        <button
          v-else
          type="button"
          :class="['pg-btn', 'tnum', n === page && 'is-active m-pill']"
          :aria-current="n === page ? 'page' : undefined"
          @click="go(n)"
        >{{ n }}</button>
      </template>
      <button type="button" class="pg-btn" :disabled="page >= pages" aria-label="下一页" @click="go(page + 1)">
        <Icon name="chevronRight" :size="15" />
      </button>
    </div>
  </nav>
</template>
