<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import Icon from '@/components/Icon.vue'
import { navGroups } from '@/composables/auth'
import { appearance } from '@/composables/appearance'
import { slideThumb } from '@/composables/motion'
import { APP } from '@/data'

const emit = defineEmits<{ (e: 'navigate'): void }>()

const route = useRoute()
const router = useRouter()
const nav = ref<HTMLElement | null>(null)
const thumb = ref<HTMLElement | null>(null)

const activeKey = computed(() => String(route.name ?? ''))
const hasActive = computed(() => navGroups.value.some(g => g.items.some(i => i.key === activeKey.value)))

function go(path: string) {
  emit('navigate')
  if (route.path !== path) router.push(path)
}

function sync(instant = false) {
  const target = nav.value?.querySelector<HTMLElement>('.nav-item.is-active') ?? null
  slideThumb(thumb.value, target, 'y', instant)
}

let observer: ResizeObserver | null = null
onMounted(() => {
  nextTick(() => sync(true))
  observer = new ResizeObserver(() => sync(true))
  if (nav.value) observer.observe(nav.value)
})
onBeforeUnmount(() => observer?.disconnect())
watch(activeKey, () => nextTick(() => sync()))
watch(() => [navGroups.value, appearance.railCollapsed], () => nextTick(() => sync(true)), { deep: true })
</script>

<template>
  <aside class="rail m-rail" aria-label="主导航">
    <div class="rail-brand">
      <span class="brand-mark" aria-hidden="true"><span class="brand-dot" /></span>
      <div class="brand-text">
        <div class="brand-name">{{ APP.shortName }}</div>
        <div class="brand-sub">{{ APP.subtitle }}</div>
      </div>
    </div>

    <nav ref="nav" class="rail-nav">
      <span v-show="hasActive" ref="thumb" class="nav-thumb m-pill" aria-hidden="true" />
      <div v-for="group in navGroups" :key="group.key" class="nav-group">
        <div v-if="!group.single" class="nav-group-title">{{ group.label }}</div>
        <div class="nav-list">
          <button
            v-for="item in group.items"
            :key="item.key"
            type="button"
            :class="['nav-item', item.key === activeKey && 'is-active']"
            :aria-current="item.key === activeKey ? 'page' : undefined"
            :title="appearance.railCollapsed ? item.label : undefined"
            @click="go(item.path)"
          >
            <Icon class="nav-ico" :name="item.icon" :size="18" />
            <span class="nav-label">{{ item.label }}</span>
          </button>
        </div>
      </div>
    </nav>

    <div class="rail-foot">
      <span class="nav-label faint" style="font-size: 11px; padding: 0 10px">{{ APP.version }}</span>
    </div>
  </aside>
</template>
