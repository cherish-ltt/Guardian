<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { Button } from '@/components/ui'
import PasswordDialog from '@/components/layout/PasswordDialog.vue'
import { logout } from '@/composables/auth'

const router = useRouter()
const open = ref(true)

async function exit() {
  const task = logout()
  await router.replace({ name: 'login' })
  await task
}
</script>

<template>
  <main class="two-factor-page">
    <section class="two-factor-card m-float">
      <h1 class="page-title">请先修改密码</h1>
      <p class="soft">管理员要求你更换初始密码。修改成功后请使用新密码重新登录，随后即可访问已授权的功能。</p>
      <Button variant="primary" block @click="open = true">修改密码</Button>
      <Button variant="text" @click="exit">退出登录</Button>
    </section>
    <PasswordDialog :open="open" @close="open = false" />
  </main>
</template>
