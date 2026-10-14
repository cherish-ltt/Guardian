import { shallowRef } from 'vue'

/* 全局确认框：const ok = await confirm({ ... })。由 ConfirmHost 统一渲染。 */
export interface ConfirmOptions {
  title: string
  message: string
  confirmText?: string
  danger?: boolean
}

interface PendingConfirm extends ConfirmOptions {
  resolve: (ok: boolean) => void
}

export const pendingConfirm = shallowRef<PendingConfirm | null>(null)

export function confirm(options: ConfirmOptions): Promise<boolean> {
  pendingConfirm.value?.resolve(false)
  return new Promise(resolve => {
    pendingConfirm.value = { ...options, resolve }
  })
}

export function settleConfirm(ok: boolean) {
  const current = pendingConfirm.value
  pendingConfirm.value = null
  current?.resolve(ok)
}
