<script setup lang="ts">
/* 表单字段。默认根节点是 <label>，点标题即可聚焦输入；
   内部放开关、勾选组这类自带 label 的控件时传 group，改用 role="group" 避免 label 嵌套。 */
defineProps<{
  label?: string
  required?: boolean
  error?: string
  hint?: string
  wide?: boolean
  span?: boolean
  group?: boolean
}>()

const labelId = `fl-${Math.random().toString(36).slice(2, 8)}`
</script>

<template>
  <component
    :is="group ? 'div' : 'label'"
    :class="['field', wide && 'is-wide', span && 'span-2']"
    :role="group ? 'group' : undefined"
    :aria-labelledby="group && label ? labelId : undefined"
  >
    <span v-if="label" :id="labelId" class="field-label">{{ label }}<span v-if="required" class="req" aria-hidden="true">*</span></span>
    <slot />
    <span v-if="error" class="field-error" role="alert">{{ error }}</span>
    <span v-else-if="hint" class="field-hint">{{ hint }}</span>
  </component>
</template>
