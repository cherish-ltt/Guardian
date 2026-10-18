import test from 'node:test'
import assert from 'node:assert/strict'
import { renderComponent } from './support.ts'

test('Tag 把历史色值归档到四种语义色', async () => {
  const cases = [
    ['success', 'tone-ok'],
    ['primary', 'tone-info'],
    ['purple', 'tone-info'],
    ['error', 'tone-danger'],
    ['warning', 'tone-danger'],
    ['default', 'tone-neutral'],
  ]

  for (const [tone, expected] of cases) {
    const html = await renderComponent('ui/Tag.vue', { tone }, { default: () => '文本' })
    assert.match(html, new RegExp(`class="tag ${expected}"`), `tone=${tone} 应归档为 ${expected}`)
  }
})

test('Tag 未传 tone 时落到中性色，不渲染圆点', async () => {
  const html = await renderComponent('ui/Tag.vue', {}, { default: () => '文本' })

  assert.match(html, /class="tag tone-neutral"/)
  assert.doesNotMatch(html, /tag-dot/)
})

test('DeltaPill 按符号与 fresh 给出四种方向', async () => {
  const up = await renderComponent('ui/DeltaPill.vue', { value: 12.5 })
  assert.match(up, /is-up/)
  assert.match(up, /\+12\.5%/)

  const down = await renderComponent('ui/DeltaPill.vue', { value: -8 })
  assert.match(down, /is-down/)
  assert.match(down, /−8%/)

  const flat = await renderComponent('ui/DeltaPill.vue', { value: 0 })
  assert.match(flat, /is-flat/)
  assert.match(flat, /±0%/)

  const fresh = await renderComponent('ui/DeltaPill.vue', { value: null, fresh: true })
  assert.match(fresh, /is-new/)
  assert.match(fresh, /新增/)
})

test('DeltaPill 显式 up 覆盖数值正负', async () => {
  // 业务上「成本下降」是好事，调用方用 up 显式指定方向，不能按数值正负推断。
  const html = await renderComponent('ui/DeltaPill.vue', { value: -5, up: true })

  assert.match(html, /is-up/)
  assert.match(html, /\+5%/)
})

test('Panel 只有传了 title 或插槽时才渲染头部', async () => {
  const bare = await renderComponent('ui/Panel.vue', {}, { default: () => '内容' })
  assert.doesNotMatch(bare, /panel-head/)

  const titled = await renderComponent('ui/Panel.vue', { title: '统计', meta: '近 7 天' })
  assert.match(titled, /panel-head/)
  assert.match(titled, /统计/)
  assert.match(titled, /近 7 天/)

  const withTools = await renderComponent(
    'ui/Panel.vue',
    {},
    { tools: () => '工具', default: () => '内容' },
  )
  assert.match(withTools, /panel-head/)
  assert.match(withTools, /panel-tools/)
})

test('PageHeader 的说明与右侧区域都是可选的', async () => {
  const plain = await renderComponent('ui/PageHeader.vue', { title: '用户管理' })
  assert.match(plain, /用户管理/)
  assert.doesNotMatch(plain, /page-desc/)
  assert.doesNotMatch(plain, /page-head-side/)

  const full = await renderComponent(
    'ui/PageHeader.vue',
    { title: '用户管理', desc: '维护账号与角色' },
    { spec: () => '铭牌', default: () => '主操作' },
  )
  assert.match(full, /维护账号与角色/)
  assert.match(full, /page-head-side/)
  assert.match(full, /铭牌/)
  assert.match(full, /主操作/)
})

test('SectionLabel 按 as 切换标题层级，无 title 时不渲染标题', async () => {
  const h2 = await renderComponent('ui/SectionLabel.vue', { title: '分组' })
  assert.match(h2, /<h2 class="section-mark-title">分组<\/h2>/)

  const h3 = await renderComponent('ui/SectionLabel.vue', { title: '分组', as: 'h3' })
  assert.match(h3, /<h3 class="section-mark-title">分组<\/h3>/)

  const empty = await renderComponent('ui/SectionLabel.vue', {})
  assert.doesNotMatch(empty, /section-mark-title/)
})
