<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import RelationGraph, { type GraphLink, type GraphNode } from '@/components/charts/RelationGraph.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import StatusTag from '@/components/ui/StatusTag.vue'
import PermissionButton from './PermissionButton.vue'
import { systemApi, type UserRow } from '@/api'
import { pathOf } from '@/composables/auth'
import { toastError } from '@/composables/toast'
import { hasPermission } from '@/directives/permission'

/* ============================================================
   岗位阶梯（图谱舞台）：岗位按 sort 沿对角线自左下向右上等距铺满舞台，
   每级是带分类色上沿的珍珠玻璃台面，台面下淡色光柱垂到刻度地平线；脊线脉冲逐级向上传递。
   选中岗位 → 在岗成员以带标签胶囊触须扇形长到左上空域；右上信息卡给出编辑与跳转。
   ============================================================ */
export interface LadderPost { id: number; postName: string; postCode: string; sort: number; status: number; userCount: number }

const props = withDefaults(defineProps<{
  posts: LadderPost[]
  loading?: boolean
}>(), {
  loading: false,
})
const emit = defineEmits<{ (e: 'edit', post: LadderPost): void }>()

const router = useRouter()
const size = ref({ w: 0, h: 0 })
const steps = computed(() => [...props.posts].sort((a, b) => a.sort - b.sort || a.id - b.id))

const activeId = ref<number | null>(null)
const members = ref<UserRow[]>([])
const membersLoading = ref(false)
const canListUsers = hasPermission('system:user:list')
const usersPath = computed(() => pathOf('system/user/index'))

watch(steps, (list) => {
  if (activeId.value !== null && !list.some(p => p.id === activeId.value)) activeId.value = null
  if (activeId.value === null && list.length) activeId.value = list[list.length - 1].id
}, { immediate: true })
const active = computed(() => steps.value.find(p => p.id === activeId.value) ?? null)

async function loadMembers(id: number) {
  if (!canListUsers) { members.value = []; return }
  membersLoading.value = true
  try {
    const page = await systemApi.users.list({ postId: id, pageNum: 1, pageSize: 50 })
    members.value = page.rows as UserRow[]
  } catch (error) {
    members.value = []
    toastError(error, '在岗人员加载失败')
  } finally {
    membersLoading.value = false
  }
}
watch(activeId, (id) => { if (id !== null) loadMembers(id) }, { immediate: true })

const catOf = (i: number) => (i % 5) + 1
const MEMBER_CAP = 12

