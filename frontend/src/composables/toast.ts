import { reactive } from 'vue'
import { ApiError } from '@/api'

export type ToastTone = 'ok' | 'info' | 'danger'
export interface ToastItem { id: number; msg: string; tone: ToastTone; duration: number }

export const toasts = reactive<ToastItem[]>([])

/* 每条提示的剩余时间：悬停提示堆时暂停，移开后按剩余时间继续 */
const timers = new Map<number, { handle: number; endsAt: number; remaining: number }>()
let paused = false
let seed = 0

function dismiss(id: number) {
  const index = toasts.findIndex(t => t.id === id)
  if (index !== -1) toasts.splice(index, 1)
  const timer = timers.get(id)
  if (timer) window.clearTimeout(timer.handle)
  timers.delete(id)
}

function schedule(id: number, ms: number) {
  timers.set(id, { handle: paused ? 0 : window.setTimeout(() => dismiss(id), ms), endsAt: Date.now() + ms, remaining: ms })
}

export function pushToast(msg: string, tone: ToastTone = 'ok') {
  const id = ++seed
  const duration = tone === 'danger' ? 4200 : 2600
  toasts.push({ id, msg, tone, duration })
  if (toasts.length > 4) dismiss(toasts[0].id)
  schedule(id, duration)
}

export function pauseToasts() {
  if (paused) return
  paused = true
  timers.forEach((timer) => {
    window.clearTimeout(timer.handle)
    timer.remaining = Math.max(0, timer.endsAt - Date.now())
  })
}

export function resumeToasts() {
  if (!paused) return
  paused = false
  timers.forEach((timer, id) => schedule(id, Math.max(600, timer.remaining)))
}

export { dismiss as dismissToast }

/** 统一的失败提示：后端返回的业务消息优先，其次给出兜底文案 */
export function toastError(error: unknown, fallback: string) {
  pushToast(error instanceof ApiError ? error.message : fallback, 'danger')
}
