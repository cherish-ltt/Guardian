<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from 'vue'
import Icon from '@/components/Icon.vue'
import Tag from '@/components/ui/Tag.vue'
import StatusTag from '@/components/ui/StatusTag.vue'
import Switch from '@/components/ui/Switch.vue'
import IconButton from '@/components/ui/IconButton.vue'
import PermissionButton from './PermissionButton.vue'
import { systemApi } from '@/api'
import { flipLayout, revealCascade, settlePop } from '@/composables/motion'
import { invalidateDict } from '@/composables/dict'
import { hasPermission } from '@/directives/permission'
import { toastError } from '@/composables/toast'

/* ============================================================
   字典样本册：每个字典类型一张卡，直接渲染真实标签样式与色调分布条。
   点卡片原地展开（Flip 共享布局），行内切换色调 / 默认项 / 状态即时生效；
   新增与完整编辑仍走页面的统一表单。
   ============================================================ */
export interface DictTypeRow { id: number; dictName: string; dictType: string; status: number; remark?: string }
export interface DictItem { id: number; label: string; value: string; tone?: string; sort: number; def: boolean; status: number; remark?: string }

const props = defineProps<{ types: DictTypeRow[]; loading?: boolean }>()
const emit = defineEmits<{
  (e: 'create-item', type: DictTypeRow): void
  (e: 'edit-item', type: DictTypeRow, item: DictItem): void
  (e: 'edit-type', type: DictTypeRow): void
}>()

const TONES = [
  { value: 'neutral', label: '中性' },
  { value: 'ok', label: '状态' },
  { value: 'info', label: '信息' },
  { value: 'danger', label: '警示' },
] as const
const TONE_OF: Record<string, string> = { ok: 'ok', success: 'ok', primary: 'info', info: 'info', purple: 'info', danger: 'danger', error: 'danger', warn: 'danger', warning: 'danger' }
const toneOf = (t?: string) => TONE_OF[t ?? ''] ?? 'neutral'

const items = reactive<Record<string, DictItem[]>>({})
const expanded = ref<string | null>(null)
const grid = ref<HTMLElement | null>(null)
const canEdit = hasPermission('system:dict:edit')

async function load(dictType: string) {
  try {
    items[dictType] = ((await systemApi.dictData.list(dictType)) as unknown as DictItem[]).sort((a, b) => a.sort - b.sort)
  } catch (error) {
    toastError(error, '字典数据加载失败')
  }
}
watch(() => props.types, async (types) => {
  await Promise.all(types.filter(t => !items[t.dictType]).map(t => load(t.dictType)))
  await nextTick()
  revealCascade(grid.value, '.swatch', 0.05)
}, { immediate: true })

const distribution = (list: DictItem[] = []) => {
  const total = list.length || 1
  return TONES.map(t => ({ tone: t.value, share: list.filter(i => toneOf(i.tone) === t.value).length / total })).filter(d => d.share > 0)
}

async function toggle(type: DictTypeRow) {
  const cards = grid.value?.querySelectorAll('.swatch') ?? []
  await flipLayout(cards, async () => {
    expanded.value = expanded.value === type.dictType ? null : type.dictType
    await nextTick()
  })
  if (expanded.value) {
    await nextTick()
    const card = grid.value?.querySelector(`[data-dict="${type.dictType}"]`)
    revealCascade(card?.querySelector('.swatch-editor') ?? null, '.swatch-row', 0.04)
    card?.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
  }
}

/** 行内修改：先更新界面，接口失败再回滚 */
async function patch(type: DictTypeRow, item: DictItem, change: Partial<Pick<DictItem, 'tone' | 'def' | 'status'>>, el?: EventTarget | null) {
  const before = { tone: item.tone, def: item.def, status: item.status }
  Object.assign(item, change)
  if (change.def) items[type.dictType].forEach((i) => { if (i.id !== item.id) i.def = false })
  try {
    await systemApi.dictData.update(item.id, { dictType: type.dictType, ...change })
    invalidateDict(type.dictType)
    if (change.def) await load(type.dictType)
    settlePop((el as Element | null)?.closest('.swatch-row')?.querySelector('.tag') ?? null)
  } catch (error) {
    Object.assign(item, before)
    toastError(error, '字典数据更新失败')
  }
}

