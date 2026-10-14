<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import RelationGraph, { type GraphLink, type GraphNode } from '@/components/charts/RelationGraph.vue'
import InsightPanel, { type InsightCard, type InsightChip, type InsightGroup, type InsightTile } from './InsightPanel.vue'
import AskDock from './AskDock.vue'
import Icon from '@/components/Icon.vue'
import IconButton from '@/components/ui/IconButton.vue'
import { firstTimeInSession } from '@/composables/motion'
import type { NetLink, NetNode, NetworkData } from '@/composables/neural'
import { loadPermissionGraph, permissionIndex } from '@/composables/permissionGraph'
import { toastError } from '@/composables/toast'
import { holdersOfMenu, isSuperRole, menuPath, menusOfUser, permissionFindings, roleActive, type GraphMenu, type PermissionIndex } from '@/utils/permissionGraph'

/* ============================================================
   权限溯源（图谱舞台，三层下钻）：
   - 全景：中心珍珠球「权限全景」，用户 / 角色 / 菜单三枚玻璃刻度表盘环绕（刻度为真实占比读数）；
     各类条目排成围绕表盘的刻度弧，悬停表盘时点亮该类并浮出放射状名称与跨类授权线
   - 模块：点表盘 → 珍珠球退到左侧、表盘竖排成列，该类全部条目以带标签胶囊触须逐根扇形长出
   - 实体：点条目 → 用户 → 角色 → 菜单 完整链路与往返脉冲，右侧分析栏滑入
   面包屑与 Esc 逐层返回。实体聚焦用 'user:1' / 'role:2' / 'menu:3'，与路由 query.trace 一致。
   ============================================================ */
type Kind = 'user' | 'role' | 'menu'
const focus = defineModel<string | null>('focus', { default: null })

const CAT = { user: 2, role: 3, menu: 5 } as const
const KIND_LABEL: Record<Kind, string> = { user: '用户', role: '角色', menu: '菜单' }
const KIND_ICON: Record<Kind, string> = { user: 'users', role: 'role', menu: 'menu' }
const size = ref({ w: 0, h: 0 })
const loading = ref(true)
const burst = ref(0)
const cluster = ref<Kind | null>(null)
const hoverCluster = ref<Kind | null>(null)

onMounted(async () => {
  try {
    await loadPermissionGraph(true)
    if (firstTimeInSession('trace')) burst.value = Date.now()
  } catch (error) {
    toastError(error, '权限关系加载失败')
  } finally {
    loading.value = false
  }
})

const index = computed(() => permissionIndex.value)
const parsed = computed(() => {
  const [kind, raw] = (focus.value ?? '').split(':')
  const id = Number(raw)
  if (!['user', 'role', 'menu'].includes(kind) || !Number.isFinite(id)) return null
  const idx = index.value
  if (!idx) return null
  const exists = kind === 'user' ? idx.user.has(id) : kind === 'role' ? idx.role.has(id) : idx.menu.has(id)
  return exists ? { kind: kind as Kind, id } : null
})
// 进入实体层时记住所属模块，返回时落回模块层
watch(parsed, (f) => { if (f) cluster.value = f.kind })
const layer = computed<'hub' | 'module' | 'entity'>(() => (parsed.value ? 'entity' : cluster.value ? 'module' : 'hub'))

/* ── 工具 ── */
const pages = (idx: PermissionIndex, ids: Iterable<number>) => [...ids].map(id => idx.menu.get(id)).filter((m): m is GraphMenu => Boolean(m && m.type === 'menu'))
const buttonsUnder = (idx: PermissionIndex, menuId: number, granted?: Set<number>) => (idx.childrenOf.get(menuId) ?? []).filter(m => m.type === 'btn' && (!granted || granted.has(m.id)))
const grantedOf = (idx: PermissionIndex, roleId: number) => {
  const role = idx.role.get(roleId)
  return new Set(role && isSuperRole(role) ? idx.graph.menus.map(m => m.id) : role?.menuIds ?? [])
}
const activeGrants = (idx: PermissionIndex, menuId: number) => (idx.rolesOfMenu.get(menuId) ?? []).filter(r => roleActive(idx.role.get(r)) && !isSuperRole(idx.role.get(r))).length

