<template>
  <div class="claude-aliases">
    <!-- Header -->
    <div class="aliases-header">
      <div class="header-title">
        <h2>
          <i class="fa-solid fa-masks-theater" />
          Claude 别名
        </h2>
        <span class="alias-count">{{ settingsStore.claudeAliases.length }} 个别名</span>
      </div>
      <div class="header-actions">
        <button
          class="action-btn add"
          @click="startAdding"
        >
          <i class="fa-solid fa-plus" />
          新增别名
        </button>
      </div>
    </div>

    <!-- Hint -->
    <div class="hint-bar">
      <i class="fa-solid fa-circle-info" />
      输入别名即可在终端中以不同配置启动 claude。修改后请重新打开终端生效。
    </div>

    <!-- Content -->
    <div class="aliases-content">
      <!-- Empty State -->
      <div
        v-if="settingsStore.claudeAliases.length === 0 && !isEditing"
        class="empty-state"
      >
        <i class="fa-solid fa-masks-theater" />
        <p>暂无别名</p>
        <p class="sub">
          创建别名以快速切换不同的 Claude 配置
        </p>
        <button
          class="action-btn add"
          @click="startAdding"
        >
          <i class="fa-solid fa-plus" />
          新增别名
        </button>
      </div>

      <!-- Alias List -->
      <div
        v-for="alias in settingsStore.claudeAliases"
        :key="alias.id"
        class="alias-card"
      >
        <div class="alias-header">
          <span class="alias-name">{{ alias.name }}</span>
          <span
            v-if="alias.description"
            class="alias-desc"
          >{{ alias.description }}</span>
        </div>
        <div class="alias-env-tags">
          <span
            v-if="alias.anthropicModel"
            class="env-tag model"
          >
            <i class="fa-solid fa-robot" />
            {{ alias.anthropicModel }}
          </span>
          <span
            v-if="alias.anthropicAuthToken"
            class="env-tag token"
          >
            <i class="fa-solid fa-key" />
            {{ maskToken(alias.anthropicAuthToken) }}
          </span>
          <span
            v-if="alias.anthropicBaseUrl"
            class="env-tag url"
          >
            <i class="fa-solid fa-link" />
            {{ alias.anthropicBaseUrl }}
          </span>
          <span
            v-for="(v, k) in alias.customEnvVars"
            :key="k"
            class="env-tag custom"
          >
            <i class="fa-solid fa-gear" />
            {{ k }}={{ maskIfSecret(k, v) }}
          </span>
        </div>
        <div class="alias-actions">
          <button
            class="icon-btn edit"
            @click="startEditing(alias)"
          >
            <i class="fa-solid fa-pen" />
            编辑
          </button>
          <button
            class="icon-btn copy"
            @click="duplicateAlias(alias)"
          >
            <i class="fa-solid fa-copy" />
            复制
          </button>
          <button
            class="icon-btn delete"
            @click="confirmDelete(alias)"
          >
            <i class="fa-solid fa-trash" />
            删除
          </button>
        </div>
      </div>

      <!-- System Registration -->
      <div
        v-if="settingsStore.claudeAliases.length > 0"
        class="system-registration"
      >
        <div class="registration-header">
          <h3>
            <i class="fa-solid fa-terminal" />
            注册到系统终端
          </h3>
          <span class="registration-hint">
            将别名注册到系统 Shell 配置，使其在外部终端中可用
          </span>
        </div>

        <div class="registration-status">
          <span
            v-if="systemAliasesLoading"
            class="status-loading"
          >
            <i class="fa-solid fa-spinner fa-spin" />
            检查中...
          </span>
          <template v-else>
            <span
              v-if="systemAliasesStatus?.registered"
              class="status-tag registered"
            >
              <i class="fa-solid fa-check-circle" />
              已注册
            </span>
            <span
              v-else
              class="status-tag not-registered"
            >
              <i class="fa-solid fa-circle-xmark" />
              未注册
            </span>
          </template>
        </div>

        <div
          v-if="systemAliasesStatus && !systemAliasesLoading"
          class="registration-details"
        >
          <div class="detail-row">
            <span class="detail-label">别名文件:</span>
            <code>{{ systemAliasesStatus.aliases_file_path }}</code>
            <i
              v-if="systemAliasesStatus.file_exists"
              class="fa-solid fa-check detail-ok"
            />
            <i
              v-else
              class="fa-solid fa-times detail-missing"
            />
          </div>
          <div class="detail-row">
            <span class="detail-label">Shell 配置:</span>
            <code>{{ systemAliasesStatus.profile_path }}</code>
            <i
              v-if="systemAliasesStatus.profile_has_source"
              class="fa-solid fa-check detail-ok"
            />
            <i
              v-else
              class="fa-solid fa-times detail-missing"
            />
          </div>
        </div>

        <div class="registration-actions">
          <button
            class="action-btn register"
            :disabled="registering"
            @click="registerToSystem"
          >
            <i
              v-if="registering"
              class="fa-solid fa-spinner fa-spin"
            />
            <i
              v-else
              class="fa-solid fa-arrow-up-from-bracket"
            />
            {{ systemAliasesStatus?.registered ? '更新注册' : '注册到系统' }}
          </button>
          <button
            v-if="systemAliasesStatus?.registered"
            class="action-btn unregister"
            :disabled="registering"
            @click="unregisterFromSystem"
          >
            <i class="fa-solid fa-trash-can" />
            取消注册
          </button>
        </div>

        <div
          v-if="registrationError"
          class="form-error"
          style="margin-top: 8px;"
        >
          <i class="fa-solid fa-exclamation-circle" />
          {{ registrationError }}
        </div>

        <div
          v-if="registrationSuccess"
          class="registration-success"
        >
          <i class="fa-solid fa-check-circle" />
          {{ registrationSuccess }}
        </div>
      </div>

      <!-- Add/Edit Form -->
      <div
        v-if="isEditing"
        class="alias-form-overlay"
        @click.self="cancelEdit"
      >
        <div class="alias-form">
          <div class="form-header">
            <h3>{{ editingAlias ? '编辑别名' : '新增别名' }}</h3>
            <button
              class="close-btn"
              @click="cancelEdit"
            >
              <i class="fa-solid fa-times" />
            </button>
          </div>

          <div class="form-body">
            <!-- Alias Name -->
            <div class="form-group">
              <label>别名 <span class="required">*</span></label>
              <input
                v-model="formName"
                type="text"
                placeholder="例如: claude-opus"
                :class="{ error: formNameError }"
              >
              <span
                v-if="formNameError"
                class="field-error"
              >{{ formNameError }}</span>
              <span
                v-else
                class="field-hint"
              >字母开头，支持字母、数字、连字符、下划线</span>
            </div>

            <!-- Description -->
            <div class="form-group">
              <label>描述</label>
              <input
                v-model="formDescription"
                type="text"
                placeholder="可选，用于备忘"
              >
            </div>

            <!-- ANTHROPIC_MODEL -->
            <div class="form-group">
              <label>ANTHROPIC_MODEL</label>
              <input
                v-model="formModel"
                type="text"
                placeholder="例如: claude-opus-4-6"
              >
            </div>

            <!-- ANTHROPIC_AUTH_TOKEN -->
            <div class="form-group">
              <label>ANTHROPIC_AUTH_TOKEN</label>
              <div class="password-input">
                <input
                  v-model="formAuthToken"
                  :type="showToken ? 'text' : 'password'"
                  placeholder="例如: sk-ant-..."
                >
                <button
                  class="toggle-btn"
                  type="button"
                  @click="showToken = !showToken"
                >
                  <i
                    class="fa-solid"
                    :class="showToken ? 'fa-eye-slash' : 'fa-eye'"
                  />
                </button>
              </div>
            </div>

            <!-- ANTHROPIC_BASE_URL -->
            <div class="form-group">
              <label>ANTHROPIC_BASE_URL</label>
              <input
                v-model="formBaseUrl"
                type="text"
                placeholder="例如: https://api.anthropic.com"
              >
            </div>

            <!-- Custom Environment Variables -->
            <div class="form-group">
              <label>
                自定义环境变量
                <button
                  class="add-var-btn"
                  type="button"
                  @click="addCustomVar"
                >
                  <i class="fa-solid fa-plus" /> 添加
                </button>
              </label>
              <div
                v-for="(_, index) in formCustomVars"
                :key="index"
                class="custom-var-row"
              >
                <input
                  v-model="formCustomVars[index].key"
                  type="text"
                  placeholder="变量名"
                  class="var-key"
                >
                <input
                  v-model="formCustomVars[index].value"
                  type="text"
                  placeholder="值"
                  class="var-value"
                >
                <button
                  class="remove-var-btn"
                  type="button"
                  @click="removeCustomVar(index)"
                >
                  <i class="fa-solid fa-times" />
                </button>
              </div>
            </div>

            <!-- Error -->
            <div
              v-if="formError"
              class="form-error"
            >
              <i class="fa-solid fa-exclamation-circle" />
              {{ formError }}
            </div>
          </div>

          <div class="form-footer">
            <button
              class="btn cancel"
              @click="cancelEdit"
            >
              取消
            </button>
            <button
              class="btn save"
              :disabled="!!formNameError || saving"
              @click="saveAlias"
            >
              <i
                v-if="saving"
                class="fa-solid fa-spinner fa-spin"
              />
              {{ editingAlias ? '保存' : '创建' }}
            </button>
          </div>
        </div>
      </div>

      <!-- Delete Confirmation -->
      <div
        v-if="deletingAlias"
        class="alias-form-overlay"
        @click.self="deletingAlias = null"
      >
        <div class="alias-form delete-confirm">
          <div class="form-header">
            <h3>确认删除</h3>
          </div>
          <div class="form-body">
            <p>确定要删除别名 <strong>{{ deletingAlias.name }}</strong> 吗？</p>
          </div>
          <div class="form-footer">
            <button
              class="btn cancel"
              @click="deletingAlias = null"
            >
              取消
            </button>
            <button
              class="btn delete"
              @click="doDelete"
            >
              删除
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useSettingsStore, type ClaudeAlias } from '@/stores/settingsStore'

