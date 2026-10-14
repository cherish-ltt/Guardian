<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import Icon from '@/components/Icon.vue'
import { DataTable, Drawer, Field, Segmented, Select, StatusTag, Switch, Tag, TextInput, type Column } from '@/components/ui'
import { DictSelect, FormDialog, ListPage, PermissionButton } from '@/components/business'
import DictSwatchbook from '@/components/business/DictSwatchbook.vue'
import { useViewMode } from '@/composables/view'
import { systemApi } from '@/api'
import { useListPage } from '@/composables/list'
import { confirm } from '@/composables/confirm'
import { invalidateDict } from '@/composables/dict'
import { pushToast, toastError } from '@/composables/toast'
import { pattern, required, validate } from '@/utils/validate'

interface TypeRow { id: number; dictName: string; dictType: string; status: number; remark?: string; createTime: string }
interface DataRow { id: number; label: string; value: string; tone?: string; sort: number; def: boolean; status: number; remark?: string }

const TONES = [
  { label: '中性', value: 'neutral' },
  { label: '状态（青柠）', value: 'ok' },
  { label: '信息（蓝）', value: 'info' },
  { label: '警示（红）', value: 'danger' },
]

const list = useListPage<TypeRow, { keyword: string; status: string }>({
  query: { keyword: '', status: '' },
  fetch: params => systemApi.dictTypes.list(params) as Promise<any>,
  exporter: params => systemApi.dictTypes.export(params),
  exportName: '字典类型.csv',
})

const columns: Column[] = [
  { key: 'dictName', title: '字典名称', minWidth: 160 },
  { key: 'dictType', title: '字典类型' },
  { key: 'status', title: '状态', width: 96 },
  { key: 'remark', title: '备注' },
  { key: 'createTime', title: '创建时间', width: 150 },
  { key: 'actions', title: '操作', width: 120, className: 'col-actions' },
]

/* ── 样本册视图：全部字典类型一屏对照，卡片原地展开编辑 ── */
const view = useViewMode('dict', 'book', ['book', 'table'] as const, 'table')
const VIEWS = [{ label: '样本册', value: 'book' as const }, { label: '表格', value: 'table' as const }]
const allTypes = ref<TypeRow[]>([])
const typesLoading = ref(false)
const book = ref<{ reload: (dictType: string) => Promise<void> } | null>(null)
async function loadAllTypes() {
  typesLoading.value = true
  try {
    allTypes.value = (await systemApi.dictTypes.list({ pageNum: 1, pageSize: 200 })).rows as unknown as TypeRow[]
  } catch (error) {
    toastError(error, '字典类型加载失败')
  } finally {
    typesLoading.value = false
  }
}
watch(view, (v) => { if (v === 'book') loadAllTypes() }, { immediate: true })

/* ── 字典类型 ── */
interface TypeForm { id?: number; dictName: string; dictType: string; status: number; remark: string }
const typeDialog = ref(false)
const typeSaving = ref(false)
const typeForm = reactive<TypeForm>({ dictName: '', dictType: '', status: 1, remark: '' })
const typeErrors = ref<Partial<Record<keyof TypeForm, string>>>({})

function openTypeCreate() {
  Object.assign(typeForm, { id: undefined, dictName: '', dictType: '', status: 1, remark: '' })
  typeErrors.value = {}
  typeDialog.value = true
}

function openTypeEdit(row: TypeRow) {
  Object.assign(typeForm, { id: row.id, dictName: row.dictName, dictType: row.dictType, status: row.status, remark: row.remark ?? '' })
  typeErrors.value = {}
  typeDialog.value = true
}

async function submitType() {
  typeErrors.value = validate(typeForm, {
    dictName: [required('请输入字典名称')],
    dictType: [required('请输入字典类型'), pattern(/^[a-z][a-z0-9_]*$/, '小写字母开头，仅含小写字母、数字、下划线')],
  })
  if (Object.keys(typeErrors.value).length) return
  typeSaving.value = true
  const payload = { dictName: typeForm.dictName.trim(), dictType: typeForm.dictType.trim(), status: typeForm.status, remark: typeForm.remark }
  try {
    if (typeForm.id) {
      await systemApi.dictTypes.update(typeForm.id, payload)
      pushToast('字典类型已更新')
    } else {
      await systemApi.dictTypes.create(payload)
      pushToast('字典类型已新增')
    }
    typeDialog.value = false
    await list.load()
    if (view.value === 'book') loadAllTypes()
  } catch (error) {
    toastError(error, '字典类型保存失败')
  } finally {
    typeSaving.value = false
  }
}

async function removeType(row: TypeRow) {
  const ok = await confirm({ title: '删除字典类型', message: `确认删除「${row.dictName}」（${row.dictType}）吗？使用该字典的下拉选项将无法显示。`, confirmText: '删除', danger: true })
  if (!ok) return
  try {
    await systemApi.dictTypes.remove(row.id)
    pushToast('字典类型已删除')
    await list.load()
    if (view.value === 'book') loadAllTypes()
  } catch (error) {
    toastError(error, '字典类型删除失败')
  }
}

