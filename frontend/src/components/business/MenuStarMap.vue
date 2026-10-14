<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import RelationGraph, { type GraphLink, type GraphNode } from '@/components/charts/RelationGraph.vue'
import InsightPanel, { type InsightCard, type InsightChip } from './InsightPanel.vue'
import PermissionButton from './PermissionButton.vue'
import Icon from '@/components/Icon.vue'
import IconButton from '@/components/ui/IconButton.vue'
import { pathOf } from '@/composables/auth'
import type { NetLink, NetNode, NetworkData } from '@/composables/neural'
import { hasPermission } from '@/directives/permission'
import { loadPermissionGraph, permissionIndex } from '@/composables/permissionGraph'
import { holdersOfMenu, isSuperRole } from '@/utils/permissionGraph'
import type { Menu } from '@/data'

/* ============================================================
   导航星系（图谱舞台，三层下钻）：
   - 全景：中心珍珠球「导航」，三圈带刻度的轨道；一级目录 / 菜单是内圈玻璃刻度表盘（刻度为已授权页面占比），
     页面与按钮作为真实点位排在中圈、外圈，按一级分支着分类色；悬停表盘点亮整条分支并浮出名称
   - 分支：点表盘 → 珍珠球退左、一级项竖排，下属页面以带标签胶囊触须扇形长出
   - 页面：点页面 → 授予它的角色在左、它的按钮在右，脉冲 角色 → 页面 → 按钮；右侧分析栏可直接新增 / 编辑
   孤立页面（无角色授予）与缺权限标识的按钮发红光；左侧「导航预览」即时渲染侧栏样子。
   ============================================================ */
const props = defineProps<{ tree: Menu[] }>()
const emit = defineEmits<{ (e: 'edit', menu: Menu): void; (e: 'create', parent: Menu): void }>()
const router = useRouter()
const size = ref({ w: 0, h: 0 })
const branchId = ref<number | null>(null)
const focusId = ref<number | null>(null)
const hoverBranch = ref<number | null>(null)
const showPreview = ref(true)
const showOrphans = ref(false)
const canTrace = hasPermission('system:role:list')

onMounted(() => { if (canTrace) loadPermissionGraph().catch(() => {}) })

interface Entry { menu: Menu; parent: Menu | null; branch: number; depth: number }
const flat = computed(() => {
  const map = new Map<number, Entry>()
  const walk = (nodes: Menu[], parent: Menu | null, depth: number, branch: number) => nodes.forEach((m, i) => {
    const b = depth === 0 ? i : branch
    map.set(m.id, { menu: m, parent, branch: b, depth })
    walk(m.children ?? [], m, depth + 1, b)
  })
  walk(props.tree, null, 0, 0)
  return map
})
// 树刷新后丢失的聚焦对象自动回退
watch(flat, (map) => {
  if (branchId.value !== null && !map.has(branchId.value)) branchId.value = null
  if (focusId.value !== null && !map.has(focusId.value)) focusId.value = null
})

const catOf = (id: number) => ((flat.value.get(id)?.branch ?? 0) % 5) + 1
const pagesOf = (m: Menu) => (m.children ?? []).filter(c => c.type !== 'btn')
const buttonsOf = (m: Menu) => (m.children ?? []).filter(c => c.type === 'btn')
const isLeafTop = (m: Menu) => !pagesOf(m).length

/** 授予该菜单的启用非超级角色数；未加载权限图时为 null */
function grantCount(id: number) {
  const idx = permissionIndex.value
  if (!idx) return null
  return (idx.rolesOfMenu.get(id) ?? []).filter(r => !isSuperRole(idx.role.get(r)) && idx.role.get(r)?.status === 1).length
}
/** 表盘读数：分支内页面被普通角色授权的比例（未加载权限图时按 0） */
function grantedShare(m: Menu) {
  const pages: Menu[] = []
  const walk = (list: Menu[]) => list.forEach((c) => { if (c.type === 'menu') pages.push(c); walk(c.children ?? []) })
  walk(m.type === 'menu' ? [m] : m.children ?? [])
  if (!pages.length || !permissionIndex.value) return 0
  return pages.filter(p => (grantCount(p.id) ?? 0) > 0).length / pages.length
}
const isDanger = (m: Menu) => m.type === 'btn' && !m.permission
const isOrphan = (m: Menu) => m.type !== 'btn' && m.status === 1 && grantCount(m.id) === 0
const toneOf = (m: Menu): GraphNode['tone'] => (isDanger(m) || (showOrphans.value && isOrphan(m)) ? 'danger' : m.status !== 1 ? 'off' : undefined)

