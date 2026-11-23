<template>
  <div class="llm-router">
    <!-- Header -->
    <div class="router-header">
      <div class="header-title">
        <h2>
          <i class="fa-solid fa-route" />
          LLM 路由
        </h2>
        <span
          class="status-badge"
          :class="{ running: serviceStatus.running, stopped: !serviceStatus.running }"
        >
          <i
            class="fa-solid"
            :class="serviceStatus.running ? 'fa-circle-check' : 'fa-circle-xmark'"
          />
          {{ serviceStatus.running ? '运行中' : '未运行' }}
        </span>
      </div>
      <div class="header-actions">
        <button
          class="action-btn"
          :class="{ stop: serviceStatus.running, start: !serviceStatus.running }"
          :disabled="actionLoading || autoRestartInProgress"
          @click="toggleService"
        >
          <i
            class="fa-solid"
            :class="getToggleButtonIcon"
          />
          {{ serviceStatus.running ? '停止服务' : '启动服务' }}
        </button>
        <button
          v-if="serviceStatus.running"
          class="action-btn restart"
          :disabled="actionLoading || autoRestartInProgress"
          @click="restartService"
        >
          <i
            class="fa-solid"
            :class="actionLoading ? 'fa-spinner fa-spin' : 'fa-rotate'"
          />
          重启服务
        </button>
        <button
          class="refresh-btn"
          :disabled="checking"
          @click="checkStatus"
        >
          <i
            class="fa-solid"
            :class="checking ? 'fa-spinner fa-spin' : 'fa-rotate'"
          />
          刷新
        </button>
        <button
          class="debug-btn"
          @click="toggleDebugPanel"
        >
          <i class="fa-solid fa-bug" />
          调试
        </button>
      </div>
    </div>

    <!-- Tab Navigation -->
    <div class="tab-nav">
      <button
        class="tab-btn"
        :class="{ active: activeTab === 'config' }"
        @click="activeTab = 'config'"
      >
        <i class="fa-solid fa-sliders" />
        配置
      </button>
      <button
        class="tab-btn"
        :class="{ active: activeTab === 'router' }"
        @click="activeTab = 'router'"
      >
        <i class="fa-solid fa-network-wired" />
        路由器
      </button>
    </div>

    <!-- Tab Content -->
    <div class="tab-content">
      <!-- Config Tab -->
      <div
        v-show="activeTab === 'config'"
        class="config-tab"
      >
        <!-- Debug Panel -->
        <div
          v-if="showDebugPanel"
          class="debug-panel"
        >
          <div class="debug-header">
            <span><i class="fa-solid fa-bug" /> CCR 调试信息</span>
            <button
              class="close-btn"
              @click="showDebugPanel = false"
            >
              <i class="fa-solid fa-times" />
            </button>
          </div>
          <div class="debug-content">
            <div
              v-if="debugLoading"
              class="debug-loading"
            >
              <i class="fa-solid fa-spinner fa-spin" /> 加载中...
            </div>
            <template v-else-if="debugInfo">
              <div class="debug-item">
                <label>Node.js 路径:</label>
                <code>{{ debugInfo.node_path || '未找到' }}</code>
              </div>
              <div class="debug-item">
                <label>CCR 路径:</label>
                <code>{{ debugInfo.ccr_path || '未找到' }}</code>
              </div>
              <div class="debug-item">
                <label>CCR Bin 路径:</label>
                <code>{{ debugInfo.ccr_bin_path || '未找到' }}</code>
              </div>
              <div class="debug-item">
                <label>配置文件路径:</label>
                <code>{{ debugInfo.config_path || '未找到' }}</code>
              </div>
              <div class="debug-item">
                <label>ANTHROPIC_MODEL (Rust 进程):</label>
                <code>{{ debugInfo.anthropic_model_env || '未设置' }}</code>
              </div>
              <div class="debug-item">
                <label>ANTHROPIC_BASE_URL (Rust 进程):</label>
                <code>{{ debugInfo.anthropic_base_url_env || '未设置' }}</code>
              </div>
              <div class="debug-item">
                <label>ANTHROPIC_MODEL (Settings Store):</label>
                <code>{{ currentAnthropicModel || '未设置' }}</code>
              </div>
              <div class="debug-item full-width">
                <label>配置文件内容:</label>
                <pre>{{ debugInfo.config_content || '无法读取' }}</pre>
              </div>
            </template>
            <div
              v-else
              class="debug-error"
            >
              无法获取调试信息
            </div>
            <button
              class="refresh-debug-btn"
              @click="loadDebugInfo"
            >
              <i class="fa-solid fa-rotate" /> 刷新调试信息
            </button>
          </div>
        </div>

        <!-- Model Provider Selection -->
        <div class="model-provider-section">
          <div class="section-header">
            <i class="fa-solid fa-robot" />
            <span>模型提供商</span>
            <span
              v-if="currentAnthropicModel"
              class="current-model"
            >
              当前: <code>{{ currentAnthropicModel }}</code>
            </span>
          </div>
          <div class="provider-options">
            <div
              v-for="provider in modelProviders"
              :key="provider.id"
              class="provider-option"
              :class="{ active: selectedProvider === provider.id }"
              @click="selectProvider(provider.id)"
            >
              <div class="radio-circle">
                <div
                  v-if="selectedProvider === provider.id"
                  class="radio-dot"
                />
              </div>
              <i
                :class="provider.icon"
                :style="{ color: provider.color }"
              />
              <div class="provider-info">
                <span class="provider-name">{{ provider.name }}</span>
                <input
                  v-model="provider.model"
                  type="text"
                  class="provider-model-input"
                  :title="'点击编辑模型名称'"
                  @click.stop
                  @change="onModelChange(provider)"
                  @keyup.enter="($event.target as HTMLInputElement).blur()"
                >
              </div>
            </div>
          </div>
          <p class="provider-hint">
            <i class="fa-solid fa-info-circle" />
            选择模型提供商后，使用 mcd 命令启动的 Claude Code 将通过路由使用对应模型
          </p>
        </div>

        <!-- mcd CLI Installation -->
        <div class="mcd-cli-section">
          <div class="section-header">
            <i class="fa-solid fa-terminal" />
            <span>mcd CLI (LLM 路由)</span>
          </div>
          <p
            class="provider-hint"
            style="margin-top: 0;"
          >
            <i class="fa-solid fa-info-circle" />
            mcd 命令用于通过 LLM Router 调用 Claude，自动注入环境变量和启用路由功能。
          </p>
          <div class="mcd-actions">
            <button
              class="action-btn start"
              :disabled="mcdInstalling"
              @click="installMcdCli"
            >
              <i
                v-if="mcdInstalling"
                class="fa-solid fa-spinner fa-spin"
              />
              <i
                v-else
                class="fa-solid fa-download"
              />
              {{ mcdInstalling ? '安装中...' : '安装 mcd CLI' }}
            </button>
          </div>
          <div
            v-if="mcdError"
            class="mcd-error-banner"
          >
            <i class="fa-solid fa-exclamation-triangle" />
            <span>{{ mcdError }}</span>
            <button
              class="close-btn"
              @click="mcdError = ''"
            >
              <i class="fa-solid fa-times" />
            </button>
          </div>
          <div
            v-if="mcdSuccess"
            class="mcd-success-banner"
          >
            <i class="fa-solid fa-check-circle" />
            <span>mcd CLI 安装成功！请打开新终端使用 <code>mcd</code> 命令。</span>
          </div>
        </div>

        <!-- Thinking Level Section -->
        <div
          v-if="selectedProvider === 'openai'"
          class="thinking-level-section"
        >
          <div class="section-header">
            <i class="fa-solid fa-brain" />
            <span>推理深度 (Reasoning Level)</span>
            <span class="current-level">
              当前: <code>{{ selectedThinkingLevel }}</code>
            </span>
          </div>
          <div class="level-options">
            <div
              v-for="level in thinkingLevels"
              :key="level.id"
              class="level-option"
              :class="{ active: selectedThinkingLevel === level.id }"
              @click="selectThinkingLevel(level.id)"
            >
              <div class="radio-circle">
                <div
                  v-if="selectedThinkingLevel === level.id"
                  class="radio-dot"
                />
              </div>
              <span class="level-name">{{ level.name }}</span>
              <span class="level-desc">{{ level.description }}</span>
            </div>
          </div>
          <p class="level-hint">
            <i class="fa-solid fa-info-circle" />
            推理深度影响 GPT-5.2 的思考过程，更高的深度会产生更详细的推理，但响应更慢。修改后需重启服务生效
          </p>
        </div>
      </div>

      <!-- Router Tab -->
      <div
        v-show="activeTab === 'router'"
        class="router-tab"
      >
        <!-- Service Not Running State -->
        <template v-if="!serviceStatus.running">
          <div class="not-running-state">
            <div class="state-icon">
              <i class="fa-solid fa-power-off" />
            </div>
            <h3>服务未运行</h3>
            <p>点击"启动服务"按钮启动 LLM 路由服务</p>
            <button
              class="start-btn"
              :disabled="actionLoading"
              @click="startService"
            >
              <i
                class="fa-solid"
                :class="actionLoading ? 'fa-spinner fa-spin' : 'fa-play'"
              />
              启动服务
            </button>
            <p class="hint">
              <i class="fa-solid fa-info-circle" />
              LLM 路由服务允许您将 Claude Code 请求路由到其他 LLM 提供商
            </p>
          </div>
        </template>

        <!-- Service Running - Show WebView -->
        <template v-else>
          <div class="webview-container">
            <iframe
              :src="routerUrl"
              frameborder="0"
              @load="onIframeLoad"
              @error="onIframeError"
            />
            <div
              v-if="iframeLoading"
              class="iframe-loading"
            >
              <i class="fa-solid fa-spinner fa-spin" />
              <span>加载中...</span>
            </div>
          </div>
        </template>
      </div>
    </div>

    <!-- Auto-restart notification -->
    <div
      v-if="autoRestartInProgress"
      class="info-banner"
    >
      <i class="fa-solid fa-spinner fa-spin" />
      <span>检测到服务停止，正在自动重启...</span>
    </div>

    <!-- Error Message -->
    <div
      v-if="errorMessage"
      class="error-banner"
    >
      <i class="fa-solid fa-exclamation-triangle" />
      <span>{{ errorMessage }}</span>
      <button
        class="close-btn"
        @click="errorMessage = ''"
      >
        <i class="fa-solid fa-times" />
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useSettingsStore } from '@/stores/settingsStore'

