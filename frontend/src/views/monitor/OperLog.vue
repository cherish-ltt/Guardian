<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { Drawer, Field, Metric, SectionLabel, Segmented, Select, Tag, TextInput, type Column } from '@/components/ui'
import { ListPage, PermissionButton } from '@/components/business'
import TrendChart from '@/components/charts/TrendChart.vue'
import Swimlane from '@/components/charts/Swimlane.vue'
import { systemApi, type OperLogStats, type OperTimeline, type StatsRange } from '@/api'
import { useListPage } from '@/composables/list'
import { confirm } from '@/composables/confirm'
import { pushToast, toastError } from '@/composables/toast'
import { revealCascade } from '@/composables/motion'

interface OperRow {
  id: number
  module: string
  action: string
  operName: string
  dept?: string
  method: string
  url: string
  ip?: string
  location?: string
  params?: string
  result: number
  errorMsg?: string
  cost: number
  operTime: string
}

const route = useRoute()
const list = useListPage<OperRow, { keyword: string; result: string }>({
  // 从工作台泳道跳转时带入模块关键词
  query: { keyword: typeof route.query.keyword === 'string' ? route.query.keyword : '', result: '' },
  fetch: params => systemApi.operLogs.list(params) as Promise<any>,
  exporter: params => systemApi.operLogs.export(params),
  exportName: '操作日志.csv',
})

const columns: Column[] = [
  { key: 'module', title: '操作', minWidth: 180 },
  { key: 'operName', title: '操作人' },
  { key: 'url', title: '请求' },
  { key: 'ip', title: '来源' },
  { key: 'result', title: '结果', width: 90 },
  { key: 'cost', title: '耗时', width: 90, align: 'right' },
  { key: 'operTime', title: '操作时间', width: 168 },
  { key: 'actions', title: '', width: 60, className: 'col-actions' },
]

const RESULTS = [{ label: '全部结果', value: '' }, { label: '成功', value: '1' }, { label: '失败', value: '0' }]
const METHOD_TONE: Record<string, string> = { GET: 'neutral', POST: 'info', PUT: 'info', DELETE: 'danger' }

const detail = ref<OperRow | null>(null)
const detailBody = ref<HTMLElement | null>(null)
watch(detail, async (row) => {
  if (!row) return
  await nextTick()
  revealCascade(detailBody.value)
})
const prettyParams = computed(() => {
  const raw = detail.value?.params
  if (!raw) return '（无）'
  try { return JSON.stringify(JSON.parse(raw), null, 2) } catch { return raw }
})

/* ── 01 概览 / 02 趋势：横带分层的上两层 ── */
const RANGES = [{ label: '24 小时', value: '24h' as StatsRange }, { label: '7 天', value: '7d' as StatsRange }, { label: '30 天', value: '30d' as StatsRange }]
const range = ref<StatsRange>('24h')
const view = ref<'trend' | 'lanes'>('trend')
const stats = ref<OperLogStats | null>(null)
const timeline = ref<OperTimeline | null>(null)
const series = computed(() => stats.value?.series ?? [])
const tableAnchor = ref<HTMLElement | null>(null)

async function loadStats() {
  try {
    stats.value = await systemApi.operLogs.stats(range.value)
  } catch (error) {
    toastError(error, '统计加载失败')
  }
}
async function loadTimeline() {
  try {
    timeline.value = await systemApi.operLogs.timeline()
  } catch (error) {
    toastError(error, '泳道加载失败')
  }
}
watch(range, loadStats)
watch(view, (v) => { if (v === 'lanes' && !timeline.value) loadTimeline() })
onMounted(loadStats)

/** 泳道块 → 明细：按模块过滤并滚到表格 */
function focusModule(module: string) {
  list.query.keyword = module
  list.search()
  tableAnchor.value?.scrollIntoView({ behavior: 'smooth', block: 'start' })
}

function reloadAll() {
  loadStats()
  if (timeline.value) loadTimeline()
}

async function batchRemove(ids: (string | number)[]) {
  const ok = await confirm({ title: '删除日志', message: `确认删除选中的 ${ids.length} 条操作日志吗？`, confirmText: '删除', danger: true })
  if (!ok) return
  try {
    await Promise.all(ids.map(id => systemApi.operLogs.remove(id)))
    pushToast(`已删除 ${ids.length} 条日志`)
    await list.load()
    reloadAll()
  } catch (error) {
    toastError(error, '日志删除失败')
  }
}

async function clearAll() {
  const ok = await confirm({ title: '清空操作日志', message: '确认清空全部操作日志吗？清空后无法恢复，建议先导出留档。', confirmText: '清空', danger: true })
  if (!ok) return
  try {
    await systemApi.operLogs.clear()
    pushToast('操作日志已清空')
    await list.search()
    reloadAll()
  } catch (error) {
    toastError(error, '清空失败')
  }
}
</script>

