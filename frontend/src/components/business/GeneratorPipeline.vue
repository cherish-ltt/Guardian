<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from 'vue'
import RelationGraph, { type GraphLink, type GraphNode } from '@/components/charts/RelationGraph.vue'
import Modal from '@/components/ui/Modal.vue'
import Button from '@/components/ui/Button.vue'
import Field from '@/components/ui/Field.vue'
import TextInput from '@/components/ui/TextInput.vue'
import Switch from '@/components/ui/Switch.vue'
import Icon from '@/components/Icon.vue'
import { portalApi, type GeneratorFile } from '@/api'
import { gsap, EASE, motionSkipped, slideThumb } from '@/composables/motion'
import { pushToast, toastError } from '@/composables/toast'
import { copyText } from '@/utils/clipboard'

/* ============================================================
   代码生成流水线：① 表信息 → ② 字段映射 → ③ 代码预览，一个弹窗走完。
   字段映射用关系图把「数据库字段 → Java 属性 → 页面呈现」连起来，脉冲沿生成方向流动；
   代码预览按行级联显现。字段级配置沿用现有接口（只读展示），表说明与确认状态可保存。
   ============================================================ */
export interface GenTable { id: number; tableName: string; tableComment: string; className: string; module: string; businessName: string; synced: boolean }
interface ColumnRow { id: number; col: string; comment: string; javaField: string; javaType: string; jdbcType: string; insert: boolean; edit: boolean; list: boolean; query: boolean; queryType: string; formType: string; required: boolean }

const props = defineProps<{ table: GenTable | null; step?: 1 | 2 | 3 }>()
const emit = defineEmits<{ (e: 'close'): void; (e: 'saved'): void; (e: 'generate', table: GenTable): void }>()

const step = ref<1 | 2 | 3>(1)
const columns = ref<ColumnRow[]>([])
const files = ref<GeneratorFile[]>([])
const activePath = ref('')
const form = reactive({ tableComment: '', synced: false })
const saving = ref(false)
const loading = ref(false)
const size = ref({ w: 0, h: 0 })
const stepper = ref<HTMLElement | null>(null)
const thumb = ref<HTMLElement | null>(null)
const stage = ref<HTMLElement | null>(null)
const codeEl = ref<HTMLElement | null>(null)

watch(() => props.table, async (table) => {
  if (!table) return
  step.value = props.step ?? 1
  Object.assign(form, { tableComment: table.tableComment, synced: table.synced })
  columns.value = []
  files.value = []
  loading.value = true
  try {
    const [cols, preview] = await Promise.all([portalApi.generator.tables.columns(table.id), portalApi.generator.tables.preview(table.id)])
    columns.value = cols as unknown as ColumnRow[]
    files.value = preview.files
    activePath.value = preview.files.find(f => f.name === preview.activeFile)?.path ?? preview.files[0]?.path ?? ''
  } catch (error) {
    toastError(error, '生成配置加载失败')
  } finally {
    loading.value = false
  }
}, { immediate: true })

/* ── 步骤切换：玻璃滑块 + 内容横移 ── */
const STEPS = [{ n: 1 as const, label: '表信息', icon: 'database' }, { n: 2 as const, label: '字段映射', icon: 'layers' }, { n: 3 as const, label: '代码预览', icon: 'code' }]
watch(step, async (next, prev) => {
  await nextTick()
  slideThumb(thumb.value, stepper.value?.querySelector<HTMLElement>(`[data-step="${next}"]`) ?? null, 'x', prev === undefined)
  if (stage.value && prev !== undefined && !motionSkipped()) {
    gsap.fromTo(stage.value, { x: (next > prev ? 1 : -1) * 28, opacity: 0 }, { x: 0, opacity: 1, duration: 0.5, ease: EASE.glide, clearProps: 'transform,opacity' })
  }
  if (next === 3) playCode()
}, { flush: 'post' })
watch(() => Boolean(props.table), open => { if (open) nextTick(() => slideThumb(thumb.value, stepper.value?.querySelector<HTMLElement>(`[data-step="${step.value}"]`) ?? null, 'x', true)) })

