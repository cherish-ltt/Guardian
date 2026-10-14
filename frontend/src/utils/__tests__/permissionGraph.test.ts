import test from 'node:test'
import assert from 'node:assert/strict'
import { holdersOfMenu, indexPermissionGraph, menuPath, menusOfUser, permissionFindings } from '../permissionGraph.ts'
import { matchIntent, permissionFromQuestion } from '../askMatch.ts'

const graph = {
  usersVisible: true,
  users: [
    { id: 1, username: 'admin', nickname: '管理员', deptName: '总部', status: 1 },
    { id: 2, username: 'li', nickname: '李四', deptName: '研发部', status: 1 },
    { id: 3, username: 'wang', nickname: '王五', deptName: null, status: 1 },
  ],
  roles: [
    { id: 1, name: '超级管理员', code: 'super_admin', dataScope: '全部数据权限', status: 1, builtin: true, userCount: 1, menuIds: [] },
    { id: 2, name: '系统管理员', code: 'sys_admin', dataScope: '本部门', status: 1, builtin: false, userCount: 1, menuIds: [10, 11, 12] },
    { id: 3, name: '访客', code: 'guest', dataScope: '仅本人', status: 0, builtin: false, userCount: 0, menuIds: [10] },
  ],
  menus: [
    { id: 10, parentId: 0, name: '系统管理', type: 'dir', icon: null, path: '/system', permission: null, sort: 1, visible: 1, status: 1 },
    { id: 11, parentId: 10, name: '用户管理', type: 'menu', icon: null, path: 'user', permission: 'system:user:list', sort: 1, visible: 1, status: 1 },
    { id: 12, parentId: 11, name: '删除用户', type: 'btn', icon: null, path: null, permission: 'system:user:remove', sort: 1, visible: 1, status: 1 },
    { id: 13, parentId: 10, name: '参数设置', type: 'menu', icon: null, path: 'config', permission: 'system:config:list', sort: 2, visible: 1, status: 1 },
    { id: 14, parentId: 13, name: '新增参数', type: 'btn', icon: null, path: null, permission: null, sort: 1, visible: 1, status: 1 },
  ],
  userRoles: [[1, 1], [2, 2]],
} as const

const index = indexPermissionGraph(graph as never)

test('menusOfUser unions active roles and gives super admin every menu', () => {
  assert.equal(menusOfUser(index, 1).size, 5)
  assert.deepEqual([...menusOfUser(index, 2)].sort(), [10, 11, 12])
  assert.equal(menusOfUser(index, 3).size, 0)
})

test('holdersOfMenu and menuPath trace a button back to roles, users and its page', () => {
  const { roles, users } = holdersOfMenu(index, 12)
  assert.deepEqual(roles.map(r => r.code).sort(), ['super_admin', 'sys_admin'])
  assert.deepEqual(users.map(u => u.id).sort(), [1, 2])
  assert.deepEqual(menuPath(index, 12).map(m => m.name), ['系统管理', '用户管理', '删除用户'])
})

test('permissionFindings reports unused roles, users without role, orphan menus and buttons without code', () => {
  const f = permissionFindings(index)
  assert.deepEqual(f.unusedRoles.map(r => r.code), ['guest'])
  assert.deepEqual(f.usersWithoutRole.map(u => u.id), [3])
  assert.deepEqual(f.orphanMenus.map(m => m.id), [13, 14])
  assert.deepEqual(f.buttonsWithoutCode.map(m => m.id), [14])
})

test('ask matching resolves intents and permission codes from Chinese questions', () => {
  const intents = [
    { id: 'perm.who', patterns: [/谁(能|可以)/] },
    { id: 'perm.unused', patterns: [/没人用.*角色|角色.*没人用/] },
  ]
  assert.equal(matchIntent('谁能删除用户？', intents), 'perm.who')
  assert.equal(matchIntent('哪些角色没人用', intents), 'perm.unused')
  assert.equal(matchIntent('今天天气', intents), null)
  assert.equal(permissionFromQuestion('谁能删除用户？'), 'system:user:remove')
  assert.equal(permissionFromQuestion('谁可以导出操作日志'), 'monitor:operlog:list')
  assert.equal(permissionFromQuestion('谁有 system:dict:edit'), 'system:dict:edit')
  assert.equal(permissionFromQuestion('谁能看'), null)
})

test('buildHeatGrid fills 7x24, levels by quartile and flags risky cells', async () => {
  const { buildHeatGrid } = await import('../heatmap.ts')
  const { grid, peak, total } = buildHeatGrid([
    { dow: 0, hour: 9, total: 20, failed: 1 },
    { dow: 0, hour: 10, total: 5, failed: 3 },
    { dow: 6, hour: 23, total: 1, failed: 0 },
    { dow: 9, hour: 1, total: 99, failed: 0 },
  ])
  assert.equal(grid.length, 7)
  assert.equal(grid[0].length, 24)
  assert.equal(total, 26)
  assert.equal(peak?.hour, 9)
  assert.equal(grid[0][9].level, 4)
  assert.equal(grid[3][3].level, 0)
  assert.equal(grid[0][10].risky, true)
  assert.equal(grid[0][9].risky, false)
})

test('inferConfigKind and splitConfigName pick controls and groups for settings', async () => {
  const { inferConfigKind, splitConfigName } = await import('../configValue.ts')
  assert.deepEqual(inferConfigKind('sys.account.captchaEnabled', 'true'), { kind: 'boolean' })
  assert.deepEqual(inferConfigKind('sys.user.initPassword', 'Yy@123456'), { kind: 'secret' })
  assert.deepEqual(inferConfigKind('sys.index.appearance', 'light', '可选 light / dark / auto'), { kind: 'choice', options: ['light', 'dark', 'auto'] })
  assert.deepEqual(inferConfigKind('sys.login.alertThreshold', '5'), { kind: 'number', decimals: 0 })
  assert.deepEqual(inferConfigKind('sys.site.name', '后台'), { kind: 'text' })
  assert.deepEqual(splitConfigName('用户管理-账号初始密码', 'sys.user.initPassword'), { group: '用户管理', title: '账号初始密码' })
  assert.deepEqual(splitConfigName('站点名称', 'sys.site.name'), { group: 'site', title: '站点名称' })
})
