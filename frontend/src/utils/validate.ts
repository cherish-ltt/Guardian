/* 表单校验：规则返回错误文案，返回空表示通过。 */
export type Rule = (value: unknown) => string | undefined

export type Rules<T> = Partial<Record<keyof T, Rule[]>>

const isEmpty = (value: unknown) => value === undefined || value === null || String(value).trim() === ''

export const required = (message: string): Rule => value => (isEmpty(value) ? message : undefined)

export const pattern = (regexp: RegExp, message: string): Rule =>
  value => (isEmpty(value) || regexp.test(String(value)) ? undefined : message)

export const maxLength = (max: number, message?: string): Rule =>
  value => (isEmpty(value) || String(value).length <= max ? undefined : message ?? `不能超过 ${max} 个字符`)

export const phone = pattern(/^[\d-]{7,20}$/, '手机号格式不正确')
export const email = pattern(/^\S+@\S+\.\S+$/, '邮箱格式不正确')

/** 返回 { 字段: 错误文案 }，空对象表示全部通过 */
export function validate<T extends Record<string, unknown>>(form: T, rules: Rules<T>) {
  const errors: Partial<Record<keyof T, string>> = {}
  for (const key of Object.keys(rules) as (keyof T)[]) {
    for (const rule of rules[key] ?? []) {
      const message = rule(form[key])
      if (message) {
        errors[key] = message
        break
      }
    }
  }
  return errors
}
