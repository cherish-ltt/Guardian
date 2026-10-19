import type { CurrentUser, LoginResponse } from './auth'

const TOKEN_KEY = 'guardian.token'
const USER_KEY = 'guardian.user'
const EXPIRES_KEY = 'guardian.expiresAt'

export interface AuthSessionOptions {
  localStorage?: Storage
  sessionStorage?: Storage
}

export interface StoredUser {
  id?: number
  username: string
  nickname: string
  deptName?: string
  roles?: string[]
  permissions?: string[]
}

export function createAuthSession(options: AuthSessionOptions = {}) {
  const local = options.localStorage ?? safeStorage('localStorage')
  const session = options.sessionStorage ?? safeStorage('sessionStorage')

  function activeStorage() {
    return local.getItem(TOKEN_KEY) ? local : session
  }

  function save(login: LoginResponse, user: StoredUser | CurrentUser, remember: boolean) {
    clear()
    const storage = remember ? local : session
    storage.setItem(TOKEN_KEY, login.token)
    storage.setItem(USER_KEY, JSON.stringify(user))
    storage.setItem(EXPIRES_KEY, String(Date.now() + login.expiresIn * 1000))
  }

  function token() {
    const storage = activeStorage()
    const expiresAt = Number(storage.getItem(EXPIRES_KEY) || 0)
    if (expiresAt && expiresAt < Date.now()) {
      clear()
      return null
    }
    return storage.getItem(TOKEN_KEY)
  }

  function user(): StoredUser | null {
    const storage = activeStorage()
    const raw = storage.getItem(USER_KEY)
    if (!raw) return null
    try {
      const parsed: unknown = JSON.parse(raw)
      if (!parsed || typeof parsed !== 'object') throw new Error('not an object')
      const candidate = parsed as StoredUser
      if (typeof candidate.username !== 'string') throw new Error('missing username')
      return candidate
    } catch {
      // 缓存被写坏（旧版本结构、手工改过、存储配额异常）时清掉它，
      // 否则应用会在路由初始化阶段抛错，用户连登录页都看不到。
      clear()
      return null
    }
  }

  /** 当前登录是否选择了「记住登录状态」（存于 localStorage） */
  function isRemembered() {
    return Boolean(local.getItem(TOKEN_KEY))
  }

  function expiresAt() {
    return Number(activeStorage().getItem(EXPIRES_KEY) || 0)
  }

  function isAuthenticated() {
    return Boolean(token())
  }

  function clear() {
    ;[local, session].forEach(storage => {
      storage.removeItem(TOKEN_KEY)
      storage.removeItem(USER_KEY)
      storage.removeItem(EXPIRES_KEY)
    })
  }

  return { save, token, user, isAuthenticated, isRemembered, expiresAt, clear }
}

function safeStorage(name: 'localStorage' | 'sessionStorage'): Storage {
  if (typeof window !== 'undefined' && window[name]) return window[name]
  const data = new Map<string, string>()
  return {
    get length() { return data.size },
    clear: () => data.clear(),
    getItem: (key: string) => data.get(key) ?? null,
    key: (index: number) => Array.from(data.keys())[index] ?? null,
    removeItem: (key: string) => data.delete(key),
    setItem: (key: string, value: string) => data.set(key, value),
  }
}

export const authSession = createAuthSession()
