<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { createFrameLoop, motionSkipped } from '@/composables/motion'

/* ============================================================
   神经网络画布：能力模块节点缓慢呼吸，脉冲沿连线往返（导 Conduct）。
   burstKey 变化时中心迸发短划，随后节点收束成对勾折线（登录成功）。
   减少动态效果 / 页面隐藏 / 离屏时停帧，reduced 只画静态终态。
   ============================================================ */
const props = withDefaults(defineProps<{ burstKey?: number | string }>(), { burstKey: 0 })

const wrap = ref<HTMLElement | null>(null)
const canvas = ref<HTMLCanvasElement | null>(null)

/* 节点锚点（归一化坐标）：0 是中心枢纽，其余为能力模块 */
interface NetNode { ax: number; ay: number; r: number; seed: number; x: number; y: number; tx: number; ty: number }
const ANCHORS: Array<Pick<NetNode, 'ax' | 'ay' | 'r' | 'seed'>> = [
  { ax: 0.5, ay: 0.52, r: 3.4, seed: 0.13 },
  { ax: 0.2, ay: 0.3, r: 2.2, seed: 0.31 },
  { ax: 0.34, ay: 0.74, r: 2.0, seed: 0.47 },
  { ax: 0.72, ay: 0.26, r: 2.1, seed: 0.59 },
  { ax: 0.82, ay: 0.58, r: 1.9, seed: 0.71 },
  { ax: 0.56, ay: 0.82, r: 1.8, seed: 0.83 },
  { ax: 0.14, ay: 0.58, r: 1.8, seed: 0.97 },
  { ax: 0.7, ay: 0.86, r: 1.6, seed: 0.07 },
]
/* 对勾折线顶点（归一化）：短边下压 → 拐点 → 长边上挑 */
const CHECK: Array<[number, number]> = [[0.34, 0.54], [0.46, 0.66], [0.7, 0.34]]

let nodes: NetNode[] = []
let links: Array<[number, number]> = []
let converging = false
let convergeAt = 0
let burstAt = -1
let cStroke = 'rgba(170,190,205,.16)'
let cNode = 'rgba(198,214,228,.8)'
let cAccent = 'rgba(120,165,255,.9)'
let cCheck = 'rgba(150,225,140,.9)'

function buildGraph() {
  nodes = ANCHORS.map(a => ({ ...a, x: a.ax, y: a.ay, tx: a.ax, ty: a.ay }))
  links = []
  for (let i = 1; i < nodes.length; i += 1) links.push([0, i])
  for (let i = 1; i < nodes.length; i += 1) {
    for (let j = i + 1; j < nodes.length; j += 1) {
      const d = Math.hypot(nodes[i].ax - nodes[j].ax, nodes[i].ay - nodes[j].ay)
      if (d < 0.3) links.push([i, j])
    }
  }
}

/* 对勾折线按节点数均分采样：枢纽落在拐点 */
function checkTargets(w: number, h: number): Array<[number, number]> {
  const seg = [[CHECK[0], CHECK[1]], [CHECK[1], CHECK[2]]] as const
  const len = seg.map(([a, b]) => Math.hypot(b[0] - a[0], b[1] - a[1]))
  const total = len[0] + len[1]
  const pts = (u: number): [number, number] => {
    const d = u * total
    const s = d <= len[0] ? 0 : 1
    const f = s === 0 ? d / len[0] : (d - len[0]) / len[1]
    return [(seg[s][0][0] + (seg[s][1][0] - seg[s][0][0]) * f) * w, (seg[s][0][1] + (seg[s][1][1] - seg[s][0][1]) * f) * h]
  }
  return nodes.map((_, i) => (i === 0 ? [CHECK[1][0] * w, CHECK[1][1] * h] : pts(i / (nodes.length - 1))))
}

function resolveColors() {
  const cs = getComputedStyle(document.documentElement)
  const v = (name: string, fb: string) => cs.getPropertyValue(name).trim() || fb
  cStroke = v('--tick', cStroke)
  cAccent = v('--blue', cAccent)
  cCheck = v('--lime', cCheck)
}

