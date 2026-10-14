<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import Icon from '@/components/Icon.vue'
import { Button, Drawer, Field, Segmented, Select, StatusTag, Switch, Tag, TextInput, type Column } from '@/components/ui'
import { DictSelect, FormDialog, ListPage, PermissionButton, PermissionTree } from '@/components/business'
import PermissionMatrix, { type MatrixRole } from '@/components/business/PermissionMatrix.vue'
import PermissionTrace from '@/components/business/PermissionTrace.vue'
import { invalidatePermissionGraph } from '@/composables/permissionGraph'
import { systemApi } from '@/api'
import { useListPage } from '@/composables/list'
import { useViewMode } from '@/composables/view'
import { hasPermission } from '@/directives/permission'
import { confirm } from '@/composables/confirm'
import { pushToast, toastError } from '@/composables/toast'
import { revealCascade } from '@/composables/motion'
import type { Menu } from '@/data'
import { pattern, required, validate } from '@/utils/validate'

interface RoleRow {
  id: number
  name: string
  code: string
  dataScope: string
  sort: number
  status: number
  builtin: boolean
  remark?: string
  userCount: number
  createTime: string
  menuIds?: number[]
}

/* 数据权限：后端写入用编码，列表返回中文名 */
const SCOPES = [
  { code: 'ALL', name: '全部数据权限', desc: '可查看系统内所有部门的数据' },
  { code: 'DEPT_AND_CHILD', name: '本部门及以下数据权限', desc: '可查看本部门及全部下级部门的数据' },
  { code: 'DEPT', name: '本部门数据权限', desc: '仅可查看本部门的数据' },
  { code: 'SELF', name: '仅本人数据权限', desc: '仅可查看本人创建的数据' },
  { code: 'CUSTOM', name: '自定义数据权限', desc: '按角色关联的部门范围查看数据' },
]
const scopeCodeOf = (label?: string) => SCOPES.find(s => s.name === label || s.code === label)?.code ?? 'SELF'

const list = useListPage<RoleRow, { keyword: string; status: string }>({
  query: { keyword: '', status: '' },
  fetch: params => systemApi.roles.list(params) as Promise<any>,
  exporter: params => systemApi.roles.export(params),
  exportName: '角色列表.csv',
})

const columns: Column[] = [
  { key: 'name', title: '角色', minWidth: 180 },
  { key: 'dataScope', title: '数据权限' },
  { key: 'userCount', title: '用户数', width: 90, align: 'right' },
  { key: 'sort', title: '排序', width: 80, align: 'right' },
  { key: 'status', title: '状态', width: 96 },
  { key: 'createTime', title: '创建时间', width: 150 },
  { key: 'actions', title: '操作', width: 120, className: 'col-actions' },
]

/* ── 矩阵视图：角色 × 菜单，一屏对比与批量调整授权 ── */
const view = useViewMode('roles', 'table', ['table', 'matrix', 'trace'] as const, 'table')
const VIEWS = [{ label: '列表', value: 'table' as const }, { label: '矩阵', value: 'matrix' as const }, { label: '溯源', value: 'trace' as const }]

/* ── 溯源视图：聚焦对象与地址栏 ?trace=user:1 同步，其他页面与问答可直接跳入 ── */
const route = useRoute()
const router = useRouter()
const traceFocus = computed<string | null>({
  get: () => (typeof route.query.trace === 'string' && route.query.trace ? route.query.trace : null),
  set: (value) => {
    const query = { ...route.query }
    if (value) query.trace = value
    else delete query.trace
    router.replace({ query })
  },
})
watch(() => route.query.trace, (value) => { if (value) view.value = 'trace' }, { immediate: true })
const matrixMenus = ref<Menu[]>([])
const matrixRoles = ref<MatrixRole[]>([])
const matrixGrants = ref<Record<number, number[]>>({})
const matrixLoading = ref(false)
const matrixSaving = ref(false)
const matrix = ref<{ dirty: boolean; reset: () => void } | null>(null)
const canEditPerm = hasPermission('system:role:edit')