const layer = computed<'hub' | 'branch' | 'page'>(() => (focusId.value !== null ? 'page' : branchId.value !== null ? 'branch' : 'hub'))
/** 页面层的主体：聚焦按钮时取其所属页面 */
const pageMenu = computed(() => {
  if (focusId.value === null) return null
  const e = flat.value.get(focusId.value)
  if (!e) return null
  return e.menu.type === 'btn' && e.parent ? e.parent : e.menu
})

/* ── 全景几何：以珍珠球为心的三圈椭圆 ── */
const PREVIEW_W = 226
const leftPad = computed(() => (showPreview.value && size.value.w >= 1100 && focusId.value === null ? PREVIEW_W : 0))
function geometry() {
  const { w, h } = size.value
  const left = leftPad.value
  const W = w - left
  const cx = left + W / 2
  const cy = h * 0.5
  const kx = Math.max(160, W / 2 - 60)
  const ky = Math.max(140, h / 2 - 40)
  return { cx, cy, kx, ky, W }
}
const RING = { hub: 0.52, page: 0.77, btn: 0.97 }
const leaves = (m: Menu): number => 1 + (m.children?.length ? m.children.reduce((a, c) => a + leaves(c), 0) : 0)

/** 每个一级项占据的扇区：按叶子数加权，但设保底份额，保证表盘之间留出足够间距 */
const sectors = computed(() => {
  const raw = props.tree.reduce((a, m) => a + leaves(m), 0) || 1
  const floor = raw / Math.max(1, props.tree.length * 0.85)
  const weight = (m: Menu) => Math.max(leaves(m), floor)
  const total = props.tree.reduce((a, m) => a + weight(m), 0)
  let cursor = -Math.PI / 2 - (weight(props.tree[0] ?? ({} as Menu)) / total) * Math.PI
  return props.tree.map((m) => {
    const span = (weight(m) / total) * Math.PI * 2
    const s = { menu: m, start: cursor, span, mid: cursor + span / 2 }
    cursor += span
    return s
  })
})

const network = computed<NetworkData | null>(() => {
  const { w, h } = size.value
  if (!w || !h || !props.tree.length) return null
  const { cx, cy, kx, ky } = geometry()
  const nodes: NetNode[] = []
  const links: NetLink[] = []
  sectors.value.forEach(({ menu: top, start, span, mid }, bi) => {
    const cat = (bi % 5) + 1
    nodes.push({ id: `m:${top.id}`, x: cx + Math.cos(mid) * kx * RING.hub, y: cy + Math.sin(mid) * ky * RING.hub, cat, r: 0.1, depth: 1 })
    const pages = pagesOf(top)
    const directBtns = buttonsOf(top)
    const pageSlots = pages.length || 1
    pages.forEach((page, i) => {
      const a = start + span * ((i + 0.5) / pageSlots)
      const id = `m:${page.id}`
      nodes.push({ id, x: cx + Math.cos(a) * kx * RING.page, y: cy + Math.sin(a) * ky * RING.page, cat, r: 2.8, depth: 2, label: page.name, tone: toneOf(page) === 'danger' ? 'danger' : page.status !== 1 ? 'off' : undefined })
      links.push({ a: `m:${top.id}`, b: id, cat })
      const btns = buttonsOf(page)
      const sub = (span / pageSlots) * 0.8
      btns.forEach((btn, j) => {
        const ba = a - sub / 2 + sub * ((j + 0.5) / btns.length)
        const bid = `m:${btn.id}`
        nodes.push({ id: bid, x: cx + Math.cos(ba) * kx * RING.btn, y: cy + Math.sin(ba) * ky * RING.btn, cat, r: 1.5, depth: 3, label: btn.permission ?? btn.name, tone: isDanger(btn) ? 'danger' : btn.status !== 1 ? 'off' : undefined })
        links.push({ a: id, b: bid, cat })
      })
    })
    directBtns.forEach((btn, j) => {
      const ba = start + span * ((j + 0.5) / directBtns.length)
      const bid = `m:${btn.id}`
      nodes.push({ id: bid, x: cx + Math.cos(ba) * kx * RING.btn, y: cy + Math.sin(ba) * ky * RING.btn, cat, r: 1.5, depth: 3, label: btn.permission ?? btn.name, tone: isDanger(btn) ? 'danger' : undefined })
      links.push({ a: `m:${top.id}`, b: bid, cat })
    })
  })
  return { nodes, links }
})

