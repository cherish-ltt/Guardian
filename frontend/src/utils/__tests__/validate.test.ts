import test from 'node:test'
import assert from 'node:assert/strict'
import { email, maxLength, phone, required, validate } from '../validate.ts'

test('validate returns the first failing message per field', () => {
  const errors = validate(
    { name: '', phone: 'abc', email: 'a@example.com', code: 'x'.repeat(5) },
    {
      name: [required('请输入名称')],
      phone: [phone],
      email: [email],
      code: [required('请输入编码'), maxLength(4)],
    },
  )
  assert.deepEqual(errors, { name: '请输入名称', phone: '手机号格式不正确', code: '不能超过 4 个字符' })
})

test('optional format rules skip empty values', () => {
  assert.deepEqual(validate({ phone: '', email: '  ' }, { phone: [phone], email: [email] }), {})
})
