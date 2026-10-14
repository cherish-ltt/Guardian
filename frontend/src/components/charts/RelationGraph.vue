<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import Icon from '@/components/Icon.vue'
import { bezierAt, createFrameLoop, motionSkipped, spring, springOf, type SpringState } from '@/composables/motion'
import { useViewportPan } from '@/composables/viewport'
import { paintDial, paintDot, paintHorizon, paintNetwork, paintOrb, paintOrbit, paintPillar, paintPulse, paintRipple, paintThread, readStagePalette, type NetworkData, type StagePalette } from '@/composables/neural'

/* ============================================================
   关系图：节点与连线由页面按当前层级算好目标坐标，本组件负责「活起来」——
   - 节点以弹簧追随目标，层级切换即布局变形；新节点从 from 节点处长出（delay 依次长出 = 触须）
   - 连线是三次贝塞尔，带单向 / 往返脉冲（导 Conduct）
   - 悬停节点：一层邻居保持，其余降到 25%
   - 方向键在节点间空间导航，Esc 发出 back 由页面逐层返回
   - variant="stage"：图谱舞台（珍珠玻璃）。连线、脉冲、珍珠球（core）、玻璃刻度表盘（hub）、
     圆点（dot）、台阶光柱（slab）与背景网络（network，真实数据点位）画在一张 Canvas 上；
     DOM 只保留可点的标签与命中区。右上角带「重播 / 暂停」。
   坐标单位是容器像素，容器尺寸通过 v-model:size 回传给页面做布局。
   ============================================================ */
export interface GraphNode {
  id: string
  x: number
  y: number
  label: string
  sub?: string
  /** core 中心盘 / hub 能力岛卡 / item 胶囊 / dot 圆点（标签悬停显示）/ slab 台阶（舞台下带光柱） */
  kind?: 'core' | 'hub' | 'item' | 'dot' | 'slab'
  cat?: number
  tone?: 'danger' | 'off' | 'accent' | 'lime'
  icon?: string
  count?: number | string
  /** 锚点在节点的哪侧：left = 节点在锚点右边展开（扇形条目） */
  align?: 'center' | 'left' | 'right'
  from?: string
  delay?: number
  interactive?: boolean
  title?: string
  /** 常驻轻漂（能力岛游动），只对 core / hub 生效，减少动态效果时静止 */
  float?: boolean
  /** 舞台变体中珍珠球 / 刻度表盘的直径、台阶的宽度（px） */
  size?: number
  /** 台阶眉标（如「第 2 级」） */
  rank?: string
  /** 表盘刻度的着色比例（0–1），如「已分配角色的用户占比」 */
  ratio?: number
}
export interface GraphLink {
  a: string
  b: string
  flow?: 'none' | 'forward' | 'both'
  cat?: number
  tone?: 'danger' | 'accent' | 'lime'
  shape?: 'arc' | 'h' | 'v'
  dashed?: boolean
}

const props = withDefaults(defineProps<{
  nodes: GraphNode[]
  links: GraphLink[]
  activeId?: string | null
  /** 额外高亮的节点（如溯源链路），为空时只按悬停高亮 */
  highlight?: string[] | null
  variant?: 'light' | 'stage'
  /** 舞台背景网络：真实数据点位与关系 */
  network?: NetworkData | null
  /** 背景网络只高亮该分类 */
  networkFocus?: number | null
  /** 背景网络整体亮度（下钻时退远） */
  networkAlpha?: number
  /** 舞台轨道：以 core 节点为圆心的椭圆半径 */
  orbits?: Array<{ rx: number; ry: number }>
  /** 改变该值即在中心播放一次迸发 */
  burstKey?: number | string
  label?: string
  pannable?: boolean
}>(), {
  activeId: null,
  highlight: null,
  variant: 'light',
  network: null,
  networkFocus: null,
  networkAlpha: 1,
  orbits: () => [],
  burstKey: 0,
  label: '关系图',
  pannable: false,
})
const emit = defineEmits<{
  (e: 'select', node: GraphNode): void
  (e: 'back'): void
  (e: 'hover', id: string | null): void
}>()
const size = defineModel<{ w: number; h: number }>('size', { default: () => ({ w: 0, h: 0 }) })

