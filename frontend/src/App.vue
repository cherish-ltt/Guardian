<script setup lang="ts">
import ToastHost from '@/components/ui/ToastHost.vue'
import ConfirmHost from '@/components/ui/ConfirmHost.vue'
</script>

<template>
  <router-view />
  <ToastHost />
  <ConfirmHost />
  <div class="grain-layer" aria-hidden="true" />
</template>