const settingsStore = useSettingsStore()

// System aliases registration
interface SystemAliasesStatus {
  registered: boolean
  file_exists: boolean
  profile_has_source: boolean
  aliases_file_path: string
  profile_path: string
}

const systemAliasesStatus = ref<SystemAliasesStatus | null>(null)
const systemAliasesLoading = ref(false)
const registering = ref(false)
const registrationError = ref('')
const registrationSuccess = ref('')

async function checkSystemAliasesStatus() {
  systemAliasesLoading.value = true
  try {
    systemAliasesStatus.value = await invoke<SystemAliasesStatus>('check_system_aliases_status')
  } catch (e) {
    console.error('Failed to check system aliases status:', e)
  } finally {
    systemAliasesLoading.value = false
  }
}

function buildAliasConfigs() {
  return settingsStore.claudeAliases
    .filter(a => a.name.trim())
    .map(a => ({
      name: a.name.trim(),
      anthropic_model: a.anthropicModel || null,
      anthropic_auth_token: a.anthropicAuthToken || null,
      anthropic_base_url: a.anthropicBaseUrl || null,
      custom_env_vars: Object.keys(a.customEnvVars || {}).length > 0 ? a.customEnvVars : null,
    }))
}

async function registerToSystem() {
  registering.value = true
  registrationError.value = ''
  registrationSuccess.value = ''
  try {
    const aliases = buildAliasConfigs()
    await invoke('register_system_aliases', { aliases })
    registrationSuccess.value = '别名已成功注册到系统终端。请重新打开终端窗口以生效。'
    await checkSystemAliasesStatus()
    setTimeout(() => { registrationSuccess.value = '' }, 5000)
  } catch (e) {
    registrationError.value = `注册失败: ${e}`
  } finally {
    registering.value = false
  }
}

