<script setup lang="ts">
import { computed } from 'vue'
import Button from '@/components/ui/Button.vue'
import IconButton from '@/components/ui/IconButton.vue'
import { hasPermission } from '@/directives/permission'

/* 权限按钮：没有权限码时不渲染（而不是禁用），与后端 @RequiresPermission 使用同一套编码。
   传 iconOnly 时渲染为行内图标按钮，label 作为提示与无障碍名称。 */
const props = withDefaults(defineProps<{
  perm?: string | string[]
  variant?: 'primary' | 'secondary' | 'ghost' | 'text' | 'danger'
  icon?: string
  label?: string
  iconOnly?: boolean
  danger?: boolean
  size?: 'md' | 'sm'
  loading?: boolean
  disabled?: boolean
}>(), {
  variant: 'secondary',
  size: 'md',
})

const allowed = computed(() => hasPermission(props.perm))
</script>

<template>
  <IconButton v-if="allowed && iconOnly" :name="icon ?? 'more'" :label="label ?? ''" :danger="danger" :disabled="disabled" small />
  <Button v-else-if="allowed" :variant="variant" :icon="icon" :size="size" :danger="danger" :loading="loading" :disabled="disabled"><slot>{{ label }}</slot></Button>
</template>
