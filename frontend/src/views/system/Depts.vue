<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { Avatar, Field, Segmented, SpecPlate, StatusTag, Switch, TextInput, type Column } from '@/components/ui'
import { DeptSelect, DictSelect, FormDialog, ListPage, PermissionButton, UserSelect } from '@/components/business'
import OrgCanvas from '@/components/business/OrgCanvas.vue'
import { systemApi, type UserRow } from '@/api'
import { useViewMode } from '@/composables/view'
import { hasPermission } from '@/directives/permission'
import { useTreeListPage } from '@/composables/list'
import { confirm } from '@/composables/confirm'
import { pushToast, toastError } from '@/composables/toast'
import { collectDeptIds, reloadDepts } from '@/composables/dept'
import type { Dept } from '@/data'
import { email, phone, required, validate } from '@/utils/validate'

const list = useTreeListPage<Dept, { keyword: string; status: string }>({
  query: { keyword: '', status: '' },
  fetch: params => systemApi.depts.tree(params) as unknown as Promise<Dept[]>,
  exporter: params => systemApi.depts.export(params),
  exportName: '部门列表.csv',
})

/* ── 画布视图：组织架构 + 四角浮动面板 ── */
const view = useViewMode('depts', 'canvas', ['canvas', 'table'] as const, 'table')
const VIEWS = [{ label: '画布', value: 'canvas' as const }, { label: '表格', value: 'table' as const }]
const selectedId = ref<number | null>(null)
const members = ref<UserRow[]>([])
const membersTotal = ref(0)
const canSeeUsers = hasPermission('system:user:list')

const tree = computed(() => list.rows.value as Dept[])
function findPath(nodes: Dept[], id: number, trail: Dept[] = []): Dept[] | null {
  for (const node of nodes) {
    const next = [...trail, node]
    if (node.id === id) return next
    const hit = findPath(node.children ?? [], id, next)
    if (hit) return hit
  }
  return null
}
const selectedPath = computed(() => (selectedId.value === null ? null : findPath(tree.value, selectedId.value)))
const selected = computed(() => selectedPath.value?.[selectedPath.value.length - 1] ?? null)
const sumMembers = (node: Dept): number => (node.memberCount ?? 0) + (node.children ?? []).reduce((a, c) => a + sumMembers(c), 0)
const countDescendants = (node: Dept): number => (node.children ?? []).reduce((a, c) => a + 1 + countDescendants(c), 0)

// 默认选中根部门；树刷新后选中项不存在时回到根
watch(tree, (nodes) => {
  if (!nodes.length) { selectedId.value = null; return }
  if (selectedId.value === null || !findPath(nodes, selectedId.value)) selectedId.value = nodes[0].id
}, { immediate: true })

let memberRequest = 0
watch(selectedId, async (id) => {
  members.value = []
  membersTotal.value = 0
  if (id === null || !canSeeUsers) return
  const current = ++memberRequest
  try {
    const page = await systemApi.users.list({ deptId: id, pageNum: 1, pageSize: 6 })
    if (current !== memberRequest) return
    members.value = page.rows
    membersTotal.value = page.total
  } catch (error) {
    if (current === memberRequest) toastError(error, '成员加载失败')
  }
})

const columns: Column[] = [
  { key: 'name', title: '部门名称', minWidth: 240 },
  { key: 'leader', title: '负责人' },
  { key: 'phone', title: '联系电话' },
  { key: 'email', title: '邮箱' },
  { key: 'memberCount', title: '人数', width: 70, align: 'right' },
  { key: 'sort', title: '排序', width: 70, align: 'right' },
  { key: 'status', title: '状态', width: 90 },
  { key: 'actions', title: '操作', width: 120, className: 'col-actions' },
]

interface DeptForm {
  id?: number
  parentId: number | null
  name: string
  leader: string | null
  phone: string
  email: string
  sort: number | null
  status: number
}
const emptyForm = (parent?: Dept): DeptForm => ({ parentId: parent?.id ?? 0, name: '', leader: null, phone: '', email: '', sort: 10, status: 1 })
const dialog = ref(false)
const saving = ref(false)
const form = reactive<DeptForm>(emptyForm())
const errors = ref<Partial<Record<keyof DeptForm, string>>>({})
const excludeIds = ref<number[]>([])

