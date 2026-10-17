<script setup lang="ts">
import { onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import QRCode from 'qrcode'
import { authApi, type Enrollment, type Profile } from '@/api'
import { clearSession, refreshUser, sessionVersion } from '@/composables/auth'
import { pushToast, toastError } from '@/composables/toast'
import { Avatar, Button, Field, PageHeader, Panel, Tag, TextInput } from '@/components/ui'
import Icon from '@/components/Icon.vue'
import { FormDialog } from '@/components/business'
import PasswordDialog from '@/components/layout/PasswordDialog.vue'

const router = useRouter()
const profile = ref<Profile | null>(null)
const form = reactive({ nickname: '', phone: '', email: '' })
const editing = ref(false)
const passwordOpen = ref(false)
const saving = ref(false)
const loadingError = ref('')
const securityAction = ref<'setup' | 'enable' | 'disable' | 'recovery' | null>(null)
const proof = reactive({ password: '', secondFactor: '' })
const enrollment = ref<Enrollment | null>(null)
const qr = ref('')
const codes = ref<string[]>([])
const securityError = ref('')
const securityBusy = ref(false)
const finished = ref(false)
let alive = true
onBeforeUnmount(() => { alive = false; proof.password = ''; proof.secondFactor = ''; codes.value = []; enrollment.value = null; qr.value = '' })

async function load() {
  const version = sessionVersion.value
  loadingError.value = ''
  try {
    const result = await authApi.profile()
    if (!alive || version !== sessionVersion.value) return
    profile.value = result
    Object.assign(form, { nickname: result.nickname, phone: result.phone ?? '', email: result.email ?? '' })
  } catch (error) { if (alive && version === sessionVersion.value) loadingError.value = error instanceof Error ? error.message : '加载失败' }
}
onMounted(load)

async function saveProfile() {
  if (saving.value) return
  const version = sessionVersion.value
  saving.value = true
  try {
    await authApi.updateProfile({ nickname: form.nickname.trim(), phone: form.phone.trim() || null, email: form.email.trim() || null })
    if (!alive || version !== sessionVersion.value) return
    await refreshUser()
    if (!alive || version !== sessionVersion.value) return
    editing.value = false
    pushToast('个人资料已保存', 'ok')
    await load()
  } catch (error) { if (alive && version === sessionVersion.value) toastError(error, '保存失败') }
  finally { saving.value = false }
}

function openSecurity(action: 'setup' | 'disable' | 'recovery') {
  proof.password = ''
  proof.secondFactor = ''
  securityError.value = ''
  enrollment.value = null
  qr.value = ''
  securityAction.value = action
}
function closeSecurity() {
  if (securityBusy.value) return
  securityAction.value = null
  proof.password = ''
  proof.secondFactor = ''
  enrollment.value = null
  qr.value = ''
}
async function submitSecurity() {
  const action = securityAction.value
  if (!action || securityBusy.value) return
  if (!proof.password || (action !== 'setup' && !proof.secondFactor.trim())) { securityError.value = '请填写密码和所需的动态码或恢复码'; return }
  const version = sessionVersion.value
  securityBusy.value = true
  securityError.value = ''
  try {
    const body = { password: proof.password, secondFactor: proof.secondFactor.trim() || undefined }
    if (action === 'setup') {
      const result = await authApi.setupTwoFactor(body)
      const image = await QRCode.toDataURL(result.otpauthUrl, { width: 220, margin: 2 })
      if (!alive || version !== sessionVersion.value) return
      enrollment.value = result
      qr.value = image
      securityAction.value = 'enable'
      return
    }
    const result = action === 'enable' ? await authApi.enableTwoFactor(body)
      : action === 'recovery' ? await authApi.regenerateRecoveryCodes(body)
        : (await authApi.disableTwoFactor(body), null)
    if (!alive || version !== sessionVersion.value) return
    codes.value = result?.recoveryCodes ?? []
    finished.value = true
    securityAction.value = null
    proof.password = ''
    proof.secondFactor = ''
    enrollment.value = null
    qr.value = ''
    // 服务端已撤销会话，恢复码仅保留在本页内存，确认保存后回登录页。
    clearSession()
  } catch (error) { if (alive && version === sessionVersion.value) securityError.value = error instanceof Error ? error.message : '操作失败' }
  finally { securityBusy.value = false }
}
function finish() { codes.value = []; router.replace({ name: 'login' }) }
</script>

<template>
  <div class="page profile-page">
    <PageHeader title="个人资料" desc="管理本人资料、登录密码与二次验证，保护你的后台账号。" />
    <Panel v-if="finished" title="安全设置已更新" class="profile-result" role="status">
      <div class="panel-body profile-result-body">
      <p class="muted">所有旧会话已失效，请重新登录。</p>
      <template v-if="codes.length">
        <p><strong>请立即保存恢复码。本页关闭后无法再次查看，每个恢复码只能使用一次。</strong></p>
        <p>丢失认证器时，可用密码和一枚恢复码登录，再到本页关闭或重新绑定认证器。</p>
        <pre class="profile-codes">{{ codes.join('\n') }}</pre>
      </template>
      <div class="profile-result-actions"><Button variant="primary" @click="finish">{{ codes.length ? '已保存恢复码，重新登录' : '重新登录' }}</Button></div>
      </div>
    </Panel>
    <div v-else-if="profile" class="profile-grid">
      <Panel title="基本资料" class="profile-card">
        <template #tools><Button size="sm" icon="edit" @click="editing = true">编辑资料</Button></template>
        <div class="panel-body">
          <div class="profile-identity">
            <Avatar :name="profile.nickname" :size="52" />
            <div><h2>{{ profile.nickname }}</h2><p class="soft">@{{ profile.username }}</p></div>
          </div>
          <dl class="profile-details">
            <div><dt>登录账号</dt><dd class="mono">{{ profile.username }}</dd></div>
            <div><dt>昵称</dt><dd>{{ profile.nickname }}</dd></div>
            <div><dt>手机号码</dt><dd :class="{ soft: !profile.phone }">{{ profile.phone || '未设置' }}</dd></div>
            <div><dt>电子邮箱</dt><dd :class="{ soft: !profile.email }">{{ profile.email || '未设置' }}</dd></div>
            <div><dt>所属部门</dt><dd>{{ profile.deptName || '未分配' }}</dd></div>
            <div><dt>角色</dt><dd class="profile-roles"><Tag v-for="role in profile.roles" :key="role" tone="info">{{ role }}</Tag><span v-if="!profile.roles.length" class="soft">未分配</span></dd></div>
          </dl>
          <p class="profile-note"><Icon name="lock" :size="14" /><span>账号、部门和角色由管理员维护，仅支持修改昵称与联系方式。</span></p>
        </div>
      </Panel>
      <Panel title="账号安全" class="profile-card">
        <template #tools><Tag :tone="profile.twoFactorEnabled ? 'ok' : 'neutral'" dot>{{ profile.twoFactorEnabled ? '2FA 已启用' : '2FA 未启用' }}</Tag></template>
        <div class="panel-body profile-security">
          <div class="profile-security-row">
            <span class="profile-security-icon"><Icon name="lock" :size="18" /></span>
            <div class="profile-security-copy"><h3>登录密码</h3><p>修改密码后，所有设备上的旧会话都将失效。</p></div>
            <Button @click="passwordOpen = true">修改密码</Button>
          </div>
          <div class="profile-security-row">
            <span class="profile-security-icon"><Icon name="shield" :size="18" /></span>
            <div class="profile-security-copy"><h3>认证器验证</h3><p>使用6位动态码，在账号密码之外增加一道登录验证。</p></div>
            <Button v-if="!profile.twoFactorEnabled" variant="primary" @click="openSecurity('setup')">启用2FA</Button>
            <Button v-else danger @click="openSecurity('disable')">关闭2FA</Button>
          </div>
          <div class="profile-security-row">
            <span class="profile-security-icon"><Icon name="key" :size="18" /></span>
            <div class="profile-security-copy"><h3>恢复码</h3><p>{{ profile.twoFactorEnabled ? '剩余 ' + profile.recoveryCodesRemaining + ' 枚。重新生成后，原有恢复码立即失效。' : '启用二次验证后生成，用于丢失认证器时恢复登录。' }}</p></div>
            <Button v-if="profile.twoFactorEnabled" @click="openSecurity('recovery')">重新生成恢复码</Button>
            <span v-else class="profile-security-state soft">启用后可用</span>
          </div>
          <p class="profile-note"><Icon name="shield" :size="14" /><span>同一动态码不能重复使用。请将恢复码保存在独立、安全的位置。</span></p>
        </div>
      </Panel>
    </div>
    <Panel v-else title="基本资料"><div class="panel-body profile-result-body"><p role="status" class="muted">{{ loadingError || '正在加载个人资料…' }}</p><Button v-if="loadingError" icon="refresh" @click="load">重新加载</Button></div></Panel>
    <FormDialog :open="editing" title="编辑个人资料" :saving="saving" @close="editing = false" @submit="saveProfile">
      <Field label="昵称" required span><TextInput v-model="form.nickname" :maxlength="64" /></Field>
      <Field label="手机" span><TextInput v-model="form.phone" type="tel" :maxlength="32" /></Field>
      <Field label="邮箱" span><TextInput v-model="form.email" type="email" :maxlength="254" /></Field>
    </FormDialog>
    <PasswordDialog :open="passwordOpen" @close="passwordOpen = false" />
    <FormDialog :open="securityAction !== null" :title="securityAction === 'disable' ? '关闭2FA' : securityAction === 'recovery' ? '重新生成恢复码' : '绑定认证器'" :saving="securityBusy" :submit-text="securityAction === 'setup' ? '生成绑定信息' : '确认'" @close="closeSecurity" @submit="submitSecurity">
      <div v-if="enrollment" class="profile-enrollment">
        <img :src="qr" alt="认证器绑定二维码" width="220" height="220" />
        <p class="muted">使用认证器扫码，或手动输入密钥</p>
        <code class="mono">{{ enrollment.secret }}</code><p class="soft">SHA1 / 6位 / 30秒 · 绑定信息有效期 {{ enrollment.expiresIn }} 秒</p>
      </div>
      <Field label="当前密码" required span><TextInput v-model="proof.password" type="password" autocomplete="current-password" /></Field>
      <Field v-if="securityAction !== 'setup'" :label="securityAction === 'enable' ? '认证器动态码' : '动态码或恢复码'" required span><TextInput v-model="proof.secondFactor" autocomplete="one-time-code" :maxlength="64" /></Field>
      <p v-if="securityError" class="profile-error" role="alert">{{ securityError }}</p>
      <p v-if="securityAction !== 'setup'" class="soft profile-security-hint">确认后所有旧会话失效。重新生成恢复码会立即废止原有恢复码。</p>
    </FormDialog>
  </div>
</template>
