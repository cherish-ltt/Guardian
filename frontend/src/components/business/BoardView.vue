<script setup lang="ts" generic="T extends Record<string, any>">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'
import Icon from '@/components/Icon.vue'
import { gsap, motionSkipped, createFrameLoop, spring, springOf, EASE } from '@/composables/motion'

/* ============================================================
   看板分列：按状态 / 部门 / 角色等分列展示卡片，列头着分类色。
   - 可拖拽时（draggable）：指针拖起卡片 → 落到其它列 → 调用 onMove（页面负责确认与接口）
     成功：幽灵卡飞向新位置落定（结 Crystallize）；失败或取消：飞回原处（停摆 Halt）
   - 键盘替代：卡片右上角「移动到…」菜单
   - 数据变化后其余卡片用 FLIP 平滑让位
   ============================================================ */
export interface BoardColumn<Row> {
  key: string | number
  title: string
  items: Row[]
  /** 该列是否接受拖入（默认接受） */
  accepts?: boolean
}

const props = withDefaults(defineProps<{
  columns: BoardColumn<T>[]
  itemKey: string
  draggable?: boolean
  /** 返回 true 表示已移动成功（页面随后刷新数据） */
  onMove?: (payload: { item: T; from: BoardColumn<T>; to: BoardColumn<T> }) => Promise<boolean>
  emptyText?: string
}>(), {
  draggable: false,
  emptyText: '暂无',
})

const root = ref<HTMLElement | null>(null)
const overKey = ref<string | number | null>(null)
const menuFor = ref<string | number | null>(null)
const busy = ref(false)

/* ── FLIP：数据变更前记录位置，变更后从旧位置滑到新位置 ── */
let snapshot: Map<string, DOMRect> | null = null
function capture() {
  snapshot = new Map()
  root.value?.querySelectorAll<HTMLElement>('[data-card]').forEach(el => snapshot!.set(el.dataset.card!, el.getBoundingClientRect()))
}
watch(() => props.columns, async () => {
  if (!snapshot || motionSkipped()) { snapshot = null; return }
  const before = snapshot
  snapshot = null
  await nextTick()
  root.value?.querySelectorAll<HTMLElement>('[data-card]').forEach((el) => {
    const old = before.get(el.dataset.card!)
    if (!old) return
    const now = el.getBoundingClientRect()
    const dx = old.left - now.left
    const dy = old.top - now.top
    if (Math.abs(dx) < 1 && Math.abs(dy) < 1) return
    gsap.fromTo(el, { x: dx, y: dy }, { x: 0, y: 0, duration: 0.5, ease: EASE.glide, clearProps: 'transform' })
  })
}, { deep: false })

/* ── 指针拖拽 ── */
interface DragState { id: number; item: T; from: BoardColumn<T>; card: HTMLElement; ghost: HTMLElement | null; sx: number; sy: number; ox: number; oy: number; origin: DOMRect }
let drag: DragState | null = null

function keyOf(item: T) {
  return String(item[props.itemKey])
}

function onPointerDown(event: PointerEvent, item: T, column: BoardColumn<T>) {
  if (!props.draggable || busy.value || event.button !== 0 || (event.target as Element).closest('button, a, input')) return
  const card = event.currentTarget as HTMLElement
  const rect = card.getBoundingClientRect()
  drag = { id: event.pointerId, item, from: column, card, ghost: null, sx: event.clientX, sy: event.clientY, ox: event.clientX - rect.left, oy: event.clientY - rect.top, origin: rect }
  window.addEventListener('pointermove', onPointerMove)
  window.addEventListener('pointerup', onPointerUp)
  window.addEventListener('pointercancel', onPointerUp)
}

function startGhost(state: DragState) {
  const ghost = state.card.cloneNode(true) as HTMLElement
  ghost.classList.add('board-ghost')
  ghost.removeAttribute('data-card')
  ghost.style.width = `${state.origin.width}px`
  document.body.appendChild(ghost)
  gsap.set(ghost, { x: state.origin.left, y: state.origin.top })
  if (!motionSkipped()) gsap.to(ghost, { rotate: -2.5, scale: 1.03, duration: 0.24, ease: EASE.glide })
  state.card.classList.add('is-placeholder')
  state.ghost = ghost
}

function columnAt(x: number, y: number) {
  const el = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-column]')
  if (!el || !root.value?.contains(el)) return null
  return props.columns.find(c => String(c.key) === el.dataset.column) ?? null
}

function onPointerMove(event: PointerEvent) {
  if (!drag || event.pointerId !== drag.id) return
  if (!drag.ghost) {
    if (Math.hypot(event.clientX - drag.sx, event.clientY - drag.sy) < 5) return
    startGhost(drag)
  }
  gsap.set(drag.ghost, { x: event.clientX - drag.ox, y: event.clientY - drag.oy })
  const col = columnAt(event.clientX, event.clientY)
  overKey.value = col && col.key !== drag.from.key && col.accepts !== false ? col.key : null
}

async function onPointerUp(event: PointerEvent) {
  if (!drag || event.pointerId !== drag.id) return
  window.removeEventListener('pointermove', onPointerMove)
  window.removeEventListener('pointerup', onPointerUp)
  window.removeEventListener('pointercancel', onPointerUp)
  const state = drag
  drag = null
  const target = overKey.value === null ? null : props.columns.find(c => c.key === overKey.value) ?? null
  overKey.value = null
  if (!state.ghost) return
  await settle(state, target)
}

