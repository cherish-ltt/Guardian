import type { RouteLocationRaw } from 'vue-router'
import { systemApi } from '@/api'
import { pathOf } from '@/composables/auth'
import { hasPermission } from '@/directives/permission'
import { loadPermissionGraph } from '@/composables/permissionGraph'
import { matchIntent, permissionFromQuestion } from '@/utils/askMatch'
import { inferConfigKind } from '@/utils/configValue'
import { findMenusByPermission, holdersOfMenu, menuPath, menusOfUser, permissionFindings } from '@/utils/permissionGraph'

/* ============================================================
   「问系统」：本地规则问答，不调用 AI。各模块注册意图（问题模板 + 正则 + 所需权限 + 取数），
   命令面板与溯源视图的问答坞共用。答案是卡片：语气色条 + 结论 + 可跳转芯片。
   ============================================================ */
export interface AnswerChip { key: string; label: string; sub?: string; to?: RouteLocationRaw; tone?: 'danger' | 'off' }
export interface AnswerCard { tone: 'info' | 'warn' | 'ok' | 'danger'; title: string; text: string; chips?: AnswerChip[] }
export interface AskIntent {
  id: string
  /** 预设问题（展示为芯片） */
  question: string
  patterns: RegExp[]
  permission?: string
  run: (question: string) => Promise<AnswerCard[]>
}

const intents: AskIntent[] = []
export function registerIntent(intent: AskIntent) {
  if (!intents.some(i => i.id === intent.id)) intents.push(intent)
}

const available = () => intents.filter(i => !i.permission || hasPermission(i.permission))

export function presetQuestions(group?: string) {
  return available().filter(i => !group || i.id.startsWith(group)).map(i => i.question)
}

export async function ask(question: string): Promise<AnswerCard[]> {
  const list = available()
  const id = matchIntent(question, list)
  const intent = list.find(i => i.id === id)
  if (intent) return intent.run(question)
  return [{ tone: 'warn', title: '没有识别这个问题', text: `可以试试：${list.slice(0, 4).map(i => i.question).join('、') || '暂无可用问题'}` }]
}

/* ── 权限类意图 ───────────────────────────────────────────── */
const tracePath = () => pathOf('system/role/index')
const traceTo = (kind: 'user' | 'role' | 'menu', id: number): RouteLocationRaw | undefined => {
  const path = tracePath()
  return path ? { path, query: { trace: `${kind}:${id}` } } : undefined
}

registerIntent({
  id: 'perm.who',
  question: '谁能删除用户？',
  patterns: [/谁(能|可以|有权)/, /哪些(人|角色)(能|可以)/, /[a-z]+:[a-z]+:[a-zA-Z]+/],
  permission: 'system:role:list',
  async run(question) {
    const index = await loadPermissionGraph()
    const code = permissionFromQuestion(question)
    const menus = code ? findMenusByPermission(index, code) : []
    if (!menus.length) return [{ tone: 'warn', title: '没有找到对应权限', text: code ? `系统里没有权限标识「${code}」。` : '请说明动作和对象，例如「谁能导出操作日志」。' }]
    return menus.slice(0, 3).map((menu) => {
      const { roles, users } = holdersOfMenu(index, menu.id)
      const path = menuPath(index, menu.id).map(m => m.name).join(' / ')
      return {
        tone: users.length > 5 ? 'warn' : 'info',
        title: `${menu.name}${menu.permission ? ` · ${menu.permission}` : ''}`,
        text: `${path}。${roles.length} 个角色授予，${index.graph.usersVisible ? `覆盖 ${users.length} 名用户` : '无用户查询权限，未统计人数'}。`,
        chips: [
          ...roles.map(r => ({ key: `r${r.id}`, label: r.name, sub: r.code, to: traceTo('role', r.id) })),
          ...users.slice(0, 8).map(u => ({ key: `u${u.id}`, label: u.nickname, sub: u.deptName ?? '', to: traceTo('user', u.id) })),
        ],
      } satisfies AnswerCard
    })
  },
})

