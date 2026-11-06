<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { Field, TextInput } from '@/components/ui'
import { FormDialog } from '@/components/business'
import { authApi, authSession } from '@/api'
import { clearSession, sessionVersion } from '@/composables/auth'
import { pushToast, toastError } from '@/composables/toast'
import { passwordLength, required, validate } from '@/utils/validate'

const props = defineProps<{ open: boolean }>()
const emit = defineEmits<{ (e: 'close'): void }>()

const router = useRouter()

const form = reactive({ oldPassword: '', newPassword: '', confirmPassword: '' })
const errors = ref<Partial<Record<keyof typeof form, string>>>({})
const saving = ref(false)

watch(() => props.open, (open) => {
  if (!open) return
  Object.assign(form, { oldPassword: '', newPassword: '', confirmPassword: '' })
  errors.value = {}
})

async function submit() {
  if (saving.value) return
  errors.value = validate(form, {
    oldPassword: [required('请输入当前密码')],
    newPassword: [required('请输入新密码'), passwordLength],
    confirmPassword: [required('请再次输入新密码'), v => (v !== form.newPassword ? '两次输入的密码不一致' : undefined)],
  })
  if (Object.keys(errors.value).length) return
  saving.value = true
  const version = sessionVersion.value
  const token = authSession.token()
  try {
    await authApi.changePassword({ oldPassword: form.oldPassword, newPassword: form.newPassword })
    if (version !== sessionVersion.value || token !== authSession.token()) return
    /* 改密同时撤销了本账号的全部会话（含当前这个），必须先清掉本地会话再回登录页 */
    pushToast('密码已修改，请重新登录', 'ok')
    emit('close')
    clearSession()
    await router.replace({ name: 'login' })
  } catch (error) {
    if (version === sessionVersion.value && token === authSession.token()) toastError(error, '密码修改失败')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <FormDialog :open="open" title="修改密码" :width="440" :saving="saving" submit-text="确认修改" @close="emit('close')" @submit="submit">
    <Field label="当前密码" required :error="errors.oldPassword" span>
      <TextInput v-model="form.oldPassword" type="password" autocomplete="current-password" :invalid="!!errors.oldPassword" />
    </Field>
    <Field label="新密码" required :error="errors.newPassword" hint="6 到 64 个字符" span>
      <TextInput v-model="form.newPassword" type="password" autocomplete="new-password" :invalid="!!errors.newPassword" />
    </Field>
    <Field label="确认新密码" required :error="errors.confirmPassword" span>
      <TextInput v-model="form.confirmPassword" type="password" autocomplete="new-password" :invalid="!!errors.confirmPassword" />
    </Field>
  </FormDialog>
</template>