// Types
interface LlmRouterStatus {
  running: boolean
  port: number | null
  error: string | null
}

interface ModelProvider {
  id: string
  name: string
  model: string
  icon: string
  color: string
}

interface CcrDebugInfo {
  node_path: string | null
  ccr_path: string | null
  ccr_bin_path: string | null
  config_path: string | null
  config_content: string | null
  anthropic_model_env: string | null
  anthropic_base_url_env: string | null
}

// Store
const settingsStore = useSettingsStore()

// Default model providers configuration
const defaultModelProviders: ModelProvider[] = [
  {
    id: 'claude',
    name: 'Claude',
    model: 'Sonnet',
    icon: 'fa-solid fa-robot',
    color: '#d97706'
  },
  {
    id: 'openai',
    name: 'OpenAI',
    model: 'gpt-5.2',
    icon: 'fa-solid fa-brain',
    color: '#10a37f'
  },
  {
    id: 'gemini',
    name: 'Gemini',
    model: 'gemini-3-pro-preview',
    icon: 'fa-solid fa-gem',
    color: '#4285f4'
  }
]

// Normalize model name (strip legacy provider prefix if present)
const normalizeModelName = (model: string): string => {
  // If model has comma (legacy format like "moonton,model-name"), extract just the model name
  if (model.includes(',')) {
    return model.split(',').pop() || model
  }
  return model
}

