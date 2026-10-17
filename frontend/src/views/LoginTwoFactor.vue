<script setup lang="ts">
import { onBeforeUnmount, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Button, Field, TextInput } from '@/components/ui'
import { clearSession, pendingLogin, sessionVersion, verifyLogin } from '@/composables/auth'

const route = useRoute()
const router = useRouter()
const code = ref('')
const busy = ref(false)
const error = ref('')
const pending = pendingLogin.value
const redirect = typeof route.query.redirect === 'string' && route.query.redirect.startsWith('/') ? route.query.redirect : '/'
let alive = true
onBeforeUnmount(() => {
  alive = false
  code.value = ''
  if (pendingLogin.value === pending) clearSession()
})

function back() {
  clearSession()
  router.replace({ name: 'login', query: { redirect } })
}
async function submit() {
  if (busy.value || !code.value.trim()) return
  const version = sessionVersion.value
  busy.value = true
  error.value = ''
  try {
    await verifyLogin(code.value.trim())
    if (alive && version === sessionVersion.value) await router.replace(redirect)
  } catch (cause) {
    if (alive && version === sessionVersion.value) error.value = cause instanceof Error ? cause.message : '验证失败'
  } finally { busy.value = false }
}
</script>

<template>
  <main class="two-factor-page">
    <section class="two-factor-card m-float">
      <h1 class="page-title">二次验证</h1>
      <template v-if="pending">
        <p class="soft">账号密码已验证。请输入认证器动态码或一枚未使用的恢复码，验证成功后才会登录。</p>
        <form class="login-form" @submit.prevent="submit">
          <Field label="动态码或恢复码" required>
            <TextInput v-model="code" placeholder="请输入动态码或恢复码" autocomplete="one-time-code" :maxlength="64" autofocus />
          </Field>
          <p v-if="error" class="login-error" role="alert">{{ error }}</p>
          <Button type="submit" variant="primary" block :loading="busy">验证并登录</Button>
        </form>
        <p class="soft">验证有效期为5分钟。刷新或离开此页需重新输入账号密码。</p>
      </template>
      <p v-else role="alert">验证已失效，请返回登录页重新登录。</p>
      <Button variant="text" @click="back">返回登录</Button>
    </section>
  </main>
</template>