const networkLit = computed(() => {
  if (layer.value !== 'hub' || hoverBranch.value === null) return null
  return catOf(hoverBranch.value)
})

/* ── 场景 ── */
const COL_TOP = 76
const COL_BOTTOM = 40
function column<T>(items: T[], x: number, H: number, make: (item: T, y: number, i: number, n: number) => GraphNode) {
  const span = Math.max(120, H - COL_TOP - COL_BOTTOM)
  const n = items.length
  const gap = n > 1 ? Math.min(52, span / (n - 1)) : 0
  const y0 = COL_TOP + span / 2 - (gap * (n - 1)) / 2
  return items.map((item, i) => make(item, y0 + i * gap, i, n))
}
const stagger = (i: number, n: number, base = 0) => base + i * Math.min(0.09, 1.6 / Math.max(n, 1))

const scene = computed<{ nodes: GraphNode[]; links: GraphLink[] }>(() => {
  const { w, h } = size.value
  if (!w || !h || !props.tree.length) return { nodes: [], links: [] }
  const nodes: GraphNode[] = []
  const links: GraphLink[] = []

  if (layer.value === 'hub') {
    const { cx, cy, kx, ky } = geometry()
    nodes.push({ id: 'root', x: cx, y: cy, size: 72, label: '导航', sub: `${flat.value.size} 项`, kind: 'core', interactive: false, float: true })
    sectors.value.forEach(({ menu: m, mid }, i) => {
      const id = `m:${m.id}`
      nodes.push({
        id, x: cx + Math.cos(mid) * kx * RING.hub, y: cy + Math.sin(mid) * ky * RING.hub,
        size: 80, kind: 'hub', label: m.name, sub: isLeafTop(m) ? (m.path ?? '') : `${pagesOf(m).length} 个页面`,
        icon: m.icon || 'menu', cat: catOf(m.id), tone: toneOf(m), count: leaves(m) - 1 || undefined, ratio: grantedShare(m),
        from: 'root', delay: 0.7 + i * 0.12, float: true,
      })
      links.push({ a: 'root', b: id, flow: 'both', cat: catOf(m.id), dashed: m.visible === 0 })
    })
    return { nodes, links }
  }

  const H = h
  const left = leftPad.value
  const W = w - left - (layer.value === 'page' && w >= 900 ? 392 : 0)
  const cy = COL_TOP + (H - COL_TOP - COL_BOTTOM) / 2
  const branch = flat.value.get(branchId.value!)?.menu
  if (!branch) return { nodes, links }

  if (layer.value === 'branch') {
    const coreX = left + Math.max(76, W * 0.06)
    const colX = coreX + 130
    nodes.push({ id: 'root', x: coreX, y: cy, size: 56, label: '导航', kind: 'core', interactive: false })
    // 一级项竖排：选中居中放大，其余上下排开并退暗
    const others = props.tree.filter(m => m.id !== branch.id)
    const order = [...others.slice(0, Math.ceil(others.length / 2)), branch, ...others.slice(Math.ceil(others.length / 2))]
    const k = order.indexOf(branch)
    const gap = Math.min(130, (H - 120) / Math.max(order.length - 1, 1))
    order.forEach((m, i) => {
      const sel = m.id === branch.id
      const id = `m:${m.id}`
      nodes.push({ id, x: colX, y: cy + (i - k) * gap, size: sel ? 100 : 56, ratio: grantedShare(m), kind: 'hub', label: m.name, sub: sel ? `${pagesOf(m).length} 个页面` : undefined, icon: m.icon || 'menu', cat: catOf(m.id), tone: toneOf(m), count: sel ? leaves(m) - 1 : undefined, from: 'root', float: true })
      links.push({ a: 'root', b: id, flow: sel ? 'both' : 'none', shape: 'h', cat: catOf(m.id) })
    })
    const pages = pagesOf(branch)
    const fanX = Math.min(Math.max(colX + 280, left + W * 0.42), left + W - 300)
    const fan = column(pages, fanX, H, (p, y, i, n) => ({
      id: `m:${p.id}`, x: fanX, y, kind: 'item', align: 'left', label: p.name,
      sub: p.path ?? p.permission ?? '', count: buttonsOf(p).length ? `${buttonsOf(p).length} 按钮` : undefined,
      icon: p.icon || undefined, cat: catOf(p.id), tone: toneOf(p), from: `m:${branch.id}`, delay: 0.15 + stagger(i, n),
    }))
    nodes.push(...fan)
    fan.forEach(n => links.push({ a: `m:${branch.id}`, b: n.id, flow: 'forward', shape: 'h', cat: catOf(branch.id), tone: n.tone === 'danger' ? 'danger' : undefined, dashed: flat.value.get(Number(n.id.slice(2)))?.menu.visible === 0 }))
    return { nodes, links }
  }

  // 页面层：角色 → 页面 → 按钮
  const page = pageMenu.value!
  const pid = `m:${page.id}`
  const cat = catOf(page.id)
  nodes.push({ id: pid, x: left + W * 0.44, y: cy, kind: 'item', label: page.name, sub: page.permission ?? page.path ?? '', icon: page.icon || 'menu', cat, tone: toneOf(page), count: page.type === 'dir' ? '目录' : '页面' })
  const btns = buttonsOf(page)
  nodes.push(...column(btns, left + W * 0.72, H, (b, y, i, n) => ({ id: `m:${b.id}`, x: left + W * 0.72, y, kind: 'item', align: 'left', label: b.name, sub: b.permission ?? '缺少权限标识', cat, tone: toneOf(b), from: pid, delay: stagger(i, n, 0.2) })))
  btns.forEach(b => links.push({ a: pid, b: `m:${b.id}`, flow: 'forward', shape: 'h', cat, tone: isDanger(b) ? 'danger' : undefined }))
  const idx = permissionIndex.value
  if (idx) {
    const roles = holdersOfMenu(idx, page.id).roles
    nodes.push(...column(roles, left + W * 0.16, H, (r, y, i, n) => ({ id: `role:${r.id}`, x: left + W * 0.16, y, kind: 'item', align: 'right', label: r.name, sub: r.code, cat: 3, tone: r.status === 1 ? undefined : 'off', from: pid, delay: stagger(i, n, 0.1) })))
    roles.forEach(r => links.push({ a: `role:${r.id}`, b: pid, flow: 'forward', shape: 'h', cat: 3 }))
  }
  return { nodes, links }
})

