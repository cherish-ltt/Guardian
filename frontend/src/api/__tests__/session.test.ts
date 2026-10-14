import test from 'node:test'
import assert from 'node:assert/strict'
import { createAuthSession } from '../session.ts'

function memoryStorage(): Storage {
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

test('auth session persists token to selected storage and exposes current auth state', () => {
  const local = memoryStorage()
  const session = memoryStorage()
  const authSession = createAuthSession({ localStorage: local, sessionStorage: session })

  authSession.save({ token: 'token-1', expiresIn: 7200 }, { username: 'admin', nickname: '陈立诚' }, true)

  assert.equal(authSession.token(), 'token-1')
  assert.equal(authSession.isAuthenticated(), true)
  assert.equal(JSON.parse(local.getItem('guardian.user') ?? '{}').nickname, '陈立诚')
  assert.equal(session.getItem('guardian.token'), null)
})

test('auth session clears both persistent and session storages on logout', () => {
  const local = memoryStorage()
  const session = memoryStorage()
  const authSession = createAuthSession({ localStorage: local, sessionStorage: session })
  authSession.save({ token: 'token-2', expiresIn: 7200 }, { username: 'admin', nickname: '陈立诚' }, false)

  authSession.clear()

  assert.equal(authSession.token(), null)
  assert.equal(authSession.isAuthenticated(), false)
  assert.equal(local.length, 0)
  assert.equal(session.length, 0)
})