const root = ref<HTMLElement | null>(null)
const layer = ref<HTMLElement | null>(null)
const stageCanvas = ref<HTMLCanvasElement | null>(null)
const hoverId = ref<string | null>(null)
const isStage = computed(() => props.variant === 'stage')
watch(hoverId, id => emit('hover', id))
const paused = ref(false)

/* ── 节点状态：弹簧 + 出生时间；被移除的节点作为幽灵退回来处后再消失 ── */
interface NodeState { x: SpringState; y: SpringState; o: SpringState; s: SpringState; born: number; el: HTMLElement | null }
const states = new Map<string, NodeState>()
const ghosts = shallowRef<Array<GraphNode & { ghost: true }>>([])
let lastNodes = new Map<string, GraphNode>()

const allNodes = computed(() => [...props.nodes, ...ghosts.value])
const nodeById = computed(() => new Map(allNodes.value.map(n => [n.id, n])))

function ensureState(node: GraphNode, now: number) {
  let st = states.get(node.id)
  if (st) return st
  const origin = node.from ? states.get(node.from) : null
  const snap = motionSkipped()
  const sx = snap ? node.x : origin ? origin.x.x : node.x
  const sy = snap ? node.y : origin ? origin.y.x : node.y
  st = { x: springOf(sx), y: springOf(sy), o: springOf(snap ? 1 : 0), s: springOf(snap ? 1 : origin ? 0.4 : 0.82), born: now, el: null }
  states.set(node.id, st)
  return st
}

watch(() => props.nodes, (next) => {
  const now = performance.now()
  const ids = new Set(next.map(n => n.id))
  // 已移除的节点变成幽灵：退回 from 节点（或原地）淡出
  const leaving: Array<GraphNode & { ghost: true }> = []
  lastNodes.forEach((node, id) => {
    if (ids.has(id) || !states.has(id) || motionSkipped()) {
      if (!ids.has(id)) states.delete(id)
      return
    }
    const back = node.from && ids.has(node.from) ? next.find(n => n.id === node.from) : null
    leaving.push({ ...node, x: back?.x ?? node.x, y: back?.y ?? node.y, interactive: false, ghost: true })
  })
  ghosts.value = [...ghosts.value.filter(g => !ids.has(g.id)), ...leaving]
  next.forEach(n => ensureState(n, now))
  lastNodes = new Map(next.map(n => [n.id, n]))
}, { immediate: true })

/* ── 高亮集合：悬停节点的一层邻居 + 页面传入的链路 ── */
const neighbors = computed(() => {
  const map = new Map<string, Set<string>>()
  for (const l of props.links) {
    if (!map.has(l.a)) map.set(l.a, new Set())
    if (!map.has(l.b)) map.set(l.b, new Set())
    map.get(l.a)!.add(l.b)
    map.get(l.b)!.add(l.a)
  }
  return map
})
const lit = computed<Set<string> | null>(() => {
  if (hoverId.value) {
    const set = new Set([hoverId.value, ...(neighbors.value.get(hoverId.value) ?? [])])
    return set
  }
  return props.highlight?.length ? new Set(props.highlight) : null
})
const isDim = (id: string) => Boolean(lit.value && !lit.value.has(id))
const linkLit = (l: GraphLink) => !lit.value || (lit.value.has(l.a) && lit.value.has(l.b))

/* ── 连线几何 ── */
interface Geo { x1: number; y1: number; c1x: number; c1y: number; c2x: number; c2y: number; x2: number; y2: number }
function geometry(l: GraphLink, t: number, live: boolean): Geo | null {
  const A = states.get(l.a)
  const B = states.get(l.b)
  if (!A || !B) return null
  const x1 = A.x.x, y1 = A.y.x, x2 = B.x.x, y2 = B.y.x
  const dx = x2 - x1, dy = y2 - y1
  if (l.shape === 'h') {
    const sway = live ? Math.sin(t * 1.1 + x2 * 0.013 + y2 * 0.017) * 6 : 0
    return { x1, y1, c1x: x1 + dx * 0.5, c1y: y1 + sway, c2x: x2 - dx * 0.45, c2y: y2 - sway * 0.5, x2, y2 }
  }
  if (l.shape === 'v') return { x1, y1, c1x: x1, c1y: y1 + dy * 0.5, c2x: x2, c2y: y2 - dy * 0.5, x2, y2 }
  const d = Math.hypot(dx, dy) || 1
  const bend = (live ? 14 * Math.sin(t * 0.5 + (x1 + y2) * 0.01) : 0) + d * 0.08
  const ux = dx / d, uy = dy / d
  return { x1, y1, c1x: x1 + dx * 0.35 - uy * bend, c1y: y1 + dy * 0.35 + ux * bend, c2x: x1 + dx * 0.7 - uy * bend, c2y: y1 + dy * 0.7 + ux * bend, x2, y2 }
}

