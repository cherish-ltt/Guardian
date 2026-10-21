import test from 'node:test'
import assert from 'node:assert/strict'
import { searchEnabled } from '../ui/select-shared.ts'

test('下拉搜索框按选项数自动开启，显式传值时以传入为准', () => {
  assert.equal(searchEnabled('auto', 7), false)
  assert.equal(searchEnabled('auto', 8), true)
  assert.equal(searchEnabled(true, 1), true)
  assert.equal(searchEnabled(false, 100), false)
})
