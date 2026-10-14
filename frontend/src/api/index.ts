import { httpClient } from './http.ts'
import { createAuthApi } from './auth.ts'
import { createSystemApi } from './system.ts'
import { createPortalApi } from './portal.ts'

export * from './http.ts'
export * from './auth.ts'
export * from './session.ts'
export * from './system.ts'
export * from './portal.ts'

export const authApi = createAuthApi(httpClient)
export const systemApi = createSystemApi(httpClient)
export const portalApi = createPortalApi(httpClient)
