import type { HttpClient } from './http'

export interface LoginRequest {
  username: string
  password: string
  /** 参数 sys.account.captchaEnabled 关闭时后端不要求，对应 LoginCommand 里的 Option */
  captcha?: string
  captchaUuid?: string
}

export interface Profile {
  username: string
  nickname: string
  phone: string | null
  email: string | null
  deptName: string | null
  roles: string[]
  twoFactorEnabled: boolean
  recoveryCodesRemaining: number
}

export interface SecurityProof { password: string; secondFactor?: string }
export interface Enrollment { secret: string; otpauthUrl: string; expiresIn: number }
export interface RecoveryCodes { recoveryCodes: string[] }

export interface CaptchaResponse {
  /** 参数 sys.account.captchaEnabled 为 false 时为 false，此时 uuid / image 为空串 */
  enabled: boolean
  uuid: string
  /** `data:image/svg+xml;base64,...`，直接放进 `<img src>`；明文只留在服务端 */
  image: string
}

export interface LoginResponse {
  token: string
  expiresIn: number
}

export interface LoginChallenge { twoFactorRequired: true; challenge: string; expiresIn: number }
export type LoginOutcome = LoginResponse | LoginChallenge

export interface ChangePasswordRequest {
  oldPassword: string
  newPassword: string
}

export interface CurrentUser {
  id?: number
  username: string
  nickname: string
  deptId?: number
  deptName?: string
  roles: string[]
  permissions: string[]
}

export interface RouteItem {
  id?: number
  parentId?: number
  name: string
  routeName?: string
  routePath?: string
  path?: string
  component?: string
  icon?: string
  permission?: string
  children?: RouteItem[]
}

export interface RoutesResponse {
  routes: RouteItem[]
}

export function createAuthApi(client: HttpClient) {
  return {
    profile: () => client.get<Profile>('/auth/profile'),
    updateProfile: (body: Pick<Profile, 'nickname' | 'phone' | 'email'>) => client.put<void>('/auth/profile', body),
    setupTwoFactor: (body: SecurityProof) => client.post<Enrollment>('/auth/2fa/setup', body),
    enableTwoFactor: (body: SecurityProof) => client.post<RecoveryCodes>('/auth/2fa/enable', body),
    disableTwoFactor: (body: SecurityProof) => client.post<void>('/auth/2fa/disable', body),
    regenerateRecoveryCodes: (body: SecurityProof) => client.post<RecoveryCodes>('/auth/2fa/recovery-codes', body),
    captcha: () => client.get<CaptchaResponse>('/auth/captcha'),
    login: (body: LoginRequest) => client.post<LoginOutcome>('/auth/login', body),
    verifyLogin: (body: { challenge: string; secondFactor: string }) => client.post<LoginResponse>('/auth/login/2fa', body),
    me: () => client.get<CurrentUser>('/auth/me'),
    routes: () => client.get<RoutesResponse>('/auth/routes'),
    changePassword: (body: ChangePasswordRequest) => client.put<void>('/auth/password', body),
    logout: () => client.post<void>('/auth/logout'),
  }
}
