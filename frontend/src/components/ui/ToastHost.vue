<script setup lang="ts">
import { computed, ref } from 'vue'
import { dismissToast, pauseToasts, resumeToasts, toasts } from '@/composables/toast'
import { gsap, EASE, prefersReducedMotion } from '@/composables/motion'

/* 提示牌堆：最新一条在最前，较早的缩小叠在后面；悬停展开成列表并暂停计时。
   位置（牌堆 / 展开）由外层 slot 的 CSS 过渡负责，进出场由 GSAP 负责内层，两条通道互不覆盖。 */
const expanded = ref(false)
const count = computed(() => toasts.length)

function enter(el: Element, done: () => void) {
  const card = el.querySelector('.toast')
  if (!card || prefersReducedMotion() || document.hidden) return done()
  // 落定：一次 back.out 的弹入（结 Crystallize）
  gsap.fromTo(card, { opacity: 0, y: 16, scale: 0.9 }, { opacity: 1, y: 0, scale: 1, duration: 0.5, ease: EASE.settle, clearProps: 'transform', onComplete: done })
}
function leave(el: Element, done: () => void) {
  const card = el.querySelector('.toast')
  if (!card || prefersReducedMotion() || document.hidden) return done()
  gsap.to(card, { opacity: 0, y: 8, scale: 0.96, duration: 0.2, ease: EASE.exit, onComplete: done })
}

function onEnter() {
  expanded.value = true
  pauseToasts()
}
function onLeave() {
  expanded.value = false
  resumeToasts()
}
</script>

<template>
  <div
    :class="['toast-host', { 'is-expanded': expanded }]"
    role="status"
    aria-live="polite"
    :style="{ '--toast-count': count }"
    @pointerenter="onEnter"
    @pointerleave="onLeave"
    @focusin="onEnter"
    @focusout="onLeave"
  >
    <TransitionGroup :css="false" @enter="enter" @leave="leave">
      <div
        v-for="(t, i) in toasts"
        :key="t.id"
        class="toast-slot"
        :style="{ '--depth': count - 1 - i }"
      >
        <div :class="['toast', 'm-float', `tone-${t.tone}`, { 'is-paused': expanded }]">
          <span class="toast-dot" aria-hidden="true" />
          <span class="toast-msg">{{ t.msg }}</span>
          <button type="button" class="toast-close" aria-label="关闭提示" @click="dismissToast(t.id)">
            <svg class="toast-ring" width="18" height="18" viewBox="0 0 18 18" aria-hidden="true">
              <circle class="ring-track" cx="9" cy="9" r="7" />
              <circle class="ring-time" cx="9" cy="9" r="7" :style="{ animationDuration: `${t.duration}ms` }" />
              <path d="M6.6 6.6l4.8 4.8M11.4 6.6l-4.8 4.8" />
            </svg>
          </button>
        </div>
      </div>
    </TransitionGroup>
  </div>
</template>
