/**
 * 下拉浮层定位：面板 Teleport 到 body 后按触发器的视口矩形摆位，
 * 下方放不下且上方更宽裕时向上展开，左右夹在视口内。
 */
export function placePopover(trigger: HTMLElement | null, menu: HTMLElement | null) {
  const rect = trigger?.getBoundingClientRect()
  if (!rect) return null
  const menuHeight = menu?.offsetHeight ?? 260
  const flip = window.innerHeight - rect.bottom < menuHeight + 12 && rect.top > menuHeight + 12
  const width = Math.max(rect.width, 160)
  const left = Math.min(Math.max(rect.left, 8), Math.max(8, window.innerWidth - width - 8))
  const top = flip ? rect.top - menuHeight - 6 : rect.bottom + 6
  return { top, left, width }
}

/** 打开期间跟随滚动与窗口尺寸重新摆位；返回解绑函数 */
export function trackPopover(update: () => void) {
  const onScroll = (event: Event) => {
    const target = event.target as Element | null
    // 面板自身滚动不改触发器的位置
    if (target?.closest?.('.select-menu')) return
    update()
  }
  const onResize = () => update()
  document.addEventListener('scroll', onScroll, true)
  window.addEventListener('resize', onResize)
  return () => {
    document.removeEventListener('scroll', onScroll, true)
    window.removeEventListener('resize', onResize)
  }
}
