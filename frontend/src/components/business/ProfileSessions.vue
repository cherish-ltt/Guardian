<script setup lang="ts">
import { onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { authApi, type LoginSession } from '@/api'
import { sessionVersion } from '@/composables/auth'
import { Button, DataTable, Field, Panel, Tag, TextInput, type Column } from '@/components/ui'
import { FormDialog } from '@/components/business'

const props = defineProps<{ twoFactorEnabled: boolean }>()
const emit = defineEmits<{ (e: 'revoked'): void }>()
const sessions = ref<LoginSession[]>([])
const nextCursor = ref<string | null>(null)
const loading = ref(false)
const loadingError = ref('')
const target = ref<LoginSession | null>(null)
const proof = reactive({ password: '', secondFactor: '' })
const saving = ref(false)
const error = ref('')
const result = ref('')
const columns: Column[] = [
  { key: 'client', title: '浏览器 / 系统' },
  { key: 'issuedAt', title: '登录时间' },
  { key: 'expiresAt', title: '过期时间' },
  { key: 'actions', title: '操作', align: 'right' },
]
let alive = true
onBeforeUnmount(() => { alive = false; proof.password = ''; proof.secondFactor = '' })

async function load(more = false) {
  if (loading.value) return
  const version = sessionVersion.value
  const cursor = more ? nextCursor.value ?? undefined : undefined
  loading.value = true
  loadingError.value = ''
  if (!more) { sessions.value = []; nextCursor.value = null }
  try {
    const page = await authApi.sessions(cursor)
    if (!alive || version !== sessionVersion.value) return
    sessions.value = [...new Map([...sessions.value, ...page.items].map(session => [session.id, session])).values()]
    nextCursor.value = page.nextCursor
  } catch (cause) {
    if (alive && version === sessionVersion.value) loadingError.value = cause instanceof Error ? cause.message : '登录会话加载失败'
  } finally {
    if (alive && version === sessionVersion.value) loading.value = false
  }
}
onMounted(() => load())

function open(session: LoginSession) {
  target.value = session
  proof.password = ''
  proof.secondFactor = ''
  error.value = ''
  result.value = ''
}
function close() {
  if (saving.value) return
  target.value = null
  proof.password = ''
  proof.secondFactor = ''
}
async function revoke() {
  const session = target.value
  if (!session || saving.value) return
  if (!proof.password || (props.twoFactorEnabled && !proof.secondFactor.trim())) {
    error.value = props.twoFactorEnabled ? '请填写当前密码和动态码或恢复码' : '请填写当前密码'
    return
  }
  const version = sessionVersion.value
  saving.value = true
  error.value = ''
  try {
    const response = await authApi.revokeSession(session.id, { password: proof.password, secondFactor: props.twoFactorEnabled ? proof.secondFactor.trim() : undefined })
    if (!alive || version !== sessionVersion.value) return
    result.value = response.revoked ? '该会话已撤销，当前及其他会话保持登录。' : '该会话已失效，无需再次撤销。'
    target.value = null
    proof.password = ''
    proof.secondFactor = ''
    emit('revoked')
    await load()
  } catch (cause) {
    if (alive && version === sessionVersion.value) error.value = cause instanceof Error ? cause.message : '会话撤销失败'
  } finally {
    if (alive && version === sessionVersion.value) saving.value = false
  }
}
</script>

<template>
  <Panel title="登录会话" class="profile-sessions">
    <template #tools><Button size="sm" icon="refresh" :loading="loading" :disabled="saving" @click="load()">刷新</Button></template>
    <div class="panel-body profile-session-intro">
      <p class="muted">查看尚未过期的登录，或退出无法再操作的旧终端；列表不代表设备正在使用。</p>
      <p class="soft">浏览器和系统仅供辨认，可能相同或被伪造。退出当前会话请使用右上角账号菜单。</p>
      <p v-if="result" role="status">{{ result }}</p>
      <p v-if="loadingError" class="profile-error" role="alert">{{ loadingError }}</p>
    </div>
    <DataTable :columns="columns" :data="sessions" :loading="loading" :min-width="650" :empty-text="loadingError ? '暂时无法显示登录会话，请重试' : '暂无有效登录会话'">
      <template #cell-client="{ row }">
        <div class="profile-session-client"><span>{{ row.browser || '未知浏览器' }} / {{ row.os || '未知系统' }}</span><Tag v-if="row.current" tone="ok">当前会话</Tag></div>
      </template>
      <template #cell-issuedAt="{ row }"><time :datetime="new Date(row.issuedAt * 1000).toISOString()" class="tnum">{{ new Date(row.issuedAt * 1000).toLocaleString() }}</time></template>
      <template #cell-expiresAt="{ row }"><time :datetime="new Date(row.expiresAt * 1000).toISOString()" class="tnum">{{ new Date(row.expiresAt * 1000).toLocaleString() }}</time></template>
      <template #cell-actions="{ row }"><span v-if="row.current" class="soft">当前登录</span><Button v-else size="sm" danger :disabled="loading || saving" @click="open(row)">撤销会话</Button></template>
    </DataTable>
    <div v-if="nextCursor !== null" class="panel-body profile-session-more"><Button :loading="loading" :disabled="saving" @click="load(true)">{{ loadingError ? '重试加载更多' : '加载更多' }}</Button></div>
  </Panel>
  <FormDialog :open="target !== null" title="撤销登录会话" :saving="saving" submit-text="确认撤销" @close="close" @submit="revoke">
    <p class="profile-security-hint">确认退出 {{ target?.browser || '未知浏览器' }} / {{ target?.os || '未知系统' }} 的登录？当前及其他会话将保持登录。</p>
    <Field label="当前密码" required span><TextInput v-model="proof.password" type="password" autocomplete="current-password" /></Field>
    <Field v-if="twoFactorEnabled" label="动态码或恢复码" required span><TextInput v-model="proof.secondFactor" autocomplete="one-time-code" :maxlength="64" /></Field>
    <p v-if="twoFactorEnabled" class="soft profile-security-hint">同一动态码不能重复使用，恢复码使用后即失效。</p>
    <p v-if="error" class="profile-error" role="alert">{{ error }}</p>
  </FormDialog>
</template>
