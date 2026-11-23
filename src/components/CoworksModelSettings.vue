<script setup lang="ts">
import { ref, computed } from 'vue'
import { useSettingsStore, type CoworksModelProvider } from '@/stores/settingsStore'

const settingsStore = useSettingsStore()

// Custom providers list
const providers = computed(() => settingsStore.coworksModelConfig?.providers || [])

// Coworks behavior settings
const defaultModel = computed({
  get: () => settingsStore.coworksModelConfig?.defaultModel || 'auto',
  set: (val: string) => settingsStore.setCoworksDefaultModel(val),
})

const maxTurns = computed({
  get: () => settingsStore.coworksModelConfig?.maxTurns ?? 30,
  set: (val: number) => settingsStore.setCoworksConfig({ maxTurns: val }),
})

const streamingEnabled = computed({
  get: () => settingsStore.coworksModelConfig?.streamingEnabled ?? true,
  set: (val: boolean) => settingsStore.setCoworksConfig({ streamingEnabled: val }),
})

// Default model options (built-in + custom providers)
const defaultModelOptions = computed(() => {
  const builtins = [
    { value: 'auto', label: 'Auto (环境变量)' },
    { value: 'Sonnet', label: 'Claude Sonnet' },
    { value: 'Opus', label: 'Claude Opus' },
    { value: 'Haiku', label: 'Claude Haiku' },
  ]
  const custom = providers.value.map(p => ({
    value: p.id,
    label: p.name,
  }))
  return [...builtins, ...custom]
})

// Add provider form state
const showAddForm = ref(false)
const editingProviderId = ref<string | null>(null)
const formName = ref('')
const formModel = ref('')
const formBaseUrl = ref('')
const formApiKey = ref('')

function openAddForm() {
  formName.value = ''
  formModel.value = ''
  formBaseUrl.value = ''
  formApiKey.value = ''
  editingProviderId.value = null
  showAddForm.value = true
}

function openEditForm(provider: CoworksModelProvider) {
  formName.value = provider.name
  formModel.value = provider.model
  formBaseUrl.value = provider.baseUrl
  formApiKey.value = provider.apiKey
  editingProviderId.value = provider.id
  showAddForm.value = true
}

function cancelForm() {
  showAddForm.value = false
  editingProviderId.value = null
}

async function saveProvider() {
  if (!formName.value.trim() || !formModel.value.trim()) return

  if (editingProviderId.value) {
    await settingsStore.updateCoworksProvider(editingProviderId.value, {
      name: formName.value.trim(),
      model: formModel.value.trim(),
      baseUrl: formBaseUrl.value.trim(),
      apiKey: formApiKey.value.trim(),
    })
  } else {
    await settingsStore.addCoworksProvider({
      name: formName.value.trim(),
      model: formModel.value.trim(),
      baseUrl: formBaseUrl.value.trim(),
      apiKey: formApiKey.value.trim(),
    })
  }

  showAddForm.value = false
  editingProviderId.value = null
}

async function removeProvider(id: string) {
  await settingsStore.removeCoworksProvider(id)
}

function maskApiKey(key: string): string {
  if (!key) return ''
  if (key.length <= 8) return '****'
  return key.slice(0, 4) + '****' + key.slice(-4)
}
</script>

