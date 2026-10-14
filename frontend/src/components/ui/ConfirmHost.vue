<script setup lang="ts">
import { shallowRef, watch } from 'vue'
import Icon from '@/components/Icon.vue'
import Modal from './Modal.vue'
import Button from './Button.vue'
import { pendingConfirm, settleConfirm } from '@/composables/confirm'

// 退场动画期间保留上一份内容，避免文案先消失
const state = shallowRef(pendingConfirm.value)
watch(pendingConfirm, (value) => { if (value) state.value = value })
</script>

<template>
  <Modal :open="Boolean(pendingConfirm)" :title="state?.title" :width="440" close-on-overlay @close="settleConfirm(false)">
    <div class="confirm-body">
      <span :class="['confirm-icon', state?.danger && 'is-danger']"><Icon :name="state?.danger ? 'alert' : 'info'" :size="18" /></span>
      <p class="confirm-text">{{ state?.message }}</p>
    </div>
    <template #footer>
      <Button variant="ghost" @click="settleConfirm(false)">取消</Button>
      <Button :variant="state?.danger ? 'danger' : 'primary'" autofocus @click="settleConfirm(true)">{{ state?.confirmText ?? '确定' }}</Button>
    </template>
  </Modal>
</template>
