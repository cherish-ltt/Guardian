import type { PermissionGraph } from '../api/system.ts'

/* ============================================================
   权限关系图的索引与洞察（纯函数）：溯源视图、菜单星图、问答规则共用。
   ============================================================ */
export type GraphRole = PermissionGraph['roles'][number]
export type GraphMenu = PermissionGraph['menus'][number]
export type GraphUser = PermissionGraph['users'][number]

export interface PermissionIndex {
  graph: PermissionGraph
  user: Map<number, GraphUser>
  role: Map<number, GraphRole>
  menu: Map<number, GraphMenu>
  rolesOfUser: Map<number, number[]>
  usersOfRole: Map<number, number[]>
  rolesOfMenu: Map<number, number[]>
  childrenOf: Map<number, GraphMenu[]>
}

function push<K, V>(map: Map<K, V[]>, key: K, value: V) {
  const list = map.get(key)
  if (list) list.push(value)
  else map.set(key, [value])
}

export function indexPermissionGraph(graph: PermissionGraph): PermissionIndex {
  const rolesOfUser = new Map<number, number[]>()
  const usersOfRole = new Map<number, number[]>()
  for (const [userId, roleId] of graph.userRoles) {
    push(rolesOfUser, userId, roleId)
    push(usersOfRole, roleId, userId)
  }
  const rolesOfMenu = new Map<number, number[]>()
  for (const role of graph.roles) for (const menuId of role.menuIds ?? []) push(rolesOfMenu, menuId, role.id)
  const childrenOf = new Map<number, GraphMenu[]>()
  for (const menu of [...graph.menus].sort((a, b) => a.sort - b.sort || a.id - b.id)) push(childrenOf, menu.parentId ?? 0, menu)
  return {
    graph,
    user: new Map(graph.users.map(u => [u.id, u])),
    role: new Map(graph.roles.map(r => [r.id, r])),
    menu: new Map(graph.menus.map(m => [m.id, m])),
    rolesOfUser,
    usersOfRole,
    rolesOfMenu,
    childrenOf,
  }
}

/** 角色是否生效（启用）；超级管理员（super_admin，与后端 SystemPermissionResolver 一致）拥有全部菜单 */
export const roleActive = (role: GraphRole | undefined) => Boolean(role && role.status === 1)
export const isSuperRole = (role: GraphRole | undefined) => Boolean(role && role.code === 'super_admin')

/** 用户实际拥有的菜单 ID（启用角色的并集） */
export function menusOfUser(index: PermissionIndex, userId: number) {
  const out = new Set<number>()
  for (const roleId of index.rolesOfUser.get(userId) ?? []) {
    const role = index.role.get(roleId)
    if (!roleActive(role)) continue
    if (isSuperRole(role)) index.graph.menus.forEach(m => out.add(m.id))
    else role!.menuIds.forEach(id => out.add(id))
  }
  return out
}

/** 菜单从根到自身的路径 */
export function menuPath(index: PermissionIndex, menuId: number) {
  const path: GraphMenu[] = []
  let cur = index.menu.get(menuId)
  const guard = new Set<number>()
  while (cur && !guard.has(cur.id)) {
    guard.add(cur.id)
    path.unshift(cur)
    cur = index.menu.get(cur.parentId)
  }
  return path
}

/** 按权限码找按钮 / 菜单（精确优先，再模糊） */
export function findMenusByPermission(index: PermissionIndex, keyword: string) {
  const key = keyword.trim().toLowerCase()
  if (!key) return []
  const exact = index.graph.menus.filter(m => (m.permission ?? '').toLowerCase() === key)
  if (exact.length) return exact
  return index.graph.menus.filter(m => (m.permission ?? '').toLowerCase().includes(key) || m.name.toLowerCase().includes(key))
}

/** 能访问某菜单的角色（含超级角色）与用户 */
export function holdersOfMenu(index: PermissionIndex, menuId: number) {
  const roleIds = new Set(index.rolesOfMenu.get(menuId) ?? [])
  index.graph.roles.filter(isSuperRole).forEach(r => roleIds.add(r.id))
  const roles = [...roleIds].map(id => index.role.get(id)!).filter(roleActive)
  const userIds = new Set<number>()
  roles.forEach(r => (index.usersOfRole.get(r.id) ?? []).forEach(u => userIds.add(u)))
  return { roles, users: [...userIds].map(id => index.user.get(id)).filter((u): u is GraphUser => Boolean(u)) }
}

export interface PermissionFindings {
  unusedRoles: GraphRole[]
  usersWithoutRole: GraphUser[]
  orphanMenus: GraphMenu[]
  buttonsWithoutCode: GraphMenu[]
  overlapUsers: Array<{ user: GraphUser; roles: GraphRole[] }>
}

/** 全局体检：没人用的角色、没有角色的用户、没有任何角色授予的菜单、缺权限码的按钮、叠加多角色的用户 */
export function permissionFindings(index: PermissionIndex): PermissionFindings {
  const granted = new Set<number>()
  index.graph.roles.filter(r => roleActive(r) && !isSuperRole(r)).forEach(r => r.menuIds.forEach(id => granted.add(id)))
  return {
    unusedRoles: index.graph.roles.filter(r => !(index.usersOfRole.get(r.id)?.length) && !(r.userCount > 0)),
    usersWithoutRole: index.graph.users.filter(u => !(index.rolesOfUser.get(u.id)?.length)),
    orphanMenus: index.graph.menus.filter(m => m.status === 1 && !granted.has(m.id)),
    buttonsWithoutCode: index.graph.menus.filter(m => m.type === 'btn' && !m.permission),
    overlapUsers: index.graph.users
      .map(user => ({ user, roles: (index.rolesOfUser.get(user.id) ?? []).map(id => index.role.get(id)!).filter(roleActive) }))
      .filter(item => item.roles.length >= 3),
  }
}
