<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { pathOf } from '@/composables/auth'
import { Avatar, Field, HoverCard, Segmented, Select, StatusTag, Switch, Tag, TextInput, type Column } from '@/components/ui'
import { DeptSelect, DeptTree, DictSelect, FormDialog, ListPage, PermissionButton } from '@/components/business'
import BoardView, { type BoardColumn } from '@/components/business/BoardView.vue'
import { systemApi, type UserRow } from '@/api'
import { useListPage } from '@/composables/list'
import { useViewMode } from '@/composables/view'
import { hasPermission } from '@/directives/permission'
import { confirm } from '@/composables/confirm'
import { pushToast, toastError } from '@/composables/toast'
import { flattenDepts } from '@/composables/dept'
import type { Dept } from '@/data'
import { email, phone, required, validate } from '@/utils/validate'

const route = useRoute()
const router = useRouter()
/* 权限溯源：跳到角色页的溯源视图并聚焦该用户 */
const tracePath = computed(() => pathOf('system/role/index'))
function openTrace(row: UserRow) {
  if (tracePath.value) router.push({ path: tracePath.value, query: { trace: `user:${row.id}` } })
}
const list = useListPage<UserRow, { keyword: string; status: string; deptId: number | null }>({
  // 从命令面板选中用户时带入账号关键词
  query: { keyword: typeof route.query.keyword === 'string' ? route.query.keyword : '', status: '', deptId: null },
  fetch: params => systemApi.users.list(params),
  exporter: params => systemApi.users.export(params),
  exportName: '用户列表.csv',
})

const columns: Column[] = [
  { key: 'nickname', title: '用户', minWidth: 200 },
  { key: 'deptName', title: '部门' },
  { key: 'postName', title: '岗位' },
  { key: 'roleName', title: '角色' },
  { key: 'phone', title: '手机号' },
  { key: 'status', title: '状态', width: 96 },
  { key: 'createTime', title: '创建时间', width: 150 },
  { key: 'actions', title: '操作', width: 150, className: 'col-actions' },
]

/* ── 看板视图：按部门 / 角色 / 状态分列；只有「按状态」允许拖动（改状态是单字段接口） ── */
const view = useViewMode('users', 'table', ['table', 'board'] as const, 'table')
const VIEWS = [{ label: '列表', value: 'table' as const }, { label: '看板', value: 'board' as const }]
type GroupBy = 'dept' | 'role' | 'status'
const groupBy = useViewMode<GroupBy>('users.group', 'dept', ['dept', 'role', 'status'] as const)
const GROUPS = [{ label: '按部门', value: 'dept' as GroupBy }, { label: '按角色', value: 'role' as GroupBy }, { label: '按状态', value: 'status' as GroupBy }]
const BOARD_LIMIT = 100 // 与后端分页上限一致
const boardRows = ref<UserRow[]>([])
const boardTotal = ref(0)
const boardKeyword = ref(list.query.keyword)
const boardLoading = ref(false)
const canEditStatus = hasPermission('system:user:edit')

async function loadBoard() {
  boardLoading.value = true
  try {
    const page = await systemApi.users.list({ keyword: boardKeyword.value || undefined, pageNum: 1, pageSize: BOARD_LIMIT })
    boardRows.value = page.rows
    boardTotal.value = page.total
  } catch (error) {
    toastError(error, '看板数据加载失败')
  } finally {
    boardLoading.value = false
  }
}
watch(view, (v) => { if (v === 'board') loadBoard() }, { immediate: true })
let keywordTimer = 0
watch(boardKeyword, () => {
  window.clearTimeout(keywordTimer)
  keywordTimer = window.setTimeout(loadBoard, 320)
})

