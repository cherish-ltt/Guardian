<script setup lang="ts">
import Modal from '@/components/ui/Modal.vue'
import Button from '@/components/ui/Button.vue'

/* 表单弹窗：两列网格 + 取消 / 保存。页面只写字段与校验。 */
withDefaults(defineProps<{
  open: boolean
  title: string
  sub?: string
  width?: number
  saving?: boolean
  submitText?: string
}>(), {
  width: 620,
  submitText: '保存',
})
const emit = defineEmits<{ (e: 'close'): void; (e: 'submit'): void }>()
</script>

<template>
  <Modal :open="open" :title="title" :sub="sub" :width="width" @close="emit('close')">
    <form class="form-grid" novalidate @submit.prevent="emit('submit')">
      <slot />
      <button type="submit" hidden aria-hidden="true" tabindex="-1" />
    </form>
    <slot name="extra" />
    <template #footer>
      <Button variant="ghost" @click="emit('close')">取消</Button>
      <Button variant="primary" :loading="saving" @click="emit('submit')">{{ submitText }}</Button>
    </template>
  </Modal>
</template>