async function loadMatrix() {
  matrixLoading.value = true
  try {
    const [tree, page] = await Promise.all([systemApi.menus.tree(), systemApi.roles.list({ pageNum: 1, pageSize: 100 })])
    const roles = (page.rows as unknown as RoleRow[])
    const assigned = await Promise.all(roles.map(r => systemApi.roles.menus(r.id)))
    matrixMenus.value = tree as unknown as Menu[]
    matrixRoles.value = roles.map(r => ({ id: r.id, name: r.name, code: r.code, builtin: r.builtin, status: r.status }))
    matrixGrants.value = Object.fromEntries(roles.map((r, i) => [r.id, (assigned[i].menuIds ?? []).map(Number)]))
  } catch (error) {
    toastError(error, '权限矩阵加载失败')
  } finally {
    matrixLoading.value = false
  }
}

async function saveMatrix(changes: Array<{ roleId: number; menuIds: number[] }>) {
  matrixSaving.value = true
  try {
    // 逐个角色提交，任一失败即停下并保留未保存的改动
    for (const change of changes) await systemApi.roles.update(change.roleId, { menuIds: change.menuIds })
    invalidatePermissionGraph()
    pushToast(`已更新 ${changes.length} 个角色的权限，相关用户重新登录后生效`)
    await Promise.all([loadMatrix(), list.load()])
  } catch (error) {
    toastError(error, '权限保存失败')
  } finally {
    matrixSaving.value = false
  }
}

watch(view, async (next, prev) => {
  if (prev === 'matrix' && matrix.value?.dirty) {
    const ok = await confirm({ title: '放弃未保存的改动', message: '权限矩阵中还有未保存的改动，切换视图将放弃这些改动。', confirmText: '放弃改动', danger: true })
    if (!ok) { view.value = 'matrix'; return }
    matrix.value.reset()
  }
  if (next === 'matrix') loadMatrix()
  if (next !== 'trace' && route.query.trace) traceFocus.value = null
}, { immediate: true })

/* ── 新增 / 编辑 ── */
interface RoleForm { id?: number; name: string; code: string; dataScope: string; sort: number | null; status: number; remark: string }
const emptyForm = (): RoleForm => ({ name: '', code: '', dataScope: 'SELF', sort: 10, status: 1, remark: '' })
const dialog = ref(false)
const saving = ref(false)
const form = reactive<RoleForm>(emptyForm())
const errors = ref<Partial<Record<keyof RoleForm, string>>>({})
const scopeOptions = SCOPES.map(s => ({ label: s.name, value: s.code }))

function openCreate() {
  Object.assign(form, emptyForm(), { id: undefined })
  errors.value = {}
  dialog.value = true
}

function openEdit(row: RoleRow) {
  Object.assign(form, { id: row.id, name: row.name, code: row.code, dataScope: scopeCodeOf(row.dataScope), sort: row.sort, status: row.status, remark: row.remark ?? '' })
  errors.value = {}
  dialog.value = true
}

async function submit() {
  errors.value = validate(form, {
    name: [required('请输入角色名称')],
    code: [required('请输入角色编码'), pattern(/^[a-z][a-z0-9_]*$/, '小写字母开头，仅含小写字母、数字、下划线')],
  })
  if (Object.keys(errors.value).length) return
  saving.value = true
  const payload = { name: form.name.trim(), code: form.code.trim(), dataScope: form.dataScope, sort: form.sort ?? 0, status: form.status, remark: form.remark }
  try {
    if (form.id) {
      await systemApi.roles.update(form.id, payload)
      pushToast('角色已更新')
    } else {
      await systemApi.roles.create(payload)
      pushToast('角色已新增')
    }
    dialog.value = false
    invalidatePermissionGraph()
    await list.load()
  } catch (error) {
    toastError(error, '角色保存失败')
  } finally {
    saving.value = false
  }
}

async function remove(row: RoleRow) {
  const ok = await confirm({ title: '删除角色', message: `确认删除角色「${row.name}」吗？已分配该角色的用户将失去对应权限。`, confirmText: '删除', danger: true })
  if (!ok) return
  try {
    await systemApi.roles.remove(row.id)
    invalidatePermissionGraph()
    pushToast('角色已删除')
    await list.load()
  } catch (error) {
    toastError(error, '角色删除失败')
  }
}