function openCreate(parent?: Dept) {
  Object.assign(form, emptyForm(parent), { id: undefined })
  excludeIds.value = []
  errors.value = {}
  dialog.value = true
}

function openEdit(row: Dept) {
  Object.assign(form, {
    id: row.id, parentId: row.parentId ?? 0, name: row.name, leader: row.leader || null,
    phone: row.phone ?? '', email: row.email ?? '', sort: row.sort ?? 0, status: row.status,
  })
  excludeIds.value = collectDeptIds(row)
  errors.value = {}
  dialog.value = true
}

async function submit() {
  errors.value = validate(form, {
    name: [required('请输入部门名称')],
    parentId: [v => (v === null ? '请选择上级部门' : undefined)],
    phone: [phone],
    email: [email],
  })
  if (Object.keys(errors.value).length) return
  saving.value = true
  const payload = {
    parentId: form.parentId ?? 0,
    name: form.name.trim(),
    leader: form.leader ?? '',
    phone: form.phone,
    email: form.email,
    sort: form.sort ?? 0,
    status: form.status,
  }
  try {
    if (form.id) {
      await systemApi.depts.update(form.id, payload)
      pushToast('部门已更新')
    } else {
      await systemApi.depts.create(payload)
      pushToast('部门已新增')
    }
    dialog.value = false
    await Promise.all([list.load(), reloadDepts()])
  } catch (error) {
    toastError(error, '部门保存失败')
  } finally {
    saving.value = false
  }
}

async function remove(row: Dept) {
  if (row.children?.length) {
    await confirm({ title: '无法删除', message: `「${row.name}」下还有 ${row.children.length} 个下级部门，请先调整或删除下级部门。`, confirmText: '知道了' })
    return
  }
  const ok = await confirm({ title: '删除部门', message: `确认删除部门「${row.name}」吗？`, confirmText: '删除', danger: true })
  if (!ok) return
  try {
    await systemApi.depts.remove(row.id)
    pushToast('部门已删除')
    await Promise.all([list.load(), reloadDepts()])
  } catch (error) {
    toastError(error, '部门删除失败')
  }
}
</script>

