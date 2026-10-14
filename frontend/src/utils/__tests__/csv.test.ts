import test from 'node:test'
import assert from 'node:assert/strict'
import { toCsv } from '../csv.ts'

test('toCsv serializes headers, rows and escaped cell values', () => {
  const csv = toCsv(
    [
      { title: '名称', key: 'name' },
      { title: '备注', key: 'remark' },
      { title: '状态', value: row => row.status === 1 ? '正常' : '停用' },
      { title: '空值', key: 'missing' },
    ],
    [
      { name: '研发,平台', remark: '包含"引号"', status: 1 },
      { name: '测试质量部', remark: '跨行\n说明', status: 0 },
    ]
  )

  assert.equal(csv, [
    '名称,备注,状态,空值',
    '"研发,平台","包含""引号""",正常,',
    '测试质量部,"跨行\n说明",停用,',
  ].join('\n'))
})