/* ── 分配权限 ── */
const permRole = ref<RoleRow | null>(null)
const permTab = ref<'menu' | 'data'>('menu')
const menuTree = ref<Menu[]>([])
const checked = ref<number[]>([])
const scope = ref('SELF')
const permSaving = ref(false)
const permLoading = ref(false)
const permBody = ref<HTMLElement | null>(null)
watch([permRole, permLoading, permTab], async () => {
  if (!permRole.value) return
  await nextTick()
  revealCascade(permBody.value)
})

const menuCount = computed(() => {
  const types = new Map<number, string>()
  const walk = (nodes: Menu[]) => nodes.forEach(n => { types.set(n.id, n.type); walk(n.children ?? []) })
  walk(menuTree.value)
  const set = new Set(checked.value)
  let pages = 0
  let buttons = 0
  types.forEach((type, id) => { if (set.has(id)) { if (type === 'btn') buttons += 1; else if (type === 'menu') pages += 1 } })
  return { pages, buttons }
})

async function openPermission(row: RoleRow) {
  permRole.value = row
  permTab.value = 'menu'
  scope.value = scopeCodeOf(row.dataScope)
  permLoading.value = true
  try {
    const [tree, assigned] = await Promise.all([systemApi.menus.tree(), systemApi.roles.menus(row.id)])
    menuTree.value = tree as unknown as Menu[]
    checked.value = (assigned.menuIds ?? []).map(Number)
  } catch (error) {
    toastError(error, '权限数据加载失败')
  } finally {
    permLoading.value = false
  }
}

async function savePermission() {
  if (!permRole.value) return
  permSaving.value = true
  try {
    await systemApi.roles.update(permRole.value.id, { menuIds: checked.value, dataScope: scope.value })
    invalidatePermissionGraph()
    pushToast('权限已保存，相关用户重新登录后生效')
    permRole.value = null
    await list.load()
  } catch (error) {
    toastError(error, '权限保存失败')
  } finally {
    permSaving.value = false
  }
}
</script>