async function unregisterFromSystem() {
  registering.value = true
  registrationError.value = ''
  registrationSuccess.value = ''
  try {
    await invoke('unregister_system_aliases')
    registrationSuccess.value = '已从系统终端取消注册。'
    await checkSystemAliasesStatus()
    setTimeout(() => { registrationSuccess.value = '' }, 5000)
  } catch (e) {
    registrationError.value = `取消注册失败: ${e}`
  } finally {
    registering.value = false
  }
}

onMounted(() => {
  checkSystemAliasesStatus()
})

const RESERVED_NAMES = ['claude', 'cd', 'ls', 'dir', 'git', 'npm', 'node', 'mcd', 'ccr', 'powershell', 'bash', 'zsh', 'exit', 'clear']

// Form state
const isEditing = ref(false)
const editingAlias = ref<ClaudeAlias | null>(null)
const deletingAlias = ref<ClaudeAlias | null>(null)
const saving = ref(false)
const showToken = ref(false)

const formName = ref('')
const formDescription = ref('')
const formModel = ref('')
const formAuthToken = ref('')
const formBaseUrl = ref('')
const formCustomVars = ref<{ key: string; value: string }[]>([])
const formError = ref('')

// Validation
const formNameError = computed(() => {
  const name = formName.value.trim()
  if (!name) return ''
  if (!/^[a-zA-Z][a-zA-Z0-9_-]*$/.test(name)) {
    return '名称必须字母开头，只能包含字母、数字、连字符、下划线'
  }
  if (RESERVED_NAMES.includes(name.toLowerCase())) {
    return `"${name}" 是保留名称，请使用其他名称`
  }
  const duplicate = settingsStore.claudeAliases.some(
    a => a.name === name && a.id !== editingAlias.value?.id
  )
  if (duplicate) {
    return '已存在同名别名'
  }
  return ''
})

