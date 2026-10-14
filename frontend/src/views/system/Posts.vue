<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { Field, Segmented, StatusTag, Switch, TextInput, type Column } from '@/components/ui'
import { DictSelect, FormDialog, ListPage, PermissionButton } from '@/components/business'
import PostLadder from '@/components/business/PostLadder.vue'
import { systemApi } from '@/api'
import { useListPage } from '@/composables/list'
import { useViewMode } from '@/composables/view'
import { confirm } from '@/composables/confirm'
import { pushToast, toastError } from '@/composables/toast'
import { pattern, required, validate } from '@/utils/validate'

interface PostRow { id: number; postCode: string; postName: string; sort: number; status: number; remark?: string; userCount: number; createTime: string }

const list = useListPage<PostRow, { keyword: string; status: string }>({
  query: { keyword: '', status: '' },
  fetch: params => systemApi.posts.list(params) as Promise<any>,
  exporter: params => systemApi.posts.export(params),
  exportName: '岗位列表.csv',
})

/* ── 阶梯视图：按 sort 排成上升台阶，点岗位看成员触须 ── */
const view = useViewMode('posts', 'ladder', ['ladder', 'table'] as const, 'table')
const VIEWS = [{ label: '阶梯', value: 'ladder' as const }, { label: '表格', value: 'table' as const }]
const ladderRows = ref<PostRow[]>([])
const ladderLoading = ref(false)
async function loadLadder() {
  ladderLoading.value = true
  try {
    const page = await systemApi.posts.list({ pageNum: 1, pageSize: 200 })
    ladderRows.value = page.rows as unknown as PostRow[]
  } catch (error) {
    toastError(error, '岗位阶梯加载失败')
  } finally {
    ladderLoading.value = false
  }
}
watch(view, (v) => { if (v === 'ladder') loadLadder() }, { immediate: true })

const columns: Column[] = [
  { key: 'postName', title: '岗位名称', minWidth: 160 },
  { key: 'postCode', title: '岗位编码' },
  { key: 'userCount', title: '在岗人数', width: 100, align: 'right' },
  { key: 'sort', title: '排序', width: 80, align: 'right' },
  { key: 'status', title: '状态', width: 96 },
  { key: 'remark', title: '备注' },
  { key: 'actions', title: '操作', width: 96, className: 'col-actions' },
]

interface PostForm { id?: number; postName: string; postCode: string; sort: number | null; status: number; remark: string }
const emptyForm = (): PostForm => ({ postName: '', postCode: '', sort: 10, status: 1, remark: '' })
const dialog = ref(false)
const saving = ref(false)
const form = reactive<PostForm>(emptyForm())
const errors = ref<Partial<Record<keyof PostForm, string>>>({})

function openCreate() {
  Object.assign(form, emptyForm(), { id: undefined })
  errors.value = {}
  dialog.value = true
}

function openEdit(row: PostRow) {
  Object.assign(form, { id: row.id, postName: row.postName, postCode: row.postCode, sort: row.sort, status: row.status, remark: row.remark ?? '' })
  errors.value = {}
  dialog.value = true
}

async function submit() {
  errors.value = validate(form, {
    postName: [required('请输入岗位名称')],
    postCode: [required('请输入岗位编码'), pattern(/^[A-Za-z][\w-]*$/, '字母开头，仅含字母、数字、下划线、短横线')],
  })
  if (Object.keys(errors.value).length) return
  saving.value = true
  const payload = { postName: form.postName.trim(), postCode: form.postCode.trim(), sort: form.sort ?? 0, status: form.status, remark: form.remark }
  try {
    if (form.id) {
      await systemApi.posts.update(form.id, payload)
      pushToast('岗位已更新')
    } else {
      await systemApi.posts.create(payload)
      pushToast('岗位已新增')
    }
    dialog.value = false
    await list.load()
    if (view.value === 'ladder') loadLadder()
  } catch (error) {
    toastError(error, '岗位保存失败')
  } finally {
    saving.value = false
  }
}