const highlight = computed(() => {
  if (layer.value === 'branch') return scene.value.nodes.filter(n => !(n.kind === 'hub' && n.id !== `m:${branchId.value}`)).map(n => n.id)
  if (layer.value === 'page' && focusId.value !== null) {
    const f = flat.value.get(focusId.value)
    if (f?.menu.type === 'btn') return [`m:${f.menu.id}`, `m:${pageMenu.value?.id}`, ...scene.value.nodes.filter(n => n.id.startsWith('role:')).map(n => n.id)]
  }
  return null
})

function onSelect(node: GraphNode) {
  if (node.id.startsWith('role:')) {
    const path = pathOf('system/role/index')
    if (path) router.push({ path, query: { trace: node.id } })
    return
  }
  const id = Number(node.id.slice(2))
  const e = flat.value.get(id)
  if (!e) return
  if (e.depth === 0) {
    branchId.value = id
    focusId.value = isLeafTop(e.menu) ? id : null
    hoverBranch.value = null
    return
  }
  focusId.value = id
}
function back() {
  if (focusId.value !== null) {
    const leaf = branchId.value !== null && focusId.value === branchId.value
    const f = flat.value.get(focusId.value)
    // 按钮聚焦先退回所属页面，再退回分支
    if (f?.menu.type === 'btn' && f.parent) { focusId.value = f.parent.id; return }
    focusId.value = null
    if (leaf) branchId.value = null
    return
  }
  branchId.value = null
}
function home() { focusId.value = null; branchId.value = null }
function onHover(id: string | null) {
  const n = id?.startsWith('m:') ? flat.value.get(Number(id.slice(2))) : null
  hoverBranch.value = n && n.depth === 0 ? n.menu.id : null
}
function jump(id: number) {
  const e = flat.value.get(id)
  if (!e) return
  let top: Entry | undefined = e
  while (top && top.parent) top = flat.value.get(top.parent.id)
  branchId.value = top?.menu.id ?? null
  focusId.value = e.depth === 0 && !isLeafTop(e.menu) ? null : id
}

