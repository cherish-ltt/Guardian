<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import Icon from '@/components/Icon.vue'
import { Button, Field, IconButton, Segmented, SectionLabel, Switch, Tag, TextInput, type Column } from '@/components/ui'
import { FormDialog, ListPage, PermissionButton } from '@/components/business'
import { systemApi } from '@/api'
import { useListPage } from '@/composables/list'
import { confirm } from '@/composables/confirm'
import { pushToast, toastError } from '@/composables/toast'
import { pattern, required, validate } from '@/utils/validate'
import { useViewMode } from '@/composables/view'
import { hasPermission } from '@/directives/permission'
import { floatBarEnter, floatBarLeave, revealCascade, settlePop } from '@/composables/motion'
import { inferConfigKind, splitConfigName } from '@/utils/configValue'

interface ConfigRow { id: number; name: string; key: string; value: string; type: string; builtin: boolean; remark?: string; createTime: string }

/* 枚举取值的业务名：值保持原样写回，只换显示文案 */
const CHOICE_LABEL: Record<string, string> = { light: '浅色', dark: '深色', auto: '跟随系统', true: '开启', false: '关闭' }
const route = useRoute()
const list = useListPage<ConfigRow, { keyword: string }>({
  query: { keyword: typeof route.query.keyword === 'string' ? route.query.keyword : '' },
  fetch: params => systemApi.configs.list(params) as Promise<any>,
  exporter: params => systemApi.configs.export(params),
  exportName: '参数配置.csv',
})

const columns: Column[] = [
  { key: 'name', title: '参数名称', minWidth: 180 },
  { key: 'key', title: '参数键名' },
  { key: 'value', title: '参数值' },
  { key: 'builtin', title: '类型', width: 90 },
  { key: 'remark', title: '备注' },
  { key: 'actions', title: '操作', width: 96, className: 'col-actions' },
]

/* ── 面板视图：按分组的设置中心，行内直接改值，改动累积到底部浮条一次提交 ── */
const view = useViewMode('configs', 'panel', ['panel', 'table'] as const, 'table')
const VIEWS = [{ label: '面板', value: 'panel' as const }, { label: '表格', value: 'table' as const }]
const canEdit = hasPermission('system:config:edit')
const all = ref<ConfigRow[]>([])
const panelLoading = ref(false)
const filter = ref('')
const drafts = reactive<Record<number, string>>({})
const failed = ref<Set<number>>(new Set())
const revealed = ref<Set<number>>(new Set())
const showDiff = ref(false)
const committing = ref(false)
const body = ref<HTMLElement | null>(null)

async function loadPanel() {
  panelLoading.value = true
  try {
    const page = await systemApi.configs.list({ pageNum: 1, pageSize: 500 })
    all.value = page.rows as unknown as ConfigRow[]
    await nextTick()
    revealCascade(body.value, '.settings-group', 0.08)
  } catch (error) {
    toastError(error, '参数加载失败')
  } finally {
    panelLoading.value = false
  }
}
watch(view, async (next, prev) => {
  if (prev === 'panel' && Object.keys(drafts).length) {
    const ok = await confirm({ title: '放弃未保存的改动', message: '面板中还有未保存的参数改动，切换视图将放弃这些改动。', confirmText: '放弃改动', danger: true })
    if (!ok) { view.value = 'panel'; return }
    discard()
  }
  if (next === 'panel') loadPanel()
}, { immediate: true })

interface SettingItem { row: ConfigRow; title: string; kind: ReturnType<typeof inferConfigKind>; locked: boolean }
const groups = computed(() => {
  const key = filter.value.trim().toLowerCase()
  const map = new Map<string, SettingItem[]>()
  for (const row of all.value) {
    if (key && !`${row.name} ${row.key} ${row.remark ?? ''}`.toLowerCase().includes(key)) continue
    const { group, title } = splitConfigName(row.name, row.key)
    const item = { row, title, kind: inferConfigKind(row.key, row.value, row.remark), locked: row.builtin || !canEdit }
    map.set(group, [...(map.get(group) ?? []), item])
  }
  return [...map.entries()].map(([name, items], i) => ({ name, items, index: String(i + 1).padStart(2, '0') }))
})

