/* 浮层焦点管理：打开时把焦点交给浮层，Tab 在浮层内循环，关闭后还给触发者。 */
const FOCUSABLE = 'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
interface FocusScope { container: HTMLElement; restoreTo: HTMLElement | null }
const scopes: FocusScope[] = []

function updateLayers() {
  const base = Number.parseInt(getComputedStyle(document.documentElement).getPropertyValue('--layer-overlay')) || 200
  scopes.forEach((scope, index) => {
    const overlay = scope.container.closest<HTMLElement>('.overlay')
    if (overlay) overlay.style.zIndex = String(base + index * 20)
    scope.container.setAttribute('aria-modal', String(index === scopes.length - 1))
  })
}

/** 下拉框可以 Teleport 到 body，层级仍跟随其所在浮层。 */
export function floatingLayerFor(anchor: HTMLElement | null) {
  const overlay = anchor?.closest<HTMLElement>('.overlay')
  return overlay ? Number(overlay.style.zIndex || getComputedStyle(overlay).zIndex) + 10 : 120
}

export function createFocusTrap() {
  let scope: FocusScope | null = null

  const isTop = () => Boolean(scope && scopes[scopes.length - 1] === scope)

  function onKey(event: KeyboardEvent) {
    if (event.key !== 'Tab' || !scope || !isTop()) return
    const container = scope.container
    const items = Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(el => el.offsetParent !== null)
    if (!items.length) return
    const first = items[0]
    const last = items[items.length - 1]
    if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus() }
    else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus() }
  }

  return {
    isTop,
    activate(el: HTMLElement | null) {
      if (!el || scope) return
      scope = { container: el, restoreTo: document.activeElement as HTMLElement | null }
      scopes.push(scope)
      updateLayers()
      const autofocus = el.querySelector<HTMLElement>('[autofocus], .modal-body input, .drawer-body input, .modal-body textarea') ?? el.querySelector<HTMLElement>(FOCUSABLE)
      autofocus?.focus({ preventScroll: true })
      document.addEventListener('keydown', onKey)
    },
    deactivate() {
      document.removeEventListener('keydown', onKey)
      if (!scope) return
      const top = isTop()
      const restoreTo = scope.restoreTo
      scopes.splice(scopes.indexOf(scope), 1)
      scope = null
      updateLayers()
      if (top && restoreTo && document.contains(restoreTo)) restoreTo.focus({ preventScroll: true })
    },
  }
}
