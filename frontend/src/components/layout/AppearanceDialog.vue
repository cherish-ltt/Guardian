<script setup lang="ts">
import Modal from '@/components/ui/Modal.vue'
import Segmented from '@/components/ui/Segmented.vue'
import Button from '@/components/ui/Button.vue'
import { appearance, type Density, type ThemeMode } from '@/composables/appearance'

defineProps<{ open: boolean }>()
const emit = defineEmits<{ (e: 'close'): void }>()

const themes: Array<{ label: string; value: ThemeMode }> = [
  { label: '浅色', value: 'light' },
  { label: '深色', value: 'dark' },
  { label: '跟随系统', value: 'auto' },
]
const densities: Array<{ label: string; value: Density }> = [
  { label: '宽松', value: 'comfy' },
  { label: '标准', value: 'regular' },
  { label: '紧凑', value: 'compact' },
]
</script>

<template>
  <Modal :open="open" title="外观设置" sub="仅保存在当前浏览器" :width="460" close-on-overlay @close="emit('close')">
    <div class="pref-list">
      <div class="pref-row">
        <div>
          <div class="pref-title">主题</div>
          <div class="pref-desc">跟随系统时随操作系统的浅色 / 深色自动切换</div>
        </div>
        <Segmented v-model="appearance.theme" :options="themes" label="主题" />
      </div>
      <div class="pref-row">
        <div>
          <div class="pref-title">表格密度</div>
          <div class="pref-desc">影响所有列表的行高</div>
        </div>
        <Segmented v-model="appearance.density" :options="densities" label="表格密度" />
      </div>
    </div>
    <template #footer>
      <Button variant="primary" @click="emit('close')">完成</Button>
    </template>
  </Modal>
</template>