async function remove(row: PostRow) {
  const ok = await confirm({ title: '删除岗位', message: `确认删除岗位「${row.postName}」吗？`, confirmText: '删除', danger: true })
  if (!ok) return
  try {
    await systemApi.posts.remove(row.id)
    pushToast('岗位已删除')
    await list.load()
    if (view.value === 'ladder') loadLadder()
  } catch (error) {
    toastError(error, '岗位删除失败')
  }
}

async function batchRemove(ids: (string | number)[]) {
  const ok = await confirm({ title: '批量删除', message: `确认删除选中的 ${ids.length} 个岗位吗？`, confirmText: '删除', danger: true })
  if (!ok) return
  try {
    await Promise.all(ids.map(id => systemApi.posts.remove(Number(id))))
    pushToast(`已删除 ${ids.length} 个岗位`)
    await list.load()
  } catch (error) {
    toastError(error, '批量删除失败')
  }
}
</script>

<template>
  <ListPage title="岗位管理" desc="岗位描述员工在组织中的职务，可在用户管理中为用户指定岗位。" table-title="岗位列表" :list="list" :columns="columns" selectable :alt-view="view === 'ladder'">
    <template #actions>
      <Segmented v-model="view" label="视图" :options="VIEWS" />
      <PermissionButton perm="system:post:add" variant="primary" icon="plus" @click="openCreate">新增岗位</PermissionButton>
    </template>

    <template #alt>
      <PostLadder :posts="ladderRows" :loading="ladderLoading" @edit="openEdit(ladderRows.find(r => r.id === $event.id)!)" />
    </template>

    <template #search>
      <Field label="关键词" wide>
        <TextInput v-model="list.query.keyword" icon="search" placeholder="岗位名称 / 编码" clearable @enter="list.search()" />
      </Field>
      <Field label="状态">
        <DictSelect v-model="list.query.status" type="sys_normal_disable" all-label="全部状态" />
      </Field>
    </template>

    <template #batch="{ ids }">
      <PermissionButton perm="system:post:remove" variant="text" size="sm" danger @click="batchRemove(ids)">删除</PermissionButton>
    </template>

    <template #cell-postName="{ row }"><span class="cell-title">{{ row.postName }}</span></template>
    <template #cell-postCode="{ row }"><span class="code-chip">{{ row.postCode }}</span></template>
    <template #cell-userCount="{ row }"><span class="tnum">{{ row.userCount }}</span></template>
    <template #cell-sort="{ row }"><span class="tnum soft">{{ row.sort }}</span></template>
    <template #cell-status="{ row }"><StatusTag :status="row.status" /></template>
    <template #cell-remark="{ row }"><span class="muted">{{ row.remark || '—' }}</span></template>
    <template #cell-actions="{ row }">
      <div class="row-actions">
        <PermissionButton perm="system:post:edit" icon-only icon="edit" label="编辑" @click="openEdit(row)" />
        <PermissionButton perm="system:post:remove" icon-only icon="trash" label="删除" danger @click="remove(row)" />
      </div>
    </template>

    <FormDialog :open="dialog" :title="form.id ? '编辑岗位' : '新增岗位'" :saving="saving" @close="dialog = false" @submit="submit">
      <Field label="岗位名称" required :error="errors.postName">
        <TextInput v-model="form.postName" placeholder="如：研发工程师" :invalid="!!errors.postName" />
      </Field>
      <Field label="岗位编码" required :error="errors.postCode">
        <TextInput v-model="form.postCode" placeholder="如：dev" :invalid="!!errors.postCode" />
      </Field>
      <Field label="显示排序">
        <TextInput v-model="form.sort" type="number" />
      </Field>
      <Field label="岗位状态" group>
        <Switch :model-value="form.status === 1" :label="form.status === 1 ? '正常' : '停用'" @update:model-value="form.status = $event ? 1 : 0" />
      </Field>
      <Field label="备注" span>
        <TextInput v-model="form.remark" multiline :rows="2" placeholder="岗位职责说明" :maxlength="200" />
      </Field>
    </FormDialog>
  </ListPage>
</template>
