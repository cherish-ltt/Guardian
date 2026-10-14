<script setup lang="ts">
import { useRouter } from 'vue-router'
import Button from '@/components/ui/Button.vue'
import EmptyState from '@/components/ui/EmptyState.vue'

const router = useRouter()
</script>

<template>
  <div class="page">
    <div class="m-surface panel reveal" style="padding: 48px 16px">
      <EmptyState title="页面不存在或没有访问权限" desc="请从左侧菜单进入，或联系管理员分配菜单权限。" icon="alert">
        <Button variant="primary" icon="home" style="margin-top: 8px" @click="router.replace('/')">返回首页</Button>
      </EmptyState>
    </div>
  </div>
</template>