const boardColumns = computed<BoardColumn<UserRow>[]>(() => {
  const rows = boardRows.value
  if (groupBy.value === 'status') {
    return [
      { key: 1, title: '正常', items: rows.filter(r => r.status === 1) },
      { key: 0, title: '停用', items: rows.filter(r => r.status !== 1) },
    ]
  }
  const field = groupBy.value === 'dept' ? 'deptName' : 'roleName'
  const groups = new Map<string, UserRow[]>()
  rows.forEach((row) => {
    const name = row[field] || (groupBy.value === 'dept' ? '未分配部门' : '未分配角色')
    groups.set(name, [...(groups.get(name) ?? []), row])
  })
  return [...groups.entries()]
    .sort((a, b) => b[1].length - a[1].length)
    .map(([title, items]) => ({ key: title, title, items, accepts: false }))
})

async function moveStatus({ item, to }: { item: UserRow; to: BoardColumn<UserRow> }) {
  const status = Number(to.key)
  if (status === 0) {
    const ok = await confirm({ title: '停用账号', message: `确认停用「${item.nickname}」吗？停用后该账号将无法登录。`, confirmText: '停用', danger: true })
    if (!ok) return false
  }
  try {
    await systemApi.users.updateStatus(item.id, status)
    pushToast(`已${status ? '启用' : '停用'}「${item.nickname}」`)
    await Promise.all([loadBoard(), list.load()])
    return true
  } catch (error) {
    toastError(error, '状态更新失败')
    return false
  }
}

/* ── 选项 ── */
const roles = ref<Array<{ id: number; name: string }>>([])
const posts = ref<Array<{ id: number; name: string }>>([])
const departments = ref<Dept[]>([])
const initPassword = ref('')
const roleOptions = computed(() => roles.value.map(r => ({ label: r.name, value: r.id })))
const postOptions = computed(() => posts.value.map(p => ({ label: p.name, value: p.id })))

onMounted(async () => {
  try {
    const options = await systemApi.users.options()
    roles.value = options.roles
    posts.value = options.posts
    departments.value = options.depts as unknown as Dept[]
    initPassword.value = options.initPassword
  } catch (error) {
    toastError(error, '角色与岗位选项加载失败')
  }
})

// 左侧部门树筛选
watch(() => list.query.deptId, () => list.search())

/* ── 新增 / 编辑 ── */
interface UserForm {
  id?: number
  username: string
  nickname: string
  password: string
  phone: string
  email: string
  deptId: number | null
  roleId: number | null
  postId: number | null
  status: number
}
const emptyForm = (): UserForm => ({
  username: '', nickname: '', password: initPassword.value, phone: '', email: '',
  deptId: list.query.deptId, roleId: null, postId: null, status: 1,
})
const dialog = ref(false)
const saving = ref(false)
const form = reactive<UserForm>(emptyForm())
const errors = ref<Partial<Record<keyof UserForm, string>>>({})
const isEdit = computed(() => Boolean(form.id))

// 初始密码参数晚于弹窗到达时补填
watch(initPassword, (value) => {
  if (dialog.value && !form.id && !form.password) form.password = value
})

function openCreate() {
  Object.assign(form, emptyForm(), { id: undefined })
  errors.value = {}
  dialog.value = true
}

function openEdit(row: UserRow) {
  const dept = flattenDepts(departments.value).find(d => d.dept.name === row.deptName)
  Object.assign(form, {
    id: row.id,
    username: row.username,
    nickname: row.nickname,
    password: '',
    phone: row.phone ?? '',
    email: row.email ?? '',
    deptId: dept?.dept.id ?? null,
    roleId: roles.value.find(r => r.name === row.roleName)?.id ?? null,
    postId: posts.value.find(p => p.name === row.postName)?.id ?? null,
    status: row.status,
  })
  errors.value = {}
  dialog.value = true
}

