<script setup lang="ts">
import { ref } from 'vue'
import Icon from '@/components/Icon.vue'
import { Button, Checkbox, Field, Modal, Tag, TextInput, type Column } from '@/components/ui'
import { ListPage, PermissionButton } from '@/components/business'
import GeneratorPipeline from '@/components/business/GeneratorPipeline.vue'
import { portalApi } from '@/api'
import { useListPage } from '@/composables/list'
import { pushToast, toastError } from '@/composables/toast'
import { downloadBlob } from '@/utils/download'

interface TableRow { id: number; tableName: string; tableComment: string; className: string; module: string; businessName: string; synced: boolean; createTime: string }

const list = useListPage<TableRow, { keyword: string }>({
  query: { keyword: '' },
  fetch: params => portalApi.generator.tables.list(params) as Promise<any>,
})

const columns: Column[] = [
  { key: 'tableName', title: '数据表', minWidth: 220 },
  { key: 'className', title: '实体类' },
  { key: 'module', title: '模块', width: 100 },
  { key: 'synced', title: '状态', width: 100 },
  { key: 'createTime', title: '导入时间', width: 150 },
  { key: 'actions', title: '操作', width: 210, className: 'col-actions' },
]

/* ── 导入 ── */
const importOpen = ref(false)
const importKeyword = ref('')
const importLoading = ref(false)
const importing = ref(false)
const candidates = ref<Array<{ tableName: string; tableComment: string; imported: boolean }>>([])
const picked = ref<string[]>([])

async function openImport() {
  importOpen.value = true
  picked.value = []
  await loadCandidates()
}

async function loadCandidates() {
  importLoading.value = true
  try {
    candidates.value = (await portalApi.generator.tables.importable({ keyword: importKeyword.value })) as any
  } catch (error) {
    toastError(error, '数据库表加载失败')
  } finally {
    importLoading.value = false
  }
}

function togglePick(name: string) {
  picked.value = picked.value.includes(name) ? picked.value.filter(n => n !== name) : [...picked.value, name]
}

async function doImport() {
  if (!picked.value.length) return
  importing.value = true
  try {
    await portalApi.generator.tables.import({ tableNames: picked.value })
    pushToast(`已导入 ${picked.value.length} 张表`)
    importOpen.value = false
    await list.search()
  } catch (error) {
    toastError(error, '导入失败')
  } finally {
    importing.value = false
  }
}

/* ── 流水线：表信息 → 字段映射 → 代码预览 ── */
const pipeTable = ref<TableRow | null>(null)
const pipeStep = ref<1 | 2 | 3>(1)
function openPipeline(row: TableRow, step: 1 | 2 | 3) {
  pipeStep.value = step
  pipeTable.value = row
}
async function onPipelineSaved() {
  await list.load()
  if (pipeTable.value) pipeTable.value = list.rows.value.find(r => r.id === pipeTable.value!.id) ?? pipeTable.value
}

/* ── 生成 ── */
const generating = ref<number | null>(null)
async function generate(row: TableRow) {
  generating.value = row.id
  try {
    await portalApi.generator.tables.generate(row.id)
    const blob = await portalApi.generator.tables.download(row.id)
    downloadBlob(blob, `${row.className}.zip`)
    pushToast(`已生成「${row.tableComment || row.tableName}」代码包`)
  } catch (error) {
    toastError(error, '代码生成失败')
  } finally {
    generating.value = null
  }
}

</script>

<template>
  <ListPage
    title="代码生成"
    desc="导入数据库表，预览并下载后端四层、前端页面与菜单权限 SQL，作为新业务模块的起点。"
    table-title="已导入的表"
    :list="list"
    :columns="columns"
    :min-width="960"
  >
    <template #actions>
      <PermissionButton perm="tool:gen:list" variant="primary" icon="upload" @click="openImport">导入数据表</PermissionButton>
    </template>

    <template #search>
      <Field label="关键词" wide>
        <TextInput v-model="list.query.keyword" icon="search" placeholder="表名 / 表说明" clearable @enter="list.search()" />
      </Field>
    </template>

    <template #cell-tableName="{ row }">
      <div class="cell-main">
        <span class="row-ico m-groove"><Icon name="database" :size="16" /></span>
        <div style="min-width: 0">
          <div class="cell-title mono" style="font-size: 12.5px">{{ row.tableName }}</div>
          <div class="cell-sub ellipsis">{{ row.tableComment || '未填写表说明' }}</div>
        </div>
      </div>
    </template>
    <template #cell-className="{ row }"><span class="mono">{{ row.className }}</span></template>
    <template #cell-module="{ row }"><span class="code-chip">{{ row.module }}</span></template>
    <template #cell-synced="{ row }"><Tag :tone="row.synced ? 'ok' : 'neutral'" dot>{{ row.synced ? '已确认' : '待确认' }}</Tag></template>
    <template #cell-createTime="{ row }"><span class="tnum soft">{{ row.createTime }}</span></template>
    <template #cell-actions="{ row }">
      <div class="row-actions">
        <Button variant="text" size="sm" icon="eye" @click="openPipeline(row, 3)">预览</Button>
        <Button variant="text" size="sm" icon="sliders" @click="openPipeline(row, 1)">配置</Button>
        <Button variant="text" size="sm" icon="download" :loading="generating === row.id" @click="generate(row)">生成</Button>
      </div>
    </template>

    <!-- 导入 -->
    <Modal :open="importOpen" title="导入数据表" sub="从当前数据库选择要生成代码的表" :width="620" @close="importOpen = false">
      <form class="import-search" @submit.prevent="loadCandidates">
        <TextInput v-model="importKeyword" icon="search" placeholder="表名 / 表说明" clearable />
        <Button type="submit" :loading="importLoading">查询</Button>
      </form>
      <div class="import-list m-groove">
        <div v-if="importLoading && !candidates.length" style="padding: 12px">
          <span v-for="i in 5" :key="i" class="skeleton" :style="{ width: `${50 + (i * 11) % 40}%`, margin: '12px 0' }" />
        </div>
        <div
          v-for="c in candidates"
          v-else
          :key="c.tableName"
          :class="['import-row', picked.includes(c.tableName) && 'is-on', c.imported && 'is-disabled']"
          @click="!c.imported && togglePick(c.tableName)"
        >
          <Checkbox :model-value="picked.includes(c.tableName)" :disabled="c.imported" @update:model-value="togglePick(c.tableName)" />
          <span class="mono" style="font-size: 12.5px">{{ c.tableName }}</span>
          <span class="muted ellipsis" style="flex: 1">{{ c.tableComment || '未填写表说明' }}</span>
          <Tag v-if="c.imported">已导入</Tag>
        </div>
        <div v-if="!importLoading && !candidates.length" class="select-empty" style="padding: 28px">没有可导入的表</div>
      </div>
      <template #footer>
        <span class="soft" style="margin-right: auto; font-size: 12px">已选 {{ picked.length }} 张</span>
        <Button variant="ghost" @click="importOpen = false">取消</Button>
        <Button variant="primary" :disabled="!picked.length" :loading="importing" @click="doImport">导入</Button>
      </template>
    </Modal>

    <GeneratorPipeline :table="pipeTable" :step="pipeStep" @close="pipeTable = null" @saved="onPipelineSaved" @generate="generate(pipeTable!)" />
  </ListPage>
</template>