/* ── ② 字段映射图 ── */
const FORM_LABEL: Record<string, string> = { input: '输入框', textarea: '多行文本', select: '下拉选择', radio: '单选', checkbox: '多选', datetime: '日期时间', date: '日期', number: '数字', upload: '上传', editor: '富文本' }
const mapping = computed<{ nodes: GraphNode[]; links: GraphLink[] }>(() => {
  const { w, h } = size.value
  const cols = columns.value
  if (!w || !h || !cols.length) return { nodes: [], links: [] }
  const nodes: GraphNode[] = []
  const links: GraphLink[] = []
  const gap = Math.min(40, (h - 48) / Math.max(1, cols.length - 1))
  const y0 = h / 2 - (gap * (cols.length - 1)) / 2
  const xDb = w * 0.24, xJava = w * 0.5, xOut = w * 0.78
  cols.forEach((c, i) => {
    const y = y0 + i * gap
    nodes.push({ id: `db:${c.id}`, x: xDb, y, label: c.col, sub: c.jdbcType + (c.required ? ' · 必填' : ''), kind: 'item', align: 'right', cat: 2, delay: i * 0.04, title: c.comment })
    nodes.push({ id: `java:${c.id}`, x: xJava, y, label: c.javaField, sub: c.javaType, kind: 'item', cat: 3, from: `db:${c.id}`, delay: 0.3 + i * 0.04 })
    links.push({ a: `db:${c.id}`, b: `java:${c.id}`, flow: 'forward', shape: 'h', cat: 2 })
  })
  // 右列：去重后的页面呈现目标
  const targets = new Map<string, { label: string; sub: string; cat: number; members: number[] }>()
  const add = (key: string, label: string, sub: string, cat: number, id: number) => {
    if (!targets.has(key)) targets.set(key, { label, sub, cat, members: [] })
    targets.get(key)!.members.push(id)
  }
  cols.forEach((c) => {
    if (c.insert || c.edit) add(`form:${c.formType}`, `表单 · ${FORM_LABEL[c.formType] ?? c.formType}`, c.formType, 5, c.id)
    if (c.list) add('list', '列表列', 'DataTable', 1, c.id)
    if (c.query) add(`query:${c.queryType}`, `查询条件 · ${c.queryType}`, 'SearchForm', 4, c.id)
  })
  const tlist = [...targets.entries()]
  const tgap = Math.min(64, (h - 64) / Math.max(1, tlist.length - 1))
  const ty0 = h / 2 - (tgap * (tlist.length - 1)) / 2
  tlist.forEach(([key, t], i) => {
    nodes.push({ id: `out:${key}`, x: xOut, y: ty0 + i * tgap, label: t.label, sub: t.sub, count: t.members.length, kind: 'item', align: 'left', cat: t.cat, delay: 0.8 + i * 0.08 })
    t.members.forEach(id => links.push({ a: `java:${id}`, b: `out:${key}`, flow: 'forward', shape: 'h', cat: t.cat }))
  })
  return { nodes, links }
})
const mapHeight = computed(() => Math.max(380, Math.min(620, columns.value.length * 40 + 80)))
const stats = computed(() => ({
  fields: columns.value.length,
  form: columns.value.filter(c => c.insert || c.edit).length,
  list: columns.value.filter(c => c.list).length,
  query: columns.value.filter(c => c.query).length,
  required: columns.value.filter(c => c.required).length,
}))

/* ── ③ 代码预览：按行级联 ── */
const activeFile = computed(() => files.value.find(f => f.path === activePath.value))
const lines = computed(() => (activeFile.value?.code ?? '').split('\n'))
const folderOf = (path: string) => path.split('/').slice(0, -1).join('/')
const tree = computed(() => {
  const groups = new Map<string, GeneratorFile[]>()
  files.value.forEach(f => groups.set(folderOf(f.path), [...(groups.get(folderOf(f.path)) ?? []), f]))
  return [...groups.entries()]
})
function playCode() {
  nextTick(() => {
    if (!codeEl.value || motionSkipped()) return
    const rows = Array.from(codeEl.value.querySelectorAll('.code-line')).slice(0, 48)
    gsap.fromTo(rows, { opacity: 0, x: -8 }, { opacity: 1, x: 0, duration: 0.36, ease: EASE.reveal, stagger: 0.012, clearProps: 'opacity,transform', overwrite: 'auto' })
  })
}
watch(activePath, () => { if (step.value === 3) playCode() })

async function copyCode() {
  const ok = await copyText(activeFile.value?.code ?? '')
  pushToast(ok ? '已复制当前文件' : '当前浏览器不支持写入剪贴板', ok ? 'ok' : 'danger')
}