async function submit() {
  errors.value = validate(form, {
    username: [required('请输入登录账号')],
    nickname: [required('请输入用户姓名')],
    password: isEdit.value ? [] : [required('请输入初始密码'), v => (String(v).length < 6 ? '密码至少 6 位' : undefined)],
    phone: [phone],
    email: [email],
    deptId: [required('请选择所属部门')],
    roleId: [required('请选择角色')],
  })
  if (Object.keys(errors.value).length) return
  saving.value = true
  const payload = {
    username: form.username.trim(),
    nickname: form.nickname.trim(),
    phone: form.phone || undefined,
    email: form.email || undefined,
    deptId: form.deptId ?? undefined,
    status: form.status,
    roleIds: form.roleId ? [form.roleId] : [],
    postIds: form.postId ? [form.postId] : [],
  }
  try {
    if (form.id) {
      await systemApi.users.update(form.id, payload)
      pushToast('用户信息已更新')
    } else {
      await systemApi.users.create({ ...payload, password: form.password })
      pushToast('用户已新增')
    }
    dialog.value = false
    await list.load()
  } catch (error) {
    toastError(error, '用户保存失败')
  } finally {
    saving.value = false
  }
}

/* ── 重置密码 ── */
const resetTarget = ref<UserRow | null>(null)
const resetPassword = ref('')
const resetError = ref('')
const resetting = ref(false)

function openReset(row: UserRow) {
  resetTarget.value = row
  resetPassword.value = initPassword.value
  resetError.value = ''
}

async function submitReset() {
  if (!resetTarget.value) return
  if (resetPassword.value.length < 6) {
    resetError.value = '密码至少 6 位'
    return
  }
  resetting.value = true
  try {
    await systemApi.users.resetPassword(resetTarget.value.id, resetPassword.value)
    pushToast(`已重置「${resetTarget.value.nickname}」的密码`)
    resetTarget.value = null
  } catch (error) {
    toastError(error, '密码重置失败')
  } finally {
    resetting.value = false
  }
}

/* ── 删除 / 批量 ── */
async function remove(row: UserRow) {
  const ok = await confirm({ title: '删除用户', message: `确认删除用户「${row.nickname}」吗？删除后该账号将无法登录。`, confirmText: '删除', danger: true })
  if (!ok) return
  try {
    await systemApi.users.remove(row.id)
    pushToast('用户已删除')
    await list.load()
  } catch (error) {
    toastError(error, '用户删除失败')
  }
}

async function batchStatus(ids: (string | number)[], status: 0 | 1) {
  try {
    await Promise.all(ids.map(id => systemApi.users.updateStatus(Number(id), status)))
    pushToast(`已${status ? '启用' : '停用'} ${ids.length} 个用户`)
    await list.load()
  } catch (error) {
    toastError(error, '批量更新状态失败')
  }
}

async function batchRemove(ids: (string | number)[]) {
  const ok = await confirm({ title: '批量删除', message: `确认删除选中的 ${ids.length} 个用户吗？`, confirmText: '删除', danger: true })
  if (!ok) return
  try {
    await Promise.all(ids.map(id => systemApi.users.remove(Number(id))))
    pushToast(`已删除 ${ids.length} 个用户`)
    await list.load()
  } catch (error) {
    toastError(error, '批量删除失败')
  }
}
</script>

