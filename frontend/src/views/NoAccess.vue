<script setup lang="ts">
import EmptyState from '@/components/ui/EmptyState.vue'
</script>

<template>
  <div class="page">
    <div class="m-surface panel reveal" style="padding: 48px 16px">
      <EmptyState title="暂未分配可用菜单" desc="请联系管理员分配菜单权限后重新登录。" icon="lock" />
    </div>
  </div>
</template>
