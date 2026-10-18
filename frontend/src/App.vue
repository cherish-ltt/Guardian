<script setup lang="ts">
import { ConfirmHost, ToastHost } from '@/components/ui'
</script>

<template>
  <router-view />
  <ToastHost />
  <ConfirmHost />
  <div class="grain-layer" aria-hidden="true" />
</template>
