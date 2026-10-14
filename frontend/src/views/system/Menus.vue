<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import Icon from '@/components/Icon.vue'
import { Field, Segmented, Select, StatusTag, Switch, Tag, TextInput, type Column } from '@/components/ui'
import { DictSelect, FormDialog, ListPage, PermissionButton } from '@/components/business'
import MenuStarMap from '@/components/business/MenuStarMap.vue'
import { useViewMode } from '@/composables/view'
import { invalidatePermissionGraph } from '@/composables/permissionGraph'
import { systemApi } from '@/api'
import { useTreeListPage } from '@/composables/list'
import { confirm } from '@/composables/confirm'
import { pushToast, toastError } from '@/composables/toast'
import { hasPermission } from '@/directives/permission'
import type { Menu } from '@/data'
import { MENU_ICON_OPTIONS } from '@/data/icons'
import { pattern, required, validate } from '@/utils/validate'

const list = useTreeListPage<Menu, { keyword: string; status: string }>({
  query: { keyword: '', status: '' },
  fetch: params => systemApi.menus.tree(params) as unknown as Promise<Menu[]>,
  exporter: params => systemApi.menus.export(params),
  exportName: '菜单列表.csv',
})

const columns: Column[] = [
  { key: 'name', title: '菜单名称', minWidth: 240 },
  { key: 'type', title: '类型', width: 80 },
  { key: 'permission', title: '权限标识' },
  { key: 'component', title: '路由 / 组件' },
  { key: 'sort', title: '排序', width: 70, align: 'right' },
  { key: 'visible', title: '显示', width: 80 },
  { key: 'status', title: '状态', width: 90 },
  { key: 'actions', title: '操作', width: 120, className: 'col-actions' },
]

/* ── 星图视图：目录 → 页面 → 按钮三环，选中节点看路径与授权 ── */
const view = useViewMode('menus', 'star', ['star', 'table'] as const, 'table')
const VIEWS = [{ label: '星图', value: 'star' as const }, { label: '表格', value: 'table' as const }]

const TYPE_LABEL: Record<string, string> = { dir: '目录', menu: '菜单', btn: '按钮' }
const TYPE_TONE: Record<string, string> = { dir: 'neutral', menu: 'info', btn: 'ok' }
const iconOptions = MENU_ICON_OPTIONS.map(name => ({ label: name, value: name, icon: name }))

/* 上级菜单：只有目录和菜单能挂子项 */
const parentOptions = computed(() => {
  const options: Array<{ label: string; value: number; depth: number; disabled?: boolean }> = [{ label: '顶级', value: 0, depth: 0 }]
  const walk = (nodes: Menu[], depth: number) => nodes.forEach(node => {
    if (node.type === 'btn') return
    options.push({ label: node.name, value: node.id, depth, disabled: excluded.value.has(node.id) })
    walk(node.children ?? [], depth + 1)
  })
  walk(list.rows.value as Menu[], 1)
  return options
})

/* ── 表单 ── */
interface MenuForm {
  id?: number
  parentId: number
  type: 'dir' | 'menu' | 'btn'
  name: string
  icon: string
  path: string
  component: string
  permission: string
  sort: number | null
  visible: number
  status: number
}
const emptyForm = (parent?: Menu): MenuForm => ({
  parentId: parent?.id ?? 0,
  type: !parent ? 'dir' : parent.type === 'dir' ? 'menu' : 'btn',
  name: '', icon: '', path: '', component: '', permission: '', sort: 10, visible: 1, status: 1,
})
const dialog = ref(false)
const saving = ref(false)
const form = reactive<MenuForm>(emptyForm())
const errors = ref<Partial<Record<keyof MenuForm, string>>>({})
const excluded = ref(new Set<number>())

function collectIds(node: Menu): number[] {
  return [node.id, ...(node.children ?? []).flatMap(collectIds)]
}

function openCreate(parent?: Menu) {
  Object.assign(form, emptyForm(parent), { id: undefined })
  excluded.value = new Set()
  errors.value = {}
  dialog.value = true
}