<template>
  <ListPage
    title="角色管理"
    desc="角色决定用户可见的菜单、可用的按钮以及可查看的数据范围。"
    table-title="角色列表"
    :list="list"
    :columns="columns"
    :min-width="900"
    :alt-view="view !== 'table'"
  >
    <template #alt>
      <PermissionTrace v-if="view === 'trace'" v-model:focus="traceFocus" />
      <PermissionMatrix
        v-else-if="matrixMenus.length"
        ref="matrix"
        :menus="matrixMenus"
        :roles="matrixRoles"
        :grants="matrixGrants"
        :editable="canEditPerm"
        :saving="matrixSaving"
        @save="saveMatrix"
      />
      <p v-if="view === 'matrix' && !matrixMenus.length" class="band-empty faint">{{ matrixLoading ? '正在加载权限矩阵…' : '暂无菜单数据' }}</p>
    </template>

    <template #actions>
      <Segmented v-model="view" label="视图" :options="VIEWS" />
      <PermissionButton perm="system:role:add" variant="primary" icon="plus" @click="openCreate">新增角色</PermissionButton>
    </template>

    <template #search>
      <Field label="关键词" wide>
        <TextInput v-model="list.query.keyword" icon="search" placeholder="角色名称 / 编码" clearable @enter="list.search()" />
      </Field>
      <Field label="状态">
        <DictSelect v-model="list.query.status" type="sys_normal_disable" all-label="全部状态" />
      </Field>
    </template>

    <template #cell-name="{ row }">
      <div style="min-width: 0">
        <div class="cell-title" style="display: flex; align-items: center; gap: 6px">{{ row.name }}<Tag v-if="row.builtin">内置</Tag></div>
        <div class="cell-sub mono">{{ row.code }}</div>
      </div>
    </template>
    <template #cell-dataScope="{ row }"><span class="muted">{{ row.dataScope }}</span></template>
    <template #cell-userCount="{ row }"><span class="tnum">{{ row.userCount }}</span></template>
    <template #cell-sort="{ row }"><span class="tnum soft">{{ row.sort }}</span></template>
    <template #cell-status="{ row }"><StatusTag :status="row.status" /></template>
    <template #cell-createTime="{ row }"><span class="tnum soft">{{ row.createTime }}</span></template>
    <template #cell-actions="{ row }">
      <div class="row-actions">
        <template v-if="!row.builtin">
          <PermissionButton perm="system:role:edit" icon-only icon="edit" label="编辑" @click="openEdit(row)" />
          <PermissionButton perm="system:role:edit" icon-only icon="shield" label="分配权限" @click="openPermission(row)" />
          <PermissionButton perm="system:role:remove" icon-only icon="trash" label="删除" danger @click="remove(row)" />
        </template>
        <span v-else class="faint" style="font-size: 12px" title="内置角色拥有全部权限，不可修改">全部权限</span>
      </div>
    </template>

    <FormDialog :open="dialog" :title="form.id ? '编辑角色' : '新增角色'" :saving="saving" @close="dialog = false" @submit="submit">
      <Field label="角色名称" required :error="errors.name">
        <TextInput v-model="form.name" placeholder="如：系统管理员" :invalid="!!errors.name" />
      </Field>
      <Field label="角色编码" required :error="errors.code" hint="用于后端判断角色，创建后不建议修改">
        <TextInput v-model="form.code" placeholder="如：sys_admin" :invalid="!!errors.code" />
      </Field>
      <Field label="数据权限">
        <Select v-model="form.dataScope" :options="scopeOptions" />
      </Field>
      <Field label="显示排序">
        <TextInput v-model="form.sort" type="number" />
      </Field>
      <Field label="备注" span>
        <TextInput v-model="form.remark" multiline :rows="2" placeholder="角色用途说明" :maxlength="200" />
      </Field>
      <Field label="角色状态" group>
        <Switch :model-value="form.status === 1" :label="form.status === 1 ? '正常' : '停用'" @update:model-value="form.status = $event ? 1 : 0" />
      </Field>
    </FormDialog>

    <Drawer :open="Boolean(permRole)" :title="`分配权限 · ${permRole?.name ?? ''}`" :sub="permRole?.code" :width="560" @close="permRole = null">
      <div ref="permBody" class="drawer-stack">
      <div class="drawer-toolbar">
        <Segmented v-model="permTab" label="权限类型" :options="[{ label: '菜单与按钮', value: 'menu' }, { label: '数据范围', value: 'data' }]" />
        <span v-if="permTab === 'menu'" class="soft" style="font-size: 12px">已选 {{ menuCount.pages }} 个页面 · {{ menuCount.buttons }} 个按钮</span>
      </div>

      <template v-if="permTab === 'menu'">
        <div v-if="permLoading" style="padding: 8px 4px">
          <span v-for="i in 8" :key="i" class="skeleton" :style="{ width: `${40 + (i * 13) % 45}%`, margin: '14px 0' }" />
        </div>
        <PermissionTree v-else v-model="checked" :menus="menuTree" />
      </template>

      <div v-else class="scope-list" role="radiogroup" aria-label="数据范围">
        <button
          v-for="s in SCOPES"
          :key="s.code"
          type="button"
          role="radio"
          :aria-checked="scope === s.code"
          :class="['scope-option', scope === s.code && 'is-on m-pill']"
          @click="scope = s.code"
        >
          <span class="scope-radio" aria-hidden="true"><Icon v-if="scope === s.code" name="check" :size="12" :stroke="2.6" /></span>
          <span>
            <span class="scope-name">{{ s.name }}</span>
            <span class="scope-desc">{{ s.desc }}</span>
          </span>
        </button>
      </div>
      </div>

      <template #footer>
        <Button variant="ghost" @click="permRole = null">取消</Button>
        <Button variant="primary" :loading="permSaving" @click="savePermission">保存权限</Button>
      </template>
    </Drawer>
  </ListPage>
</template>