defineExpose({ reload: load })
const sorted = computed(() => props.types)
</script>

<template>
  <div ref="grid" class="swatchbook">
    <p v-if="loading && !types.length" class="band-empty faint">正在加载字典…</p>
    <article
      v-for="t in sorted"
      :key="t.dictType"
      :data-dict="t.dictType"
      :class="['swatch m-surface', { 'is-open': expanded === t.dictType, 'is-off': t.status !== 1, lift: expanded !== t.dictType }]"
    >
      <button type="button" class="swatch-head" :aria-expanded="expanded === t.dictType" @click="toggle(t)">
        <span class="swatch-title">
          <b>{{ t.dictName }}</b>
          <span class="code-chip">{{ t.dictType }}</span>
        </span>
        <span class="swatch-count tnum">{{ items[t.dictType]?.length ?? '—' }}<small>项</small></span>
        <Icon :name="expanded === t.dictType ? 'chevronDown' : 'chevronRight'" :size="15" class="swatch-chevron" />
      </button>
      <div class="swatch-tags">
        <template v-for="i in (items[t.dictType] ?? []).slice(0, expanded === t.dictType ? 0 : 8)" :key="i.id">
          <Tag :tone="i.tone" dot :class="{ 'is-off': i.status !== 1 }">{{ i.label }}<Icon v-if="i.def" name="check" :size="11" /></Tag>
        </template>
        <span v-if="expanded !== t.dictType && (items[t.dictType]?.length ?? 0) > 8" class="faint" style="font-size: 12px">+{{ items[t.dictType]!.length - 8 }}</span>
        <span v-if="items[t.dictType] && !items[t.dictType].length" class="faint" style="font-size: 12px">还没有字典数据</span>
      </div>
      <div class="swatch-bar" aria-hidden="true">
        <i v-for="d in distribution(items[t.dictType])" :key="d.tone" :class="`tone-${d.tone}`" :style="{ flexGrow: d.share }" />
      </div>
      <footer v-if="expanded !== t.dictType" class="swatch-foot">
        <StatusTag :status="t.status" />
        <span class="faint ellipsis">{{ t.remark || '未填写用途说明' }}</span>
      </footer>

      <div v-if="expanded === t.dictType" class="swatch-editor">
        <div v-for="i in items[t.dictType] ?? []" :key="i.id" class="swatch-row">
          <Tag :tone="i.tone" dot :class="{ 'is-off': i.status !== 1 }">{{ i.label }}</Tag>
          <span class="code-chip">{{ i.value }}</span>
          <span class="swatch-tones" role="radiogroup" :aria-label="`${i.label} 的标签色调`">
            <button
              v-for="tone in TONES"
              :key="tone.value"
              type="button"
              role="radio"
              :aria-checked="toneOf(i.tone) === tone.value"
              :aria-label="tone.label"
              :title="tone.label"
              :disabled="!canEdit"
              :class="['swatch-tone', `tone-${tone.value}`, { 'is-on': toneOf(i.tone) === tone.value }]"
              @click="patch(t, i, { tone: tone.value }, $event.currentTarget)"
            />
          </span>
          <button type="button" :class="['swatch-def', { 'is-on': i.def }]" :disabled="!canEdit || i.def" :aria-pressed="i.def" :title="i.def ? '默认项' : '设为默认项'" @click="patch(t, i, { def: true }, $event.currentTarget)">
            <Icon name="check" :size="12" />{{ i.def ? '默认' : '设为默认' }}
          </button>
          <Switch :model-value="i.status === 1" :disabled="!canEdit" :label="i.status === 1 ? '启用' : '停用'" @update:model-value="patch(t, i, { status: $event ? 1 : 0 }, null)" />
          <IconButton v-if="canEdit" name="edit" label="编辑字典数据" small @click="emit('edit-item', t, i)" />
        </div>
        <div class="swatch-actions">
          <PermissionButton perm="system:dict:edit" variant="ghost" size="sm" icon="edit" @click="emit('edit-type', t)">编辑类型</PermissionButton>
          <PermissionButton perm="system:dict:add" variant="secondary" size="sm" icon="plus" @click="emit('create-item', t)">新增数据</PermissionButton>
        </div>
      </div>
    </article>
  </div>
</template>