// Masking helpers
function maskToken(token: string): string {
  if (token.length <= 8) return '****'
  return token.slice(0, 6) + '****' + token.slice(-4)
}

function maskIfSecret(key: string, value: string): string {
  const lower = key.toLowerCase()
  if (lower.includes('token') || lower.includes('key') || lower.includes('secret') || lower.includes('password')) {
    return maskToken(value)
  }
  return value
}

// Duplicate alias
async function duplicateAlias(alias: ClaudeAlias) {
  const existingNames = new Set(settingsStore.claudeAliases.map(a => a.name))
  let counter = 1
  let newName = ''
  while (true) {
    const candidate = `${alias.name}-${counter}`
    if (!existingNames.has(candidate)) {
      newName = candidate
      break
    }
    counter++
  }
  await settingsStore.addClaudeAlias({
    name: newName,
    description: alias.description,
    anthropicModel: alias.anthropicModel,
    anthropicAuthToken: alias.anthropicAuthToken,
    anthropicBaseUrl: alias.anthropicBaseUrl,
    customEnvVars: { ...alias.customEnvVars }
  })
}

// Form actions
function startAdding() {
  editingAlias.value = null
  formName.value = ''
  formDescription.value = ''
  formModel.value = ''
  formAuthToken.value = ''
  formBaseUrl.value = ''
  formCustomVars.value = []
  formError.value = ''
  showToken.value = false
  isEditing.value = true
}