<template>
  <div class="coworks-model-settings">
    <h3>对话模型设置</h3>

    <!-- General Settings -->
    <div class="setting-group">
      <label class="group-label">常规</label>

      <div class="setting-row">
        <div class="setting-label">
          <span>新会话默认模型</span>
          <span class="setting-hint">新建对话时自动使用的模型</span>
        </div>
        <select
          v-model="defaultModel"
          class="setting-select"
        >
          <option
            v-for="opt in defaultModelOptions"
            :key="opt.value"
            :value="opt.value"
          >
            {{ opt.label }}
          </option>
        </select>
      </div>

      <div class="setting-row">
        <div class="setting-label">
          <span>最大对话轮次</span>
          <span class="setting-hint">单次请求的最大 Agent 循环次数</span>
        </div>
        <input
          v-model.number="maxTurns"
          type="number"
          class="setting-input-number"
          min="1"
          max="200"
        >
      </div>

      <div class="setting-row">
        <div class="setting-label">
          <span>流式输出</span>
          <span class="setting-hint">实时显示 AI 回复内容</span>
        </div>
        <label class="toggle">
          <input
            v-model="streamingEnabled"
            type="checkbox"
          >
          <span class="toggle-slider" />
        </label>
      </div>
    </div>

    <!-- Custom Providers Section -->
    <div class="setting-group">
      <div class="section-header">
        <label class="group-label">自定义模型</label>
        <button
          v-if="!showAddForm"
          class="add-btn"
          @click="openAddForm"
        >
          <i class="fa-solid fa-plus" />
          添加
        </button>
      </div>
      <p class="section-desc">
        添加第三方模型。通过 ANTHROPIC_BASE_URL 代理转发，支持 ANTHROPIC 兼容接口。添加后可在对话窗口的模型选择器中使用。
      </p>

      <!-- Provider list -->
      <div
        v-if="providers.length > 0"
        class="provider-list"
      >
        <div
          v-for="p in providers"
          :key="p.id"
          class="provider-card"
        >
          <div class="provider-info">
            <div class="provider-name">
              {{ p.name }}
            </div>
            <div class="provider-meta">
              <span class="meta-tag">{{ p.model }}</span>
              <span
                v-if="p.baseUrl"
                class="meta-tag url"
              >{{ p.baseUrl }}</span>
              <span
                v-if="p.apiKey"
                class="meta-tag key"
              >{{ maskApiKey(p.apiKey) }}</span>
            </div>
          </div>
          <div class="provider-actions">
            <button
              class="icon-btn"
              title="编辑"
              @click="openEditForm(p)"
            >
              <i class="fa-solid fa-pen-to-square" />
            </button>
            <button
              class="icon-btn danger"
              title="删除"
              @click="removeProvider(p.id)"
            >
              <i class="fa-solid fa-trash" />
            </button>
          </div>
        </div>
      </div>

      <!-- Add/Edit Form -->
      <div
        v-if="showAddForm"
        class="provider-form"
      >
        <div class="form-title">
          {{ editingProviderId ? '编辑模型' : '添加模型' }}
        </div>
        <div class="form-field">
          <label>名称 <span class="required">*</span></label>
          <input
            v-model="formName"
            type="text"
            placeholder="例如: DeepSeek"
          >
        </div>
        <div class="form-field">
          <label>模型 ID <span class="required">*</span></label>
          <input
            v-model="formModel"
            type="text"
            placeholder="例如: deepseek-chat"
          >
        </div>
        <div class="form-field">
          <label>API Base URL</label>
          <input
            v-model="formBaseUrl"
            type="text"
            placeholder="例如: https://api.deepseek.com/v1"
          >
        </div>
        <div class="form-field">
          <label>API Key</label>
          <input
            v-model="formApiKey"
            type="password"
            placeholder="sk-xxxxx"
          >
        </div>
        <div class="form-actions">
          <button
            class="cancel-btn"
            @click="cancelForm"
          >
            取消
          </button>
          <button
            class="save-btn"
            :disabled="!formName.trim() || !formModel.trim()"
            @click="saveProvider"
          >
            {{ editingProviderId ? '保存' : '添加' }}
          </button>
        </div>
      </div>

      <!-- Empty state -->
      <div
        v-if="providers.length === 0 && !showAddForm"
        class="empty-state"
      >
        暂无自定义模型，点击上方按钮添加
      </div>
    </div>
  </div>
</template>

<style scoped>
.coworks-model-settings {
  h3 {
    margin: 0 0 20px 0;
    font-size: 18px;
    font-weight: 500;
    color: #333;
    padding-bottom: 12px;
    border-bottom: 1px solid #eee;
  }
}

.section-desc {
  font-size: 13px;
  color: #888;
  margin: 0 0 16px 0;
  line-height: 1.5;
}

.setting-group {
  margin-bottom: 32px;
}

.group-label {
  display: block;
  font-size: 14px;
  font-weight: 600;
  color: #333;
  margin-bottom: 16px;
}

/* Setting rows */
.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 0;
  border-bottom: 1px solid #f0eeeb;

  &:last-child {
    border-bottom: none;
  }
}

.setting-label {
  display: flex;
  flex-direction: column;
  gap: 2px;

  span:first-child {
    font-size: 14px;
    color: #333;
  }
}

.setting-hint {
  font-size: 12px;
  color: #aaa;
}

.setting-select {
  min-width: 180px;
  padding: 7px 12px;
  border: 1px solid #e0d8d0;
  border-radius: 6px;
  font-size: 13px;
  color: #333;
  background: #fff;
  outline: none;
  cursor: pointer;

  &:focus {
    border-color: rgba(196, 112, 75, 0.5);
  }
}

