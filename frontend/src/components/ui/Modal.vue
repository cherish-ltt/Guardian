<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'
import IconButton from './IconButton.vue'
import { overlayEnter, overlayLeave } from '@/composables/motion'
import { createFocusTrap } from '@/composables/focus'

const props = withDefaults(defineProps<{
  open: boolean
  title?: string
  sub?: string
  width?: number
  /** 点击遮罩是否关闭（表单弹窗默认不关，避免误触丢失输入） */
  closeOnOverlay?: boolean
}>(), {
  width: 560,
})
const emit = defineEmits<{ (e: 'close'): void }>()

const panel = ref<HTMLElement | null>(null)
const trap = createFocusTrap()
const titleId = `mt-${Math.random().toString(36).slice(2, 8)}`

function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape' && trap.isTop()) {
    event.stopImmediatePropagation()
    emit('close')
  }
}

watch(() => props.open, async (open) => {
  if (open) {
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
  if (props.closeOnOverlay && event.target === event.currentTarget) emit('close')
}
</script>

<template>
  <Teleport to="body">
    <Transition :css="false" @enter="(el, done) => overlayEnter(el, done, 'modal')" @leave="(el, done) => overlayLeave(el, done, 'modal')">
      <div v-if="open" class="overlay" @mousedown="onOverlay">
        <div
          ref="panel"
          class="modal"
          role="dialog"
          aria-modal="true"
          :aria-labelledby="title ? titleId : undefined"
          :style="{ maxWidth: `${width}px` }"
        >
          <header class="modal-head">
            <div style="min-width: 0">
              <h2 :id="titleId" class="modal-title">{{ title }}</h2>
              <p v-if="sub" class="modal-sub">{{ sub }}</p>
            </div>
            <IconButton name="x" label="关闭" @click="emit('close')" />
          </header>
          <div class="modal-body"><slot /></div>
          <footer v-if="$slots.footer" class="modal-foot"><slot name="footer" /></footer>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