/* 纵列：上留面包屑、下留问答坞 */
const COL_TOP = 76
const COL_BOTTOM = 96
function column<T>(items: T[], x: number, H: number, make: (item: T, y: number, i: number, n: number) => GraphNode, moreId: string, align: GraphNode['align'], minGap = 32) {
  const span = Math.max(120, H - COL_TOP - COL_BOTTOM)
  const cap = Math.max(4, Math.floor(span / minGap) + 1)
  const shown = items.slice(0, cap)
  const n = shown.length + (items.length > cap ? 1 : 0)
  const gap = n > 1 ? Math.min(46, span / (n - 1)) : 0
  const y0 = COL_TOP + span / 2 - (gap * (n - 1)) / 2
  const nodes = shown.map((item, i) => make(item, y0 + i * gap, i, n))
  if (items.length > cap) nodes.push({ id: moreId, x, y: y0 + shown.length * gap, label: `还有 ${items.length - cap} 项`, kind: 'item', align, interactive: false, delay: 0.5 })
  return nodes
}

/* ── 模块条目：每类条目带读数，孤立 / 停用着色 ── */
interface Entry { id: string; label: string; sub: string; count?: string; tone?: GraphNode['tone']; icon?: string }
function entriesOf(idx: PermissionIndex, kind: Kind): Entry[] {
  if (kind === 'role') {
    return idx.graph.roles.map((r) => {
      const users = idx.usersOfRole.get(r.id)?.length ?? r.userCount ?? 0
      const pageCount = pages(idx, grantedOf(idx, r.id)).length
      const idle = !users
      return { id: `role:${r.id}`, label: r.name, sub: idle ? '闲置 · 没有持有人' : `${users} 人持有 · ${pageCount} 个页面`, tone: r.status !== 1 ? 'off' : idle ? 'danger' : undefined }
    })
  }
  if (kind === 'user') {
    return [...idx.graph.users]
      .sort((a, b) => (idx.rolesOfUser.get(b.id)?.length ?? 0) - (idx.rolesOfUser.get(a.id)?.length ?? 0))
      .map((u) => {
        const roles = idx.rolesOfUser.get(u.id)?.length ?? 0
        return { id: `user:${u.id}`, label: u.nickname, sub: roles ? `${u.deptName ?? '未分配部门'} · ${roles} 个角色` : '未分配角色', tone: u.status !== 1 ? 'off' : roles ? undefined : 'danger' }
      })
  }
  return idx.graph.menus.filter(m => m.type === 'menu').map((m) => {
    const grants = activeGrants(idx, m.id)
    return { id: `menu:${m.id}`, label: m.name, sub: grants ? `${grants} 个角色授予` : '孤立 · 仅超级管理员可见', tone: m.status !== 1 ? 'off' : grants ? undefined : 'danger' }
  })
}

/* ── 背景网络：三类真实点位排成围绕表盘的刻度弧，表盘到点位为放射细线；跨类授权线只在悬停聚焦时出现 ── */
function arcs(n: number, hx: number, hy: number, away: number, W: number, H: number) {
  const perRing = 16
  const rings = Math.max(1, Math.ceil(n / perRing))
  const out: Array<{ x: number; y: number; depth: number; angle: number }> = []
  for (let k = 0; k < rings; k += 1) {
    const m = Math.min(perRing, n - k * perRing)
    const spread = Math.min(Math.PI * 0.95, 0.5 + m * 0.16)
    // 相邻点位弧长至少 22px，点少时贴近表盘
    const r = Math.max(84 + k * 22, (22 * Math.max(m - 1, 1)) / spread)
    for (let i = 0; i < m; i += 1) {
      const a = away + (m > 1 ? (i / (m - 1) - 0.5) * spread : 0)
      out.push({ x: Math.min(W - 14, Math.max(14, hx + Math.cos(a) * r)), y: Math.min(H - 14, Math.max(14, hy + Math.sin(a) * r)), depth: 2 + k, angle: a })
    }
  }
  return out
}

interface HubPos { kind: Kind; x: number; y: number }
function hubLayout(W: number, H: number): { core: { x: number; y: number }; hubs: HubPos[] } {
  const narrow = W < 640
  const cx = W / 2, cy = H * (narrow ? 0.46 : 0.54)
  const rx = narrow ? W * 0.3 : Math.min(W * 0.32, 420), ry = Math.min(H * (narrow ? 0.24 : 0.29), 190)
  // 标签卡宽约 140px：窄屏时把表盘夹在可视区内
  const at = (deg: number) => ({ x: Math.min(W - 82, Math.max(82, cx + Math.cos((deg * Math.PI) / 180) * rx)), y: cy + Math.sin((deg * Math.PI) / 180) * ry })
  return {
    core: { x: cx, y: cy },
    hubs: [
      { kind: 'role', ...at(-90) },
      { kind: 'user', ...at(W < 640 ? 150 : 162) },
      { kind: 'menu', ...at(W < 640 ? 30 : 18) },
    ],
  }
}

