import test from 'node:test'
import assert from 'node:assert/strict'
import { copyText } from '../clipboard.ts'

test('copyText writes text through navigator clipboard', async () => {
  const copied: string[] = []
  const originalNavigator = globalThis.navigator
  Object.defineProperty(globalThis, 'navigator', {
    configurable: true,
    value: { clipboard: { writeText: async (value: string) => { copied.push(value) } } },
  })

  try {
    assert.equal(await copyText('hello'), true)
    assert.deepEqual(copied, ['hello'])
  } finally {
    Object.defineProperty(globalThis, 'navigator', { configurable: true, value: originalNavigator })
  }
})

test('copyText returns false when clipboard api is unavailable', async () => {
  const originalNavigator = globalThis.navigator
  Object.defineProperty(globalThis, 'navigator', { configurable: true, value: {} })

  try {
    assert.equal(await copyText('hello'), false)
  } finally {
    Object.defineProperty(globalThis, 'navigator', { configurable: true, value: originalNavigator })
  }
})
