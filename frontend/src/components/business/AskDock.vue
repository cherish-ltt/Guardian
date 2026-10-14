<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref } from 'vue'
import { useRouter } from 'vue-router'
import Icon from '@/components/Icon.vue'
import IconButton from '@/components/ui/IconButton.vue'
import { ask, presetQuestions, type AnswerCard, type AnswerChip } from '@/composables/ask'
import { revealCascade, typeOut } from '@/composables/motion'
import { toastError } from '@/composables/toast'

/* ============================================================
   问答坞：输入问题或点预设，问题逐字打出（蓄 Charge）→ 答案卡级联浮现（结 Crystallize）。
   规则全部在本地（composables/ask.ts），芯片带路由即可跳转 / 聚焦。
   ============================================================ */
const props = withDefaults(defineProps<{ group?: string; placeholder?: string; compact?: boolean }>(), {
  placeholder: '问问系统…例如「谁能删除用户」',
})
const emit = defineEmits<{ (e: 'chip', chip: AnswerChip): void }>()
const router = useRouter()
const question = ref('')
const cards = ref<AnswerCard[] | null>(null)
const asked = ref('')
const loading = ref(false)
const results = ref<HTMLElement | null>(null)
const presets = computed(() => presetQuestions(props.group))
let cancelTyping: () => void = () => {}

async function run(text: string) {
  const q = text.trim()
  if (!q || loading.value) return
  loading.value = true
  asked.value = q
  try {
    cards.value = await ask(q)
    await nextTick()
    revealCascade(results.value, '.ask-card', 0.09)
  } catch (error) {
    toastError(error, '问题分析失败')
  } finally {
    loading.value = false
  }
}

function typeAsk(text: string) {
  cancelTyping()
  cancelTyping = typeOut(text, (v) => { question.value = v }, () => run(text))
}

function openChip(chip: AnswerChip) {
  emit('chip', chip)
  if (chip.to) router.push(chip.to)
}

onBeforeUnmount(() => cancelTyping())
</script>

<template>
  <div :class="['ask-dock', { 'is-compact': compact }]">
    <section v-if="cards" ref="results" class="ask-results m-float" aria-live="polite">
      <header class="ask-results-head">
        <span><Icon name="activity" :size="13" />{{ asked }}</span>
        <small>本地规则分析</small>
        <IconButton name="x" label="关闭分析结果" small @click="cards = null" />
      </header>
      <article v-for="c in cards" :key="c.title" :class="['ask-card', `tone-${c.tone}`]">
        <h4>{{ c.title }}</h4>
        <p>{{ c.text }}</p>
        <div v-if="c.chips?.length" class="insight-chips">
          <button v-for="chip in c.chips.slice(0, 18)" :key="chip.key" type="button" :class="['insight-chip', chip.tone ? `tone-${chip.tone}` : '']" @click="openChip(chip)">
            {{ chip.label }}<small v-if="chip.sub">{{ chip.sub }}</small>
          </button>
          <span v-if="c.chips.length > 18" class="insight-chip is-more">+{{ c.chips.length - 18 }}</span>
        </div>
      </article>
    </section>
    <div class="ask-panel m-float">
      <form class="ask-form" @submit.prevent="run(question)">
        <Icon name="command" :size="15" class="ask-form-icon" />
        <input v-model="question" :placeholder="placeholder" aria-label="向系统提问" @input="cancelTyping()">
        <button type="submit" class="ask-send" :disabled="loading || !question.trim()" aria-label="提交问题">
          <Icon :name="loading ? 'refresh' : 'arrowRight'" :size="15" :class="{ spin: loading }" />
        </button>
      </form>
      <div v-if="presets.length" class="ask-presets">
        <button v-for="p in presets" :key="p" type="button" @click="typeAsk(p)">{{ p }}</button>
      </div>
    </div>
  </div>
</template>
