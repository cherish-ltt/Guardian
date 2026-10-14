/* ============================================================
   图谱舞台的 Canvas 绘制件（珍珠玻璃语汇）：珍珠球、玻璃刻度表盘、实色细线与脉冲、
   开场涟漪、真实数据背景网络、刻度轨道、台阶光柱。
   规则与浅色图谱一致：实色细线 + 同色淡光环 + 软阴影，不用叠加发光；颜色取 --graph-* 三元组，
   浅 / 深主题各一套。纯绘制函数，不持有状态；帧循环、弹簧与命中区由 RelationGraph 负责。
   ============================================================ */

export type RGB = string // "r, g, b"，直接拼进 rgba()

export interface StagePalette {
  cat: RGB[] // 下标 1–5 对应 --graph-1~5，0 为中性墨色
  ink: RGB
  accent: RGB
  lime: RGB
  danger: RGB
  paper: RGB
}

export function readStagePalette(el: Element): StagePalette {
  const css = getComputedStyle(el)
  const get = (name: string, fallback: RGB) => css.getPropertyValue(name).trim() || fallback
  const ink = get('--graph-ink', '46, 58, 62')
  return {
    cat: [ink, get('--graph-1', '186, 140, 52'), get('--graph-2', '58, 122, 206'), get('--graph-3', '122, 96, 204'), get('--graph-4', '200, 90, 118'), get('--graph-5', '58, 150, 92')],
    ink,
    accent: get('--graph-accent', '35, 109, 233'),
    lime: get('--graph-lime', '110, 190, 36'),
    danger: get('--graph-danger', '224, 68, 59'),
    paper: get('--graph-paper', '255, 255, 255'),
  }
}

const TAU = Math.PI * 2
const rgba = (c: RGB, a: number) => `rgba(${c}, ${Math.max(0, Math.min(1, a))})`

/** 稳定伪随机：同一 seed 每帧一致 */
export const seedOf = (id: string) => {
  let h = 0
  for (let i = 0; i < id.length; i += 1) h = (h * 31 + id.charCodeAt(i)) | 0
  return (Math.abs(h) % 1000) / 1000
}

/** 刻度环：n 根径向短刻度，前 lit 比例着色 */
function tickRing(ctx: CanvasRenderingContext2D, x: number, y: number, r: number, len: number, n: number, base: string, litColor: string, lit: number, width = 1.2) {
  ctx.lineCap = 'round'
  ctx.lineWidth = width
  const litCount = Math.round(n * Math.max(0, Math.min(1, lit)))
  for (let i = 0; i < n; i += 1) {
    const a = -Math.PI / 2 + (i / n) * TAU
    ctx.strokeStyle = i < litCount ? litColor : base
    ctx.beginPath()
    ctx.moveTo(x + Math.cos(a) * r, y + Math.sin(a) * r)
    ctx.lineTo(x + Math.cos(a) * (r + len), y + Math.sin(a) * (r + len))
    ctx.stroke()
  }
}