async function save() {
  if (!props.table) return
  saving.value = true
  try {
    await portalApi.generator.tables.update(props.table.id, { tableComment: form.tableComment, synced: form.synced })
    pushToast('生成配置已保存')
    emit('saved')
  } catch (error) {
    toastError(error, '配置保存失败')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <Modal :open="Boolean(table)" title="代码生成流水线" :sub="table?.tableName" :width="1120" @close="emit('close')">
    <div ref="stepper" class="pipe-steps m-groove" role="tablist" aria-label="生成步骤">
      <span ref="thumb" class="pipe-thumb m-pill" aria-hidden="true" />
      <button v-for="s in STEPS" :key="s.n" type="button" role="tab" :data-step="s.n" :aria-selected="step === s.n" :class="['pipe-step', { 'is-on': step === s.n, 'is-done': step > s.n }]" @click="step = s.n">
        <span class="pipe-step-n tnum">{{ s.n }}</span><Icon :name="s.icon" :size="14" />{{ s.label }}
      </button>
    </div>

    <div ref="stage" class="pipe-stage">
      <!-- ① 表信息 -->
      <div v-if="step === 1" class="pipe-info">
        <div class="form-grid">
          <Field label="表说明" span hint="用于生成页面标题与菜单名称">
            <TextInput v-model="form.tableComment" placeholder="如：客户档案" />
          </Field>
          <Field label="实体类"><TextInput :model-value="table?.className" readonly /></Field>
          <Field label="模块 / 业务名"><TextInput :model-value="`${table?.module} / ${table?.businessName}`" readonly /></Field>
          <Field label="配置状态" group span hint="确认字段无误后标记为已确认，便于团队区分">
            <Switch v-model="form.synced" :label="form.synced ? '已确认' : '待确认'" />
          </Field>
        </div>
        <dl class="pipe-readouts">
          <div><dt>字段</dt><dd class="tnum">{{ stats.fields }}</dd></div>
          <div><dt>表单项</dt><dd class="tnum">{{ stats.form }}</dd></div>
          <div><dt>列表列</dt><dd class="tnum">{{ stats.list }}</dd></div>
          <div><dt>查询条件</dt><dd class="tnum">{{ stats.query }}</dd></div>
          <div><dt>必填</dt><dd class="tnum">{{ stats.required }}</dd></div>
          <div><dt>生成文件</dt><dd class="tnum">{{ files.length }}</dd></div>
        </dl>
      </div>

      <!-- ② 字段映射 -->
      <div v-else-if="step === 2" class="pipe-map">
        <div class="pipe-map-head" aria-hidden="true">
          <span style="left: 24%">数据库字段</span><span style="left: 50%">Java 属性</span><span style="left: 78%">页面呈现</span>
        </div>
        <RelationGraph v-model:size="size" class="pipe-graph" :style="{ height: `${mapHeight}px` }" label="字段映射图" :nodes="mapping.nodes" :links="mapping.links" />
        <p class="faint pipe-note">字段的表单类型、查询方式按数据库类型自动推断；悬停任一节点查看它的去向。</p>
      </div>

      <!-- ③ 代码预览 -->
      <div v-else class="preview pipe-preview">
        <nav class="preview-files" aria-label="生成文件">
          <template v-for="[dir, list] in tree" :key="dir">
            <span class="pipe-dir mono">{{ dir }}</span>
            <button
              v-for="f in list"
              :key="f.path"
              type="button"
              :class="['preview-file', f.path === activePath && 'is-active']"
              :aria-current="f.path === activePath ? 'true' : undefined"
              @click="activePath = f.path"
            >
              <span class="preview-name">{{ f.name }}</span>
            </button>
          </template>
        </nav>
        <div class="preview-code">
          <div class="preview-head">
            <span class="mono soft ellipsis">{{ activeFile?.path }}</span>
            <span class="faint tnum" style="font-size: 11.5px; margin-left: auto">{{ lines.length }} 行</span>
            <Button size="sm" icon="copy" @click="copyCode">复制</Button>
          </div>
          <pre ref="codeEl" class="code-block pipe-code"><span v-for="(line, i) in lines" :key="`${activePath}-${i}`" class="code-line"><i class="tnum">{{ i + 1 }}</i>{{ line || ' ' }}</span></pre>
        </div>
      </div>
      <p v-if="loading" class="faint" style="margin: 8px 0 0">正在读取字段与生成预览…</p>
    </div>

    <template #footer>
      <Button v-if="step > 1" variant="ghost" icon="chevronLeft" @click="step = (step - 1) as 1 | 2">上一步</Button>
      <span style="flex: 1" />
      <Button variant="secondary" :loading="saving" @click="save">保存配置</Button>
      <Button v-if="step < 3" variant="primary" @click="step = (step + 1) as 2 | 3">下一步<Icon name="arrowRight" :size="14" /></Button>
      <Button v-else variant="primary" icon="download" @click="table && emit('generate', table)">生成并下载</Button>
    </template>
  </Modal>
</template>