// Load saved model providers from settingsStore.llmRouterConfig
const loadModelProviders = (): ModelProvider[] => {
  const savedConfig = settingsStore.llmRouterConfig
  if (savedConfig?.modelProviders) {
    return defaultModelProviders.map(p => {
      const savedModel = savedConfig.modelProviders[p.id]
      if (savedModel) {
        // Migrate legacy model values (strip provider prefix if present)
        return {
          ...p,
          model: normalizeModelName(savedModel)
        }
      }
      return { ...p }
    })
  }
  return [...defaultModelProviders]
}

// Save model providers to settingsStore.llmRouterConfig
const saveModelProviders = () => {
  const models: Record<string, string> = {}
  modelProviders.value.forEach(p => {
    models[p.id] = p.model
  })
  settingsStore.setLlmRouterConfig({ modelProviders: models })
}

// Model providers configuration
const modelProviders = ref<ModelProvider[]>(loadModelProviders())

// Selected provider state
const selectedProvider = ref<string>('claude')

// Thinking levels configuration
const thinkingLevels = [
  { id: 'none', name: '无', description: '不使用推理' },
  { id: 'low', name: '低', description: '快速响应，适合简单任务' },
  { id: 'medium', name: '中等', description: '平衡速度与深度' },
  { id: 'high', name: '高', description: '深度推理，适合复杂任务' }
] as const

// Selected thinking level
const selectedThinkingLevel = ref<'none' | 'low' | 'medium' | 'high'>('medium')

// Current ANTHROPIC_MODEL value (from llmRouterConfig, not environmentVariables)
const currentAnthropicModel = computed(() => {
  const config = settingsStore.llmRouterConfig
  if (config?.selectedProviderId && config?.modelProviders) {
    return config.modelProviders[config.selectedProviderId] || ''
  }
  return ''
})