/* ── 珍珠球：软阴影 + 珠光渐变 + 青柠 / 蓝的淡虹彩 + 缓慢扫描的刻度环 ── */
export function paintOrb(ctx: CanvasRenderingContext2D, x: number, y: number, r: number, t: number, p: StagePalette, opts: { alpha?: number; live?: boolean } = {}) {
  const alpha = opts.alpha ?? 1
  if (alpha <= 0.01 || r <= 1) return
  const live = opts.live ?? true
  ctx.save()
  ctx.globalAlpha = alpha
  // 外圈刻度：常态浅墨，一段蓝色扫描弧缓慢转动（导 Conduct）
  const ringR = r * 1.3
  const n = 72
  const head = live ? (t * 0.35) % TAU : 0.9
  ctx.lineCap = 'round'
  ctx.lineWidth = 1.2
  for (let i = 0; i < n; i += 1) {
    const a = (i / n) * TAU
    const behind = ((head - a) % TAU + TAU) % TAU
    const k = behind < 1.1 ? 1 - behind / 1.1 : 0
    const major = i % 6 === 0
    ctx.strokeStyle = k > 0 ? rgba(p.accent, 0.2 + k * 0.65) : rgba(p.ink, major ? 0.22 : 0.12)
    const len = major ? 6 : 3.5
    ctx.beginPath()
    ctx.moveTo(x + Math.cos(a) * ringR, y + Math.sin(a) * ringR)
    ctx.lineTo(x + Math.cos(a) * (ringR + len), y + Math.sin(a) * (ringR + len))
    ctx.stroke()
  }
  // 绕环一枚青柠卫星
  const sa = live ? -t * 0.22 : -0.6
  const sx = x + Math.cos(sa) * (ringR + 15), sy = y + Math.sin(sa) * (ringR + 15)
  ctx.fillStyle = rgba(p.lime, 0.2)
  ctx.beginPath(); ctx.arc(sx, sy, 7, 0, TAU); ctx.fill()
  ctx.fillStyle = rgba(p.lime, 1)
  ctx.beginPath(); ctx.arc(sx, sy, 3, 0, TAU); ctx.fill()
  // 球体：软阴影 + 珠光
  ctx.shadowColor = rgba(p.ink, 0.2)
  ctx.shadowBlur = r * 0.6
  ctx.shadowOffsetY = r * 0.2
  const body = ctx.createRadialGradient(x - r * 0.38, y - r * 0.42, r * 0.05, x, y, r)
  body.addColorStop(0, 'rgba(255, 255, 255, 1)')
  body.addColorStop(0.55, 'rgba(246, 249, 248, 1)')
  body.addColorStop(1, 'rgba(214, 223, 221, 1)')
  ctx.fillStyle = body
  ctx.beginPath(); ctx.arc(x, y, r, 0, TAU); ctx.fill()
  ctx.shadowColor = 'transparent'
  // 淡虹彩：右下青柠、左下蓝，缓慢游移
  const drift = live ? Math.sin(t * 0.5) * 0.12 : 0
  const sheen = (cx: number, cy: number, c: RGB, a: number) => {
    const g = ctx.createRadialGradient(cx, cy, 0, cx, cy, r * 0.9)
    g.addColorStop(0, rgba(c, a)); g.addColorStop(1, rgba(c, 0))
    ctx.fillStyle = g
    ctx.beginPath(); ctx.arc(x, y, r, 0, TAU); ctx.fill()
  }
  sheen(x + r * (0.45 + drift), y + r * 0.5, p.lime, 0.22)
  sheen(x - r * (0.5 - drift), y + r * 0.45, p.accent, 0.14)
  // 边缘与高光
  ctx.lineWidth = 1
  ctx.strokeStyle = rgba(p.ink, 0.1)
  ctx.beginPath(); ctx.arc(x, y, r, 0, TAU); ctx.stroke()
  ctx.strokeStyle = 'rgba(255, 255, 255, .95)'
  ctx.lineWidth = 1.6
  ctx.beginPath(); ctx.arc(x, y, r * 0.84, Math.PI * 1.08, Math.PI * 1.42); ctx.stroke()
  ctx.restore()
}

/* ── 玻璃刻度表盘：玻璃圆盘 + 内圈刻度（ratio 比例着分类色），激活时外圈淡光环 ── */
export function paintDial(ctx: CanvasRenderingContext2D, x: number, y: number, s: number, t: number, c: RGB, p: StagePalette, opts: { alpha?: number; ratio?: number; active?: boolean; live?: boolean } = {}) {
  const alpha = opts.alpha ?? 1
  if (alpha <= 0.01 || s <= 4) return
  const R = s / 2
  const ratio = Math.max(0, Math.min(1, opts.ratio ?? 0))
  ctx.save()
  ctx.globalAlpha = alpha
  if (opts.active) {
    const halo = ctx.createRadialGradient(x, y, R, x, y, R + 20)
    halo.addColorStop(0, rgba(c, 0.18)); halo.addColorStop(1, rgba(c, 0))
    ctx.fillStyle = halo
    ctx.beginPath(); ctx.arc(x, y, R + 20, 0, TAU); ctx.fill()
  }
  ctx.shadowColor = rgba(p.ink, 0.13)
  ctx.shadowBlur = 24
  ctx.shadowOffsetY = 9
  const body = ctx.createLinearGradient(x, y - R, x, y + R)
  body.addColorStop(0, rgba(p.paper, 1))
  body.addColorStop(1, rgba(p.paper, 0.92))
  ctx.fillStyle = body
  ctx.beginPath(); ctx.arc(x, y, R, 0, TAU); ctx.fill()
  ctx.shadowColor = 'transparent'
  ctx.lineWidth = 1
  ctx.strokeStyle = opts.active ? rgba(c, 0.55) : rgba(p.ink, 0.09)
  ctx.beginPath(); ctx.arc(x, y, R - 0.5, 0, TAU); ctx.stroke()
  // 顶部白色内高光
  ctx.strokeStyle = 'rgba(255, 255, 255, .9)'
  ctx.beginPath(); ctx.arc(x, y, R - 2, Math.PI * 1.15, Math.PI * 1.85); ctx.stroke()
  // 内圈刻度：比例弧着色；激活时沿着色段走一枚亮刻度
  const n = s > 90 ? 60 : 40
  tickRing(ctx, x, y, R * 0.76, Math.max(4, R * 0.11), n, rgba(p.ink, 0.12), rgba(c, 0.92), ratio, s > 90 ? 1.5 : 1.2)
  if (opts.active && opts.live !== false && ratio > 0) {
    const a = -Math.PI / 2 + ((t * 0.5) % 1) * TAU * ratio
    ctx.fillStyle = rgba(c, 1)
    ctx.beginPath(); ctx.arc(x + Math.cos(a) * R * 0.94, y + Math.sin(a) * R * 0.94, 2.2, 0, TAU); ctx.fill()
  }
  ctx.restore()
}