const pathEls: Array<SVGPathElement | null> = []
const pulseEls: Array<Array<SVGCircleElement | null>> = []
const nodeEls = new Map<string, HTMLElement>()
function bindNode(id: string, el: unknown) {
  if (el instanceof HTMLElement) nodeEls.set(id, el)
  else nodeEls.delete(id)
}

/* ── 帧循环 ── */
const loop = createFrameLoop((dt, t) => {
  const now = performance.now()
  const snap = motionSkipped()
  const live = !snap && !paused.value
  // 节点
  allNodes.value.forEach((node, i) => {
    const st = states.get(node.id)
    if (!st) return
    const ghost = 'ghost' in node
    const ready = snap || now - st.born >= (node.delay ?? 0) * 1000
    const drifts = node.float && live && (node.kind === 'hub' || node.kind === 'core')
    const drift = drifts ? { x: Math.sin(t * 0.37 + i * 1.7) * 7, y: Math.sin(t * 0.9 + i * 2.1) * 6 } : { x: 0, y: 0 }
    const tx = ready || ghost ? node.x + drift.x : st.x.x
    const ty = ready || ghost ? node.y + drift.y : st.y.x
    if (snap) {
      st.x.x = tx; st.y.x = ty; st.o.x = ghost ? 0 : 1; st.s.x = 1
    } else {
      spring(st.x, tx, dt, 70)
      spring(st.y, ty, dt, 70)
      spring(st.o, ghost ? 0 : ready ? 1 : 0, dt, ghost ? 120 : 50)
      spring(st.s, ghost ? 0.5 : ready ? 1 : st.s.x, dt, 60)
    }
    const el = nodeEls.get(node.id)
    if (el) {
      const o = Math.max(0, Math.min(1, st.o.x))
      el.style.transform = `translate3d(${st.x.x}px, ${st.y.x}px, 0) scale(${st.s.x})`
      el.style.opacity = String(o)
      el.style.visibility = o < 0.01 ? 'hidden' : ''
    }
  })
  // 幽灵退场完毕即移除
  if (ghosts.value.length && ghosts.value.every(g => (states.get(g.id)?.o.x ?? 0) < 0.02)) {
    ghosts.value.forEach(g => states.delete(g.id))
    ghosts.value = []
  }
  if (isStage.value) { drawStage(t, live); return }
  // 连线与脉冲
  props.links.forEach((l, i) => {
    const path = pathEls[i]
    if (!path) return
    const g = geometry(l, t, live)
    const A = states.get(l.a), B = states.get(l.b)
    if (!g || !A || !B) { path.style.opacity = '0'; return }
    path.setAttribute('d', `M${g.x1},${g.y1} C${g.c1x},${g.c1y} ${g.c2x},${g.c2y} ${g.x2},${g.y2}`)
    const vis = Math.max(0, Math.min(A.o.x, B.o.x, 1))
    path.style.opacity = String(vis)
    const pulses = pulseEls[i] ?? []
    pulses.forEach((c, k) => {
      if (!c) return
      if (!live || vis < 0.5 || l.flow === 'none' || !l.flow) { c.style.opacity = '0'; return }
      const speed = k ? 0.24 : 0.3
      let u = (t * speed + i * 0.137 + k * 0.5) % 1
      if (k === 1) u = 1 - u
      c.setAttribute('cx', String(bezierAt(g.x1, g.c1x, g.c2x, g.x2, u)))
      c.setAttribute('cy', String(bezierAt(g.y1, g.c1y, g.c2y, g.y2, u)))
      c.style.opacity = String(Math.sin(u * Math.PI) * vis * (linkLit(l) ? 1 : 0.25))
    })
  })
})