const network = computed<NetworkData | null>(() => {
  const idx = index.value
  const { w: W, h: H } = size.value
  if (!idx || !W || !H) return null
  const { core, hubs } = hubLayout(W, H)
  const nodes: NetNode[] = []
  const links: NetLink[] = []
  const lists: Record<Kind, Array<{ id: string; label: string; tone?: NetNode['tone'] }>> = {
    user: idx.graph.users.map(u => ({ id: `user:${u.id}`, label: u.nickname, tone: u.status !== 1 ? 'off' : undefined })),
    role: idx.graph.roles.map(r => ({ id: `role:${r.id}`, label: r.name, tone: r.status !== 1 ? 'off' : undefined })),
    menu: idx.graph.menus.map(m => ({ id: `menu:${m.id}`, label: m.name, tone: m.status === 1 && m.type === 'menu' && !activeGrants(idx, m.id) ? 'danger' : m.status !== 1 ? 'off' : undefined })),
  }
  hubs.forEach((hub) => {
    const away = Math.atan2(hub.y - core.y, hub.x - core.x)
    const list = lists[hub.kind]
    const pts = arcs(list.length, hub.x, hub.y, away, W, H)
    const anchor = `anchor:${hub.kind}`
    nodes.push({ id: anchor, x: hub.x, y: hub.y, cat: CAT[hub.kind], depth: 1, r: 0.01 })
    list.forEach((item, i) => {
      nodes.push({ id: item.id, x: pts[i].x, y: pts[i].y, cat: CAT[hub.kind], depth: pts[i].depth, angle: pts[i].angle, label: item.label, tone: item.tone, r: hub.kind === 'menu' ? 2.2 : 2.8 })
      links.push({ a: anchor, b: item.id, cat: CAT[hub.kind] })
    })
  })
  const has = new Set(nodes.map(n => n.id))
  idx.graph.userRoles.forEach(([u, r]) => { if (has.has(`user:${u}`) && has.has(`role:${r}`)) links.push({ a: `user:${u}`, b: `role:${r}`, cat: CAT.user, cross: true }) })
  idx.graph.roles.forEach((r) => {
    if (isSuperRole(r)) return
    r.menuIds.forEach((m) => { if (has.has(`menu:${m}`)) links.push({ a: `role:${r.id}`, b: `menu:${m}`, cat: CAT.role, cross: true }) })
  })
  return { nodes, links }
})