/* ── 连线：同色实线，激活时加极淡宽晕 ── */
export interface Curve { x1: number; y1: number; c1x: number; c1y: number; c2x: number; c2y: number; x2: number; y2: number }
export function paintThread(ctx: CanvasRenderingContext2D, g: Curve, c: RGB, alpha: number, width = 1, dashed = false) {
  if (alpha <= 0.01) return
  ctx.save()
  ctx.lineCap = 'round'
  if (dashed) ctx.setLineDash([3, 5])
  const path = new Path2D()
  path.moveTo(g.x1, g.y1)
  path.bezierCurveTo(g.c1x, g.c1y, g.c2x, g.c2y, g.x2, g.y2)
  if (width > 1.2) {
    ctx.strokeStyle = rgba(c, alpha * 0.1)
    ctx.lineWidth = width * 4
    ctx.stroke(path)
  }
  ctx.strokeStyle = rgba(c, alpha)
  ctx.lineWidth = width
  ctx.stroke(path)
  ctx.restore()
}

/* ── 脉冲：实心点 + 纸色芯 + 同色 18% 光环 ── */
export function paintPulse(ctx: CanvasRenderingContext2D, x: number, y: number, c: RGB, alpha: number, r = 3, paper: RGB = '255, 255, 255') {
  if (alpha <= 0.02) return
  ctx.save()
  ctx.fillStyle = rgba(c, alpha * 0.18)
  ctx.beginPath(); ctx.arc(x, y, r + 5, 0, TAU); ctx.fill()
  ctx.fillStyle = rgba(c, alpha)
  ctx.beginPath(); ctx.arc(x, y, r, 0, TAU); ctx.fill()
  ctx.fillStyle = rgba(paper, alpha * 0.9)
  ctx.beginPath(); ctx.arc(x, y, r * 0.4, 0, TAU); ctx.fill()
  ctx.restore()
}

/* ── 节点圆点：分类色实心 + 纸色描边，悬停加同色光环 ── */
export function paintDot(ctx: CanvasRenderingContext2D, x: number, y: number, r: number, c: RGB, alpha: number, p: StagePalette, ring = false) {
  if (alpha <= 0.01) return
  ctx.save()
  if (ring) {
    ctx.fillStyle = rgba(c, alpha * 0.16)
    ctx.beginPath(); ctx.arc(x, y, r + 6, 0, TAU); ctx.fill()
  }
  ctx.fillStyle = rgba(c, alpha)
  ctx.beginPath(); ctx.arc(x, y, r, 0, TAU); ctx.fill()
  ctx.strokeStyle = rgba(p.paper, alpha * 0.95)
  ctx.lineWidth = 1.2
  ctx.stroke()
  ctx.restore()
}

/* ── 开场涟漪：三道细环从中心荡开后消失（k 为 0→1 进度） ── */
export function paintRipple(ctx: CanvasRenderingContext2D, x: number, y: number, k: number, R: number, p: StagePalette) {
  if (k <= 0 || k >= 1) return
  ctx.save()
  for (let i = 0; i < 3; i += 1) {
    const kk = Math.max(0, Math.min(1, (k - i * 0.14) / 0.72))
    if (kk <= 0 || kk >= 1) continue
    const e = 1 - Math.pow(1 - kk, 3)
    ctx.strokeStyle = rgba(i === 1 ? p.lime : p.accent, (1 - kk) * 0.45)
    ctx.lineWidth = 1.4
    ctx.beginPath(); ctx.arc(x, y, 30 + e * R, 0, TAU); ctx.stroke()
  }
  ctx.restore()
}