/* ── 分析栏 ── */
const TYPE = { dir: '目录', menu: '页面', btn: '按钮' } as const
const panel = computed(() => {
  if (focusId.value === null) return null
  const entry = flat.value.get(focusId.value)
  if (!entry) return null
  const m = entry.menu
  const crumbs: string[] = []
  let cur: Entry | undefined = entry
  while (cur) { crumbs.unshift(cur.menu.name); cur = cur.parent ? flat.value.get(cur.parent.id) : undefined }
  const idx = permissionIndex.value
  const holders = idx ? holdersOfMenu(idx, m.id) : null
  const insights: InsightCard[] = []
  if (isDanger(m)) insights.push({ tone: 'danger', title: '缺少权限标识', text: '按钮没有权限标识，前后端都无法据此校验。' })
  if (isOrphan(m)) insights.push({ tone: 'warn', title: '孤立菜单', text: '除超级管理员外没有角色授予，普通用户无法访问。' })
  if (m.visible === 0) insights.push({ tone: 'info', title: '导航中隐藏', text: '不显示在侧栏，有权限的用户仍可通过地址访问。' })
  if (m.status !== 1) insights.push({ tone: 'warn', title: '已停用', text: '停用后所有角色都无法访问该项及其下级。' })
  const roleChips: InsightChip[] = holders?.roles.map(r => ({ id: `role:${r.id}`, label: r.name, sub: r.code, cat: 3 })) ?? []
  return {
    key: String(m.id),
    kind: TYPE[m.type], kindIcon: m.type === 'btn' ? 'key' : (m.icon || 'menu'), cat: catOf(m.id),
    title: m.name,
    subtitle: m.permission || m.path || '',
    status: crumbs.join(' / '),
    tiles: [
      { label: '下级', value: m.children?.length ?? 0 },
      { label: '授予角色', value: holders ? holders.roles.length : '—' },
      { label: '覆盖用户', value: holders && idx?.graph.usersVisible ? holders.users.length : '—' },
    ],
    insights,
    groups: [
      ...(holders ? [{ title: '授予角色', icon: 'role', chips: roleChips }] : []),
      ...(m.children?.length ? [{ title: m.type === 'menu' ? '页面按钮' : '下级菜单', icon: 'layers', chips: m.children.map(c => ({ id: `menu:${c.id}`, label: c.name, sub: c.permission ?? '', tone: isDanger(c) ? 'danger' as const : c.status !== 1 ? 'off' as const : undefined })) }] : []),
    ],
    menu: m,
  }
})

function onChip(chip: InsightChip) {
  if (chip.id.startsWith('menu:')) { focusId.value = Number(chip.id.slice(5)); return }
  const path = pathOf('system/role/index')
  if (path) router.push({ path, query: { trace: chip.id } })
}

/* ── 导航预览与图例 ── */
const preview = computed(() => props.tree.filter(m => m.type !== 'btn'))
const findings = computed(() => {
  let orphan = 0, noCode = 0, hidden = 0
  flat.value.forEach(({ menu }) => {
    if (isDanger(menu)) noCode += 1
    else if (isOrphan(menu)) orphan += 1
    if (menu.visible === 0 && menu.type !== 'btn') hidden += 1
  })
  return { orphan, noCode, hidden }
})
const crumbs = computed(() => {
  const list: Array<{ id: number; label: string }> = []
  const b = branchId.value !== null ? flat.value.get(branchId.value) : null
  if (b) list.push({ id: b.menu.id, label: b.menu.name })
  const pm = pageMenu.value
  if (pm && pm.id !== b?.menu.id) list.push({ id: pm.id, label: pm.name })
  const f = focusId.value !== null ? flat.value.get(focusId.value) : null
  if (f && f.menu.type === 'btn') list.push({ id: f.menu.id, label: f.menu.name })
  return list
})
const orbits = computed(() => {
  if (layer.value !== 'hub' || !size.value.w) return []
  const { kx, ky } = geometry()
  return [RING.hub, RING.page, RING.btn].map(r => ({ rx: kx * r, ry: ky * r }))
})
</script>