/* ── 场景 ── */
const scene = computed<{ nodes: GraphNode[]; links: GraphLink[] }>(() => {
  const idx = index.value
  const { w, h } = size.value
  if (!idx || !w || !h) return { nodes: [], links: [] }
  const f = parsed.value
  const panel = f && w >= 900 ? 392 : 0
  const W = w - panel
  const H = h
  const nodes: GraphNode[] = []
  const links: GraphLink[] = []
  const stagger = (i: number, n: number, base = 0) => base + i * Math.min(0.09, 1.8 / Math.max(n, 1))
  const counts: Record<Kind, number | string> = { user: idx.graph.usersVisible ? idx.graph.users.length : '—', role: idx.graph.roles.length, menu: idx.graph.menus.length }
  const subs: Record<Kind, string> = { user: idx.graph.usersVisible ? '刻度：已分配角色占比' : '无用户查询权限', role: '刻度：有人持有占比', menu: '刻度：已授权页面占比' }
  // 表盘刻度是真实读数：已分配角色的用户、有持有人的角色、被普通角色授权的页面
  const pageMenus = idx.graph.menus.filter(m => m.type === 'menu')
  const ratios: Record<Kind, number> = {
    user: idx.graph.users.length ? idx.graph.users.filter(u => (idx.rolesOfUser.get(u.id)?.length ?? 0) > 0).length / idx.graph.users.length : 0,
    role: idx.graph.roles.length ? idx.graph.roles.filter(r => (idx.usersOfRole.get(r.id)?.length ?? r.userCount ?? 0) > 0).length / idx.graph.roles.length : 0,
    menu: pageMenus.length ? pageMenus.filter(m => activeGrants(idx, m.id) > 0).length / pageMenus.length : 0,
  }

  if (layer.value === 'hub') {
    const { core, hubs } = hubLayout(W, H)
    nodes.push({ id: 'core', x: core.x, y: core.y, size: W < 640 ? 72 : 104, label: '权限全景', sub: `${idx.graph.userRoles.length} 条授权`, kind: 'core', interactive: false, float: true })
    hubs.forEach((hub, hi) => {
      nodes.push({ id: `hub:${hub.kind}`, x: hub.x, y: hub.y, size: W < 640 ? 84 : 116, label: KIND_LABEL[hub.kind], sub: subs[hub.kind], kind: 'hub', icon: KIND_ICON[hub.kind], cat: CAT[hub.kind], count: counts[hub.kind], ratio: ratios[hub.kind], from: 'core', delay: 0.9 + hi * 0.17, float: true })
      links.push({ a: 'core', b: `hub:${hub.kind}`, flow: 'both', cat: CAT[hub.kind] })
    })
    return { nodes, links }
  }

  const cy = COL_TOP + (H - COL_TOP - COL_BOTTOM) / 2
  if (layer.value === 'module') {
    const k = cluster.value!
    const coreX = Math.max(84, W * 0.07)
    const colX = coreX + 150
    nodes.push({ id: 'core', x: coreX, y: cy, size: 64, label: '权限全景', kind: 'core', interactive: false })
    const order: Kind[] = (['user', 'role', 'menu'] as Kind[]).filter(x => x !== k)
    const slots: Array<{ kind: Kind; y: number }> = [{ kind: order[0], y: cy - 180 }, { kind: k, y: cy - 10 }, { kind: order[1], y: cy + 170 }]
    slots.forEach((slot) => {
      const sel = slot.kind === k
      nodes.push({ id: `hub:${slot.kind}`, x: colX, y: slot.y, size: sel ? 112 : 72, label: KIND_LABEL[slot.kind], sub: sel ? subs[slot.kind] : undefined, kind: 'hub', icon: KIND_ICON[slot.kind], cat: CAT[slot.kind], count: counts[slot.kind], ratio: ratios[slot.kind], from: 'core', float: true })
      links.push({ a: 'core', b: `hub:${slot.kind}`, flow: sel ? 'both' : 'none', cat: CAT[slot.kind], shape: 'h' })
    })
    const entries = entriesOf(idx, k)
    const fanX = Math.min(Math.max(colX + 300, W * 0.42), W - 320)
    const fan = column(entries, fanX, H, (e, y, i, n) => ({ id: e.id, x: fanX, y, label: e.label, sub: e.sub, kind: 'item', align: 'left', cat: CAT[k], tone: e.tone, from: `hub:${k}`, delay: 0.16 + stagger(i, n) }), `more:${k}`, 'left', 34)
    nodes.push(...fan)
    fan.forEach((n) => { if (!n.id.startsWith('more:')) links.push({ a: `hub:${k}`, b: n.id, flow: 'forward', shape: 'h', cat: n.tone === 'danger' ? undefined : CAT[k], tone: n.tone === 'danger' ? 'danger' : undefined }) })
    return { nodes, links }
  }

  const fe = f!
  if (fe.kind === 'role') {
    const role = idx.role.get(fe.id)!
    const userIds = idx.usersOfRole.get(role.id) ?? []
    const granted = grantedOf(idx, role.id)
    const center = `role:${role.id}`
    nodes.push({ id: center, x: W * 0.47, y: cy, label: role.name, sub: role.code, kind: 'item', icon: 'role', cat: CAT.role, count: `${userIds.length} 人`, tone: role.status === 1 ? undefined : 'off' })
    nodes.push(...column(userIds, W * 0.2, H, (id, y, i, n) => {
      const u = idx.user.get(id)
      return { id: `user:${id}`, x: W * 0.2, y, label: u?.nickname ?? `用户 ${id}`, sub: u?.deptName ?? '', kind: 'item', align: 'right', cat: CAT.user, tone: u?.status === 0 ? 'off' : undefined, from: center, delay: stagger(i, n, 0.1) }
    }, 'more:user', 'right'))
    nodes.push(...column(pages(idx, granted), W * 0.74, H, (m, y, i, n) => {
      const btns = buttonsUnder(idx, m.id, granted).length
      return { id: `menu:${m.id}`, x: W * 0.74, y, label: m.name, sub: m.permission ?? m.path ?? '', count: btns ? `${btns} 按钮` : undefined, kind: 'item', align: 'left', cat: CAT.menu, tone: m.status === 1 ? undefined : 'off', from: center, delay: stagger(i, n, 0.25) }
    }, 'more:menu', 'left'))
    nodes.forEach((n) => {
      if (n.id.startsWith('user:')) links.push({ a: n.id, b: center, flow: 'forward', shape: 'h', cat: CAT.user })
      if (n.id.startsWith('menu:')) links.push({ a: center, b: n.id, flow: 'forward', shape: 'h', cat: CAT.menu })
    })
    return { nodes, links }
  }

  if (fe.kind === 'user') {
    const user = idx.user.get(fe.id)!
    const center = `user:${user.id}`
    const roleIds = idx.rolesOfUser.get(user.id) ?? []
    const menuIds = menusOfUser(idx, user.id)
    nodes.push({ id: center, x: W * 0.14, y: cy, label: user.nickname, sub: user.deptName ?? user.username, kind: 'item', icon: 'user', cat: CAT.user, count: `${roleIds.length} 角色`, tone: user.status === 1 ? undefined : 'off' })
    nodes.push(...column(roleIds, W * 0.42, H, (id, y, i, n) => {
      const r = idx.role.get(id)
      return { id: `role:${id}`, x: W * 0.42, y, label: r?.name ?? `角色 ${id}`, sub: r?.code, kind: 'item', cat: CAT.role, tone: roleActive(r) ? undefined : 'off', from: center, delay: stagger(i, n, 0.1) }
    }, 'more:role', 'center'))
    const pageList = pages(idx, menuIds)
    nodes.push(...column(pageList, W * 0.7, H, (m, y, i, n) => ({ id: `menu:${m.id}`, x: W * 0.7, y, label: m.name, sub: m.permission ?? m.path ?? '', count: buttonsUnder(idx, m.id, menuIds).length || undefined, kind: 'item', align: 'left', cat: CAT.menu, from: center, delay: stagger(i, n, 0.35) }), 'more:menu', 'left'))
    const shown = new Set(nodes.map(n => n.id))
    roleIds.forEach((rid) => {
      if (!shown.has(`role:${rid}`)) return
      links.push({ a: center, b: `role:${rid}`, flow: 'forward', shape: 'h', cat: CAT.user })
      const r = idx.role.get(rid)
      if (!roleActive(r)) return
      pageList.forEach((m) => {
        if (shown.has(`menu:${m.id}`) && (isSuperRole(r) || r!.menuIds.includes(m.id))) links.push({ a: `role:${rid}`, b: `menu:${m.id}`, flow: 'forward', shape: 'h', cat: CAT.menu })
      })
    })
    return { nodes, links }
  }

  const menu = idx.menu.get(fe.id)!
  const center = `menu:${menu.id}`
  const { roles, users } = holdersOfMenu(idx, menu.id)
  const orphan = !roles.some(r => !isSuperRole(r))
  nodes.push({ id: center, x: W * 0.76, y: cy, label: menu.name, sub: menu.permission ?? menu.path ?? menu.type, kind: 'item', icon: menu.type === 'btn' ? 'key' : 'menu', cat: CAT.menu, count: `${roles.length} 角色`, tone: orphan ? 'danger' : menu.status === 1 ? undefined : 'off' })
  nodes.push(...column(roles, W * 0.48, H, (r, y, i, n) => ({ id: `role:${r.id}`, x: W * 0.48, y, label: r.name, sub: r.code, kind: 'item', cat: CAT.role, from: center, delay: stagger(i, n, 0.1) }), 'more:role', 'center'))
  nodes.push(...column(users, W * 0.2, H, (u, y, i, n) => ({ id: `user:${u.id}`, x: W * 0.2, y, label: u.nickname, sub: u.deptName ?? '', kind: 'item', align: 'right', cat: CAT.user, tone: u.status === 1 ? undefined : 'off', from: center, delay: stagger(i, n, 0.35) }), 'more:user', 'right'))
  const shown = new Set(nodes.map(n => n.id))
  roles.forEach((r) => {
    if (!shown.has(`role:${r.id}`)) return
    links.push({ a: `role:${r.id}`, b: center, flow: 'forward', shape: 'h', cat: CAT.menu })
    ;(idx.usersOfRole.get(r.id) ?? []).forEach((u) => { if (shown.has(`user:${u}`)) links.push({ a: `user:${u}`, b: `role:${r.id}`, flow: 'forward', shape: 'h', cat: CAT.user }) })
  })
  return { nodes, links }
})