<template>
  <ListPage title="操作日志" desc="记录后台所有写操作的操作人、请求参数、结果与耗时，用于审计与问题追溯。" table-title="操作记录" :list="list" :columns="columns" selectable :min-width="1040">
    <template #before>
      <section class="band reveal">
        <div class="band-head">
          <SectionLabel title="概览" />
          <Segmented v-model="range" label="统计范围" :options="RANGES" />
        </div>
        <div class="readout-strip">
          <Metric label="操作次数" :value="stats?.total" unit="次" />
          <Metric label="成功率" :value="stats?.successRate" :decimals="1" unit="%" />
          <Metric label="平均耗时" :value="stats?.avgCost" unit="ms" />
          <Metric label="P95 耗时" :value="stats?.p95Cost" unit="ms" />
          <Metric label="失败" :value="stats?.failed" unit="次" :class="{ 'is-alert': stats?.failed }" />
          <Metric label="活跃账号" :value="stats?.activeUsers" unit="个" />
        </div>
      </section>

      <section class="band reveal">
        <div class="band-head">
          <SectionLabel :title="view === 'trend' ? '调用与失败趋势' : '今日操作泳道'" />
          <Segmented v-model="view" label="趋势视图" :options="[{ label: '趋势', value: 'trend' }, { label: '泳道', value: 'lanes' }]" />
        </div>
        <TrendChart
          v-if="view === 'trend'"
          :values="series.map(b => b.total)"
          :secondary="series.map(b => b.failed)"
          :labels="series.map(b => b.bucket)"
          :band="series.length ? [series.length - 2, series.length - 1] : null"
          :height="236"
          unit=" 次"
          primary-label="操作"
          secondary-label="失败"
          :dots="false"
        />
        <template v-else>
          <Swimlane v-if="timeline?.lanes.length" :lanes="timeline.lanes" :now-sec="timeline.nowSec" @select="focusModule($event.lane.module)" />
          <p v-else-if="timeline" class="band-empty faint">今天还没有操作记录</p>
        </template>
      </section>

      <div ref="tableAnchor" class="band-label reveal">
        <SectionLabel title="明细" />
      </div>
    </template>

    <template #actions>
      <PermissionButton perm="monitor:operlog:remove" variant="secondary" icon="trash" danger @click="clearAll">清空日志</PermissionButton>
    </template>

    <template #search>
      <Field label="关键词" wide>
        <TextInput v-model="list.query.keyword" icon="search" placeholder="模块 / 操作人 / 请求地址" clearable @enter="list.search()" />
      </Field>
      <Field label="结果">
        <Select v-model="list.query.result" :options="RESULTS" />
      </Field>
    </template>

    <template #batch="{ ids }">
      <PermissionButton perm="monitor:operlog:remove" variant="text" size="sm" danger @click="batchRemove(ids)">删除</PermissionButton>
    </template>

    <template #cell-module="{ row }">
      <div>
        <div class="cell-title">{{ row.module }}</div>
        <div class="cell-sub">{{ row.action }}</div>
      </div>
    </template>
    <template #cell-url="{ row }">
      <span style="display: inline-flex; align-items: center; gap: 6px; min-width: 0">
        <Tag :tone="METHOD_TONE[row.method] ?? 'neutral'">{{ row.method }}</Tag>
        <span class="mono ellipsis muted" style="max-width: 220px">{{ row.url }}</span>
      </span>
    </template>
    <template #cell-ip="{ row }">
      <div>
        <div class="tnum" style="font-size: 12.5px">{{ row.ip || '—' }}</div>
        <div v-if="row.location" class="cell-sub">{{ row.location }}</div>
      </div>
    </template>
    <template #cell-result="{ row }"><Tag :tone="row.result === 1 ? 'ok' : 'danger'" dot>{{ row.result === 1 ? '成功' : '失败' }}</Tag></template>
    <template #cell-cost="{ row }"><span class="tnum soft">{{ row.cost }} ms</span></template>
    <template #cell-operTime="{ row }"><span class="tnum soft">{{ row.operTime }}</span></template>
    <template #cell-actions="{ row }">
      <PermissionButton perm="monitor:operlog:list" icon-only icon="eye" label="查看详情" @click="detail = row" />
    </template>

    <Drawer :open="Boolean(detail)" origin-from="tr" title="操作详情" :sub="detail ? `${detail.module} · ${detail.action}` : ''" :width="560" @close="detail = null">
      <template v-if="detail">
        <div ref="detailBody" class="drawer-stack">
        <div class="kv-grid">
          <div class="kv"><div class="kv-label">操作人</div><div class="kv-value">{{ detail.operName }}<span v-if="detail.dept" class="soft"> · {{ detail.dept }}</span></div></div>
          <div class="kv"><div class="kv-label">操作时间</div><div class="kv-value tnum">{{ detail.operTime }}</div></div>
          <div class="kv span-2"><div class="kv-label">请求</div><div class="kv-value mono">{{ detail.method }} {{ detail.url }}</div></div>
          <div class="kv"><div class="kv-label">来源</div><div class="kv-value tnum">{{ detail.ip || '—' }} {{ detail.location || '' }}</div></div>
          <div class="kv"><div class="kv-label">结果 / 耗时</div><div class="kv-value"><Tag :tone="detail.result === 1 ? 'ok' : 'danger'" dot>{{ detail.result === 1 ? '成功' : '失败' }}</Tag> <span class="tnum soft">{{ detail.cost }} ms</span></div></div>
        </div>
        <div class="section-label" style="margin: 20px 0 10px">请求参数</div>
        <pre class="code-block">{{ prettyParams }}</pre>
        <template v-if="detail.errorMsg">
          <div class="section-label" style="margin: 20px 0 10px">异常信息</div>
          <pre class="code-block" style="white-space: pre-wrap">{{ detail.errorMsg }}</pre>
        </template>
        </div>
      </template>
    </Drawer>
  </ListPage>
</template>
