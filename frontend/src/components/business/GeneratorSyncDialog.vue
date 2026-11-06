<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { Button, Tag } from '@/components/ui'
import { FormDialog } from '@/components/business'
import { authSession, portalApi, type GeneratorSyncColumn, type GeneratorSyncPreview } from '@/api'
import { sessionVersion } from '@/composables/auth'
import { pushToast } from '@/composables/toast'

const props = defineProps<{ table: { id: number; tableName: string } | null }>()
const emit = defineEmits<{ (event: 'close'): void; (event: 'saved'): void }>()
const preview = ref<GeneratorSyncPreview | null>(null)
const loading = ref(false)
const saving = ref(false)
const error = ref('')
let requestId = 0
let dialogToken: string | null = null
let dialogVersion = 0

function sessionIsCurrent() {
  return dialogToken !== null && dialogVersion === sessionVersion.value && dialogToken === authSession.token()
}

onBeforeUnmount(() => {
  requestId++
  dialogToken = null
})

watch(() => props.table, () => {
  requestId++
  dialogToken = props.table ? authSession.token() : null
  dialogVersion = sessionVersion.value
  preview.value = null
  error.value = ''
  loading.value = false
  saving.value = false
  if (props.table) load()
}, { immediate: true })

watch(sessionVersion, () => {
  requestId++
  dialogToken = null
  preview.value = null
  loading.value = false
  saving.value = false
  emit('close')
})

async function load() {
  if (!props.table || saving.value || !sessionIsCurrent()) return
  const id = props.table.id
  const current = ++requestId
  loading.value = true
  preview.value = null
  error.value = ''
  try {
    const result = await portalApi.generator.tables.syncPreview(id)
    if (current === requestId && sessionIsCurrent()) preview.value = result
  } catch (cause) {
    if (current === requestId && sessionIsCurrent()) error.value = cause instanceof Error ? cause.message : '结构差异加载失败'
  } finally {
    if (current === requestId && sessionIsCurrent()) loading.value = false
  }
}

function close() {
  if (saving.value) return
  requestId++
  dialogToken = null
  emit('close')
}

async function synchronize() {
  if (!props.table || !preview.value?.changes.length || loading.value || saving.value || !sessionIsCurrent()) return
  const id = props.table.id
  const current = ++requestId
  const revision = preview.value.revision
  saving.value = true
  error.value = ''
  try {
    const result = await portalApi.generator.tables.sync(id, revision)
    if (current !== requestId || !sessionIsCurrent()) return
    pushToast(`已同步 ${result.changed} 个字段，请重新确认生成配置`)
    emit('saved')
    emit('close')
  } catch (cause) {
    if (current === requestId && sessionIsCurrent()) {
      error.value = cause instanceof Error ? cause.message : '结构同步失败'
      preview.value = null
    }
  } finally {
    if (current === requestId && sessionIsCurrent()) saving.value = false
  }
}

const labels = { columnType: '类型', nullable: '可空性', primaryKey: '主键', sort: '顺序' }
const kinds = { added: '新增', removed: '删除', changed: '变更' }

function describe(column: GeneratorSyncColumn | null) {
  if (!column) return '无'
  return `${column.columnType || '未声明类型'} · ${column.nullable ? '可空' : '非空'} · ${column.primaryKey ? '主键' : '普通列'} · 第 ${column.sort + 1} 列`
}

function derived(column: GeneratorSyncColumn | null) {
  if (!column) return '无'
  const flags = [column.required ? '必填' : '非必填', column.insert ? '参与新增' : '', column.edit ? '参与编辑' : '', column.list ? '参与列表' : ''].filter(Boolean)
  return `${column.rustType} · ${column.formType} · ${flags.join(' / ')}`
}
</script>

<template>
  <FormDialog
    :open="!!table"
    title="同步表结构"
    :sub="table?.tableName"
    :width="860"
    :saving="saving"
    :submit-disabled="loading || !preview?.changes.length"
    submit-text="确认同步"
    @close="close"
    @submit="synchronize"
  >
    <template #extra>
      <p class="soft">同步将更新生成配置，并将状态设为待确认。业务表和已有代码文件保持原样；字段重命名按删除和新增处理。</p>
      <p v-if="loading" role="status">正在读取结构差异…</p>
      <div v-else-if="error" role="alert">
        <p>{{ error }}</p>
        <Button @click="load">重新预览</Button>
      </div>
      <p v-else-if="preview && !preview.changes.length" role="status">表结构没有变化，无需同步。</p>
      <div v-else-if="preview" class="sync-changes">
        <article v-for="change in preview.changes" :key="change.columnName" class="sync-change m-groove">
          <div class="sync-heading">
            <strong class="mono">{{ change.columnName }}</strong>
            <Tag :tone="change.kind === 'removed' ? 'danger' : 'neutral'">{{ kinds[change.kind] }}</Tag>
            <span class="soft">{{ change.changedAttributes.map(key => labels[key]).join('、') }}</span>
          </div>
          <dl class="sync-values">
            <dt>当前配置</dt><dd>{{ describe(change.before) }}</dd>
            <dt>同步后</dt><dd>{{ describe(change.after) }}</dd>
            <dt>当前推导项</dt><dd>{{ derived(change.before) }}</dd>
            <dt>同步后推导项</dt><dd>{{ derived(change.after) }}</dd>
          </dl>
        </article>
      </div>
    </template>
  </FormDialog>
</template>

<style scoped>
.sync-changes { display: grid; gap: 12px; margin-top: 16px; }
.sync-change { padding: 14px; border-radius: 12px; }
.sync-heading { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
.sync-values { display: grid; grid-template-columns: 100px minmax(0, 1fr); gap: 8px 12px; margin-bottom: 0; font-size: 12px; }
.sync-values dt { opacity: .65; }
.sync-values dd { margin: 0; overflow-wrap: anywhere; }
</style>
