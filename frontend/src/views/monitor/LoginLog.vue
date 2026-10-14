<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import Icon from '@/components/Icon.vue'
import { Field, Metric, SectionLabel, Segmented, Select, Tag, TextInput, type Column } from '@/components/ui'
import { ListPage, PermissionButton } from '@/components/business'
import TrendChart from '@/components/charts/TrendChart.vue'
import HeatGrid from '@/components/charts/HeatGrid.vue'
import { systemApi, type LoginHeatmap, type LoginLogStats, type StatsRange } from '@/api'
import { useViewMode } from '@/composables/view'
import { revealCascade } from '@/composables/motion'
import { useListPage } from '@/composables/list'
import { confirm } from '@/composables/confirm'
import { pushToast, toastError } from '@/composables/toast'

interface LoginRow { id: number; username: string; status: number; msg?: string; ip?: string; location?: string; browser?: string; os?: string; loginTime: string }

const list = useListPage<LoginRow, { keyword: string; status: string }>({
  query: { keyword: '', status: '' },
  fetch: params => systemApi.loginLogs.list(params) as Promise<any>,
  exporter: params => systemApi.loginLogs.export(params),
  exportName: '登录日志.csv',
})

const columns: Column[] = [
  { key: 'username', title: '登录账号', minWidth: 140 },
  { key: 'status', title: '结果', width: 100 },
  { key: 'msg', title: '说明' },
  { key: 'ip', title: '来源' },
  { key: 'client', title: '客户端' },
  { key: 'loginTime', title: '登录时间', width: 168 },
]

const STATUSES = [{ label: '全部结果', value: '' }, { label: '成功', value: '1' }, { label: '失败', value: '0' }]

/* ── 01 概览 / 02 趋势 ── */
const RANGES = [{ label: '24 小时', value: '24h' as StatsRange }, { label: '7 天', value: '7d' as StatsRange }, { label: '30 天', value: '30d' as StatsRange }]
const range = ref<StatsRange>('24h')
const stats = ref<LoginLogStats | null>(null)
const series = computed(() => stats.value?.series ?? [])

async function loadStats() {
  try {
    stats.value = await systemApi.loginLogs.stats(range.value)
  } catch (error) {
    toastError(error, '统计加载失败')
  }
}
/* ── 02 热力：星期 × 小时；03 异常：失败聚合告警，点卡片带条件筛选明细 ── */
const trendView = useViewMode('loginlog.trend', 'heat', ['heat', 'trend'] as const)
const TREND_VIEWS = [{ label: '热力', value: 'heat' as const }, { label: '趋势', value: 'trend' as const }]
const heat = ref<LoginHeatmap | null>(null)
const heatLoading = ref(false)
const heatDays = computed<7 | 30>(() => (range.value === '30d' ? 30 : 7))
const alerts = ref<HTMLElement | null>(null)

async function loadHeat() {
  heatLoading.value = true
  try {
    heat.value = await systemApi.loginLogs.heatmap(heatDays.value)
    await nextTick()
    revealCascade(alerts.value, '.alert-card', 0.06)
  } catch (error) {
    toastError(error, '登录热力加载失败')
  } finally {
    heatLoading.value = false
  }
}

function inspect(item: LoginHeatmap['anomalies'][number]) {
  list.query.keyword = item.key
  list.query.status = '0'
  list.search()
  nextTick(() => document.querySelector('.band-label.is-detail')?.scrollIntoView({ behavior: 'smooth', block: 'start' }))
}

const route = useRoute()
watch(range, () => { loadStats(); loadHeat() })
onMounted(() => {
  // 问答与告警跳转带来的筛选条件
  if (typeof route.query.keyword === 'string') {
    list.query.keyword = route.query.keyword
    if (route.query.status === '0' || route.query.status === '1') list.query.status = route.query.status
    list.search()
  }
  loadStats()
  loadHeat()
})

async function batchRemove(ids: (string | number)[]) {
  const ok = await confirm({ title: '删除日志', message: `确认删除选中的 ${ids.length} 条登录日志吗？`, confirmText: '删除', danger: true })
  if (!ok) return
  try {
    await Promise.all(ids.map(id => systemApi.loginLogs.remove(id)))
    pushToast(`已删除 ${ids.length} 条日志`)
    await list.load()
    loadStats()
    loadHeat()
  } catch (error) {
    toastError(error, '日志删除失败')
  }
}

async function clearAll() {
  const ok = await confirm({ title: '清空登录日志', message: '确认清空全部登录日志吗？清空后无法恢复，建议先导出留档。', confirmText: '清空', danger: true })
  if (!ok) return
  try {
    await systemApi.loginLogs.clear()
    pushToast('登录日志已清空')
    await list.search()
    loadStats()
    loadHeat()
  } catch (error) {
    toastError(error, '清空失败')
  }
}
</script>