// Initialize selected provider based on llmRouterConfig
const initSelectedProvider = () => {
  const config = settingsStore.llmRouterConfig
  if (config?.selectedProviderId) {
    selectedProvider.value = config.selectedProviderId
  } else {
    selectedProvider.value = 'claude'
  }
}

// Initialize thinking level from store
const initThinkingLevel = () => {
  const config = settingsStore.llmRouterConfig
  if (config?.thinkingLevel) {
    selectedThinkingLevel.value = config.thinkingLevel
  }
}

// Select thinking level
const selectThinkingLevel = (level: 'none' | 'low' | 'medium' | 'high') => {
  selectedThinkingLevel.value = level
  settingsStore.setLlmRouterConfig({ thinkingLevel: level })
}

// Select a provider and update llmRouterConfig (not environmentVariables)
const selectProvider = (providerId: string) => {
  selectedProvider.value = providerId
  const provider = modelProviders.value.find(p => p.id === providerId)
  if (provider) {
    settingsStore.setLlmRouterProvider(providerId, provider.model)
  }
}

// Handle model name change
const onModelChange = (provider: ModelProvider) => {
  // Save to settingsStore.llmRouterConfig
  saveModelProviders()
  // If this is the currently selected provider, ensure llmRouterConfig is updated
  if (selectedProvider.value === provider.id) {
    settingsStore.setLlmRouterProvider(provider.id, provider.model)
  }
}

// Install mcd CLI
const mcdInstalling = ref(false)
const mcdError = ref('')
const mcdSuccess = ref(false)

const installMcdCli = async () => {
  mcdInstalling.value = true
  mcdError.value = ''
  mcdSuccess.value = false
  try {
    await invoke<string>('install_mcd_cli_cmd')
    mcdSuccess.value = true
  } catch (e) {
    mcdError.value = String(e)
  } finally {
    mcdInstalling.value = false
  }
}

// State
const checking = ref(false)
const actionLoading = ref(false)
const iframeLoading = ref(true)
const errorMessage = ref('')
const activeTab = ref<'config' | 'router'>('config')
const serviceStatus = ref<LlmRouterStatus>({
  running: false,
  port: null,
  error: null
})

// Auto-restart watchdog state
const previousRunning = ref(false)
const autoRestartInProgress = ref(false)
const autoRestartCount = ref(0)
const MAX_AUTO_RESTARTS = 3

// Debug state
const showDebugPanel = ref(false)
const debugLoading = ref(false)
const debugInfo = ref<CcrDebugInfo | null>(null)

// Debug methods
const toggleDebugPanel = () => {
  showDebugPanel.value = !showDebugPanel.value
  if (showDebugPanel.value && !debugInfo.value) {
    loadDebugInfo()
  }
}

const loadDebugInfo = async () => {
  debugLoading.value = true
  try {
    debugInfo.value = await invoke<CcrDebugInfo>('get_ccr_debug_info')
  } catch (e) {
    console.error('Failed to load debug info:', e)
    errorMessage.value = `获取调试信息失败: ${e}`
  } finally {
    debugLoading.value = false
  }
}

// Computed
const routerUrl = computed(() => {
  const port = serviceStatus.value.port || 3456
  return `http://127.0.0.1:${port}/ui`
})

const getToggleButtonIcon = computed(() => {
  if (actionLoading.value) {
    return 'fa-spinner fa-spin'
  }
  return serviceStatus.value.running ? 'fa-stop' : 'fa-play'
})

// Methods
const checkServiceStatus = async () => {
  const wasRunning = previousRunning.value
  try {
    const status = await invoke<LlmRouterStatus>('check_llm_router_status')
    serviceStatus.value = status
    if (status.error) {
      errorMessage.value = status.error
    }

    const isRunning = status.running

    // Reset auto-restart counter when service is confirmed running
    if (isRunning) {
      autoRestartCount.value = 0
    }

    // Watchdog: detect unexpected service stop and auto-restart
    if (wasRunning && !isRunning && !actionLoading.value && !autoRestartInProgress.value) {
      if (autoRestartCount.value < MAX_AUTO_RESTARTS) {
        performAutoRestart()
      } else {
        errorMessage.value = `服务连续 ${MAX_AUTO_RESTARTS} 次重启失败，请手动检查`
      }
    }

    previousRunning.value = isRunning
  } catch (e) {
    console.error('Failed to check service status:', e)
    serviceStatus.value = { running: false, port: null, error: String(e) }
    previousRunning.value = false
  }
}