function startEditing(alias: ClaudeAlias) {
  editingAlias.value = alias
  formName.value = alias.name
  formDescription.value = alias.description
  formModel.value = alias.anthropicModel
  formAuthToken.value = alias.anthropicAuthToken
  formBaseUrl.value = alias.anthropicBaseUrl
  formCustomVars.value = Object.entries(alias.customEnvVars || {}).map(([key, value]) => ({ key, value }))
  formError.value = ''
  showToken.value = false
  isEditing.value = true
}

function cancelEdit() {
  isEditing.value = false
  editingAlias.value = null
  formError.value = ''
}

function addCustomVar() {
  formCustomVars.value = [...formCustomVars.value, { key: '', value: '' }]
}

function removeCustomVar(index: number) {
  formCustomVars.value = formCustomVars.value.filter((_, i) => i !== index)
}

async function saveAlias() {
  const name = formName.value.trim()
  if (!name) {
    formError.value = '请输入别名名称'
    return
  }
  if (formNameError.value) {
    formError.value = formNameError.value
    return
  }

  const hasAnyEnv = formModel.value.trim() ||
    formAuthToken.value.trim() ||
    formBaseUrl.value.trim() ||
    formCustomVars.value.some(v => v.key.trim() && v.value.trim())

  if (!hasAnyEnv) {
    formError.value = '请至少设置一个环境变量'
    return
  }

  // Build custom env vars record with key validation
  const ENV_KEY_RE = /^[A-Za-z_][A-Za-z0-9_]*$/
  const customEnvVars: Record<string, string> = {}
  for (const v of formCustomVars.value) {
    const key = v.key.trim()
    const value = v.value.trim()
    if (key && !ENV_KEY_RE.test(key)) {
      formError.value = `变量名 "${key}" 无效，只能包含字母、数字和下划线，且必须字母或下划线开头`
      return
    }
    if (key && value) {
      customEnvVars[key] = value
    }
  }

  saving.value = true
  formError.value = ''

  try {
    if (editingAlias.value) {
      await settingsStore.updateClaudeAlias(editingAlias.value.id, {
        name,
        description: formDescription.value.trim(),
        anthropicModel: formModel.value.trim(),
        anthropicAuthToken: formAuthToken.value.trim(),
        anthropicBaseUrl: formBaseUrl.value.trim(),
        customEnvVars
      })
    } else {
      await settingsStore.addClaudeAlias({
        name,
        description: formDescription.value.trim(),
        anthropicModel: formModel.value.trim(),
        anthropicAuthToken: formAuthToken.value.trim(),
        anthropicBaseUrl: formBaseUrl.value.trim(),
        customEnvVars
      })
    }
    isEditing.value = false
    editingAlias.value = null
  } catch (e) {
    formError.value = `保存失败: ${e}`
  } finally {
    saving.value = false
  }
}

function confirmDelete(alias: ClaudeAlias) {
  deletingAlias.value = alias
}

async function doDelete() {
  if (!deletingAlias.value) return
  try {
    await settingsStore.removeClaudeAlias(deletingAlias.value.id)
  } catch (e) {
    console.error('Failed to delete alias:', e)
  }
  deletingAlias.value = null
}
</script>

<style scoped lang="less">
.claude-aliases {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #f5f5f5;
  position: relative;
}

.aliases-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  background: white;
  border-bottom: 1px solid #e0e0e0;
  flex-shrink: 0;

  .header-title {
    display: flex;
    align-items: center;
    gap: 12px;

    h2 {
      margin: 0;
      font-size: 18px;
      font-weight: 500;
      display: flex;
      align-items: center;
      gap: 10px;
      color: #333;

      i {
        color: #8b5cf6;
      }
    }

    .alias-count {
      font-size: 13px;
      color: #888;
      background: #f0f0f0;
      padding: 2px 10px;
      border-radius: 10px;
    }
  }

  .header-actions {
    display: flex;
    gap: 8px;
  }
}