/* ── 背景网络：真实数据点位，按层级从中心逐层蔓延；聚焦某类时其余降亮 ── */
export interface NetNode { id: string; x: number; y: number; cat: number; r?: number; depth?: number; label?: string; tone?: 'danger' | 'off'; /** 放射方向（弧度）：有值时标签沿该方向向外排，左半边翻转保持可读 */ angle?: number }
export interface NetLink { a: string; b: string; cat?: number; both?: boolean; /** 跨类关系：只在聚焦某类时出现 */ cross?: boolean }
export interface NetworkData { nodes: NetNode[]; links: NetLink[] }

export function paintNetwork(ctx: CanvasRenderingContext2D, net: NetworkData, opts: {
  t: number
  since: number
  live: boolean
  palette: StagePalette
  /** 只高亮该类（cat）；null 为全部正常 */
  focusCat?: number | null
  /** 整体透明度（下钻时退远） */
  alpha?: number
  labels?: boolean
}) {
  const { t, since, live, palette: p } = opts
  const alpha = opts.alpha ?? 1
  if (alpha <= 0.01 || !net.nodes.length) return
  const pos = new Map<string, { x: number; y: number; g: number; n: NetNode }>()
  net.nodes.forEach((n) => {
    const seed = seedOf(n.id)
    const g = live ? Math.min(1, Math.max(0, (since - 0.4 - (n.depth ?? 1) * 0.3 - seed * 0.3) / 0.8)) : 1
    const e = 1 - Math.pow(1 - g, 3)
    const sway = live ? Math.sin(t * 0.5 + seed * 6.28) * 2 : 0
    pos.set(n.id, { x: n.x + sway, y: n.y + (live ? Math.cos(t * 0.4 + seed * 9) * 1.6 : 0), g: e, n })
  })
  const focused = Boolean(opts.focusCat)
  const on = (n: NetNode) => !focused || n.cat === opts.focusCat
  ctx.save()
  ctx.lineWidth = 0.8
  net.links.forEach((l, i) => {
    const A = pos.get(l.a), B = pos.get(l.b)
    if (!A || !B) return
    const g = Math.min(A.g, B.g)
    if (g <= 0.02) return
    if (l.cross && (!focused || !(on(A.n) || on(B.n)))) return
    const lit = l.cross ? true : on(A.n) && on(B.n)
    const c = p.cat[l.cat ?? A.n.cat] ?? p.ink
    const a = (l.cross ? 0.3 : l.both ? 0.24 : 0.15) * g * alpha * (lit ? (focused ? 2.2 : 1) : 0.3)
    ctx.strokeStyle = rgba(c, a)
    ctx.beginPath()
    // 线随生长从起点伸出
    const bx = A.x + (B.x - A.x) * g, by = A.y + (B.y - A.y) * g
    if (l.both) {
      const mx = (A.x + bx) / 2 + (by - A.y) * 0.12, my = (A.y + by) / 2 - (bx - A.x) * 0.12
      ctx.moveTo(A.x, A.y); ctx.quadraticCurveTo(mx, my, bx, by)
    } else {
      ctx.moveTo(A.x, A.y); ctx.lineTo(bx, by)
    }
    ctx.stroke()
    if (live && lit && focused && i < 60 && g > 0.95) {
      const u = (t * 0.18 + seedOf(l.a + l.b)) % 1
      paintPulse(ctx, A.x + (B.x - A.x) * u, A.y + (B.y - A.y) * u, c, Math.sin(u * Math.PI) * alpha * 0.9, 2.2, p.paper)
    }
  })
  ctx.restore()
  // 聚焦类条目过多时不画名称（进入模块层看带标签的胶囊），避免文字互相压叠
  const labelOk = opts.labels && net.nodes.filter(n => on(n) && n.label).length <= 18
  pos.forEach(({ x, y, g, n }) => {
    if (g <= 0) return
    const lit = on(n)
    const c = n.tone === 'danger' ? p.danger : p.cat[n.cat] ?? p.ink
    const r = (n.r ?? 2.4) * (focused && lit ? 1.25 : 1)
    paintDot(ctx, x, y, r, c, g * alpha * (lit ? 0.9 : 0.3) * (n.tone === 'off' ? 0.5 : 1), p)
    if (labelOk && n.label && g > 0.9 && lit && focused && alpha > 0.6) {
      ctx.save()
      ctx.fillStyle = rgba(p.ink, 0.78 * alpha)
      ctx.font = '11px -apple-system, "PingFang SC", "Microsoft YaHei", sans-serif'
      ctx.textBaseline = 'middle'
      const text = n.label.length > 9 ? `${n.label.slice(0, 8)}…` : n.label
      if (n.angle === undefined) {
        ctx.fillText(text, x + r + 6, y)
      } else {
        ctx.translate(x, y)
        const left = Math.cos(n.angle) < 0
        ctx.rotate(left ? n.angle + Math.PI : n.angle)
        ctx.textAlign = left ? 'right' : 'left'
        ctx.fillText(text, left ? -(r + 6) : r + 6, 0)
      }
      ctx.restore()
    }
  })
}