const checkStatus = async () => {
  checking.value = true
  errorMessage.value = ''
  try {
    await checkServiceStatus()
  } finally {
    checking.value = false
  }
}

const startService = async () => {
  actionLoading.value = true
  errorMessage.value = ''
  try {
    // Get ANTHROPIC_AUTH_TOKEN from settings store to pass to Rust backend
    const authToken = settingsStore.environmentVariables['ANTHROPIC_AUTH_TOKEN'] || null
    // Get thinking level for GPT-5.2 reasoning effort
    const thinkingLevel = settingsStore.llmRouterConfig.thinkingLevel || 'medium'
    await invoke('start_llm_router', { authToken, thinkingLevel })
    // Wait a moment for the service to start
    await new Promise(resolve => setTimeout(resolve, 1500))
    await checkServiceStatus()
    if (!serviceStatus.value.running) {
      errorMessage.value = '服务启动后未能检测到运行状态，请稍后刷新'
    }
  } catch (e) {
    console.error('Failed to start service:', e)
    errorMessage.value = `启动失败: ${e}`
  } finally {
    actionLoading.value = false
  }
}

const stopService = async () => {
  actionLoading.value = true
  errorMessage.value = ''
  try {
    await invoke('stop_llm_router')
    // Wait a moment for the service to stop
    await new Promise(resolve => setTimeout(resolve, 500))
    await checkServiceStatus()
  } catch (e) {
    console.error('Failed to stop service:', e)
    errorMessage.value = `停止失败: ${e}`
  } finally {
    actionLoading.value = false
  }
}

const toggleService = async () => {
  if (serviceStatus.value.running) {
    await stopService()
  } else {
    await startService()
  }
}

const onIframeLoad = () => {
  iframeLoading.value = false
}

const onIframeError = () => {
  iframeLoading.value = false
  // If service was running and iframe fails, it likely means the CCR process died
  // (e.g. from "保存并重启"). Trigger immediate status check to kick off auto-restart.
  if (previousRunning.value && !actionLoading.value && !autoRestartInProgress.value) {
    checkServiceStatus()
  } else {
    errorMessage.value = '无法加载路由配置界面，请检查服务是否正常运行'
  }
}

// Reload the CCR iframe to pick up new config after restart
const reloadIframe = () => {
  const iframe = document.querySelector('.webview-container iframe') as HTMLIFrameElement
  if (iframe) {
    const src = iframe.src
    iframe.src = 'about:blank'
    setTimeout(() => { iframe.src = src }, 100)
  }
}

// Auto-restart after detecting unexpected service stop (e.g. CCR "Save and Restart")
const performAutoRestart = async () => {
  autoRestartInProgress.value = true
  autoRestartCount.value += 1
  errorMessage.value = ''

  try {
    // skip_config_update=true preserves user's CCR web UI config changes
    await invoke('restart_llm_router', { skipConfigUpdate: true })

    // Wait for the service to come back up
    await new Promise(resolve => setTimeout(resolve, 2000))
    await checkServiceStatus()

    if (serviceStatus.value.running) {
      reloadIframe()
    }
  } catch (e) {
    console.error('Auto-restart failed:', e)
    errorMessage.value = `自动重启失败: ${e}`
  } finally {
    autoRestartInProgress.value = false
  }
}

// Manual restart from the header button — applies settings store config
const restartService = async () => {
  actionLoading.value = true
  errorMessage.value = ''

  try {
    const authToken = settingsStore.environmentVariables['ANTHROPIC_AUTH_TOKEN'] || null
    const thinkingLevel = settingsStore.llmRouterConfig.thinkingLevel || 'medium'

    await invoke('restart_llm_router', {
      authToken,
      thinkingLevel,
      skipConfigUpdate: false
    })

    await new Promise(resolve => setTimeout(resolve, 2000))
    await checkServiceStatus()

    if (serviceStatus.value.running) {
      reloadIframe()
    } else {
      errorMessage.value = '服务重启后未能检测到运行状态，请稍后刷新'
    }
  } catch (e) {
    console.error('Failed to restart service:', e)
    errorMessage.value = `重启失败: ${e}`
  } finally {
    actionLoading.value = false
  }
}

// Polling to keep status updated
let statusInterval: number | null = null

const SLOW_POLL_MS = 10000
const FAST_POLL_MS = 2000

const startPolling = (intervalMs: number) => {
  if (statusInterval) {
    clearInterval(statusInterval)
  }
  statusInterval = window.setInterval(() => {
    checkServiceStatus()
  }, intervalMs)
}

