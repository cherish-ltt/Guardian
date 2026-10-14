<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'
import IconButton from './IconButton.vue'
import { originAtPointer, overlayEnter, overlayLeave } from '@/composables/motion'
import { createFocusTrap } from '@/composables/focus'

const props = withDefaults(defineProps<{
  open: boolean
  title?: string
  sub?: string
  width?: number
  /** 共享元素来处：传入选择器（如 'tr'），打开时从最近一次被点的该元素生长成抽屉 */
  originFrom?: string
}>(), {
  width: 520,
})
const emit = defineEmits<{ (e: 'close'): void }>()

const panel = ref<HTMLElement | null>(null)
const trap = createFocusTrap()
const titleId = `dt-${Math.random().toString(36).slice(2, 8)}`
// 打开瞬间记录来处；关闭时若该元素仍在页面中就收回原处
let origin: Element | null = null

function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape' && trap.isTop()) {
    event.stopImmediatePropagation()
    emit('close')
  }
}

watch(() => props.open, async (open) => {
  if (open) {
    origin = props.originFrom ? originAtPointer(props.originFrom) : null
    document.addEventListener('keydown', onKey)
    await nextTick()
    if (!props.open) return
    trap.activate(panel.value)
  } else {
    document.removeEventListener('keydown', onKey)
    trap.deactivate()
  }
}, { immediate: true })

onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKey)
  trap.deactivate()
})

function onOverlay(event: MouseEvent) {
  if (event.target === event.currentTarget) emit('close')
}
</script>

<template>
  <Teleport to="body">
    <Transition :css="false" @enter="(el, done) => overlayEnter(el, done, 'drawer', origin)" @leave="(el, done) => overlayLeave(el, done, 'drawer', origin)">
      <div v-if="open" class="overlay drawer-overlay" @mousedown="onOverlay">
        <aside
          ref="panel"
          class="drawer"
          role="dialog"
          aria-modal="true"
          :aria-labelledby="title ? titleId : undefined"
          :style="{ width: `${width}px` }"
        >
          <header class="drawer-head">
            <div style="min-width: 0">
              <h2 :id="titleId" class="modal-title">{{ title }}</h2>
              <p v-if="sub" class="modal-sub">{{ sub }}</p>
            </div>
            <IconButton name="x" label="关闭" @click="emit('close')" />
          </header>
          <div class="drawer-body"><slot /></div>
          <footer v-if="$slots.footer" class="drawer-foot"><slot name="footer" /></footer>
        </aside>
      </div>
    </Transition>
  </Teleport>
</template>