registerIntent({
  id: 'perm.unused',
  question: '哪些角色没人用？',
  patterns: [/(没人|无人|闲置|未使用|没有用户).*角色/, /角色.*(没人|无人|闲置|未使用)/],
  permission: 'system:role:list',
  async run() {
    const index = await loadPermissionGraph()
    const { unusedRoles } = permissionFindings(index)
    return [{
      tone: unusedRoles.length ? 'warn' : 'ok',
      title: '闲置角色',
      text: unusedRoles.length ? `${unusedRoles.length} 个角色没有分配给任何用户，可考虑停用或合并。` : '每个角色都至少分配给了一名用户。',
      chips: unusedRoles.map(r => ({ key: `r${r.id}`, label: r.name, sub: r.status === 1 ? r.code : '已停用', tone: r.status === 1 ? undefined : 'off', to: traceTo('role', r.id) })),
    }]
  },
})

registerIntent({
  id: 'perm.noRole',
  question: '哪些用户还没分配角色？',
  patterns: [/(没有|没|未|无)(分配)?角色/, /没.*角色.*用户/],
  permission: 'system:user:list',
  async run() {
    const index = await loadPermissionGraph()
    const { usersWithoutRole } = permissionFindings(index)
    return [{
      tone: usersWithoutRole.length ? 'warn' : 'ok',
      title: '未分配角色的用户',
      text: usersWithoutRole.length ? `${usersWithoutRole.length} 名用户登录后看不到任何菜单。` : '所有可见用户都已分配角色。',
      chips: usersWithoutRole.slice(0, 16).map(u => ({ key: `u${u.id}`, label: u.nickname, sub: u.deptName ?? '', to: traceTo('user', u.id) })),
    }]
  },
})

registerIntent({
  id: 'perm.orphan',
  question: '哪些菜单没有授予任何角色？',
  patterns: [/孤立/, /(没有|未)(授予|授权).*菜单/, /菜单.*(没有|未)(授予|授权)/, /缺.*权限码|权限码.*缺/],
  permission: 'system:role:list',
  async run() {
    const index = await loadPermissionGraph()
    const { orphanMenus, buttonsWithoutCode } = permissionFindings(index)
    const cards: AnswerCard[] = [{
      tone: orphanMenus.length ? 'warn' : 'ok',
      title: '孤立菜单',
      text: orphanMenus.length ? `${orphanMenus.length} 个启用菜单只有超级管理员能访问。` : '所有启用菜单都已授予至少一个角色。',
      chips: orphanMenus.slice(0, 16).map(m => ({ key: `m${m.id}`, label: m.name, sub: m.permission ?? m.type, to: traceTo('menu', m.id) })),
    }]
    if (buttonsWithoutCode.length) cards.push({ tone: 'danger', title: '缺少权限标识的按钮', text: `${buttonsWithoutCode.length} 个按钮没有权限标识，前后端都无法校验。`, chips: buttonsWithoutCode.map(m => ({ key: `b${m.id}`, label: m.name, tone: 'danger', to: traceTo('menu', m.id) })) })
    return cards
  },
})

registerIntent({
  id: 'perm.overlap',
  question: '哪些用户叠加了多个角色？',
  patterns: [/叠加|冗余|多个角色|多角色/],
  permission: 'system:user:list',
  async run() {
    const index = await loadPermissionGraph()
    const { overlapUsers } = permissionFindings(index)
    return [{
      tone: overlapUsers.length ? 'warn' : 'ok',
      title: '多角色叠加',
      text: overlapUsers.length ? `${overlapUsers.length} 名用户同时持有 3 个及以上启用角色，权限可能超出岗位需要。` : '没有用户叠加 3 个及以上角色。',
      chips: overlapUsers.map(({ user, roles }) => ({ key: `u${user.id}`, label: user.nickname, sub: `${roles.length} 个角色`, to: traceTo('user', user.id) })),
    }]
  },
})