/* 模块层：未选中的两枚表盘退暗 */
const highlight = computed(() => {
  if (layer.value !== 'module') return null
  return scene.value.nodes.filter(n => n.id !== `hub:${otherHubs.value[0]}` && n.id !== `hub:${otherHubs.value[1]}`).map(n => n.id)
})
const otherHubs = computed(() => (['user', 'role', 'menu'] as Kind[]).filter(k => k !== cluster.value))
const networkFocus = computed(() => {
  const k = layer.value === 'hub' ? hoverCluster.value : cluster.value
  return k ? CAT[k] : null
})
const networkAlpha = computed(() => (layer.value === 'hub' ? 1 : layer.value === 'module' ? 0.35 : 0.14))

function onSelect(node: GraphNode) {
  if (node.id.startsWith('hub:')) {
    cluster.value = node.id.slice(4) as Kind
    hoverCluster.value = null
    if (focus.value) focus.value = null
    return
  }
  if (/^(user|role|menu):\d+$/.test(node.id) && node.id !== focus.value) focus.value = node.id
}
function back() {
  if (parsed.value) { focus.value = null; return }
  cluster.value = null
}
function home() {
  focus.value = null
  cluster.value = null
}
function onHover(id: string | null) {
  hoverCluster.value = id?.startsWith('hub:') ? (id.slice(4) as Kind) : null
}