// Keep fast polling the entire time the router tab is active (iframe visible),
// so auto-restart triggers within ~2s of CCR "保存并重启" killing the process.
// Revert to slow polling when switching away.
watch(activeTab, (tab) => {
  if (tab === 'router') {
    startPolling(FAST_POLL_MS)
  } else {
    startPolling(SLOW_POLL_MS)
  }
})

// Lifecycle
onMounted(() => {
  initSelectedProvider()
  initThinkingLevel()
  checkStatus()
  startPolling(activeTab.value === 'router' ? FAST_POLL_MS : SLOW_POLL_MS)
})

onUnmounted(() => {
  if (statusInterval) {
    clearInterval(statusInterval)
  }
})
</script>

<style scoped lang="less">
.llm-router {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #f5f5f5;
  position: relative;
}

.router-header {
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

    .status-badge {
      font-size: 13px;
      padding: 4px 12px;
      border-radius: 12px;
      display: flex;
      align-items: center;
      gap: 6px;

      &.running {
        background: #dcfce7;
        color: #16a34a;
      }

      &.stopped {
        background: #fee2e2;
        color: #dc2626;
      }
    }
  }

  .header-actions {
    display: flex;
    gap: 8px;
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

    &.start {
      background: #dcfce7;
      color: #16a34a;

      &:hover:not(:disabled) {
        background: #bbf7d0;
      }
    }

    &.stop {
      background: #fee2e2;
      color: #dc2626;

      &:hover:not(:disabled) {
        background: #fecaca;
      }
    }

    &.restart {
      background: #fef3c7;
      color: #d97706;

      &:hover:not(:disabled) {
        background: #fde68a;
      }
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }

  .refresh-btn {
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    display: flex;
    align-items: center;
    gap: 6px;
    background: #eff6ff;
    color: #3b82f6;
    transition: background 0.2s;

    &:hover:not(:disabled) {
      background: #dbeafe;
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }

  .debug-btn {
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    display: flex;
    align-items: center;
    gap: 6px;
    background: #fef3c7;
    color: #d97706;
    transition: background 0.2s;

    &:hover {
      background: #fde68a;
    }
  }
}

// Tab navigation
.tab-nav {
  display: flex;
  background: white;
  border-bottom: 1px solid #e0e0e0;
  flex-shrink: 0;
  padding: 0 16px;

  .tab-btn {
    padding: 12px 20px;
    border: none;
    background: transparent;
    cursor: pointer;
    font-size: 14px;
    color: #666;
    display: flex;
    align-items: center;
    gap: 8px;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    transition: all 0.2s;

    &:hover {
      color: #8b5cf6;
    }

    &.active {
      color: #8b5cf6;
      border-bottom-color: #8b5cf6;
      font-weight: 500;
    }

    i {
      font-size: 14px;
    }
  }
}

// Tab content container
.tab-content {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

// Config tab (scrollable)
.config-tab {
  flex: 1;
  overflow-y: auto;
  background: #f5f5f5;
}

// Router tab (full height for iframe)
.router-tab {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

// Debug panel
.debug-panel {
  background: #fffbeb;
  border-bottom: 1px solid #fde68a;
  max-height: 400px;
  overflow: auto;

  .debug-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 20px;
    background: #fef3c7;
    font-weight: 500;
    color: #92400e;

    span {
      display: flex;
      align-items: center;
      gap: 8px;
    }

    .close-btn {
      background: transparent;
      border: none;
      cursor: pointer;
      color: #92400e;
      padding: 4px 8px;
      border-radius: 4px;

      &:hover {
        background: #fde68a;
      }
    }
  }

  .debug-content {
    padding: 16px 20px;
  }

  .debug-loading {
    text-align: center;
    color: #92400e;
    padding: 20px;
  }

  .debug-item {
    display: flex;
    gap: 12px;
    margin-bottom: 10px;
    align-items: flex-start;

    label {
      font-weight: 500;
      color: #78350f;
      min-width: 200px;
      flex-shrink: 0;
    }

    code {
      background: #fef3c7;
      padding: 4px 8px;
      border-radius: 4px;
      font-family: monospace;
      font-size: 12px;
      color: #92400e;
      word-break: break-all;
    }

    pre {
      background: #fef3c7;
      padding: 12px;
      border-radius: 6px;
      font-family: monospace;
      font-size: 11px;
      color: #78350f;
      overflow-x: auto;
      margin: 4px 0 0 0;
      white-space: pre-wrap;
      word-break: break-all;
      max-height: 200px;
      overflow-y: auto;
    }

    &.full-width {
      flex-direction: column;
      gap: 4px;
    }
  }

  .debug-error {
    text-align: center;
    color: #dc2626;
    padding: 20px;
  }

  .refresh-debug-btn {
    margin-top: 16px;
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    display: flex;
    align-items: center;
    gap: 6px;
    background: #fde68a;
    color: #92400e;
    transition: background 0.2s;

    &:hover {
      background: #fcd34d;
    }
  }
}

// Model provider selection
.model-provider-section {
  padding: 16px 20px;
  background: white;
  border-bottom: 1px solid #e0e0e0;

  .section-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
    font-weight: 500;
    color: #333;

    i {
      color: #8b5cf6;
      width: 18px;
      text-align: center;
    }

    .current-model {
      margin-left: auto;
      font-size: 12px;
      font-weight: normal;
      color: #666;

      code {
        background: #f3f4f6;
        padding: 2px 8px;
        border-radius: 4px;
        font-family: monospace;
        color: #8b5cf6;
      }
    }
  }

  .provider-options {
    display: flex;
    gap: 12px;
  }

  .provider-option {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border: 2px solid #e5e7eb;
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.2s;
    background: #fafafa;

    &:hover {
      border-color: #a78bfa;
      background: #faf5ff;
    }

    &.active {
      border-color: #8b5cf6;
      background: #f3e8ff;
    }

    .radio-circle {
      width: 18px;
      height: 18px;
      border: 2px solid #d1d5db;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      flex-shrink: 0;

      .radio-dot {
        width: 10px;
        height: 10px;
        background: #8b5cf6;
        border-radius: 50%;
      }
    }

    &.active .radio-circle {
      border-color: #8b5cf6;
    }

    > i {
      font-size: 20px;
      flex-shrink: 0;
    }

    .provider-info {
      display: flex;
      flex-direction: column;
      gap: 2px;
      min-width: 0;

      .provider-name {
        font-weight: 500;
        color: #333;
        font-size: 14px;
      }

      .provider-model-input {
        font-family: monospace;
        font-size: 11px;
        color: #666;
        background: transparent;
        border: 1px solid transparent;
        border-radius: 4px;
        padding: 2px 6px;
        margin: -2px -6px;
        width: calc(100% + 12px);
        outline: none;
        transition: all 0.2s;

        &:hover {
          background: rgba(139, 92, 246, 0.08);
          border-color: #e5e7eb;
        }

        &:focus {
          background: white;
          border-color: #8b5cf6;
          color: #333;
          box-shadow: 0 0 0 2px rgba(139, 92, 246, 0.2);
        }
      }
    }
  }

  .provider-hint {
    margin: 12px 0 0 0;
    font-size: 12px;
    color: #888;
    display: flex;
    align-items: center;
    gap: 6px;

    i {
      color: #3b82f6;
    }
  }
}

// mcd CLI installation section
.mcd-cli-section {
  padding: 16px 20px;
  background: white;
  border-bottom: 1px solid #e0e0e0;

  .section-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
    font-weight: 500;
    color: #333;

    i {
      color: #8b5cf6;
      width: 18px;
      text-align: center;
    }
  }

  .provider-hint {
    margin: 0 0 12px 0;
    font-size: 12px;
    color: #888;
    display: flex;
    align-items: center;
    gap: 6px;

    i {
      color: #3b82f6;
    }
  }

  .mcd-actions {
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

      &.start {
        background: #dcfce7;
        color: #16a34a;

        &:hover:not(:disabled) {
          background: #bbf7d0;
        }
      }

      &:disabled {
        opacity: 0.6;
        cursor: not-allowed;
      }
    }
  }

  .mcd-error-banner {
    margin-top: 8px;
    padding: 10px 14px;
    background: #fee2e2;
    border: 1px solid #fecaca;
    border-radius: 8px;
    display: flex;
    align-items: center;
    gap: 10px;
    color: #dc2626;
    font-size: 13px;

    i {
      flex-shrink: 0;
    }

    span {
      flex: 1;
    }

    .close-btn {
      padding: 4px 8px;
      border: none;
      border-radius: 4px;
      background: transparent;
      color: #dc2626;
      cursor: pointer;
      transition: background 0.2s;

      &:hover {
        background: #fecaca;
      }
    }
  }

  .mcd-success-banner {
    margin-top: 8px;
    padding: 10px 14px;
    background: #dcfce7;
    border: 1px solid #bbf7d0;
    border-radius: 8px;
    display: flex;
    align-items: center;
    gap: 10px;
    color: #16a34a;
    font-size: 13px;

    i {
      flex-shrink: 0;
    }

    code {
      background: #bbf7d0;
      padding: 2px 6px;
      border-radius: 4px;
      font-family: monospace;
      font-size: 12px;
    }
  }
}

