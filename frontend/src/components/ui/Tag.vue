<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  tone?: string
  dot?: boolean
}>()

/* 语义色只有四种；字典里的历史色值就近归档 */
const TONE_MAP: Record<string, string> = {
  ok: 'ok', success: 'ok', primary: 'info', info: 'info', purple: 'info',
  danger: 'danger', error: 'danger', warn: 'danger', warning: 'danger', neutral: 'neutral', default: 'neutral',
}
const toneClass = computed(() => `tone-${TONE_MAP[props.tone ?? 'neutral'] ?? 'neutral'}`)
</script>

<template>
  <span :class="['tag', toneClass]"><span v-if="dot" class="tag-dot" /><slot /></span>
</template>