/* ── 分析栏 ── */
const panel = computed(() => {
  const idx = index.value
  const f = parsed.value
  if (!idx || !f) return null
  const chipUser = (id: number): InsightChip | null => { const u = idx.user.get(id); return u ? { id: `user:${u.id}`, label: u.nickname, sub: u.deptName ?? '', cat: CAT.user, tone: u.status === 1 ? undefined : 'off' } : null }
  const chipRole = (id: number): InsightChip | null => { const r = idx.role.get(id); return r ? { id: `role:${r.id}`, label: r.name, sub: r.code, cat: CAT.role, tone: r.status === 1 ? undefined : 'off' } : null }
  const chipMenu = (m: GraphMenu): InsightChip => ({ id: `menu:${m.id}`, label: m.name, sub: m.permission ?? '', cat: CAT.menu, tone: m.status === 1 ? undefined : 'off' })
  const cap = <T,>(list: T[], n = 24) => ({ chips: list.slice(0, n), more: Math.max(0, list.length - n) })
  const notNull = <T,>(v: T | null): v is T => v !== null

  if (f.kind === 'role') {
    const role = idx.role.get(f.id)!
    const users = idx.usersOfRole.get(role.id) ?? []
    const granted = new Set(isSuperRole(role) ? idx.graph.menus.map(m => m.id) : role.menuIds)
    const pageList = pages(idx, granted)
    const btns = [...granted].filter(id => idx.menu.get(id)?.type === 'btn').length
    const insights: InsightCard[] = []
    if (isSuperRole(role)) insights.push({ tone: 'info', title: '超级管理员', text: '内置角色，自动拥有全部菜单与按钮，不受授权勾选影响。' })
    if (role.status !== 1) insights.push({ tone: 'warn', title: '角色已停用', text: '持有该角色的用户不会获得其中任何权限。' })
    if (!users.length && !(role.userCount > 0)) insights.push({ tone: 'warn', title: '闲置角色', text: '没有分配给任何用户，可考虑停用或与相近角色合并。' })
    const lonely = pageList.filter(m => (idx.rolesOfMenu.get(m.id) ?? []).filter(r => roleActive(idx.role.get(r))).length === 1)
    if (lonely.length && !isSuperRole(role)) insights.push({ tone: 'info', title: '独占授权', text: `${lonely.map(m => m.name).slice(0, 4).join('、')}${lonely.length > 4 ? ' 等' : ''} 仅由该角色授予，删除角色将导致无人可访问。` })
    return {
      key: focus.value!, kind: '角色', kindIcon: 'role', cat: CAT.role, title: role.name, subtitle: role.code,
      status: `${role.dataScope} · ${role.builtin ? '内置角色' : '自定义角色'}`,
      metric: { label: '授予页面', value: pageList.length, unit: '个', bars: pageList.map(m => buttonsUnder(idx, m.id, granted).length + 1) },
      tiles: [{ label: '按钮权限', value: btns }, { label: '持有用户', value: idx.graph.usersVisible ? users.length : role.userCount }, { label: '状态', value: role.status === 1 ? '正常' : '停用' }] as InsightTile[],
      insights,
      groups: [
        { title: '持有用户', icon: 'users', ...cap(users.map(chipUser).filter(notNull)) },
        { title: '可访问页面', icon: 'menu', ...cap(pageList.map(chipMenu)) },
      ] as InsightGroup[],
    }
  }
  if (f.kind === 'user') {
    const user = idx.user.get(f.id)!
    const roleIds = idx.rolesOfUser.get(user.id) ?? []
    const menuIds = menusOfUser(idx, user.id)
    const pageList = pages(idx, menuIds)
    const btns = [...menuIds].filter(id => idx.menu.get(id)?.type === 'btn').length
    const active = roleIds.filter(id => roleActive(idx.role.get(id)))
    const insights: InsightCard[] = []
    if (!roleIds.length) insights.push({ tone: 'warn', title: '未分配角色', text: '该用户登录后看不到任何菜单。' })
    if (active.length >= 3) insights.push({ tone: 'warn', title: '多角色叠加', text: `同时持有 ${active.length} 个启用角色，权限可能超出岗位需要。` })
    if (roleIds.length > active.length) insights.push({ tone: 'info', title: '含停用角色', text: `${roleIds.length - active.length} 个角色已停用，不提供权限。` })
    if (user.status !== 1) insights.push({ tone: 'danger', title: '账号已停用', text: '账号无法登录，以上权限暂不生效。' })
    if (active.some(id => isSuperRole(idx.role.get(id)))) insights.push({ tone: 'info', title: '超级管理员', text: '拥有全部菜单与按钮权限。' })
    return {
      key: focus.value!, kind: '用户', kindIcon: 'user', cat: CAT.user, title: user.nickname, subtitle: `@${user.username}`,
      status: user.deptName ? `所属部门：${user.deptName}` : '未分配部门',
      metric: { label: '可访问页面', value: pageList.length, unit: '个', bars: active.map(id => idx.role.get(id)?.menuIds.length ?? 0) },
      tiles: [{ label: '角色', value: roleIds.length }, { label: '按钮权限', value: btns }, { label: '状态', value: user.status === 1 ? '正常' : '停用' }] as InsightTile[],
      insights,
      groups: [
        { title: '持有角色', icon: 'role', ...cap(roleIds.map(chipRole).filter(notNull)) },
        { title: '可访问页面', icon: 'menu', ...cap(pageList.map(chipMenu)) },
      ] as InsightGroup[],
    }
  }
  const menu = idx.menu.get(f.id)!
  const { roles, users } = holdersOfMenu(idx, menu.id)
  const path = menuPath(idx, menu.id).map(m => m.name).join(' / ')
  const insights: InsightCard[] = []
  if (!roles.some(r => !isSuperRole(r))) insights.push({ tone: 'warn', title: '孤立菜单', text: '除超级管理员外没有任何角色授予，普通用户无法访问。' })
  if (menu.type === 'btn' && !menu.permission) insights.push({ tone: 'danger', title: '缺少权限标识', text: '按钮没有权限标识，前后端都无法校验。' })
  if (menu.visible === 0) insights.push({ tone: 'info', title: '隐藏菜单', text: '不在侧栏显示，但有权限的用户仍可通过路由访问。' })
  const buttons = buttonsUnder(idx, menu.id)
  return {
    key: focus.value!, kind: menu.type === 'btn' ? '按钮' : menu.type === 'dir' ? '目录' : '页面', kindIcon: menu.type === 'btn' ? 'key' : 'menu', cat: CAT.menu,
    title: menu.name, subtitle: menu.permission ?? menu.path ?? '', status: path,
    metric: { label: '覆盖用户', value: idx.graph.usersVisible ? users.length : null, unit: '人', bars: roles.map(r => idx.usersOfRole.get(r.id)?.length ?? 0) },
    tiles: [{ label: '授予角色', value: roles.length }, { label: '下级按钮', value: buttons.length }, { label: '状态', value: menu.status === 1 ? '正常' : '停用' }] as InsightTile[],
    insights,
    groups: [
      { title: '授予角色', icon: 'role', ...cap(roles.map(r => chipRole(r.id)).filter(notNull)) },
      { title: '可访问用户', icon: 'users', ...cap(users.map(u => chipUser(u.id)).filter(notNull)) },
      ...(buttons.length ? [{ title: '下级按钮', icon: 'key', ...cap(buttons.map(chipMenu)) }] : []),
    ] as InsightGroup[],
  }
})