async function settle(state: DragState, target: BoardColumn<T> | null) {
  const ghost = state.ghost!
  let moved = false
  if (target && props.onMove) {
    busy.value = true
    capture()
    try {
      moved = await props.onMove({ item: state.item, from: state.from, to: target })
    } finally {
      busy.value = false
    }
    if (!moved) snapshot = null
  }
  await nextTick()
  const landing = moved
    ? root.value?.querySelector<HTMLElement>(`[data-card="${CSS.escape(keyOf(state.item))}"]`)?.getBoundingClientRect() ?? state.origin
    : state.origin
  const finish = () => {
    ghost.remove()
    state.card.classList.remove('is-placeholder')
    root.value?.querySelector<HTMLElement>(`[data-card="${CSS.escape(keyOf(state.item))}"]`)?.classList.remove('is-placeholder')
  }
  if (motionSkipped()) return finish()
  /* 弹簧飞回：落位更硬、取消回弹稍软，速度与位移双收敛即收尾 */
  const st = {
    x: springOf(Number(gsap.getProperty(ghost, 'x')) || 0),
    y: springOf(Number(gsap.getProperty(ghost, 'y')) || 0),
    r: springOf(Number(gsap.getProperty(ghost, 'rotate')) || 0),
    s: springOf(Number(gsap.getProperty(ghost, 'scaleX')) || 1),
  }
  let elapsed = 0
  settleLoop?.stop()
  settleLoop = createFrameLoop((dt) => {
    elapsed += dt
    const k = moved ? 72 : 50
    spring(st.x, landing.left, dt, k)
    spring(st.y, landing.top, dt, k)
    spring(st.r, 0, dt, k)
    spring(st.s, 1, dt, k)
    gsap.set(ghost, { x: st.x.x, y: st.y.x, rotate: st.r.x, scale: st.s.x })
    const settled = Math.abs(st.x.x - landing.left) < 1.2 && Math.abs(st.y.x - landing.top) < 1.2
      && Math.abs(st.x.v) < 24 && Math.abs(st.y.v) < 24
    if (settled || elapsed > 0.9) { settleLoop?.stop(); finish() }
  })
  settleLoop.start(ghost)
}
let settleLoop: ReturnType<typeof createFrameLoop> | null = null

/* ── 键盘替代：移动到… ── */
async function moveVia(item: T, from: BoardColumn<T>, to: BoardColumn<T>) {
  menuFor.value = null
  if (!props.onMove || busy.value) return
  busy.value = true
  capture()
  try {
    if (!await props.onMove({ item, from, to })) snapshot = null
  } finally {
    busy.value = false
  }
}

function onDocClick(event: MouseEvent) {
  if (!(event.target as Element).closest('.board-menu, .board-menu-btn')) menuFor.value = null
}
watch(menuFor, (value) => {
  if (value === null) document.removeEventListener('click', onDocClick)
  else setTimeout(() => document.addEventListener('click', onDocClick))
})
onBeforeUnmount(() => {
  settleLoop?.stop()
  window.removeEventListener('pointermove', onPointerMove)
  window.removeEventListener('pointerup', onPointerUp)
  document.removeEventListener('click', onDocClick)
  drag?.ghost?.remove()
})
</script>

<template>
  <div ref="root" :class="['board', { 'is-draggable': draggable, 'is-busy': busy }]">
    <section
      v-for="(col, ci) in columns"
      :key="col.key"
      :data-column="col.key"
      :class="['board-col', `cat-${(ci % 5) + 1}`, { 'is-over': overKey === col.key }]"
      :aria-label="`${col.title}，${col.items.length} 项`"
    >
      <header class="board-col-head">
        <span class="board-dot" aria-hidden="true" />
        <span class="board-col-title">{{ col.title }}</span>
        <span class="board-col-count tnum">{{ col.items.length }}</span>
      </header>
      <div class="board-col-body">
        <article
          v-for="item in col.items"
          :key="keyOf(item)"
          :data-card="keyOf(item)"
          class="board-card"
          @pointerdown="onPointerDown($event, item, col)"
        >
          <slot name="card" :item="item" :column="col" />
          <div v-if="draggable && onMove" class="board-card-menu">
            <button
              type="button"
              class="board-menu-btn"
              aria-label="移动到其它列"
              :aria-expanded="menuFor === keyOf(item)"
              @click.stop="menuFor = menuFor === keyOf(item) ? null : keyOf(item)"
            ><Icon name="more" :size="14" /></button>
            <div v-if="menuFor === keyOf(item)" class="board-menu m-float" role="menu">
              <span class="eyebrow">移动到</span>
              <button
                v-for="to in columns.filter(c => c.key !== col.key && c.accepts !== false)"
                :key="to.key"
                type="button"
                role="menuitem"
                class="menu-item"
                @click="moveVia(item, col, to)"
              >{{ to.title }}</button>
            </div>
          </div>
        </article>
        <p v-if="!col.items.length" class="board-empty">{{ emptyText }}</p>
      </div>
    </section>
  </div>
</template>