const loop = createFrameLoop((dt, t) => {
  const el = canvas.value
  const host = wrap.value
  if (!el || !host) return
  const w = host.clientWidth
  const h = host.clientHeight
  if (!w || !h) return
  const dpr = Math.min(window.devicePixelRatio || 1, 2)
  const bw = Math.round(w * dpr)
  if (el.width !== bw) { el.width = bw; el.height = Math.round(h * dpr) }
  const ctx = el.getContext('2d')
  if (!ctx) return
  const still = motionSkipped()
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  ctx.clearRect(0, 0, w, h)

  /* 目标位：呼吸态锚点 + 正弦轻漂；收束态对勾顶点 */
  const targets = converging ? checkTargets(w, h) : null
  nodes.forEach((n, i) => {
    if (converging && targets) {
      const k = Math.min(1, (performance.now() - convergeAt) / 900)
      const e = 1 - Math.pow(1 - k, 3)
      n.tx = n.ax * w + (targets[i][0] - n.ax * w) * e
      n.ty = n.ay * h + (targets[i][1] - n.ay * h) * e
      n.x = n.tx
      n.y = n.ty
    } else {
      const drift = still ? 0 : 1
      n.tx = n.ax * w + Math.sin(t * 0.4 + n.seed * 9) * 9 * drift
      n.ty = n.ay * h + Math.cos(t * 0.31 + n.seed * 13) * 7 * drift
      n.x += (n.tx - n.x) * Math.min(1, dt * 4)
      n.y += (n.ty - n.y) * Math.min(1, dt * 4)
    }
  })

  const dim = converging ? 0.35 : 1
  ctx.strokeStyle = cStroke
  ctx.lineWidth = 1
  ctx.globalAlpha = dim
  ctx.beginPath()
  for (const [a, b] of links) {
    ctx.moveTo(nodes[a].x, nodes[a].y)
    ctx.lineTo(nodes[b].x, nodes[b].y)
  }
  ctx.stroke()

  /* 脉冲：枢纽连线往返，普通连线只走枢纽发出的一段 */
  if (!still && !converging) {
    links.forEach(([a, b], li) => {
      // 枢纽连线全部有脉冲；普通连线每三条取一条
      if (a !== 0 && li % 3) return
      let u = (t * 0.16 + nodes[a].seed + li * 0.13) % 1
      u = u < 0.5 ? u * 2 : 2 - u * 2
      const A = nodes[a], B = nodes[b]
      const x = A.x + (B.x - A.x) * u
      const y = A.y + (B.y - A.y) * u
      ctx.globalAlpha = Math.sin(u * Math.PI) * 0.9
      ctx.fillStyle = cAccent
      ctx.beginPath(); ctx.arc(x, y, 2, 0, Math.PI * 2); ctx.fill()
      ctx.globalAlpha = Math.sin(u * Math.PI) * 0.2
      ctx.beginPath(); ctx.arc(x, y, 5.5, 0, Math.PI * 2); ctx.fill()
    })
  }

  /* 节点：白点 + 细描边；枢纽稍大带光环 */
  nodes.forEach((n, i) => {
    const pulse = still ? 0.6 : 0.55 + Math.sin(t * 1.1 + n.seed * 12) * 0.25
    ctx.globalAlpha = i === 0 ? 0.9 : pulse * 0.75
    ctx.fillStyle = converging ? cCheck : cNode
    ctx.beginPath(); ctx.arc(n.x, n.y, n.r, 0, Math.PI * 2); ctx.fill()
    if (i === 0) {
      ctx.globalAlpha = 0.18
      ctx.beginPath(); ctx.arc(n.x, n.y, n.r + 5 + (still ? 0 : Math.sin(t * 1.4) * 1.5), 0, Math.PI * 2); ctx.fill()
    }
  })
  ctx.globalAlpha = 1

  /* 对勾描边：收束后段描出 */
  if (converging) {
    const k = Math.min(1, (performance.now() - convergeAt) / 1400)
    const draw = Math.max(0, (k - 0.55) / 0.45)
    if (draw > 0) {
      const seg: Array<[[number, number], [number, number]]> = [[CHECK[0], CHECK[1]], [CHECK[1], CHECK[2]]]
      const lens = seg.map(([a, b]) => Math.hypot(b[0] - a[0], b[1] - a[1]))
      const total = lens[0] + lens[1]
      let remain = draw * total
      ctx.strokeStyle = cCheck
      ctx.lineWidth = 2.4
      ctx.lineCap = 'round'
      ctx.globalAlpha = 0.95
      ctx.beginPath()
      ctx.moveTo(CHECK[0][0] * w, CHECK[0][1] * h)
      for (let s = 0; s < 2 && remain > 0; s += 1) {
        const f = Math.min(1, remain / lens[s])
        ctx.lineTo((seg[s][0][0] + (seg[s][1][0] - seg[s][0][0]) * f) * w, (seg[s][0][1] + (seg[s][1][1] - seg[s][0][1]) * f) * h)
        remain -= lens[s]
      }
      ctx.stroke()
      ctx.globalAlpha = 1
    }
  }

  /* 迸发短划：从中心向外飞散，随后进入收束 */
  if (burstAt >= 0 && !still) {
    const k = (performance.now() - burstAt) / 1100
    if (k >= 1) { burstAt = -1 } else {
      const e = 1 - Math.pow(1 - k, 2.2)
      const cx = w * 0.5
      const cy = h * 0.52
      const R = Math.min(w, h) * 0.5
      ctx.strokeStyle = cAccent
      ctx.lineCap = 'round'
      for (let i = 0; i < 46; i += 1) {
        const a = (i / 46) * Math.PI * 2 + Math.sin(i * 12.9) * 0.28
        const v = 0.3 + ((i * 37) % 100) / 110
        const d = (18 + v * R * 0.7) * e
        ctx.globalAlpha = (1 - k) * 0.75
        ctx.lineWidth = 1 + (i % 3) * 0.5
        ctx.beginPath()
        ctx.moveTo(cx + Math.cos(a) * d * 0.72, cy + Math.sin(a) * d * 0.72)
        ctx.lineTo(cx + Math.cos(a) * d, cy + Math.sin(a) * d)
        ctx.stroke()
      }
      ctx.globalAlpha = 1
      if (!converging && k > 0.32) { converging = true; convergeAt = performance.now() }
    }
  }
})

watch(() => props.burstKey, (key) => {
  if (!key) return
  burstAt = performance.now()
  loop.flush()
})

let resizeObserver: ResizeObserver | null = null
onMounted(() => {
  buildGraph()
  resolveColors()
  resizeObserver = new ResizeObserver(() => loop.flush())
  if (wrap.value) resizeObserver.observe(wrap.value)
  loop.start(wrap.value)
})
onBeforeUnmount(() => {
  loop.stop()
  resizeObserver?.disconnect()
})
</script>

<template>
  <div ref="wrap" class="neural-net" aria-hidden="true">
    <canvas ref="canvas" />
  </div>
</template>

<style scoped>
.neural-net { position: absolute; inset: 0; z-index: -1; overflow: hidden; pointer-events: none; }
.neural-net canvas { display: block; width: 100%; height: 100%; }
</style>