/* ── 字典数据（抽屉） ── */
const current = ref<TypeRow | null>(null)
const items = ref<DataRow[]>([])
const itemsLoading = ref(false)
const itemColumns: Column[] = [
  { key: 'label', title: '标签' },
  { key: 'value', title: '键值' },
  { key: 'sort', title: '排序', width: 64, align: 'right' },
  { key: 'status', title: '状态', width: 90 },
  { key: 'actions', title: '操作', width: 84, className: 'col-actions' },
]

async function openData(row: TypeRow) {
  current.value = row
  await loadItems()
}

async function loadItems() {
  if (!current.value) return
  itemsLoading.value = true
  try {
    items.value = (await systemApi.dictData.list(current.value.dictType)) as unknown as DataRow[]
  } catch (error) {
    toastError(error, '字典数据加载失败')
  } finally {
    itemsLoading.value = false
  }
}

interface DataForm { id?: number; label: string; value: string; tone: string; sort: number | null; def: boolean; status: number; remark: string }
const dataDialog = ref(false)
const dataSaving = ref(false)
const dataForm = reactive<DataForm>({ label: '', value: '', tone: 'neutral', sort: 1, def: false, status: 1, remark: '' })
const dataErrors = ref<Partial<Record<keyof DataForm, string>>>({})
/** 表单所属字典类型：抽屉与样本册共用同一套字典数据表单 */
const dataTarget = ref<TypeRow | null>(null)

function openDataCreate(type?: TypeRow) {
  dataTarget.value = type ?? current.value
  Object.assign(dataForm, { id: undefined, label: '', value: '', tone: 'neutral', sort: items.value.length + 1, def: false, status: 1, remark: '' })
  dataErrors.value = {}
  dataDialog.value = true
}

function openDataEdit(row: DataRow, type?: TypeRow) {
  dataTarget.value = type ?? current.value
  Object.assign(dataForm, { id: row.id, label: row.label, value: row.value, tone: row.tone || 'neutral', sort: row.sort, def: row.def, status: row.status, remark: row.remark ?? '' })
  dataErrors.value = {}
  dataDialog.value = true
}

async function submitData() {
  const target = dataTarget.value
  if (!target) return
  dataErrors.value = validate(dataForm, { label: [required('请输入标签')], value: [required('请输入键值')] })
  if (Object.keys(dataErrors.value).length) return
  dataSaving.value = true
  const payload = { dictType: target.dictType, label: dataForm.label.trim(), value: dataForm.value.trim(), tone: dataForm.tone, sort: dataForm.sort ?? 0, def: dataForm.def, status: dataForm.status, remark: dataForm.remark }
  try {
    if (dataForm.id) await systemApi.dictData.update(dataForm.id, payload)
    else await systemApi.dictData.create(payload)
    pushToast('字典数据已保存')
    dataDialog.value = false
    invalidateDict(target.dictType)
    if (current.value?.dictType === target.dictType) await loadItems()
    await book.value?.reload(target.dictType)
  } catch (error) {
    toastError(error, '字典数据保存失败')
  } finally {
    dataSaving.value = false
  }
}

async function removeData(row: DataRow) {
  const ok = await confirm({ title: '删除字典数据', message: `确认删除「${row.label}」吗？`, confirmText: '删除', danger: true })
  if (!ok || !current.value) return
  try {
    await systemApi.dictData.remove(row.id)
    pushToast('字典数据已删除')
    invalidateDict(current.value.dictType)
    await loadItems()
  } catch (error) {
    toastError(error, '字典数据删除失败')
  }
}
</script>