<template>
  <RelationGraph
    v-model:size="size"
    class="starmap"
    variant="stage"
    label="导航星系"
    :nodes="scene.nodes"
    :links="scene.links"
    :active-id="focusId !== null ? `m:${focusId}` : null"
    :highlight="highlight"
    :network="network"
    :network-focus="networkLit"
    :network-alpha="layer === 'hub' ? 1 : layer === 'branch' ? 0.3 : 0.12"
    :orbits="orbits"
    @select="onSelect"
    @back="back"
    @hover="onHover"
  >
    <template #tl>
      <nav class="trace-crumbs m-float" aria-label="星系路径">
        <IconButton v-if="layer !== 'hub'" name="chevronLeft" label="返回上一层" small @click="back" />
        <button type="button" :class="['trace-crumb', { 'is-current': layer === 'hub' }]" @click="home"><Icon name="menu" :size="13" />导航</button>
        <template v-for="(c, i) in crumbs" :key="c.id">
          <span class="trace-sep" aria-hidden="true">/</span>
          <b v-if="i === crumbs.length - 1" class="trace-crumb is-current">{{ c.label }}</b>
          <button v-else type="button" class="trace-crumb" @click="jump(c.id)">{{ c.label }}</button>
        </template>
      </nav>
    </template>
    <template v-if="layer === 'hub'" #tr>
      <dl class="trace-legend m-float">
        <div v-if="permissionIndex" :class="['is-warn legend-toggle', { 'is-on': showOrphans }]" @click="showOrphans = !showOrphans"><dt><button type="button" :aria-pressed="showOrphans">孤立页面</button></dt><dd class="tnum">{{ findings.orphan }}</dd></div>
        <div class="is-warn"><dt>缺权限标识</dt><dd class="tnum">{{ findings.noCode }}</dd></div>
        <div><dt>隐藏项</dt><dd class="tnum">{{ findings.hidden }}</dd></div>
      </dl>
      <button type="button" class="stage-chip" :aria-pressed="showPreview" @click="showPreview = !showPreview"><Icon name="layers" :size="13" />导航预览</button>
    </template>
    <nav v-if="leftPad" class="nav-preview m-float" aria-label="导航预览">
      <span class="nav-preview-title">侧栏预览</span>
      <ul>
        <li v-for="dir in preview" :key="dir.id" :class="{ 'is-hidden': dir.visible === 0, 'is-off': dir.status !== 1 }">
          <button type="button" :class="['nav-preview-item', { 'is-on': branchId === dir.id }]" @click="jump(dir.id)">
            <Icon :name="dir.icon || 'menu'" :size="13" />{{ dir.name }}
            <Icon v-if="dir.visible === 0" name="eye" :size="11" class="nav-preview-flag" />
          </button>
          <ul v-if="dir.type === 'dir'">
            <li v-for="page in pagesOf(dir)" :key="page.id" :class="{ 'is-hidden': page.visible === 0, 'is-off': page.status !== 1 }">
              <button type="button" :class="['nav-preview-item is-sub', { 'is-on': pageMenu?.id === page.id }]" @click="jump(page.id)">
                {{ page.name }}<Icon v-if="page.visible === 0" name="eye" :size="11" class="nav-preview-flag" />
              </button>
            </li>
          </ul>
        </li>
      </ul>
    </nav>
    <div v-if="panel" class="trace-panel">
      <InsightPanel
        :content-key="panel.key"
        :kind="panel.kind"
        :kind-icon="panel.kindIcon"
        :cat="panel.cat"
        :title="panel.title"
        :subtitle="panel.subtitle"
        :status="panel.status"
        :tiles="panel.tiles"
        :insights="panel.insights"
        :groups="panel.groups"
        @close="back"
        @chip="onChip"
      >
        <div class="insight-actions">
          <PermissionButton v-if="panel.menu.type !== 'btn'" perm="system:menu:add" variant="secondary" size="sm" icon="plus" @click="emit('create', panel.menu)">{{ panel.menu.type === 'dir' ? '新增菜单' : '新增按钮' }}</PermissionButton>
          <PermissionButton perm="system:menu:edit" variant="primary" size="sm" icon="edit" @click="emit('edit', panel.menu)">编辑</PermissionButton>
        </div>
      </InsightPanel>
    </div>
    <p v-if="layer === 'hub' && scene.nodes.length" class="trace-hint">点击表盘展开分支 · 刻度为已授权页面占比</p>
    <p v-else-if="layer === 'branch'" class="trace-hint">点击页面查看它的按钮与授权角色</p>
  </RelationGraph>
</template>