.setting-input-number {
  width: 80px;
  padding: 7px 12px;
  border: 1px solid #e0d8d0;
  border-radius: 6px;
  font-size: 13px;
  color: #333;
  background: #fff;
  outline: none;
  text-align: center;

  &:focus {
    border-color: rgba(196, 112, 75, 0.5);
  }
}

/* Toggle switch */
.toggle {
  position: relative;
  display: inline-block;
  width: 42px;
  height: 24px;
  cursor: pointer;

  input {
    opacity: 0;
    width: 0;
    height: 0;
  }
}

.toggle-slider {
  position: absolute;
  inset: 0;
  background: #ddd;
  border-radius: 12px;
  transition: background 0.2s;

  &::before {
    content: '';
    position: absolute;
    width: 18px;
    height: 18px;
    left: 3px;
    bottom: 3px;
    background: #fff;
    border-radius: 50%;
    transition: transform 0.2s;
  }

  input:checked + & {
    background: #c4704b;
  }

  input:checked + &::before {
    transform: translateX(18px);
  }
}

/* Section header */
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.add-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  background: #f5f0ec;
  border: 1px solid #e0d8d0;
  border-radius: 6px;
  color: #666;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s;

  &:hover {
    background: #ebe4dc;
    color: #333;
  }

  i {
    font-size: 11px;
  }
}

/* Provider list */
.provider-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 12px;
}

.provider-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: #fff;
  border: 1px solid #e8e5e0;
  border-radius: 8px;
  transition: border-color 0.15s;

  &:hover {
    border-color: #d0ccc7;
  }
}

.provider-info {
  flex: 1;
  min-width: 0;
}

.provider-name {
  font-size: 14px;
  font-weight: 500;
  color: #333;
  margin-bottom: 4px;
}

.provider-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.meta-tag {
  display: inline-block;
  padding: 2px 8px;
  font-size: 11px;
  border-radius: 4px;
  background: #f0eeeb;
  color: #666;

  &.url {
    color: #2563eb;
    background: #eff6ff;
  }

  &.key {
    color: #16a34a;
    background: #f0fdf4;
    font-family: monospace;
  }
}

.provider-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.15s;

  .provider-card:hover & {
    opacity: 1;
  }
}

.icon-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  border-radius: 6px;
  color: #888;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.15s;

  &:hover {
    background: rgba(0, 0, 0, 0.06);
    color: #333;
  }

  &.danger:hover {
    background: rgba(220, 38, 38, 0.08);
    color: #dc2626;
  }
}

/* Provider form */
.provider-form {
  background: #fafaf8;
  border: 1px solid #e8e5e0;
  border-radius: 10px;
  padding: 20px;
  margin-top: 12px;
}

.form-title {
  font-size: 14px;
  font-weight: 500;
  color: #333;
  margin-bottom: 16px;
}

.form-field {
  margin-bottom: 14px;

  label {
    display: block;
    font-size: 13px;
    font-weight: 500;
    color: #555;
    margin-bottom: 6px;
  }

  .required {
    color: #dc2626;
  }

  input {
    width: 100%;
    max-width: 400px;
    padding: 8px 12px;
    border: 1px solid #e0d8d0;
    border-radius: 6px;
    font-size: 13px;
    color: #333;
    background: #fff;
    outline: none;
    transition: border-color 0.15s;

    &:focus {
      border-color: rgba(196, 112, 75, 0.5);
    }

    &::placeholder {
      color: #bbb;
    }
  }
}

.form-actions {
  display: flex;
  gap: 8px;
  margin-top: 18px;
}

.cancel-btn {
  padding: 8px 16px;
  border: 1px solid #e0d8d0;
  border-radius: 6px;
  background: #fff;
  color: #666;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s;

  &:hover {
    background: #f5f5f5;
  }
}

.save-btn {
  padding: 8px 20px;
  border: none;
  border-radius: 6px;
  background: #c4704b;
  color: #fff;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;

  &:hover {
    background: #b56340;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.empty-state {
  text-align: center;
  padding: 24px;
  color: #aaa;
  font-size: 13px;
  background: #fafaf8;
  border: 1px dashed #e0d8d0;
  border-radius: 8px;
  margin-top: 12px;
}
</style>
