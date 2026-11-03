<script setup lang="ts">
import { VueDatePicker, type TimeKey } from '@vuepic/vue-datepicker'
import { zhCN } from 'date-fns/locale/zh-CN'
import { nextTick, useId, useTemplateRef } from 'vue'

defineProps<{ modelValue: string; label: string }>()
const emit = defineEmits<{ (e: 'update:modelValue', value: string): void }>()
const units = { hours: '小时', minutes: '分钟', seconds: '秒' }
const format = 'yyyy-MM-dd HH:mm:ss'
const picker = useTemplateRef('picker')
const menuId = useId()

function restoreFocus() {
  if (picker.value?.dpMenuRef()?.value?.$el?.contains(document.activeElement)) {
    void nextTick(() => picker.value?.inputRef()?.value?.focusInput())
  }
}
</script>

<template>
  <!-- 字符串代表服务端墙上时间；中性时区仅防止浏览器 DST 改写选中的时分秒。 -->
  <VueDatePicker
    ref="picker"
    class="datetime-picker"
    :menu-id="menuId"
    :model-value="modelValue || null"
    :model-type="format"
    :formats="{ input: format, preview: format }"
    timezone="UTC"
    :locale="zhCN"
    :time-config="{ enableSeconds: true, timePickerInline: true, minutesGridIncrement: 1, secondsGridIncrement: 1, startTime: { hours: 0, minutes: 0, seconds: 0 } }"
    :action-row="{ selectBtnLabel: '确定', cancelBtnLabel: '取消' }"
    :aria-labels="{
      input: label, menu: `${label}选择`, clearInput: `清空${label}`, calendarIcon: '选择日期时间',
      openMonthsOverlay: '选择月份', openYearsOverlay: '选择年份',
      nextMonth: '下个月', prevMonth: '上个月', nextYear: '下一年', prevYear: '上一年',
      monthPicker: () => '月份', yearPicker: () => '年份',
      timePicker: '选择时间', toggleOverlay: '返回',
      openTpOverlay: (unit: TimeKey) => `选择${units[unit]}`,
      incrementValue: (unit: TimeKey) => `增加${units[unit]}`, decrementValue: (unit: TimeKey) => `减少${units[unit]}`,
      timeOverlay: (unit: TimeKey) => units[unit],
    }"
    :ui="{ menu: 'datetime-menu' }"
    :floating="{ shift: { crossAxis: true, padding: 8 } }"
    :transitions="false"
    teleport
    placeholder="选择日期和时间"
    @update:model-value="emit('update:modelValue', $event ?? '')"
    @closed="restoreFocus"
  />
</template>