<template>
  <ListPage title="登录日志" desc="记录账号的登录、退出与失败尝试，便于发现异常登录。" table-title="登录记录" :list="list" :columns="columns" selectable :min-width="900">
    <template #before>
      <section class="band reveal">
        <div class="band-head">
          <SectionLabel title="概览" />
          <Segmented v-model="range" label="统计范围" :options="RANGES" />
        </div>
        <div class="readout-strip">
          <Metric label="登录次数" :value="stats?.total" unit="次" />
          <Metric label="成功率" :value="stats?.successRate" :decimals="1" unit="%" />
          <Metric label="失败" :value="stats?.failed" unit="次" :class="{ 'is-alert': stats?.failed }" />
          <Metric label="登录账号" :value="stats?.users" unit="个" />
          <Metric label="来源 IP" :value="stats?.ips" unit="个" />
        </div>
      </section>

      <section class="band reveal">
        <div class="band-head">
          <SectionLabel :title="trendView === 'heat' ? '登录热力' : '登录与失败趋势'" />
          <span class="band-tools">
            <span class="bento-meta">{{ trendView === 'heat' ? `近 ${heatDays} 天 · 星期 × 小时` : RANGES.find(r => r.value === range)?.label }}</span>
            <Segmented v-model="trendView" label="趋势视图" :options="TREND_VIEWS" />
          </span>
        </div>
        <HeatGrid v-if="trendView === 'heat'" :cells="heat?.cells ?? []" :loading="heatLoading" />
        <TrendChart
          v-else
          :values="series.map(b => b.total)"
          :secondary="series.map(b => b.failed)"
          :labels="series.map(b => b.bucket)"
          :band="series.length ? [series.length - 2, series.length - 1] : null"
          :height="236"
          unit=" 次"
          primary-label="登录"
          secondary-label="失败"
          :dots="false"
        />
      </section>

      <section class="band reveal">
        <div class="band-head">
          <SectionLabel title="异常聚合" />
          <span class="bento-meta">同一 IP 或账号失败 ≥ {{ heat?.threshold ?? 5 }} 次</span>
        </div>
        <div v-if="heat?.anomalies.length" ref="alerts" class="alert-grid">
          <button v-for="a in heat.anomalies" :key="`${a.kind}-${a.key}`" type="button" class="alert-card" :aria-label="`查看 ${a.key} 的失败记录`" @click="inspect(a)">
            <span class="alert-head">
              <span class="alert-kind">{{ a.kind === 'ip' ? 'IP 来源' : '登录账号' }}</span>
              <Icon name="arrowRight" :size="14" />
            </span>
            <span class="alert-key">{{ a.key }}</span>
            <span class="alert-figure"><b class="tnum">{{ a.failed }}</b><span>次失败 / 共 {{ a.total }} 次 · {{ a.kind === 'ip' ? `尝试 ${a.spread} 个账号` : `来自 ${a.spread} 个 IP` }}</span></span>
            <span class="alert-meta">{{ a.firstTime }} → {{ a.lastTime }}</span>
          </button>
        </div>
        <p v-else class="alert-calm"><Icon name="check" :size="15" />近 {{ heatDays }} 天没有达到告警阈值的失败聚集。</p>
      </section>

      <div class="band-label is-detail reveal">
        <SectionLabel title="明细" />
      </div>
    </template>

    <template #actions>
      <PermissionButton perm="monitor:loginlog:remove" variant="secondary" icon="trash" danger @click="clearAll">清空日志</PermissionButton>
    </template>

    <template #search>
      <Field label="关键词" wide>
        <TextInput v-model="list.query.keyword" icon="search" placeholder="登录账号 / IP" clearable @enter="list.search()" />
      </Field>
      <Field label="结果">
        <Select v-model="list.query.status" :options="STATUSES" />
      </Field>
    </template>

    <template #batch="{ ids }">
      <PermissionButton perm="monitor:loginlog:remove" variant="text" size="sm" danger @click="batchRemove(ids)">删除</PermissionButton>
    </template>

    <template #cell-username="{ row }"><span class="cell-title">{{ row.username }}</span></template>
    <template #cell-status="{ row }"><Tag :tone="row.status === 1 ? 'ok' : 'danger'" dot>{{ row.status === 1 ? '成功' : '失败' }}</Tag></template>
    <template #cell-msg="{ row }"><span class="muted">{{ row.msg || '—' }}</span></template>
    <template #cell-ip="{ row }">
      <div>
        <div class="tnum" style="font-size: 12.5px">{{ row.ip || '—' }}</div>
        <div v-if="row.location" class="cell-sub">{{ row.location }}</div>
      </div>
    </template>
    <template #cell-client="{ row }"><span class="muted">{{ [row.browser, row.os].filter(Boolean).join(' · ') || '—' }}</span></template>
    <template #cell-loginTime="{ row }"><span class="tnum soft">{{ row.loginTime }}</span></template>
  </ListPage>
</template>