const valueOf = (row: ConfigRow) => (row.id in drafts ? drafts[row.id] : row.value)
function setValue(row: ConfigRow, value: string | number | boolean | null) {
  const next = value === null ? '' : String(value)
  if (next === row.value) delete drafts[row.id]
  else drafts[row.id] = next
  failed.value.delete(row.id)
}
const changes = computed(() => all.value.filter(r => r.id in drafts).map(r => ({ row: r, from: r.value, to: drafts[r.id] })))
const invalid = computed(() => changes.value.filter(c => !c.to.trim()).map(c => c.row.id))

function discard() {
  Object.keys(drafts).forEach(id => delete drafts[Number(id)])
  failed.value = new Set()
  showDiff.value = false
}

async function commit() {
  if (invalid.value.length) { pushToast('参数值不能为空', 'danger'); return }
  committing.value = true
  const done: number[] = []
  const bad = new Set<number>()
  // 逐条提交：失败项留在浮条里继续可改，成功项落定
  for (const c of changes.value) {
    try {
      await systemApi.configs.update(c.row.id, { value: c.to })
      c.row.value = c.to
      delete drafts[c.row.id]
      done.push(c.row.id)
    } catch {
      bad.add(c.row.id)
    }
  }
  failed.value = bad
  committing.value = false
  if (done.length) {
    pushToast(`已保存 ${done.length} 项参数`)
    nextTick(() => done.forEach(id => settlePop(document.querySelector(`[data-config="${id}"] .settings-control`))))
    list.load()
  }
  if (bad.size) pushToast(`${bad.size} 项保存失败，已保留在待提交列表`, 'danger')
  else showDiff.value = false
}