function openEdit(row: Menu) {
  Object.assign(form, {
    id: row.id, parentId: row.parentId ?? 0, type: row.type, name: row.name, icon: row.icon ?? '', path: row.path ?? '',
    component: row.component ?? '', permission: row.permission ?? '', sort: row.sort ?? 0, visible: row.visible ?? 1, status: row.status,
  })
  excluded.value = new Set(collectIds(row))
  errors.value = {}
  dialog.value = true
}

async function submit() {
  const isPage = form.type === 'menu'
  errors.value = validate(form, {
    name: [required('请输入菜单名称')],
    path: form.type === 'btn' ? [] : [required('请输入路由路径'), pattern(/^\//, '路由路径以 / 开头')],
    component: isPage ? [required('请输入组件路径')] : [],
    permission: form.type === 'btn' ? [required('请输入权限标识')] : [],
  })
  if (Object.keys(errors.value).length) return
  saving.value = true
  const payload = {
    parentId: form.parentId,
    type: form.type,
    name: form.name.trim(),
    icon: form.type === 'btn' ? '' : form.icon,
    path: form.type === 'btn' ? '' : form.path.trim(),
    component: isPage ? form.component.trim() : '',
    permission: form.type === 'dir' ? '' : form.permission.trim(),
    sort: form.sort ?? 0,
    visible: form.visible,
    status: form.status,
  }
  try {
    if (form.id) {
      await systemApi.menus.update(form.id, payload)
      pushToast('菜单已更新，刷新页面后导航生效')
    } else {
      await systemApi.menus.create(payload)
      pushToast('菜单已新增，刷新页面后导航生效')
    }
    dialog.value = false
    invalidatePermissionGraph()
    await list.load()
  } catch (error) {
    toastError(error, '菜单保存失败')
  } finally {
    saving.value = false
  }
}

async function toggleVisible(row: Menu, visible: boolean) {
  try {
    await systemApi.menus.update(row.id, { visible: visible ? 1 : 0 })
    row.visible = visible ? 1 : 0
    list.rows.value = [...list.rows.value]
  } catch (error) {
    toastError(error, '显示状态更新失败')
  }
}

async function remove(row: Menu) {
  const ok = await confirm({
    title: '删除菜单',
    message: row.children?.length ? `「${row.name}」下还有 ${row.children.length} 个子项，需先删除子项。` : `确认删除「${row.name}」吗？已分配该菜单的角色将同时失去对应权限。`,
    confirmText: row.children?.length ? '知道了' : '删除',
    danger: !row.children?.length,
  })
  if (!ok || row.children?.length) return
  try {
    await systemApi.menus.remove(row.id)
    pushToast('菜单已删除')
    invalidatePermissionGraph()
    await list.load()
  } catch (error) {
    toastError(error, '菜单删除失败')
  }
}
</script>

<template>
  <ListPage
    title="菜单管理"
    desc="维护导航目录、页面与按钮权限，配置菜单层级、显示状态和角色授权标识。"
    table-title="菜单树"
    :list="list"
    :columns="columns"
    :min-width="1000"
    :alt-view="view === 'star'"
  >
    <template #alt>
      <MenuStarMap :tree="(list.rows.value as Menu[])" @edit="openEdit" @create="openCreate" />
    </template>

    <template #actions>
      <Segmented v-model="view" label="视图" :options="VIEWS" />
      <PermissionButton perm="system:menu:add" variant="primary" icon="plus" @click="openCreate()">新增目录</PermissionButton>
    </template>

    <template #search>
      <Field label="关键词" wide>
        <TextInput v-model="list.query.keyword" icon="search" placeholder="菜单名称 / 权限标识" clearable @enter="list.search()" />
      </Field>
      <Field label="状态">
        <DictSelect v-model="list.query.status" type="sys_normal_disable" all-label="全部状态" />
      </Field>
    </template>

    <template #cell-name="{ row }">
      <span class="cell-main" style="gap: 8px">
        <Icon v-if="row.icon" :name="row.icon" :size="16" style="color: var(--ink-soft)" />
        <span :class="row.type === 'btn' ? 'muted' : 'cell-title'">{{ row.name }}</span>
      </span>
    </template>
    <template #cell-type="{ row }"><Tag :tone="TYPE_TONE[row.type]">{{ TYPE_LABEL[row.type] }}</Tag></template>
    <template #cell-permission="{ row }"><span v-if="row.permission" class="code-chip">{{ row.permission }}</span><span v-else class="faint">—</span></template>
    <template #cell-component="{ row }">
      <div v-if="row.type !== 'btn'" style="min-width: 0">
        <div class="tnum" style="font-size: 12.5px">{{ row.path || '—' }}</div>
        <div v-if="row.component" class="cell-sub mono">{{ row.component }}</div>
      </div>
      <span v-else class="faint">—</span>
    </template>
    <template #cell-sort="{ row }"><span class="tnum soft">{{ row.sort }}</span></template>
    <template #cell-visible="{ row }">
      <Switch
        v-if="row.type !== 'btn'"
        :model-value="row.visible === 1"
        :disabled="!hasPermission('system:menu:edit')"
        @update:model-value="toggleVisible(row, $event)"
      />
      <span v-else class="faint">—</span>
    </template>
    <template #cell-status="{ row }"><StatusTag :status="row.status" /></template>
    <template #cell-actions="{ row }">
      <div class="row-actions">
        <PermissionButton v-if="row.type !== 'btn'" perm="system:menu:add" icon-only icon="plus" :label="row.type === 'dir' ? '新增菜单' : '新增按钮'" @click="openCreate(row)" />
        <PermissionButton perm="system:menu:edit" icon-only icon="edit" label="编辑" @click="openEdit(row)" />
        <PermissionButton perm="system:menu:remove" icon-only icon="trash" label="删除" danger @click="remove(row)" />
      </div>
    </template>

    <FormDialog :open="dialog" :title="form.id ? '编辑菜单' : '新增菜单'" :saving="saving" @close="dialog = false" @submit="submit">
      <Field label="菜单类型" group span>
        <Segmented v-model="form.type" label="菜单类型" :options="[{ label: '目录', value: 'dir' }, { label: '菜单', value: 'menu' }, { label: '按钮', value: 'btn' }]" />
      </Field>
      <Field label="上级菜单">
        <Select v-model="form.parentId" :options="parentOptions" filterable />
      </Field>
      <Field label="菜单名称" required :error="errors.name">
        <TextInput v-model="form.name" placeholder="如：用户管理" :invalid="!!errors.name" />
      </Field>
      <Field v-if="form.type !== 'btn'" label="图标">
        <Select v-model="form.icon" :options="iconOptions" placeholder="选择图标" filterable clearable />
      </Field>
      <Field v-if="form.type !== 'btn'" label="路由路径" required :error="errors.path">
        <TextInput v-model="form.path" placeholder="如：/system/user" :invalid="!!errors.path" />
      </Field>
      <Field v-if="form.type === 'menu'" label="组件路径" required :error="errors.component" hint="填写已注册页面的组件标识">
        <TextInput v-model="form.component" placeholder="如：system/user/index" :invalid="!!errors.component" />
      </Field>
      <Field v-if="form.type !== 'dir'" label="权限标识" :required="form.type === 'btn'" :error="errors.permission" hint="用于角色授权和对应操作的权限校验">
        <TextInput v-model="form.permission" placeholder="如：system:user:add" :invalid="!!errors.permission" />
      </Field>
      <Field label="显示排序">
        <TextInput v-model="form.sort" type="number" />
      </Field>
      <Field v-if="form.type !== 'btn'" label="导航显示" group>
        <Switch :model-value="form.visible === 1" :label="form.visible === 1 ? '在导航中显示' : '隐藏'" @update:model-value="form.visible = $event ? 1 : 0" />
      </Field>
      <Field label="菜单状态" group>
        <Switch :model-value="form.status === 1" :label="form.status === 1 ? '正常' : '停用'" @update:model-value="form.status = $event ? 1 : 0" />
      </Field>
    </FormDialog>
  </ListPage>
</template>