.hint-bar {
  padding: 10px 20px;
  background: #f0f4ff;
  color: #5b6abf;
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 8px;
  border-bottom: 1px solid #e0e0e0;
  flex-shrink: 0;
}

.aliases-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  color: #aaa;

  i {
    font-size: 48px;
    margin-bottom: 16px;
    color: #d0d0d0;
  }

  p {
    margin: 0;
    font-size: 16px;
    color: #888;

    &.sub {
      font-size: 13px;
      color: #aaa;
      margin-top: 4px;
    }
  }

  .action-btn {
    margin-top: 20px;
  }
}

.alias-card {
  background: white;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  padding: 14px 16px;
  transition: border-color 0.2s;

  &:hover {
    border-color: #c0c0c0;
  }

  .alias-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 8px;

    .alias-name {
      font-family: 'Consolas', 'Courier New', monospace;
      font-size: 15px;
      font-weight: 600;
      color: #8b5cf6;
    }

    .alias-desc {
      font-size: 13px;
      color: #888;
    }
  }

  .alias-env-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 10px;
  }

  .alias-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
}

.env-tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  padding: 3px 8px;
  border-radius: 4px;
  font-family: 'Consolas', 'Courier New', monospace;

  &.model {
    background: #ede9fe;
    color: #7c3aed;
  }

  &.token {
    background: #fef3c7;
    color: #b45309;
  }

  &.url {
    background: #dbeafe;
    color: #2563eb;
  }

  &.custom {
    background: #f0fdf4;
    color: #16a34a;
  }
}

.icon-btn {
  padding: 4px 10px;
  border: 1px solid #e0e0e0;
  border-radius: 4px;
  background: white;
  cursor: pointer;
  font-size: 12px;
  display: flex;
  align-items: center;
  gap: 4px;
  color: #666;
  transition: all 0.2s;

  &.edit:hover {
    border-color: #8b5cf6;
    color: #8b5cf6;
  }

  &.copy:hover {
    border-color: #6c7ae0;
    color: #6c7ae0;
  }

  &.delete:hover {
    border-color: #dc2626;
    color: #dc2626;
  }
}

.action-btn {
  padding: 8px 16px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 6px;
  transition: background 0.2s;

  &.add {
    background: #ede9fe;
    color: #7c3aed;

    &:hover {
      background: #ddd6fe;
    }
  }
}

// Form overlay
.alias-form-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.alias-form {
  background: white;
  border-radius: 10px;
  width: 480px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);

  &.delete-confirm {
    width: 360px;
  }

  .form-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid #e0e0e0;

    h3 {
      margin: 0;
      font-size: 16px;
      font-weight: 500;
      color: #333;
    }

    .close-btn {
      background: none;
      border: none;
      cursor: pointer;
      color: #888;
      font-size: 16px;
      padding: 4px;

      &:hover {
        color: #333;
      }
    }
  }

  .form-body {
    padding: 16px 20px;
    overflow-y: auto;
    flex: 1;

    p {
      margin: 0;
      font-size: 14px;
      color: #555;

      strong {
        color: #8b5cf6;
        font-family: 'Consolas', 'Courier New', monospace;
      }
    }
  }

  .form-footer {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    padding: 12px 20px;
    border-top: 1px solid #e0e0e0;
  }
}

.form-group {
  margin-bottom: 14px;

  label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    font-weight: 500;
    color: #555;
    margin-bottom: 6px;

    .required {
      color: #dc2626;
    }
  }

  input[type="text"],
  input[type="password"] {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid #d0d0d0;
    border-radius: 6px;
    font-size: 13px;
    outline: none;
    transition: border-color 0.2s;
    box-sizing: border-box;

    &:focus {
      border-color: #8b5cf6;
    }

    &.error {
      border-color: #dc2626;
    }

    &::placeholder {
      color: #bbb;
    }
  }

  .field-error {
    font-size: 12px;
    color: #dc2626;
    margin-top: 4px;
    display: block;
  }

  .field-hint {
    font-size: 12px;
    color: #aaa;
    margin-top: 4px;
    display: block;
  }
}