/* ── 图谱舞台：一张 Canvas 画背景网络、连线、脉冲、珍珠球与刻度表盘 ── */
let palette: StagePalette | null = null
let stageStart = performance.now()
let burstAt = -1
function drawStage(t: number, live: boolean) {
  const canvas = stageCanvas.value
  const { w, h } = size.value
  if (!canvas || !w || !h) return
  const dpr = Math.min(window.devicePixelRatio || 1, 2)
  if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) { canvas.width = Math.round(w * dpr); canvas.height = Math.round(h * dpr) }
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  if (!palette && root.value) palette = readStagePalette(root.value)
  const p = palette!
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  ctx.clearRect(0, 0, w, h)
  const now = performance.now()
  if (props.orbits.length) {
    const core = props.nodes.find(n => n.kind === 'core')
    const st = core ? states.get(core.id) : null
    if (st) props.orbits.forEach(o => paintOrbit(ctx, st.x.x, st.y.x, o.rx, o.ry, t, p, Math.min(1, st.o.x)))
  }
  if (props.network) {
    paintNetwork(ctx, props.network, { t, since: (now - stageStart) / 1000, live, palette: p, focusCat: props.networkFocus, alpha: props.networkAlpha, labels: true })
  }
  const colorOf = (cat?: number, tone?: string) => (tone === 'danger' ? p.danger : tone === 'lime' ? p.lime : cat ? p.cat[cat] : p.accent)
  // 台阶光柱与地平线
  const slabs = allNodes.value.filter(n => n.kind === 'slab')
  if (slabs.length) {
    const floor = h - 34
    paintHorizon(ctx, floor, 24, w - 24, p)
    slabs.forEach((node) => {
      const st = states.get(node.id)
      if (!st) return
      const o = Math.max(0, Math.min(1, st.o.x))
      const hot = node.id === hoverId.value || node.id === props.activeId
      paintPillar(ctx, st.x.x, st.y.x + 40, floor, (node.size ?? 140) - 8, colorOf(node.cat, node.tone), o * (isDim(node.id) ? 0.35 : node.tone === 'off' ? 0.5 : 1), hot)
    })
  }
  // 连线与脉冲
  props.links.forEach((l, i) => {
    const g = geometry(l, t, live)
    const A = states.get(l.a), B = states.get(l.b)
    if (!g || !A || !B) return
    const vis = Math.max(0, Math.min(A.o.x, B.o.x, 1))
    if (vis < 0.01) return
    const on = linkLit(l)
    const active = Boolean(props.activeId && (l.a === props.activeId || l.b === props.activeId))
    const c = colorOf(l.cat, l.tone)
    const a = vis * (on ? (lit.value || active ? 0.85 : 0.42) : 0.1)
    paintThread(ctx, g, c, a, on && (lit.value || active) ? 1.6 : 1.1, l.dashed)
    if (!live || vis < 0.5 || !l.flow || l.flow === 'none') return
    const lanes = l.flow === 'both' ? 2 : 1
    for (let k = 0; k < lanes; k += 1) {
      let u = (t * (k ? 0.24 : 0.3) + i * 0.137 + k * 0.5) % 1
      if (k === 1) u = 1 - u
      paintPulse(ctx, bezierAt(g.x1, g.c1x, g.c2x, g.x2, u), bezierAt(g.y1, g.c1y, g.c2y, g.y2, u), k ? p.lime : c, Math.sin(u * Math.PI) * vis * (on ? 1 : 0.2), 3, p.paper)
    }
  })
  // 珍珠球 / 刻度表盘 / 圆点
  allNodes.value.forEach((node) => {
    const st = states.get(node.id)
    if (!st) return
    const o = Math.max(0, Math.min(1, st.o.x))
    if (o < 0.01) return
    const dim = isDim(node.id)
    const hot = node.id === hoverId.value || node.id === props.activeId
    const x = st.x.x, y = st.y.x
    if (node.kind === 'core') {
      paintOrb(ctx, x, y, ((node.size ?? 96) / 2) * st.s.x, t, p, { alpha: o * (dim ? 0.45 : 1), live })
    } else if (node.kind === 'hub') {
      paintDial(ctx, x, y, (node.size ?? 104) * st.s.x, t, colorOf(node.cat, node.tone), p, { alpha: o * (dim ? 0.35 : node.tone === 'off' ? 0.55 : 1), ratio: node.ratio, live, active: hot })
    } else if (node.kind === 'dot') {
      paintDot(ctx, x, y, hot ? 4.6 : 3.6, colorOf(node.cat, node.tone), o * (dim ? 0.25 : node.tone === 'off' ? 0.45 : 1), p, hot)
    }
  })
  // 开场涟漪
  if (burstAt >= 0) {
    const k = (now - burstAt) / 1800
    if (k >= 1 || !live) burstAt = -1
    else {
      const core = props.nodes.find(n => n.kind === 'core')
      const st = core ? states.get(core.id) : null
      paintRipple(ctx, st?.x.x ?? w / 2, st?.y.x ?? h / 2, k, Math.min(w, h) * 0.42, p)
    }
  }
}
function replay() {
  states.clear()
  ghosts.value = []
  const now = performance.now()
  props.nodes.forEach(n => ensureState(n, now))
  stageStart = now
  burstAt = now
  paused.value = false
}
watch(() => props.burstKey, (key) => { if (key) burstAt = performance.now() })

