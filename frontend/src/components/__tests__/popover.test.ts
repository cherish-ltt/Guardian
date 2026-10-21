import test from 'node:test'
import assert from 'node:assert/strict'
import { placePopover } from '../../composables/popover.ts'

test('浮层默认贴在触发器下方，左右夹在视口内', () => {
  const viewport = { innerHeight: 800, innerWidth: 1200 }
  const anchor = (rect: { top: number; bottom: number; left: number; width: number }) =>
    ({ getBoundingClientRect: () => rect }) as unknown as HTMLElement
  const menu = { offsetHeight: 200 } as unknown as HTMLElement
  const previous = globalThis.window
  globalThis.window = viewport as unknown as Window & typeof globalThis

  try {
    assert.deepEqual(
      placePopover(anchor({ top: 100, bottom: 132, left: 40, width: 200 }), menu),
      { top: 138, left: 40, width: 200 },
    )
    // 贴右边缘时左移，不越出视口
    assert.equal(placePopover(anchor({ top: 100, bottom: 132, left: 1150, width: 200 }), menu)?.left, 992)
    // 下方放不下、上方够高时向上展开
    assert.equal(placePopover(anchor({ top: 700, bottom: 732, left: 40, width: 200 }), menu)?.top, 494)
    assert.equal(placePopover(null, menu), null)
  } finally {
    globalThis.window = previous
  }
})