<template>
  <ListPage title="字典管理" desc="维护各业务页面的下拉选项和状态标签，统一选项名称、取值和显示样式。" table-title="字典类型" :list="list" :columns="columns" :min-width="860" :alt-view="view === 'book'">
    <template #alt>
      <DictSwatchbook
        ref="book"
        :types="allTypes"
        :loading="typesLoading"
        @create-item="openDataCreate($event as TypeRow)"
        @edit-item="(type, item) => openDataEdit(item as DataRow, type as TypeRow)"
        @edit-type="openTypeEdit($event as TypeRow)"
      />
    </template>
    <template #actions>
      <Segmented v-model="view" label="视图" :options="VIEWS" />
      <PermissionButton perm="system:dict:add" variant="primary" icon="plus" @click="openTypeCreate">新增字典类型</PermissionButton>
    </template>

    <template #search>
      <Field label="关键词" wide>
        <TextInput v-model="list.query.keyword" icon="search" placeholder="字典名称 / 类型" clearable @enter="list.search()" />
      </Field>
      <Field label="状态">
        <DictSelect v-model="list.query.status" type="sys_normal_disable" all-label="全部状态" />
      </Field>
    </template>

    <template #cell-dictName="{ row }">
      <button type="button" class="link-btn" @click="openData(row)">{{ row.dictName }}</button>
    </template>
    <template #cell-dictType="{ row }"><span class="code-chip">{{ row.dictType }}</span></template>
    <template #cell-status="{ row }"><StatusTag :status="row.status" /></template>
    <template #cell-remark="{ row }"><span class="muted">{{ row.remark || '—' }}</span></template>
    <template #cell-createTime="{ row }"><span class="tnum soft">{{ row.createTime }}</span></template>
    <template #cell-actions="{ row }">
      <div class="row-actions">
        <PermissionButton perm="system:dict:list" icon-only icon="dict" label="字典数据" @click="openData(row)" />
        <PermissionButton perm="system:dict:edit" icon-only icon="edit" label="编辑" @click="openTypeEdit(row)" />
        <PermissionButton perm="system:dict:remove" icon-only icon="trash" label="删除" danger @click="removeType(row)" />
      </div>
    </template>

    <FormDialog :open="typeDialog" :title="typeForm.id ? '编辑字典类型' : '新增字典类型'" :saving="typeSaving" @close="typeDialog = false" @submit="submitType">
      <Field label="字典名称" required :error="typeErrors.dictName">
        <TextInput v-model="typeForm.dictName" placeholder="如：用户性别" :invalid="!!typeErrors.dictName" />
      </Field>
      <Field label="字典类型" required :error="typeErrors.dictType" hint="前端按此编码引用">
        <TextInput v-model="typeForm.dictType" placeholder="如：sys_user_sex" :invalid="!!typeErrors.dictType" />
      </Field>
      <Field label="备注" span>
        <TextInput v-model="typeForm.remark" multiline :rows="2" placeholder="字典用途说明" :maxlength="200" />
      </Field>
      <Field label="状态" group>
        <Switch :model-value="typeForm.status === 1" :label="typeForm.status === 1 ? '正常' : '停用'" @update:model-value="typeForm.status = $event ? 1 : 0" />
      </Field>
    </FormDialog>

    <Drawer :open="Boolean(current)" :title="current?.dictName" :sub="current?.dictType" :width="600" @close="current = null">
      <div class="drawer-toolbar">
        <span class="soft" style="font-size: 12px">共 {{ items.length }} 项，按排序显示</span>
        <PermissionButton perm="system:dict:add" variant="primary" size="sm" icon="plus" @click="openDataCreate()">新增数据</PermissionButton>
      </div>
      <div class="m-surface panel" style="overflow: hidden">
        <DataTable :columns="itemColumns" :data="items" :loading="itemsLoading" :min-width="480" empty-text="还没有字典数据">
          <template #cell-label="{ row }">
            <span style="display: inline-flex; align-items: center; gap: 6px">
              <Tag :tone="row.tone" dot>{{ row.label }}</Tag>
              <Icon v-if="row.def" name="check" :size="13" style="color: var(--leaf-text)" title="默认值" />
            </span>
          </template>
          <template #cell-value="{ row }"><span class="code-chip">{{ row.value }}</span></template>
          <template #cell-sort="{ row }"><span class="tnum soft">{{ row.sort }}</span></template>
          <template #cell-status="{ row }"><StatusTag :status="row.status" /></template>
          <template #cell-actions="{ row }">
            <div class="row-actions">
              <PermissionButton perm="system:dict:edit" icon-only icon="edit" label="编辑" @click="openDataEdit(row)" />
              <PermissionButton perm="system:dict:remove" icon-only icon="trash" label="删除" danger @click="removeData(row)" />
            </div>
          </template>
        </DataTable>
      </div>
    </Drawer>

    <FormDialog :open="dataDialog" :title="dataForm.id ? '编辑字典数据' : '新增字典数据'" :sub="dataTarget?.dictName" :saving="dataSaving" @close="dataDialog = false" @submit="submitData">
      <Field label="标签" required :error="dataErrors.label">
        <TextInput v-model="dataForm.label" placeholder="显示文字，如：男" :invalid="!!dataErrors.label" />
      </Field>
      <Field label="键值" required :error="dataErrors.value">
        <TextInput v-model="dataForm.value" placeholder="存储值，如：0" :invalid="!!dataErrors.value" />
      </Field>
      <Field label="标签样式">
        <Select v-model="dataForm.tone" :options="TONES" />
      </Field>
      <Field label="显示排序">
        <TextInput v-model="dataForm.sort" type="number" />
      </Field>
      <Field label="预览" group>
        <div style="height: var(--control-h); display: flex; align-items: center"><Tag :tone="dataForm.tone" dot>{{ dataForm.label || '标签' }}</Tag></div>
      </Field>
      <Field label="默认值" group>
        <Switch v-model="dataForm.def" :label="dataForm.def ? '是' : '否'" />
      </Field>
      <Field label="状态" group>
        <Switch :model-value="dataForm.status === 1" :label="dataForm.status === 1 ? '正常' : '停用'" @update:model-value="dataForm.status = $event ? 1 : 0" />
      </Field>
    </FormDialog>
  </ListPage>
</template>