function jump(name: string) {
  document.getElementById(`settings-${name}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' })
}
const displayValue = (item: SettingItem, v: string) => (item.kind.kind === 'secret' && !revealed.value.has(item.row.id) ? '••••••' : v)
function toggleReveal(id: number) {
  const next = new Set(revealed.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  revealed.value = next
}

interface ConfigForm { id?: number; name: string; key: string; value: string; builtin: boolean; remark: string }
const emptyForm = (): ConfigForm => ({ name: '', key: '', value: '', builtin: false, remark: '' })
const dialog = ref(false)
const saving = ref(false)
const form = reactive<ConfigForm>(emptyForm())
const errors = ref<Partial<Record<keyof ConfigForm, string>>>({})

function openCreate() {
  Object.assign(form, emptyForm(), { id: undefined })
  errors.value = {}
  dialog.value = true
}

function openEdit(row: ConfigRow) {
  Object.assign(form, { id: row.id, name: row.name, key: row.key, value: row.value, builtin: row.builtin, remark: row.remark ?? '' })
  errors.value = {}
  dialog.value = true
}

async function submit() {
  errors.value = validate(form, {
    name: [required('请输入参数名称')],
    key: [required('请输入参数键名'), pattern(/^[A-Za-z][\w.-]*$/, '字母开头，仅含字母、数字、点、下划线')],
    value: [required('请输入参数值')],
  })
  if (Object.keys(errors.value).length) return
  saving.value = true
  const payload = { name: form.name.trim(), key: form.key.trim(), value: form.value, type: form.builtin ? 'Y' : 'N', builtin: form.builtin, remark: form.remark }
  try {
    if (form.id) {
      await systemApi.configs.update(form.id, payload)
      pushToast('参数已更新')
    } else {
      await systemApi.configs.create(payload)
      pushToast('参数已新增')
    }
    dialog.value = false
    await list.load()
    if (view.value === 'panel') loadPanel()
  } catch (error) {
    toastError(error, '参数保存失败')
  } finally {
    saving.value = false
  }
}

async function remove(row: ConfigRow) {
  const ok = await confirm({ title: '删除参数', message: `确认删除参数「${row.name}」（${row.key}）吗？读取该参数的功能将回到默认值。`, confirmText: '删除', danger: true })
  if (!ok) return
  try {
    await systemApi.configs.remove(row.id)
    pushToast('参数已删除')
    await list.load()
    if (view.value === 'panel') loadPanel()
  } catch (error) {
    toastError(error, '参数删除失败')
  }
}
</script>

<template>
  <ListPage title="参数配置" desc="以键值对维护系统运行参数，如账号初始密码、验证码开关等。" table-title="参数列表" :list="list" :columns="columns" :min-width="900" :alt-view="view === 'panel'">
    <template #alt>
      <div class="settings">
        <aside class="settings-nav m-surface">
          <TextInput v-model="filter" icon="search" placeholder="搜索参数" clearable />
          <nav aria-label="参数分组">
            <button v-for="g in groups" :key="g.name" type="button" class="settings-nav-item" @click="jump(g.name)">
              <span class="mono faint">{{ g.index }}</span>{{ g.name }}<b class="tnum">{{ g.items.length }}</b>
            </button>
          </nav>
          <p class="settings-nav-note faint"><Icon name="lock" :size="12" />内置参数只读，可在表格视图查看</p>
        </aside>
        <div ref="body" class="settings-body">
          <p v-if="panelLoading && !all.length" class="band-empty faint">正在加载参数…</p>
          <p v-else-if="!groups.length" class="band-empty faint">没有匹配的参数</p>
          <section v-for="g in groups" :id="`settings-${g.name}`" :key="g.name" class="settings-group m-surface">
            <SectionLabel :title="g.name" as="h3" />
            <div
              v-for="item in g.items"
              :key="item.row.id"
              :data-config="item.row.id"
              :class="['settings-row', { 'is-dirty': item.row.id in drafts, 'is-failed': failed.has(item.row.id), 'is-locked': item.locked }]"
            >
              <div class="settings-label">
                <span class="settings-title">{{ item.title }}<Icon v-if="item.row.builtin" name="lock" :size="12" class="faint" /></span>
                <span class="code-chip">{{ item.row.key }}</span>
                <span v-if="item.row.remark" class="settings-remark">{{ item.row.remark }}</span>
              </div>
              <div class="settings-control">
                <Switch v-if="item.locked && item.kind.kind === 'boolean'" :model-value="item.row.value === 'true'" :label="item.row.value === 'true' ? '开启' : '关闭'" disabled />
                <template v-else-if="item.locked">
                  <span class="settings-readonly mono">{{ displayValue(item, item.row.value) }}</span>
                  <IconButton v-if="item.kind.kind === 'secret'" name="eye" :label="revealed.has(item.row.id) ? '隐藏参数值' : '显示参数值'" small @click="toggleReveal(item.row.id)" />
                </template>
                <Switch v-else-if="item.kind.kind === 'boolean'" :model-value="valueOf(item.row) === 'true'" :label="valueOf(item.row) === 'true' ? '开启' : '关闭'" @update:model-value="setValue(item.row, $event)" />
                <Segmented v-else-if="item.kind.kind === 'choice'" :model-value="valueOf(item.row)" :label="item.title" :options="item.kind.options.map(o => ({ label: CHOICE_LABEL[o] ?? o, value: o }))" @update:model-value="setValue(item.row, $event)" />
                <TextInput v-else-if="item.kind.kind === 'number'" class="settings-number" type="number" :model-value="valueOf(item.row)" @update:model-value="setValue(item.row, $event)" />
                <TextInput v-else :type="item.kind.kind === 'secret' && !revealed.has(item.row.id) ? 'password' : 'text'" :model-value="valueOf(item.row)" :invalid="invalid.includes(item.row.id)" autocomplete="off" @update:model-value="setValue(item.row, $event)" />
                <IconButton v-if="!item.locked && item.kind.kind === 'secret'" name="eye" :label="revealed.has(item.row.id) ? '隐藏参数值' : '显示参数值'" small @click="toggleReveal(item.row.id)" />
              </div>
              <div v-if="item.row.id in drafts" class="settings-was">
                <span>原值 <s class="mono">{{ displayValue(item, item.row.value) }}</s></span>
                <button type="button" class="settings-undo" @click="setValue(item.row, item.row.value)">撤销</button>
              </div>
            </div>
          </section>
        </div>
      </div>
      <Teleport to="body">
        <Transition :css="false" @enter="floatBarEnter" @leave="floatBarLeave">
          <div v-if="changes.length" class="float-bar settings-bar" role="region" aria-label="未保存的参数改动">
            <div v-if="showDiff" class="settings-diff">
              <div v-for="c in changes" :key="c.row.id" :class="['settings-diff-row', { 'is-failed': failed.has(c.row.id) }]">
                <span class="settings-diff-name">{{ c.row.name }}</span>
                <span class="mono settings-diff-from">{{ inferConfigKind(c.row.key, c.from).kind === 'secret' ? '••••••' : c.from }}</span>
                <Icon name="arrowRight" :size="12" />
                <span class="mono settings-diff-to">{{ inferConfigKind(c.row.key, c.from).kind === 'secret' ? '••••••' : (c.to || '（空）') }}</span>
              </div>
            </div>
            <span class="float-bar-count"><b class="tnum">{{ changes.length }}</b> 项未保存{{ failed.size ? ` · ${failed.size} 项失败` : '' }}</span>
            <span class="float-bar-sep" aria-hidden="true" />
            <Button variant="text" size="sm" @click="showDiff = !showDiff">{{ showDiff ? '收起差异' : '查看差异' }}</Button>
            <Button variant="text" size="sm" @click="discard">放弃</Button>
            <Button variant="primary" size="sm" :loading="committing" @click="commit">保存改动</Button>
          </div>
        </Transition>
      </Teleport>
    </template>

    <template #actions>
      <Segmented v-model="view" label="视图" :options="VIEWS" />
      <PermissionButton perm="system:config:add" variant="primary" icon="plus" @click="openCreate">新增参数</PermissionButton>
    </template>

    <template #search>
      <Field label="关键词" wide>
        <TextInput v-model="list.query.keyword" icon="search" placeholder="参数名称 / 键名" clearable @enter="list.search()" />
      </Field>
    </template>

    <template #cell-name="{ row }"><span class="cell-title">{{ row.name }}</span></template>
    <template #cell-key="{ row }"><span class="code-chip">{{ row.key }}</span></template>
    <template #cell-value="{ row }"><span class="mono ellipsis" style="display: inline-block; max-width: 220px">{{ row.value }}</span></template>
    <template #cell-builtin="{ row }"><Tag :tone="row.builtin ? 'info' : 'neutral'">{{ row.builtin ? '系统内置' : '自定义' }}</Tag></template>
    <template #cell-remark="{ row }"><span class="muted">{{ row.remark || '—' }}</span></template>
    <template #cell-actions="{ row }">
      <div class="row-actions">
        <PermissionButton perm="system:config:edit" icon-only icon="edit" label="编辑" @click="openEdit(row)" />
        <PermissionButton v-if="!row.builtin" perm="system:config:remove" icon-only icon="trash" label="删除" danger @click="remove(row)" />
      </div>
    </template>

    <FormDialog :open="dialog" :title="form.id ? '编辑参数' : '新增参数'" :saving="saving" @close="dialog = false" @submit="submit">
      <Field label="参数名称" required :error="errors.name">
        <TextInput v-model="form.name" placeholder="如：账号初始密码" :invalid="!!errors.name" />
      </Field>
      <Field label="参数键名" required :error="errors.key">
        <TextInput v-model="form.key" placeholder="如：sys.user.initPassword" :invalid="!!errors.key" :disabled="form.builtin && Boolean(form.id)" />
      </Field>
      <Field label="参数值" required :error="errors.value" span>
        <TextInput v-model="form.value" :invalid="!!errors.value" />
      </Field>
      <Field label="备注" span>
        <TextInput v-model="form.remark" multiline :rows="2" placeholder="参数用途说明" :maxlength="200" />
      </Field>
      <Field label="系统内置" group hint="内置参数不可删除，键名不可修改">
        <Switch v-model="form.builtin" :label="form.builtin ? '是' : '否'" />
      </Field>
    </FormDialog>
  </ListPage>
</template>