// Thinking level section
.thinking-level-section {
  padding: 16px 20px;
  background: white;
  border-bottom: 1px solid #e0e0e0;

  .section-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
    font-weight: 500;
    color: #333;

    i {
      color: #10a37f;
      width: 18px;
      text-align: center;
    }

    .current-level {
      margin-left: auto;
      font-size: 12px;
      font-weight: normal;
      color: #666;

      code {
        background: #f3f4f6;
        padding: 2px 8px;
        border-radius: 4px;
        font-family: monospace;
        color: #10a37f;
      }
    }
  }

  .level-options {
    display: flex;
    gap: 10px;
  }

  .level-option {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border: 2px solid #e5e7eb;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s;
    background: #fafafa;

    &:hover {
      border-color: #6ee7b7;
      background: #ecfdf5;
    }

    &.active {
      border-color: #10a37f;
      background: #d1fae5;
    }

    .radio-circle {
      width: 16px;
      height: 16px;
      border: 2px solid #d1d5db;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      flex-shrink: 0;

      .radio-dot {
        width: 8px;
        height: 8px;
        background: #10a37f;
        border-radius: 50%;
      }
    }

    &.active .radio-circle {
      border-color: #10a37f;
    }

    .level-name {
      font-weight: 500;
      color: #333;
      font-size: 13px;
    }

    .level-desc {
      font-size: 11px;
      color: #666;
    }
  }

  .level-hint {
    margin: 12px 0 0 0;
    font-size: 12px;
    color: #888;
    display: flex;
    align-items: center;
    gap: 6px;

    i {
      color: #10a37f;
    }
  }
}