registerIntent({
  id: 'perm.can',
  question: '管理员能访问哪些菜单？',
  patterns: [/(.+?)(能|可以)(访问|看到|做|操作)(哪些|什么)/],
  permission: 'system:user:list',
  async run(question) {
    const index = await loadPermissionGraph()
    const name = question.match(/(.+?)(能|可以)(访问|看到|做|操作)/)?.[1]?.trim() ?? ''
    const user = index.graph.users.find(u => u.nickname === name || u.username === name)
      ?? index.graph.users.find(u => name && (u.nickname.includes(name) || u.username.includes(name)))
    if (!user) return [{ tone: 'warn', title: '没有找到这个用户', text: `「${name}」不在你可见的用户范围内。` }]
    const menus = [...menusOfUser(index, user.id)].map(id => index.menu.get(id)!).filter(Boolean)
    const pages = menus.filter(m => m.type === 'menu')
    const buttons = menus.filter(m => m.type === 'btn')
    return [{
      tone: 'info',
      title: `${user.nickname} 的可用权限`,
      text: `可访问 ${pages.length} 个页面、${buttons.length} 个操作按钮。`,
      chips: [
        { key: `u${user.id}`, label: '查看溯源图', sub: user.username, to: traceTo('user', user.id) },
        ...pages.map(m => ({ key: `m${m.id}`, label: m.name, sub: m.permission ?? '', to: traceTo('menu', m.id) })),
      ],
    }]
  },
})

/* ── 登录安全类意图 ───────────────────────────────────────── */
registerIntent({
  id: 'login.anomaly',
  question: '最近有异常登录吗？',
  patterns: [/异常登录|登录异常|暴力|撞库|失败.*登录|登录.*失败|可疑/],
  permission: 'monitor:loginlog:list',
  async run(question) {
    const days = /月|30/.test(question) ? 30 : 7
    const heat = await systemApi.loginLogs.heatmap(days)
    const path = pathOf('monitor/loginlog/index')
    const failed = heat.cells.reduce((a, c) => a + c.failed, 0)
    const total = heat.cells.reduce((a, c) => a + c.total, 0)
    const cards: AnswerCard[] = [{
      tone: heat.anomalies.length ? 'danger' : 'ok',
      title: heat.anomalies.length ? `发现 ${heat.anomalies.length} 处失败聚集` : '没有异常聚集',
      text: `近 ${heat.days} 天登录 ${total} 次，失败 ${failed} 次；同一 IP 或账号失败 ≥ ${heat.threshold} 次计为异常。`,
      chips: heat.anomalies.map(a => ({
        key: `${a.kind}-${a.key}`,
        label: a.key,
        sub: `${a.kind === 'ip' ? 'IP' : '账号'} · ${a.failed} 次失败`,
        tone: 'danger' as const,
        to: path ? { path, query: { keyword: a.key, status: '0' } } : undefined,
      })),
    }]
    return cards
  },
})

/* ── 参数布尔 / 敏感值意图（先于通用检索注册，避免被兜底吞掉）── */
const configChip = (row: Record<string, unknown>, sub?: string) => {
  const path = pathOf('system/config/index')
  return path ? [{ key: `c${row.id}`, label: '去参数配置', sub: sub ?? String(row.key), to: { path, query: { keyword: String(row.key) } } }] : []
}
const configRows = async () => (await systemApi.configs.list({ pageNum: 1, pageSize: 100 })).rows as Array<Record<string, unknown>>

registerIntent({
  id: 'config.boolean',
  question: '当前登录验证码是否开启？',
  patterns: [/是否(开启|启用|关闭|开着)|开没开|有没有开|开关|布尔/],
  permission: 'system:config:list',
  async run(question) {
    const rows = await configRows()
    const bools = rows.filter(r => inferConfigKind(String(r.key), String(r.value ?? ''), String(r.remark ?? '')).kind === 'boolean')
    // 「哪些参数是开关」→ 列出全部布尔参数
    if (/哪些|所有|全部|几个|多少/.test(question) && /开关|布尔|是否/.test(question)) {
      return [{
        tone: bools.length ? 'info' as const : 'warn' as const,
        title: `开关类参数 ${bools.length} 个`,
        text: bools.length ? `当前开启 ${bools.filter(r => /^true$/i.test(String(r.value))).length} 个，关闭 ${bools.filter(r => !/^true$/i.test(String(r.value))).length} 个。` : '系统里没有布尔类型的参数。',
        chips: bools.slice(0, 12).flatMap(r => configChip(r, `${r.key} = ${r.value}`)),
      }]
    }
    // 「X 是否开启」→ 去掉问句骨架后按 2–4 字滑窗匹配名称 / 键名 / 备注
    const term = question.replace(/[？?。,，!！\s]/g, '').replace(/是否|有没有|开没开|开启|启用|关闭|开着|当前|吗|呢|的|了|着|在|被|系统/g, '')
    const grams: string[] = []
    for (let len = Math.min(4, term.length); len >= 2; len--)
      for (let i = 0; i + len <= term.length; i++) grams.push(term.slice(i, i + len))
    const matched = rows
      .map((row) => {
        const text = `${row.name} ${row.key} ${row.remark ?? ''}`.toLowerCase()
        const hits = grams.filter(g => text.includes(g.toLowerCase()))
        return { row, hits: hits.length, longest: hits.length ? Math.max(...hits.map(h => h.length)) : 0 }
      })
      .filter(s => s.hits > 0)
      .sort((a, b) => b.longest - a.longest || b.hits - a.hits)
    if (!matched.length) {
      return [{ tone: 'warn' as const, title: '没有匹配的参数', text: `在 ${rows.length} 个参数里没找到与「${term || question}」相关的项，可以试试参数的名称或键名片段。` }]
    }
    return matched.slice(0, 3).map(({ row }) => {
      const on = /^true$/i.test(String(row.value))
      return {
        tone: (on ? 'ok' : 'info') as 'ok' | 'info',
        title: `${row.name} · 当前${on ? '开启' : '关闭'}`,
        text: `${row.key} = ${row.value}${row.remark ? `；${row.remark}` : ''}${row.builtin ? '（内置参数，键名不可改）' : ''}`,
        chips: configChip(row),
      }
    })
  },
})

