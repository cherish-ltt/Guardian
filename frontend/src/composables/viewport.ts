import { reactive } from 'vue'

/* ============================================================
   画布视口：拖拽平移 + Ctrl / ⌘ + 滚轮以指针为中心缩放，
   组织画布与关系图共用。状态 {x, y, k} 由本组合函数维护，
   渲染路径由各组件在 apply 回调里自行决定（quickTo 动画或直接 transform）。
   ============================================================ */
export interface ViewState { x: number; y: number; k: number }

export interface ViewportPanOptions {
  /** 监听手势与滚轮的容器元素 */
  el: () => HTMLElement | null
  /** 视口变化后的渲染回调；animate=false 表示拖拽 / 滚轮直写 */
  apply?: (view: ViewState, animate: boolean) => void
  /** 命中该选择器的子元素时不启动平移（节点、控件、面板） */
  exclude?: string
  /** 普通滚轮是否平移（false 时仅 Ctrl / ⌘ + 滚轮缩放），默认 true */
  wheelPans?: boolean
  /** 是否启用平移缩放（如 RelationGraph 的 pannable），默认启用 */
  enabled?: () => boolean
  minK?: number
  maxK?: number
}

export function useViewportPan(options: ViewportPanOptions) {
  const view = reactive<ViewState>({ x: 0, y: 0, k: 1 })
  const minK = options.minK ?? 0.3
  const maxK = options.maxK ?? 2
  const render = (animate: boolean) => options.apply?.(view, animate)
  const enabled = () => (options.enabled ? options.enabled() : true)

  let pan: { id: number; x: number; y: number; vx: number; vy: number } | null = null

  function onPointerDown(event: PointerEvent) {
    if (!enabled() || event.button !== 0) return
    if (options.exclude && (event.target as Element).closest(options.exclude)) return
    pan = { id: event.pointerId, x: event.clientX, y: event.clientY, vx: view.x, vy: view.y }
    const el = options.el()
    el?.setPointerCapture(event.pointerId)
    el?.classList.add('is-panning')
  }

  function onPointerMove(event: PointerEvent) {
    if (!pan || pan.id !== event.pointerId) return
    view.x = pan.vx + event.clientX - pan.x
    view.y = pan.vy + event.clientY - pan.y
    render(false)
  }

  function onPointerUp(event: PointerEvent) {
    if (!pan || pan.id !== event.pointerId) return
    pan = null
    options.el()?.classList.remove('is-panning')
  }

  /** cx / cy 为容器内坐标；不传时以容器中心缩放 */
  function zoomAt(factor: number, cx?: number, cy?: number) {
    const box = options.el()?.getBoundingClientRect()
    if (!box) return
    const px = cx ?? box.width / 2
    const py = cy ?? box.height / 2
    const k = Math.min(maxK, Math.max(minK, view.k * factor))
    view.x = px - ((px - view.x) / view.k) * k
    view.y = py - ((py - view.y) / view.k) * k
    view.k = k
    render(true)
  }

  function onWheel(event: WheelEvent) {
    if (!enabled()) return
    const box = options.el()?.getBoundingClientRect()
    if (!box) return
    if (event.ctrlKey || event.metaKey) {
      event.preventDefault()
      zoomAt(Math.exp(-event.deltaY * 0.0022), event.clientX - box.left, event.clientY - box.top)
    } else if (options.wheelPans !== false) {
      event.preventDefault()
      view.x -= event.deltaX
      view.y -= event.deltaY
      render(false)
    }
  }

  function reset(animate = true) {
    view.x = 0
    view.y = 0
    view.k = 1
    render(animate)
  }

  function bind() {
    options.el()?.addEventListener('wheel', onWheel, { passive: false })
  }
  function unbind() {
    options.el()?.removeEventListener('wheel', onWheel)
  }

  return { view, zoomAt, reset, bind, unbind, onPointerDown, onPointerMove, onPointerUp }
}