const findings = computed(() => (index.value ? permissionFindings(index.value) : null))
const entityLabel = computed(() => {
  const f = parsed.value
  const idx = index.value
  if (!f || !idx) return ''
  if (f.kind === 'user') return idx.user.get(f.id)?.nickname ?? ''
  if (f.kind === 'role') return idx.role.get(f.id)?.name ?? ''
  return idx.menu.get(f.id)?.name ?? ''
})
</script>

<template>
  <div class="trace">
    <RelationGraph
      v-model:size="size"
      class="trace-graph"
      variant="stage"
      label="权限溯源图"
      :nodes="scene.nodes"
      :links="scene.links"
      :active-id="parsed ? focus : null"
      :highlight="highlight"
      :network="network"
      :network-focus="networkFocus"
      :network-alpha="networkAlpha"
      :burst-key="burst"
      @select="onSelect"
      @back="back"
      @hover="onHover"
    >
      <template #tl>
        <nav class="trace-crumbs m-float" aria-label="溯源路径">
          <IconButton v-if="layer !== 'hub'" name="chevronLeft" label="返回上一层" small @click="back" />
          <button type="button" :class="['trace-crumb', { 'is-current': layer === 'hub' }]" @click="home"><Icon name="shield" :size="13" />权限全景</button>
          <template v-if="cluster && layer !== 'hub'">
            <span class="trace-sep" aria-hidden="true">/</span>
            <button type="button" :class="['trace-crumb', `cat-${CAT[cluster]}`, { 'is-current': layer === 'module' }]" @click="focus = null">{{ KIND_LABEL[cluster] }}</button>
          </template>
          <template v-if="parsed">
            <span class="trace-sep" aria-hidden="true">/</span>
            <b class="trace-crumb is-current">{{ entityLabel }}</b>
          </template>
        </nav>
      </template>
      <template v-if="layer === 'hub' && findings" #tr>
        <dl class="trace-legend m-float">
          <div class="cat-2"><dt><i />用户</dt><dd class="tnum">{{ index?.graph.users.length ?? 0 }}</dd></div>
          <div class="cat-3"><dt><i />角色</dt><dd class="tnum">{{ index?.graph.roles.length ?? 0 }}</dd></div>
          <div class="cat-5"><dt><i />菜单</dt><dd class="tnum">{{ index?.graph.menus.length ?? 0 }}</dd></div>
          <div class="is-warn"><dt>闲置角色</dt><dd class="tnum">{{ findings.unusedRoles.length }}</dd></div>
          <div class="is-warn"><dt>孤立菜单</dt><dd class="tnum">{{ findings.orphanMenus.length }}</dd></div>
          <div class="is-warn"><dt>无角色用户</dt><dd class="tnum">{{ findings.usersWithoutRole.length }}</dd></div>
        </dl>
      </template>
      <div :class="['trace-dock', layer === 'hub' ? 'is-center' : 'is-side']">
        <AskDock group="perm" :compact="layer !== 'hub'" placeholder="问问权限…例如「谁能删除用户」" />
      </div>
      <div v-if="panel" class="trace-panel">
        <InsightPanel
          :content-key="panel.key"
          :kind="panel.kind"
          :kind-icon="panel.kindIcon"
          :cat="panel.cat"
          :title="panel.title"
          :subtitle="panel.subtitle"
          :status="panel.status"
          :metric="panel.metric"
          :tiles="panel.tiles"
          :insights="panel.insights"
          :groups="panel.groups"
          @close="back"
          @chip="focus = $event.id"
        />
      </div>
      <p v-if="loading" class="trace-hint">正在汇总授权关系…</p>
      <p v-else-if="layer === 'module'" class="trace-hint">点击任一条目，展开它的完整授权链路</p>
    </RelationGraph>
  </div>
</template>
