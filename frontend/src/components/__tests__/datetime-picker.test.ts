import test from 'node:test'
import assert from 'node:assert/strict'
import { renderComponent } from './support.ts'

test('日期时间控件声明可访问的选择弹层且空日期不默认填入当前时间', async () => {
  const html = await renderComponent('ui/DateTimePicker.vue', { modelValue: '', label: '结束时间' })
  assert.match(html, /role="combobox"/)
  assert.match(html, /aria-label="结束时间"/)
  assert.match(html, /aria-haspopup="dialog"/)
  assert.match(html, /aria-controls="[^"]+"/)
  assert.match(html, / value(?:=""|(?= ))/)
  assert.match(html, /placeholder="选择日期和时间"/)
})