<template>
  <ListPage
    title="部门管理"
    desc="维护组织架构。部门层级同时决定「本部门及以下」数据权限的范围。"
    table-title="组织架构"
    :list="list"
    :columns="columns"
    :min-width="980"
    :alt-view="view === 'canvas'"
  >
    <template #actions>
      <Segmented v-model="view" label="视图" :options="VIEWS" />
      <PermissionButton perm="system:dept:add" variant="primary" icon="plus" @click="openCreate()">新增部门</PermissionButton>
    </template>

    <template #alt>
      <OrgCanvas :tree="tree" :selected-id="selectedId" @select="selectedId = $event.id">
        <template #tl>
          <section v-if="selected" class="org-card m-float">
            <span class="eyebrow">{{ selectedPath?.map(d => d.name).slice(0, -1).join(' / ') || '根部门' }}</span>
            <h2 class="org-card-title">{{ selected.name }}</h2>
            <dl class="org-card-rows">
              <dt>负责人</dt><dd>{{ selected.leader || '—' }}</dd>
              <dt>电话</dt><dd class="tnum">{{ selected.phone || '—' }}</dd>
              <dt>邮箱</dt><dd>{{ selected.email || '—' }}</dd>
              <dt>状态</dt><dd><StatusTag :status="selected.status" /></dd>
            </dl>
          </section>
        </template>
        <template #tr>
          <SpecPlate
            v-if="selected"
            title="部门读数"
            :rows="[
              { label: '本部门', value: selected.memberCount ?? 0 },
              { label: '含下级', value: sumMembers(selected), tone: 'lime' },
              { label: '下级部门', value: countDescendants(selected) },
              { label: '层级', value: (selectedPath?.length ?? 1) },
            ]"
          />
        </template>
        <template #bl>
          <section v-if="selected && canSeeUsers" class="org-card m-float">
            <span class="eyebrow">成员 · 含下级 {{ membersTotal }} 人</span>
            <ul class="org-members">
              <li v-for="u in members" :key="u.id">
                <Avatar :name="u.nickname" :size="26" />
                <span class="ellipsis">{{ u.nickname }}</span>
                <span class="faint ellipsis">{{ u.postName || u.roleName || '' }}</span>
              </li>
              <li v-if="!members.length" class="faint">暂无成员</li>
            </ul>
          </section>
        </template>
        <template #br>
          <section v-if="selected" class="org-card org-actions m-float">
            <PermissionButton perm="system:dept:add" variant="primary" size="sm" icon="plus" @click="openCreate(selected)">新增下级</PermissionButton>
            <PermissionButton perm="system:dept:edit" size="sm" icon="edit" @click="openEdit(selected)">编辑</PermissionButton>
            <PermissionButton perm="system:dept:remove" size="sm" icon="trash" danger variant="ghost" @click="remove(selected)">删除</PermissionButton>
          </section>
        </template>
      </OrgCanvas>
    </template>

    <template #search>
      <Field label="关键词" wide>
        <TextInput v-model="list.query.keyword" icon="search" placeholder="部门名称" clearable @enter="list.search()" />
      </Field>
      <Field label="状态">
        <DictSelect v-model="list.query.status" type="sys_normal_disable" all-label="全部状态" />
      </Field>
    </template>

    <template #cell-name="{ row }"><span class="cell-title">{{ row.name }}</span></template>
    <template #cell-leader="{ row }">{{ row.leader || '—' }}</template>
    <template #cell-phone="{ row }"><span class="tnum muted">{{ row.phone || '—' }}</span></template>
    <template #cell-email="{ row }"><span class="muted">{{ row.email || '—' }}</span></template>
    <template #cell-memberCount="{ row }"><span class="tnum">{{ row.memberCount ?? 0 }}</span></template>
    <template #cell-sort="{ row }"><span class="tnum soft">{{ row.sort }}</span></template>
    <template #cell-status="{ row }"><StatusTag :status="row.status" /></template>
    <template #cell-actions="{ row }">
      <div class="row-actions">
        <PermissionButton perm="system:dept:add" icon-only icon="plus" label="新增下级部门" @click="openCreate(row)" />
        <PermissionButton perm="system:dept:edit" icon-only icon="edit" label="编辑" @click="openEdit(row)" />
        <PermissionButton perm="system:dept:remove" icon-only icon="trash" label="删除" danger @click="remove(row)" />
      </div>
    </template>

    <FormDialog :open="dialog" :title="form.id ? '编辑部门' : '新增部门'" :saving="saving" @close="dialog = false" @submit="submit">
      <Field label="上级部门" required :error="errors.parentId" span>
        <DeptSelect v-model="form.parentId" root-label="无（顶级部门）" :exclude-ids="excludeIds" :invalid="!!errors.parentId" />
      </Field>
      <Field label="部门名称" required :error="errors.name">
        <TextInput v-model="form.name" placeholder="如：研发中心" :invalid="!!errors.name" />
      </Field>
      <Field label="负责人">
        <UserSelect v-model="form.leader" value-key="nickname" placeholder="选择负责人" />
      </Field>
      <Field label="联系电话" :error="errors.phone">
        <TextInput v-model="form.phone" type="tel" placeholder="选填" :invalid="!!errors.phone" />
      </Field>
      <Field label="邮箱" :error="errors.email">
        <TextInput v-model="form.email" type="email" placeholder="选填" :invalid="!!errors.email" />
      </Field>
      <Field label="显示排序">
        <TextInput v-model="form.sort" type="number" />
      </Field>
      <Field label="部门状态" group>
        <Switch :model-value="form.status === 1" :label="form.status === 1 ? '正常' : '停用'" @update:model-value="form.status = $event ? 1 : 0" />
      </Field>
    </FormDialog>
  </ListPage>
</template>