.password-input {
  display: flex;
  gap: 0;

  input {
    flex: 1;
    border-radius: 6px 0 0 6px !important;
  }

  .toggle-btn {
    padding: 8px 12px;
    border: 1px solid #d0d0d0;
    border-left: none;
    border-radius: 0 6px 6px 0;
    background: #f5f5f5;
    cursor: pointer;
    color: #888;
    font-size: 13px;

    &:hover {
      color: #555;
      background: #eee;
    }
  }
}

.custom-var-row {
  display: flex;
  gap: 6px;
  margin-bottom: 6px;

  .var-key {
    flex: 2;
  }

  .var-value {
    flex: 3;
  }

  .remove-var-btn {
    padding: 6px 10px;
    border: 1px solid #e0e0e0;
    border-radius: 6px;
    background: white;
    cursor: pointer;
    color: #aaa;
    font-size: 12px;
    flex-shrink: 0;

    &:hover {
      color: #dc2626;
      border-color: #dc2626;
    }
  }
}

.add-var-btn {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 12px;
  color: #8b5cf6;
  padding: 0;
  margin-left: auto;

  &:hover {
    color: #7c3aed;
  }
}

.form-error {
  background: #fef2f2;
  color: #dc2626;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 6px;
}

// System registration section
.system-registration {
  background: white;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  padding: 16px;

  .registration-header {
    margin-bottom: 12px;

    h3 {
      margin: 0 0 4px 0;
      font-size: 15px;
      font-weight: 500;
      color: #333;
      display: flex;
      align-items: center;
      gap: 8px;

      i {
        color: #059669;
      }
    }

    .registration-hint {
      font-size: 12px;
      color: #888;
    }
  }

  .registration-status {
    margin-bottom: 10px;

    .status-loading {
      font-size: 13px;
      color: #888;
      display: flex;
      align-items: center;
      gap: 6px;
    }
  }

  .status-tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 13px;
    padding: 4px 10px;
    border-radius: 4px;

    &.registered {
      background: #d1fae5;
      color: #059669;
    }

    &.not-registered {
      background: #f3f4f6;
      color: #6b7280;
    }
  }

  .registration-details {
    margin-bottom: 12px;
    font-size: 12px;

    .detail-row {
      display: flex;
      align-items: center;
      gap: 6px;
      margin-bottom: 4px;
      color: #666;

      .detail-label {
        flex-shrink: 0;
      }

      code {
        font-family: 'Consolas', 'Courier New', monospace;
        background: #f5f5f5;
        padding: 1px 6px;
        border-radius: 3px;
        font-size: 11px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .detail-ok {
        color: #059669;
        flex-shrink: 0;
      }

      .detail-missing {
        color: #d1d5db;
        flex-shrink: 0;
      }
    }
  }

  .registration-actions {
    display: flex;
    gap: 8px;
  }
}

.action-btn.register {
  background: #d1fae5;
  color: #059669;

  &:hover:not(:disabled) {
    background: #a7f3d0;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.action-btn.unregister {
  background: #fee2e2;
  color: #dc2626;

  &:hover:not(:disabled) {
    background: #fecaca;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.registration-success {
  margin-top: 8px;
  padding: 8px 12px;
  background: #d1fae5;
  color: #059669;
  border-radius: 6px;
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.btn {
  padding: 8px 20px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 6px;
  transition: background 0.2s;

  &.cancel {
    background: #f0f0f0;
    color: #666;

    &:hover {
      background: #e0e0e0;
    }
  }

  &.save {
    background: #8b5cf6;
    color: white;

    &:hover:not(:disabled) {
      background: #7c3aed;
    }

    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }

  &.delete {
    background: #dc2626;
    color: white;

    &:hover {
      background: #b91c1c;
    }
  }
}
</style>
