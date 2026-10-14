<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref } from 'vue'
import { overlayEnter, overlayLeave } from '@/composables/motion'

/* 悬停名片：指针停留 300ms 或键盘聚焦时出现的轻玻璃预览卡。
   只放只读信息；离开触发区后留 120ms 宽限，允许指针移进卡片。 */
const props = withDefaults(defineProps<{
  delay?: number
  placement?: 'bottom' | 'top'
  width?: number
}>(), {
  delay: 300,
  placement: 'bottom',
  width: 260,
})

const trigger = ref<HTMLElement | null>(null)
const card = ref<HTMLElement | null>(null)
const open = ref(false)
const pos = ref({ left: 0, top: 0 })
let timer = 0
const id = `hc-${Math.random().toString(36).slice(2, 8)}`

function place() {
  const t = trigger.value?.getBoundingClientRect()
  if (!t) return
  const h = card.value?.offsetHeight ?? 120
  const below = props.placement === 'bottom' ? t.bottom + h + 12 < window.innerHeight : t.top - h - 12 < 0
  const left = Math.min(Math.max(t.left + t.width / 2 - props.width / 2, 12), window.innerWidth - props.width - 12)
  pos.value = { left, top: below ? t.bottom + 8 : t.top - h - 8 }
}

function show(now = false) {
  window.clearTimeout(timer)
  timer = window.setTimeout(async () => {
    open.value = true
    await nextTick()
    place()
  }, now ? 0 : props.delay)
}
function hide() {
  window.clearTimeout(timer)
  timer = window.setTimeout(() => { open.value = false }, 120)
}
function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape' && open.value) {
    open.value = false
    event.stopPropagation()
  }
}
onBeforeUnmount(() => window.clearTimeout(timer))
</script>

<template>
  <span
    ref="trigger"
    class="hovercard-trigger"
    :aria-describedby="open ? id : undefined"
    @pointerenter="show()"
    @pointerleave="hide"
    @focusin="show(true)"
    @focusout="hide"
    @keydown="onKey"
  ><slot /></span>
  <Teleport to="body">
    <Transition :css="false" @enter="(el, done) => overlayEnter(el, done, 'pop')" @leave="(el, done) => overlayLeave(el, done, 'pop')">
      <div
        v-if="open"
        :id="id"
        ref="card"
        role="tooltip"
        class="hovercard m-float"
        :style="{ left: `${pos.left}px`, top: `${pos.top}px`, width: `${width}px` }"
        @pointerenter="show(true)"
        @pointerleave="hide"
      ><slot name="card" /></div>
    </Transition>
  </Teleport>
</template>