const scene = computed<{ nodes: GraphNode[]; links: GraphLink[] }>(() => {
  const { w, h } = size.value
  const list = steps.value
  if (!w || !h || !list.length) return { nodes: [], links: [] }
  const nodes: GraphNode[] = []
  const links: GraphLink[] = []
  const n = list.length
  // 台阶沿对角线铺满：左下 → 右上；宽度随数量收缩，绝不出界
  const pad = Math.max(28, w * 0.03)
  const width = Math.max(92, Math.min(150, (w - pad * 2) / Math.max(n, 1) - 12))
  const x0 = pad + width / 2
  const x1 = w - pad - width / 2
  const y0 = h - 120
  const y1 = Math.max(150, h * 0.28)
  list.forEach((p, i) => {
    const k = n > 1 ? i / (n - 1) : 0.5
    const id = `post:${p.id}`
    nodes.push({
      id, kind: 'slab', x: n > 1 ? x0 + (x1 - x0) * k : (x0 + x1) / 2, y: y0 + (y1 - y0) * k,
      size: width, rank: `第 ${i + 1} 级`, label: p.postName, sub: p.postCode, count: p.userCount,
      cat: catOf(i), tone: p.status !== 1 ? 'off' : undefined, delay: 0.1 + i * 0.12,
    })
    if (i > 0) links.push({ a: `post:${list[i - 1].id}`, b: id, flow: 'forward', shape: 'h', cat: catOf(i) })
  })
  const post = active.value
  if (!post) return { nodes, links }
  const root = `post:${post.id}`
  const cat = catOf(list.indexOf(post))
  const shown = members.value.slice(0, MEMBER_CAP)
  const more = members.value.length - shown.length
  // 成员排在左上空域（信息卡右侧）：最多两列，从选中台阶触须长出
  const colX = [Math.max(300, w * 0.25), Math.max(300, w * 0.25) + 220]
  const top = 76
  const bottom = Math.min(h * 0.5, h - 260)
  const perCol = Math.max(3, Math.floor((bottom - top) / 50) + 1)
  const items = [...shown.map(u => ({ id: `u:${u.id}`, label: u.nickname, sub: [u.deptName, u.username].filter(Boolean).join(' · '), interactive: Boolean(usersPath.value) }))]
  if (more > 0) items.push({ id: 'more', label: `还有 ${more} 人`, sub: '在用户页查看全部', interactive: false })
  if (!items.length && !membersLoading.value) items.push({ id: 'empty', label: canListUsers ? '暂无在岗人员' : `共 ${post.userCount} 人在岗`, sub: canListUsers ? '可在用户管理中为用户指定岗位' : '无用户查询权限，仅显示人数', interactive: false })
  items.forEach((it, i) => {
    const c = Math.floor(i / perCol)
    const r = i % perCol
    const inCol = Math.min(perCol, items.length - c * perCol)
    const gap = Math.min(50, (bottom - top) / Math.max(inCol - 1, 1))
    const y = top + (bottom - top) / 2 - (gap * (inCol - 1)) / 2 + r * gap
    nodes.push({
      id: it.id, kind: 'item', align: 'left', x: colX[Math.min(c, 1)], y,
      label: it.label, sub: it.sub, icon: it.id.startsWith('u:') ? 'user' : undefined, cat: it.id.startsWith('u:') ? cat : undefined,
      tone: it.id.startsWith('u:') ? undefined : 'off', interactive: it.interactive, from: root, delay: 0.12 + i * 0.08,
    })
    if (it.id.startsWith('u:')) links.push({ a: root, b: it.id, flow: 'forward', cat })
  })
  return { nodes, links }
})

const highlight = computed(() => {
  if (!active.value) return null
  return scene.value.nodes.filter(n => !n.id.startsWith('post:') || n.id === `post:${active.value!.id}`).map(n => n.id)
})

function onSelect(node: GraphNode) {
  if (node.id.startsWith('post:')) { activeId.value = Number(node.id.slice(5)); return }
  const user = members.value.find(u => `u:${u.id}` === node.id)
  if (user && usersPath.value) router.push({ path: usersPath.value, query: { keyword: user.username } })
}
</script>

<template>
  <div class="ladder">
    <RelationGraph
      v-model:size="size"
      class="ladder-stage"
      :style="{ minWidth: `${steps.length * 112 + 72}px` }"
      variant="stage"
      label="岗位能量阶梯"
      :nodes="scene.nodes"
      :links="scene.links"
      :active-id="active ? `post:${active.id}` : null"
      :highlight="highlight"
      @select="onSelect"
      @back="activeId = null"
    >
      <template v-if="active" #tl>
        <section class="ladder-card m-float" :class="`cat-${(steps.indexOf(active) % 5) + 1}`" aria-live="polite">
          <div class="ladder-card-head">
            <span class="ladder-card-rank">第 {{ steps.indexOf(active) + 1 }} 级 · 共 {{ steps.length }} 级</span>
            <StatusTag :status="active.status" />
          </div>
          <h3>{{ active.postName }}</h3>
          <span class="code-chip">{{ active.postCode }}</span>
          <dl>
            <div><dt>在岗</dt><dd class="tnum">{{ active.userCount }}<small>人</small></dd></div>
            <div><dt>排序</dt><dd class="tnum">{{ active.sort }}</dd></div>
          </dl>
          <div class="ladder-card-actions">
            <PermissionButton perm="system:post:edit" variant="secondary" size="sm" icon="edit" @click="emit('edit', active)">编辑岗位</PermissionButton>
          </div>
        </section>
      </template>
      <p v-if="steps.length" class="trace-hint">点击台阶查看在岗人员 · 方向键切换</p>
    </RelationGraph>
    <EmptyState v-if="!loading && !steps.length" class="ladder-empty" title="还没有岗位" icon="post" />
  </div>
</template>
