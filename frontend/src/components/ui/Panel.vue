<script setup lang="ts">
defineProps<{
  title?: string
  meta?: string
}>()
</script>

<template>
  <section class="panel m-surface">
    <header v-if="title || $slots.tools || $slots.head" class="panel-head">
      <slot name="head">
        <div style="display: flex; align-items: baseline; gap: 12px; min-width: 0">
          <h2 class="section-label">{{ title }}</h2>
          <span v-if="meta" class="panel-meta">{{ meta }}</span>
        </div>
      </slot>
      <div v-if="$slots.tools" class="panel-tools"><slot name="tools" /></div>
    </header>
    <slot />
  </section>
</template>