registerIntent({
  id: 'config.sensitive',
  question: '参数配置里有哪些敏感值？',
  patterns: [/敏感|密钥|秘密|secret/i],
  permission: 'system:config:list',
  async run() {
    const rows = await configRows()
    const secrets = rows.filter(r => inferConfigKind(String(r.key), String(r.value ?? ''), String(r.remark ?? '')).kind === 'secret')
    const mask = (v: string) => v.length <= 2 ? '••' : `${v.slice(0, 2)}${'•'.repeat(Math.min(6, v.length - 2))}${v.slice(-1)}`
    return [{
      tone: secrets.length ? 'warn' as const : 'ok' as const,
      title: `敏感参数 ${secrets.length} 个`,
      text: secrets.length ? secrets.slice(0, 8).map(r => `${r.name}（${r.key} = ${mask(String(r.value ?? ''))}）`).join('；') + '。这些值按密钥类参数渲染为密文输入，注意保存后不再明文展示。' : '系统没有密钥类参数。',
      chips: secrets.slice(0, 8).flatMap(r => configChip(r)),
    }]
  },
})

/* ── 参数类意图 ───────────────────────────────────────────── */
registerIntent({
  id: 'config.find',
  question: '哪些参数管登录安全？',
  patterns: [/参数|配置|设置|开关/],
  permission: 'system:config:list',
  async run(question) {
    const page = await systemApi.configs.list({ pageNum: 1, pageSize: 100 })
    const rows = page.rows as Array<Record<string, unknown>>
    const path = pathOf('system/config/index')
    const to = (row: Record<string, unknown>) => (path ? { path, query: { keyword: String(row.key ?? '') } } : undefined)
    const terms = (question.match(/[a-zA-Z][\w.]*|[一-龥]{2,}/g) ?? [])
      .filter(w => !/参数|配置|设置|开关|哪些|什么|管理|系统|怎么|怎样/.test(w))
    const scored = rows
      .map((row) => {
        const text = `${row.name} ${row.key} ${row.remark ?? ''}`.toLowerCase()
        return { row, hits: terms.filter(t => text.includes(t.toLowerCase())) }
      })
      .filter(s => s.hits.length)
      .sort((a, b) => b.hits.length - a.hits.length)
    if (!scored.length) {
      return [{
        tone: 'info',
        title: `共 ${page.total} 个参数`,
        text: '没有找到直接匹配问题关键词的参数，试试「验证码」「密码」「阈值」等词，或去参数配置按名称检索。',
        chips: path ? [{ key: 'go', label: '去参数配置', to: { path } }] : [],
      }]
    }
    return scored.slice(0, 3).map(({ row }) => ({
      tone: 'info' as const,
      title: String(row.name ?? row.key),
      text: `${row.key} = ${row.value}${row.remark ? `；${row.remark}` : ''}${row.builtin ? '（内置参数，键名不可改）' : ''}`,
      chips: to(row) ? [{ key: `c${row.id}`, label: '去参数配置', sub: String(row.key), to: to(row) }] : [],
    }))
  },
})