// Not running state
.not-running-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  text-align: center;

  .state-icon {
    width: 80px;
    height: 80px;
    border-radius: 50%;
    background: linear-gradient(135deg, #f3e8ff 0%, #e9d5ff 100%);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 24px;

    i {
      font-size: 36px;
      color: #8b5cf6;
    }
  }

  h3 {
    margin: 0 0 8px 0;
    font-size: 20px;
    font-weight: 500;
    color: #333;
  }

  p {
    margin: 0 0 20px 0;
    font-size: 14px;
    color: #666;
  }

  .start-btn {
    padding: 12px 32px;
    border: none;
    border-radius: 8px;
    background: linear-gradient(135deg, #8b5cf6 0%, #a78bfa 100%);
    color: white;
    font-size: 15px;
    font-weight: 500;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    transition: all 0.2s;
    box-shadow: 0 4px 12px rgba(139, 92, 246, 0.3);

    &:hover:not(:disabled) {
      transform: translateY(-2px);
      box-shadow: 0 6px 16px rgba(139, 92, 246, 0.4);
    }

    &:disabled {
      opacity: 0.7;
      cursor: not-allowed;
      transform: none;
    }
  }

  .hint {
    margin-top: 24px;
    font-size: 13px;
    color: #888;
    display: flex;
    align-items: center;
    gap: 6px;

    i {
      color: #3b82f6;
    }
  }
}

// WebView container
.webview-container {
  flex: 1;
  position: relative;
  background: white;

  iframe {
    width: 100%;
    height: 100%;
    border: none;
  }

  .iframe-loading {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(255, 255, 255, 0.9);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: #666;

    i {
      font-size: 24px;
      color: #8b5cf6;
    }

    span {
      font-size: 14px;
    }
  }
}

// Info banner (auto-restart notification)
.info-banner {
  position: absolute;
  bottom: 70px;
  left: 20px;
  right: 20px;
  padding: 12px 16px;
  background: #fef3c7;
  border: 1px solid #fde68a;
  border-radius: 8px;
  display: flex;
  align-items: center;
  gap: 10px;
  color: #92400e;
  font-size: 14px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  z-index: 10;

  i {
    flex-shrink: 0;
    color: #d97706;
  }

  span {
    flex: 1;
  }
}

// Error banner
.error-banner {
  position: absolute;
  bottom: 20px;
  left: 20px;
  right: 20px;
  padding: 12px 16px;
  background: #fee2e2;
  border: 1px solid #fecaca;
  border-radius: 8px;
  display: flex;
  align-items: center;
  gap: 10px;
  color: #dc2626;
  font-size: 14px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);

  i {
    flex-shrink: 0;
  }

  span {
    flex: 1;
  }

  .close-btn {
    padding: 4px 8px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: #dc2626;
    cursor: pointer;
    transition: background 0.2s;

    &:hover {
      background: #fecaca;
    }
  }
}
</style>