<template>
  <ListPage
    title="用户管理"
    desc="维护后台账号、所属部门、岗位与角色；停用的账号无法登录。"
    table-title="用户列表"
    :list="list"
    :columns="columns"
    selectable
    :min-width="1040"
    :alt-view="view === 'board'"
  >
    <template #actions>
      <Segmented v-model="view" label="视图" :options="VIEWS" />
      <PermissionButton perm="system:user:add" variant="primary" icon="plus" @click="openCreate">新增用户</PermissionButton>
    </template>

    <template #alt>
      <div class="board-toolbar">
        <Segmented v-model="groupBy" label="分列方式" :options="GROUPS" />
        <TextInput v-model="boardKeyword" icon="search" placeholder="账号 / 姓名 / 手机号" clearable @enter="loadBoard" />
        <span class="spacer" />
        <span class="eyebrow">{{ boardTotal > BOARD_LIMIT ? `前 ${BOARD_LIMIT} / ${boardTotal}` : `共 ${boardTotal} 个账号` }}</span>
        <span v-if="groupBy === 'status' && canEditStatus" class="faint" style="font-size: 12px">拖动卡片切换启用 / 停用</span>
      </div>
      <BoardView
        :columns="boardColumns"
        item-key="id"
        :draggable="groupBy === 'status' && canEditStatus"
        :on-move="groupBy === 'status' && canEditStatus ? moveStatus : undefined"
        :empty-text="boardLoading ? '加载中…' : '暂无账号'"
      >
        <template #card="{ item }">
          <div class="ucard-head">
            <HoverCard>
              <Avatar :name="item.nickname" :size="32" />
              <template #card>
                <div class="hc-head">
                  <Avatar :name="item.nickname" :size="40" />
                  <div>
                    <div class="hc-name">{{ item.nickname }}</div>
                    <div class="hc-sub">@{{ item.username }}</div>
                  </div>
                </div>
                <dl class="hc-rows">
                  <dt>部门</dt><dd>{{ item.deptName || '—' }}</dd>
                  <dt>岗位</dt><dd>{{ item.postName || '—' }}</dd>
                  <dt>手机</dt><dd class="tnum">{{ item.phone || '—' }}</dd>
                  <dt>邮箱</dt><dd>{{ item.email || '—' }}</dd>
                </dl>
              </template>
            </HoverCard>
            <div style="min-width: 0">
              <div class="ucard-name ellipsis">{{ item.nickname }}</div>
              <div class="ucard-sub ellipsis">@{{ item.username }} · {{ groupBy === 'dept' ? (item.postName || '未设岗位') : (item.deptName || '未分配部门') }}</div>
            </div>
          </div>
          <div class="ucard-tags">
            <Tag v-if="item.roleName && groupBy !== 'role'" tone="info">{{ item.roleName }}</Tag>
            <Tag v-if="item.postName && groupBy !== 'dept'">{{ item.postName }}</Tag>
          </div>
          <div class="ucard-foot">
            <span class="ucard-time">{{ item.lastLogin ? `最近登录 ${item.lastLogin}` : '从未登录' }}</span>
            <span :class="['ucard-state', { 'is-off': item.status !== 1 }]">{{ item.status === 1 ? '正常' : '停用' }}</span>
          </div>
        </template>
      </BoardView>
    </template>

    <template #side>
      <DeptTree v-model="list.query.deptId" :data="departments" />
    </template>

    <template #search>
      <Field label="关键词" wide>
        <TextInput v-model="list.query.keyword" icon="search" placeholder="账号 / 姓名 / 手机号" clearable @enter="list.search()" />
      </Field>
      <Field label="状态">
        <DictSelect v-model="list.query.status" type="sys_normal_disable" all-label="全部状态" />
      </Field>
    </template>

    <template #batch="{ ids }">
      <PermissionButton perm="system:user:edit" variant="text" size="sm" @click="batchStatus(ids, 1)">启用</PermissionButton>
      <PermissionButton perm="system:user:edit" variant="text" size="sm" @click="batchStatus(ids, 0)">停用</PermissionButton>
      <PermissionButton perm="system:user:remove" variant="text" size="sm" danger @click="batchRemove(ids)">删除</PermissionButton>
    </template>

    <template #cell-nickname="{ row }">
      <div class="cell-main">
        <HoverCard>
          <Avatar :name="row.nickname" :size="32" />
          <template #card>
            <div class="hc-head">
              <Avatar :name="row.nickname" :size="40" />
              <div>
                <div class="hc-name">{{ row.nickname }}</div>
                <div class="hc-sub">@{{ row.username }}</div>
              </div>
            </div>
            <dl class="hc-rows">
              <dt>部门</dt><dd>{{ row.deptName || '—' }}</dd>
              <dt>角色</dt><dd>{{ row.roleName || '—' }}</dd>
              <dt>邮箱</dt><dd>{{ row.email || '—' }}</dd>
              <dt>最近登录</dt><dd class="tnum">{{ row.lastLogin || '从未登录' }}</dd>
            </dl>
          </template>
        </HoverCard>
        <div style="min-width: 0">
          <div class="cell-title ellipsis">{{ row.nickname }}</div>
          <div class="cell-sub">@{{ row.username }}</div>
        </div>
      </div>
    </template>
    <template #cell-postName="{ row }"><span class="muted">{{ row.postName || '—' }}</span></template>
    <template #cell-roleName="{ row }"><Tag v-if="row.roleName" tone="info">{{ row.roleName }}</Tag><span v-else class="faint">未分配</span></template>
    <template #cell-phone="{ row }"><span class="tnum muted">{{ row.phone || '—' }}</span></template>
    <template #cell-status="{ row }"><StatusTag :status="row.status" /></template>
    <template #cell-createTime="{ row }"><span class="tnum soft">{{ row.createTime }}</span></template>
    <template #cell-actions="{ row }">
      <div class="row-actions">
        <PermissionButton perm="system:user:edit" icon-only icon="edit" label="编辑" @click="openEdit(row)" />
        <PermissionButton v-if="tracePath" perm="system:role:list" icon-only icon="shield" label="权限溯源" @click="openTrace(row)" />
        <PermissionButton perm="system:user:resetPwd" icon-only icon="key" label="重置密码" @click="openReset(row)" />
        <PermissionButton perm="system:user:remove" icon-only icon="trash" label="删除" danger @click="remove(row)" />
      </div>
    </template>

    <FormDialog :open="dialog" :title="isEdit ? '编辑用户' : '新增用户'" :saving="saving" @close="dialog = false" @submit="submit">
      <Field label="登录账号" required :error="errors.username">
        <TextInput v-model="form.username" placeholder="字母、数字组合" :disabled="isEdit" :invalid="!!errors.username" autocomplete="off" />
      </Field>
      <Field label="用户姓名" required :error="errors.nickname">
        <TextInput v-model="form.nickname" placeholder="真实姓名" :invalid="!!errors.nickname" />
      </Field>
      <Field v-if="!isEdit" label="初始密码" required :error="errors.password" hint="默认取自系统参数中的用户初始密码" span>
        <TextInput v-model="form.password" placeholder="至少 6 位" :invalid="!!errors.password" autocomplete="new-password" />
      </Field>
      <Field label="手机号码" :error="errors.phone">
        <TextInput v-model="form.phone" type="tel" placeholder="选填" :invalid="!!errors.phone" />
      </Field>
      <Field label="电子邮箱" :error="errors.email">
        <TextInput v-model="form.email" type="email" placeholder="选填" :invalid="!!errors.email" />
      </Field>
      <Field label="所属部门" required :error="errors.deptId">
        <DeptSelect v-model="form.deptId" :data="departments" :invalid="!!errors.deptId" />
      </Field>
      <Field label="岗位">
        <Select v-model="form.postId" :options="postOptions" placeholder="选填" clearable />
      </Field>
      <Field label="角色" required :error="errors.roleId">
        <Select v-model="form.roleId" :options="roleOptions" :invalid="!!errors.roleId" />
      </Field>
      <Field label="账号状态" group>
        <Switch :model-value="form.status === 1" :label="form.status === 1 ? '正常，允许登录' : '停用，禁止登录'" @update:model-value="form.status = $event ? 1 : 0" />
      </Field>
    </FormDialog>

    <FormDialog
      :open="Boolean(resetTarget)"
      title="重置密码"
      :sub="resetTarget ? `${resetTarget.nickname}（@${resetTarget.username}）` : ''"
      :width="440"
      :saving="resetting"
      submit-text="确认重置"
      @close="resetTarget = null"
      @submit="submitReset"
    >
      <Field label="新密码" required :error="resetError" hint="默认取自用户初始密码，请通知用户登录后修改" span>
        <TextInput v-model="resetPassword" :invalid="!!resetError" autocomplete="new-password" />
      </Field>
    </FormDialog>
  </ListPage>
</template>
