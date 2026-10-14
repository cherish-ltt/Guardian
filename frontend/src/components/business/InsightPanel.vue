<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from 'vue'
import IconButton from '@/components/ui/IconButton.vue'
import Metric from '@/components/ui/Metric.vue'
import Icon from '@/components/Icon.vue'
import { revealCascade } from '@/composables/motion'

/* ============================================================
   分析栏：关系图 / 画布的右侧详情。头部分类 → 标题与状态 → 主读数 + 迷你柱 →
   读数块 → 洞察卡 → 关联芯片（点击即下钻）。内容按阅读序级联浮现（结 Crystallize）。
   ============================================================ */
export interface InsightTile { label: string; value: number | string | null; unit?: string }
export interface InsightCard { tone?: 'info' | 'warn' | 'ok' | 'danger'; title: string; text: string }
export interface InsightChip { id: string; label: string; sub?: string; cat?: number; tone?: 'danger' | 'off' }
export interface InsightGroup { title: string; icon?: string; chips: InsightChip[]; more?: number }

const props = defineProps<{
  /** 内容主键：变化时重放级联 */
  contentKey: string
  kind: string
  kindIcon?: string
  cat?: number
  title: string
  subtitle?: string
  status?: string
  metric?: { label: string; value: number | null; unit?: string; bars?: number[] }
  tiles?: InsightTile[]
  insights?: InsightCard[]
  groups?: InsightGroup[]
}>()
const emit = defineEmits<{ (e: 'close'): void; (e: 'chip', chip: InsightChip): void }>()
const body = ref<HTMLElement | null>(null)

function play() {
  nextTick(() => revealCascade(body.value, ':scope > *'))
}
onMounted(play)
watch(() => props.contentKey, play)

const barMax = (bars: number[]) => Math.max(1, ...bars)
</script>

<template>
  <aside :class="['insight m-float', cat ? `cat-${cat}` : '']" role="region" :aria-label="`${title} 的分析`">
    <header class="insight-head">
      <span class="insight-kind"><span class="insight-kind-icon"><Icon :name="kindIcon ?? 'layers'" :size="13" /></span>{{ kind }}</span>
      <IconButton name="x" label="关闭分析" small @click="emit('close')" />
    </header>
    <div ref="body" class="insight-body">
      <div class="insight-title">
        <h3>{{ title }}</h3>
        <p v-if="subtitle" class="insight-subtitle">{{ subtitle }}</p>
        <p v-if="status" class="insight-status">{{ status }}</p>
      </div>
      <section v-if="metric" class="insight-metric">
        <span class="insight-metric-label">{{ metric.label }}</span>
        <div class="insight-metric-row">
          <Metric :value="metric.value" :unit="metric.unit" size="m" />
          <span v-if="metric.bars?.length" class="insight-bars" aria-hidden="true">
            <i v-for="(b, i) in metric.bars" :key="i" :style="{ '--h': Math.max(0.08, b / barMax(metric.bars)), '--d': `${120 + i * 28}ms` }" />
          </span>
        </div>
      </section>
      <dl v-if="tiles?.length" class="insight-tiles">
        <div v-for="t in tiles" :key="t.label">
          <dt>{{ t.label }}</dt>
          <dd class="tnum">{{ t.value ?? '—' }}<small v-if="t.unit">{{ t.unit }}</small></dd>
        </div>
      </dl>
      <section v-if="insights?.length" class="insight-cards">
        <article v-for="c in insights" :key="c.title" :class="['insight-card', `tone-${c.tone ?? 'info'}`]">
          <h4>{{ c.title }}</h4>
          <p>{{ c.text }}</p>
        </article>
      </section>
      <section v-for="g in groups" :key="g.title" class="insight-group">
        <h4><Icon v-if="g.icon" :name="g.icon" :size="12" />{{ g.title }}<b class="tnum">{{ g.chips.length + (g.more ?? 0) }}</b></h4>
        <div class="insight-chips">
          <button
            v-for="c in g.chips"
            :key="c.id"
            type="button"
            :class="['insight-chip', c.cat ? `cat-${c.cat}` : '', c.tone ? `tone-${c.tone}` : '']"
            @click="emit('chip', c)"
          >{{ c.label }}<small v-if="c.sub">{{ c.sub }}</small></button>
          <span v-if="g.more" class="insight-chip is-more">+{{ g.more }}</span>
          <span v-if="!g.chips.length" class="faint">暂无</span>
        </div>
      </section>
      <slot />
    </div>
  </aside>
</template>
