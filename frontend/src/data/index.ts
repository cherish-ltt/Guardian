/* ============================================================
   品牌与全局配置：换品牌只改这里。
   ============================================================ */
export const APP = Object.freeze({
  name: 'Guardian 后台管理系统',
  shortName: 'Guardian',
  subtitle: 'Admin Console',
  version: APP_VERSION,
  copyright: `© ${new Date().getFullYear()} Guardian`,
  /* 顶栏的 GitHub 入口；置空字符串即整体不渲染，生产部署摘链接只改这一行 */
  repo: 'https://github.com/cherish-ltt/Guardian',
})

/* ── 导航 ─────────────────────────────────────────────────── */
export interface NavLeaf {
  key: string
  label: string
  icon: string
  path: string
}

export interface NavGroup {
  key: string
  label: string
  icon: string
  /** 顶层单页菜单（如工作台）没有分组标题 */
  single?: boolean
  items: NavLeaf[]
}

/* ── 业务行类型（与后端返回字段对应） ─────────────────────── */
export interface BaseRow {
  id: number
  status: number
  createTime?: string
  [key: string]: any
}

export interface Menu extends BaseRow {
  name: string
  parentId: number
  type: 'dir' | 'menu' | 'btn'
  icon?: string
  path?: string
  component?: string
  permission?: string
  visible?: number
  sort?: number
  children?: Menu[]
}

export interface Dept extends BaseRow {
  name: string
  parentId: number
  leader?: string
  phone?: string
  email?: string
  sort?: number
  memberCount?: number
  children?: Dept[]
}

/** 头像底色：由名称稳定映射，避免同一个人每次颜色不同 */
export const AVATAR_TINTS = ['#236de9', '#3f9d3a', '#4b5a63', '#2f8f9d', '#7a5cd6', '#c2603e']