/* ── 平移缩放（可选）：共享视口逻辑，渲染走 layerStyle 响应式 transform ── */
const { view, reset: resetView, bind: bindViewport, unbind: unbindViewport, onPointerDown, onPointerMove, onPointerUp } = useViewportPan({
  el: () => root.value,
  exclude: '.rg-node, .rg-corner',
  wheelPans: false,
  enabled: () => props.pannable,
  minK: 0.45,
  maxK: 2.2,
})
const layerStyle = computed(() => (props.pannable ? { transform: `translate(${view.x}px, ${view.y}px) scale(${view.k})` } : undefined))

/* ── 键盘：方向键找该方向最近的节点 ── */
function onKey(event: KeyboardEvent, node: GraphNode) {
  if (event.key === 'Escape') { event.preventDefault(); emit('back'); return }
  const dir = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[event.key]
  if (!dir) return
  event.preventDefault()
  let best: GraphNode | null = null
  let bestScore = Infinity
  for (const other of props.nodes) {
    if (other.id === node.id || other.interactive === false) continue
    const dx = other.x - node.x, dy = other.y - node.y
    const along = dx * dir[0] + dy * dir[1]
    if (along <= 4) continue
    const across = Math.abs(dx * dir[1]) + Math.abs(dy * dir[0])
    const score = along + across * 2.2
    if (score < bestScore) { bestScore = score; best = other }
  }
  if (best) nodeEls.get(best.id)?.focus()
}
function onRootKey(event: KeyboardEvent) {
  if (event.key === 'Escape' && event.target === root.value) emit('back')
}

/* ── 尺寸与生命周期 ── */
let resizeObserver: ResizeObserver | null = null
onMounted(() => {
  stageStart = performance.now()
  resizeObserver = new ResizeObserver(([entry]) => {
    const { width, height } = entry.contentRect
    if (Math.abs(width - size.value.w) > 0.5 || Math.abs(height - size.value.h) > 0.5) size.value = { w: width, h: height }
  })
  if (root.value) {
    resizeObserver.observe(root.value)
    bindViewport()
  }
  loop.start(root.value)
  if (props.burstKey) burstAt = performance.now()
  // 主题切换后重新取发光色
  themeObserver = new MutationObserver(() => { palette = null })
  themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] })
})
let themeObserver: MutationObserver | null = null
onBeforeUnmount(() => {
  loop.stop()
  resizeObserver?.disconnect()
  themeObserver?.disconnect()
  unbindViewport()
})
// 节点变化后补一帧，保证减少动态效果时立即落到终态
watch(() => [props.nodes, props.links, paused.value], () => nextTick(() => loop.flush()))

function focusNode(id: string) { nodeEls.get(id)?.focus() }
defineExpose({ resetView, focusNode })

const linkClass = (l: GraphLink) => [
  'rg-link',
  l.cat ? `cat-${l.cat}` : '',
  l.tone ? `is-${l.tone}` : '',
  { 'is-dashed': l.dashed, 'is-lit': lit.value && linkLit(l), 'is-dim': !linkLit(l), 'is-active': props.activeId && (l.a === props.activeId || l.b === props.activeId) },
]
const nodeClass = (n: GraphNode) => [
  'rg-node',
  `is-${n.kind ?? 'item'}`,
  `align-${n.align ?? 'center'}`,
  n.cat ? `cat-${n.cat}` : '',
  n.tone ? `tone-${n.tone}` : '',
  { 'is-active': n.id === props.activeId, 'is-dim': isDim(n.id), 'is-static': n.interactive === false },
]
</script>

