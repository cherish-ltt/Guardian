/* 参数值类型推断（纯函数）：决定设置面板里用开关、数字、分段选择还是文本输入 */
export type ConfigKind =
  | { kind: 'boolean' }
  | { kind: 'number'; decimals: number }
  | { kind: 'choice'; options: string[] }
  | { kind: 'secret' }
  | { kind: 'text' }

const SECRET = /password|passwd|secret|token|apikey|api_key|accesskey|privatekey|pwd/i

export function inferConfigKind(key: string, value: string, remark = ''): ConfigKind {
  const v = value.trim()
  if (/^(true|false)$/i.test(v)) return { kind: 'boolean' }
  if (SECRET.test(key)) return { kind: 'secret' }
  // 备注里写了「可选 a / b / c」时用分段选择
  const hint = remark.match(/可选[:：]?\s*([A-Za-z0-9_\-\s/|,、]+)/)
  if (hint) {
    const options = hint[1].split(/[\/|,、\s]+/).map(s => s.trim()).filter(Boolean)
    if (options.length >= 2 && options.length <= 6 && options.includes(v)) return { kind: 'choice', options }
  }
  if (/^-?\d+(\.\d+)?$/.test(v)) return { kind: 'number', decimals: v.includes('.') ? v.split('.')[1].length : 0 }
  return { kind: 'text' }
}

/** 「用户管理-账号初始密码」→ 分组「用户管理」+ 标题「账号初始密码」；无前缀时按键名第二段分组 */
export function splitConfigName(name: string, key: string) {
  const m = name.match(/^(.+?)[-－—·](.+)$/)
  if (m) return { group: m[1].trim(), title: m[2].trim() }
  const seg = key.split('.')
  return { group: seg.length > 2 ? seg[1] : '其他', title: name }
}