/* ── 轨道：淡墨虚线椭圆 + 缓慢转动的刻度（星图仪感） ── */
export function paintOrbit(ctx: CanvasRenderingContext2D, cx: number, cy: number, rx: number, ry: number, t: number, p: StagePalette, alpha = 1) {
  if (alpha <= 0.01 || rx <= 0 || ry <= 0) return
  ctx.save()
  ctx.strokeStyle = rgba(p.ink, 0.09 * alpha)
  ctx.lineWidth = 1
  ctx.setLineDash([2, 5])
  ctx.beginPath(); ctx.ellipse(cx, cy, rx, ry, 0, 0, TAU); ctx.stroke()
  ctx.setLineDash([])
  const spin = t * 0.015
  for (let i = 0; i < 96; i += 1) {
    const a = spin + (i / 96) * TAU
    const major = i % 8 === 0
    const ex = Math.cos(a), ey = Math.sin(a)
    const x = cx + ex * rx, y = cy + ey * ry
    const nx = ex / rx, ny = ey / ry
    const nl = Math.hypot(nx, ny) || 1
    const len = major ? 6 : 2.5
    ctx.strokeStyle = rgba(p.ink, (major ? 0.2 : 0.1) * alpha)
    ctx.beginPath(); ctx.moveTo(x, y); ctx.lineTo(x + (nx / nl) * len, y + (ny / nl) * len); ctx.stroke()
  }
  ctx.restore()
}

/* ── 台阶光柱：从台面垂到地平线的分类色淡柱，选中时更实 ── */
export function paintPillar(ctx: CanvasRenderingContext2D, x: number, top: number, floor: number, width: number, c: RGB, alpha: number, active = false) {
  if (alpha <= 0.01 || floor <= top) return
  ctx.save()
  const g = ctx.createLinearGradient(0, top, 0, floor)
  g.addColorStop(0, rgba(c, (active ? 0.2 : 0.1) * alpha))
  g.addColorStop(1, rgba(c, 0.01))
  ctx.fillStyle = g
  ctx.fillRect(x - width / 2, top, width, floor - top)
  const e = ctx.createLinearGradient(0, top, 0, floor)
  e.addColorStop(0, rgba(c, (active ? 0.5 : 0.24) * alpha))
  e.addColorStop(1, rgba(c, 0))
  ctx.strokeStyle = e
  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.moveTo(x - width / 2 + 0.5, top); ctx.lineTo(x - width / 2 + 0.5, floor)
  ctx.moveTo(x + width / 2 - 0.5, top); ctx.lineTo(x + width / 2 - 0.5, floor)
  ctx.stroke()
  ctx.restore()
}

/* ── 地平线：一条带刻度的淡墨基线 ── */
export function paintHorizon(ctx: CanvasRenderingContext2D, y: number, x0: number, x1: number, p: StagePalette) {
  ctx.save()
  ctx.strokeStyle = rgba(p.ink, 0.16)
  ctx.lineWidth = 1
  ctx.beginPath(); ctx.moveTo(x0, y); ctx.lineTo(x1, y); ctx.stroke()
  for (let x = x0, i = 0; x <= x1; x += 12, i += 1) {
    ctx.strokeStyle = rgba(p.ink, i % 5 === 0 ? 0.2 : 0.1)
    ctx.beginPath(); ctx.moveTo(x, y); ctx.lineTo(x, y + (i % 5 === 0 ? 6 : 3)); ctx.stroke()
  }
  ctx.restore()
}