<template>
  <div
    ref="root"
    :class="['rg', { 'is-pannable': pannable, 'is-stage': isStage, 'is-paused': paused }]"
    role="group"
    :aria-label="label"
    tabindex="-1"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @pointercancel="onPointerUp"
    @keydown="onRootKey"
  >
    <canvas v-if="isStage" ref="stageCanvas" class="rg-stage-canvas" aria-hidden="true" />
    <div ref="layer" class="rg-layer" :style="layerStyle">
      <svg v-if="!isStage" class="rg-links" aria-hidden="true">
        <g v-for="(l, i) in links" :key="`${l.a}>${l.b}`" :class="linkClass(l)">
          <path :ref="el => { pathEls[i] = el as SVGPathElement | null }" class="rg-path" />
          <circle :ref="el => { (pulseEls[i] ||= [])[0] = el as SVGCircleElement | null }" class="rg-pulse" r="3" />
          <circle v-if="l.flow === 'both'" :ref="el => { (pulseEls[i] ||= [])[1] = el as SVGCircleElement | null }" class="rg-pulse is-back" r="2.6" />
        </g>
      </svg>
      <component
        :is="n.interactive === false ? 'div' : 'button'"
        v-for="n in allNodes"
        :key="n.id"
        :ref="(el: unknown) => bindNode(n.id, el)"
        :type="n.interactive === false ? undefined : 'button'"
        :class="nodeClass(n)"
        :style="n.size ? { '--size': `${n.size}px` } : undefined"
        :aria-pressed="n.interactive === false ? undefined : n.id === activeId"
        :aria-label="n.interactive === false ? undefined : [n.label, n.sub, n.count !== undefined ? String(n.count) : ''].filter(Boolean).join('，')"
        :title="n.kind === 'dot' ? (n.title ?? n.label) : n.title"
        :tabindex="n.interactive === false || 'ghost' in n ? -1 : 0"
        @click="n.interactive !== false && !('ghost' in n) && emit('select', nodeById.get(n.id)!)"
        @pointerenter="hoverId = n.interactive === false ? null : n.id"
        @pointerleave="hoverId = null"
        @focus="hoverId = n.id"
        @blur="hoverId = null"
        @keydown="onKey($event, n)"
      >
        <span class="rg-body">
          <span class="rg-dot" aria-hidden="true">
            <Icon v-if="n.icon && n.kind !== 'dot'" :name="n.icon" :size="n.kind === 'core' ? 22 : 13" />
          </span>
          <span v-if="n.rank" class="rg-rank">{{ n.rank }}</span>
          <span v-if="(n.kind === 'hub' || n.kind === 'slab') && n.count !== undefined" class="rg-count tnum">{{ n.count }}</span>
          <span class="rg-text">
            <span class="rg-label">{{ n.label }}</span>
            <span v-if="n.sub" class="rg-sub">{{ n.sub }}</span>
          </span>
          <span v-if="n.kind !== 'hub' && n.kind !== 'slab' && n.count !== undefined" class="rg-badge tnum">{{ n.count }}</span>
        </span>
      </component>
    </div>
    <div class="rg-corner is-tl"><slot name="tl" /></div>
    <div class="rg-corner is-tr">
      <slot name="tr" />
      <div v-if="isStage" class="rg-tools" role="toolbar" aria-label="动效控制">
        <button type="button" title="重播开场" @click="replay"><Icon name="refresh" :size="13" />重播</button>
        <button type="button" :aria-pressed="paused" :title="paused ? '继续动效' : '暂停动效'" @click="paused = !paused"><Icon :name="paused ? 'play' : 'pause'" :size="13" />{{ paused ? '继续' : '暂停' }}</button>
      </div>
    </div>
    <div class="rg-corner is-bl"><slot name="bl" /></div>
    <div class="rg-corner is-br"><slot name="br" /></div>
    <slot />
  </div>
</template>
