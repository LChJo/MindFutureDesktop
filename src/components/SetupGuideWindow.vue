<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import PluginMarketFull from './PluginMarketFull.vue'
import { useSetupGuide } from '@/composables/useSetupGuide'
import { ENV_DEFAULTS } from '@/config/defaults'

// Use shared setup guide composable
const {
  // Download progress
  gitDownloadProgress,
  uvDownloadProgress,

  // Status flags
  isChecking,
  setupStep,
  installError,

  // Node.js state
  nodejsReady,
  nodejsVersion,

  // Git state
  gitReady,
  gitVersion,
  gitDownloading,

  // UV state
  uvReady,
  uvVersion,
  uvInstalling,

  // Claude Code state
  claudeCodeReady,
  claudeCodeInstalling,

  // Environment variables state
  envVars,

  // Computed
  allDepsReady,
  allEnvVarsReady,

  // Methods
  setupProgressListener,
  cleanupProgressListener,
  checkStatus,
  installGit,
  installUv,
  installClaudeCode,
  saveEnvVars,
  loadEnvVars,
} = useSetupGuide()

// Step management (specific to this component)
const currentStep = ref(1)  // 1: 基础依赖, 2: 插件市场, 3: 环境变量

// Plugin store overlay
const showPluginStore = ref(false)

// Check status on mount
onMounted(async () => {
  await setupProgressListener()
  await checkStatus()
  await loadEnvVars()
})

// Cleanup on unmount
onUnmounted(() => {
  cleanupProgressListener()
})

// Step 2: Go to marketplace step
const goToStep2 = () => {
  currentStep.value = 2
}

// Open plugin store overlay
const openPluginStore = () => {
  showPluginStore.value = true
}

// Close plugin store overlay
const closePluginStore = () => {
  showPluginStore.value = false
}

// Go back to step 1
const goToStep1 = () => {
  currentStep.value = 1
}

// Go to step 3 (environment variables)
const goToStep3 = () => {
  currentStep.value = 3
}

// Go back to step 2
const goBackToStep2 = () => {
  currentStep.value = 2
}
</script>

<template>
  <div class="setup-guide-window">
    <!-- Plugin Store Overlay -->
    <div
      v-if="showPluginStore"
      class="plugin-store-overlay"
    >
      <div class="plugin-store-header">
        <h2>插件商店</h2>
        <button
          class="close-btn"
          @click="closePluginStore"
        >
          <i class="fa-solid fa-times" />
        </button>
      </div>
      <div class="plugin-store-content">
        <PluginMarketFull />
      </div>
    </div>

    <!-- Main Setup Guide Content -->
    <template v-if="!showPluginStore">
      <!-- Header -->
      <div class="header">
        <img
          src="/icons/icon.png"
          alt="Moonton Claude Desktop"
          class="logo"
        >
        <div class="header-text">
          <h1>环境检查</h1>
          <p class="subtitle">
            {{ currentStep === 1 ? '检查和安装运行环境依赖' : currentStep === 2 ? '配置插件市场' : '配置环境变量' }}
          </p>
        </div>
        <!-- Step indicator -->
        <div class="step-indicator">
          <span :class="{ active: currentStep === 1, completed: currentStep > 1 }">1</span>
          <span
            class="step-line"
            :class="{ completed: currentStep > 1 }"
          />
          <span :class="{ active: currentStep === 2, completed: currentStep > 2 }">2</span>
          <span
            class="step-line"
            :class="{ completed: currentStep > 2 }"
          />
          <span :class="{ active: currentStep === 3 }">3</span>
        </div>
      </div>

      <!-- Main Content -->
      <div class="main-section">
        <!-- Step 1: Basic Dependencies -->
        <template v-if="currentStep === 1">
          <div
            v-if="isChecking"
            class="checking"
          >
            <i class="fa-solid fa-spinner fa-spin" />
            <span>{{ setupStep || '正在初始化...' }}</span>
          </div>

          <template v-else>
            <!-- Status List -->
            <div class="status-list">
              <!-- Node.js Status -->
              <div
                class="status-item"
                :class="{ ready: nodejsReady }"
              >
                <span class="status-icon">
                  <i
                    v-if="nodejsReady"
                    class="fa-solid fa-check-circle"
                  />
                  <i
                    v-else
                    class="fa-regular fa-circle"
                  />
                </span>
                <span class="status-text">
                  <span class="status-name">Node.js</span>
                  <span
                    v-if="nodejsVersion"
                    class="status-version"
                  >v{{ nodejsVersion }}</span>
                </span>
                <span
                  class="status-label"
                  :class="{ ready: nodejsReady }"
                >
                  {{ nodejsReady ? '已就绪' : '未安装' }}
                </span>
              </div>

              <!-- Git Status -->
              <div
                class="status-item"
                :class="{ ready: gitReady, installing: gitDownloading }"
              >
                <span class="status-icon">
                  <i
                    v-if="gitDownloading"
                    class="fa-solid fa-spinner fa-spin"
                  />
                  <i
                    v-else-if="gitReady"
                    class="fa-solid fa-check-circle"
                  />
                  <i
                    v-else
                    class="fa-regular fa-circle"
                  />
                </span>
                <span class="status-text">
                  <span class="status-name">Git</span>
                  <span
                    v-if="gitVersion"
                    class="status-version"
                  >v{{ gitVersion }}</span>
                </span>
                <span
                  class="status-label"
                  :class="{ ready: gitReady, installing: gitDownloading }"
                >
                  {{ gitDownloading ? '下载中...' : gitReady ? '已就绪' : '未安装' }}
                </span>
                <button
                  v-if="!gitReady && !gitDownloading"
                  class="install-btn"
                  @click="installGit"
                >
                  <i class="fa-solid fa-download" />
                  安装
                </button>
              </div>

              <!-- UV Status (Optional) -->
              <div
                class="status-item optional"
                :class="{ ready: uvReady, installing: uvInstalling }"
              >
                <span class="status-icon">
                  <i
                    v-if="uvInstalling"
                    class="fa-solid fa-spinner fa-spin"
                  />
                  <i
                    v-else-if="uvReady"
                    class="fa-solid fa-check-circle"
                  />
                  <i
                    v-else
                    class="fa-regular fa-circle"
                  />
                </span>
                <span class="status-text">
                  <span class="status-name">UV <span class="optional-badge">（可选）</span></span>
                  <span
                    v-if="uvVersion"
                    class="status-version"
                  >v{{ uvVersion }}</span>
                </span>
                <span
                  class="status-label"
                  :class="{ ready: uvReady, installing: uvInstalling }"
                >
                  {{ uvInstalling ? '安装中...' : uvReady ? '已就绪' : '未安装' }}
                </span>
                <button
                  v-if="!uvReady && !uvInstalling"
                  class="install-btn"
                  @click="installUv"
                >
                  <i class="fa-solid fa-download" />
                  安装
                </button>
              </div>

              <!-- Claude Code Status -->
              <div
                class="status-item"
                :class="{ ready: claudeCodeReady, installing: claudeCodeInstalling }"
              >
                <span class="status-icon">
                  <i
                    v-if="claudeCodeInstalling"
                    class="fa-solid fa-spinner fa-spin"
                  />
                  <i
                    v-else-if="claudeCodeReady"
                    class="fa-solid fa-check-circle"
                  />
                  <i
                    v-else
                    class="fa-regular fa-circle"
                  />
                </span>
                <span class="status-text">
                  <span class="status-name">Claude Code</span>
                </span>
                <span
                  class="status-label"
                  :class="{ ready: claudeCodeReady, installing: claudeCodeInstalling }"
                >
                  {{ claudeCodeInstalling ? '安装中...' : claudeCodeReady ? '已安装' : '未安装' }}
                </span>
                <button
                  v-if="!claudeCodeReady && !claudeCodeInstalling && nodejsReady"
                  class="install-btn"
                  @click="installClaudeCode"
                >
                  <i class="fa-solid fa-download" />
                  安装
                </button>
              </div>
            </div>

            <!-- Progress hint -->
            <p
              v-if="claudeCodeInstalling"
              class="progress-hint"
            >
              首次安装可能需要几分钟，请耐心等待...
            </p>
            <!-- Git download progress -->
            <div
              v-if="gitDownloading && gitDownloadProgress"
              class="download-progress"
            >
              <div class="progress-header">
                <span class="progress-message">{{ gitDownloadProgress.message }}</span>
                <span class="progress-percent">{{ gitDownloadProgress.percent.toFixed(1) }}%</span>
              </div>
              <div class="progress-bar">
                <div
                  class="progress-fill"
                  :style="{ width: gitDownloadProgress.percent + '%' }"
                />
              </div>
              <div
                v-if="gitDownloadProgress.total > 0"
                class="progress-size"
              >
                {{ (gitDownloadProgress.downloaded / 1024 / 1024).toFixed(1) }} MB / {{ (gitDownloadProgress.total / 1024 / 1024).toFixed(1) }} MB
              </div>
            </div>
            <p
              v-else-if="gitDownloading"
              class="progress-hint"
            >
              正在下载 Git 便携版（约55MB）...
            </p>
            <!-- UV download progress -->
            <div
              v-if="uvInstalling && uvDownloadProgress"
              class="download-progress"
            >
              <div class="progress-header">
                <span class="progress-message">{{ uvDownloadProgress.message }}</span>
                <span class="progress-percent">{{ uvDownloadProgress.percent.toFixed(1) }}%</span>
              </div>
              <div class="progress-bar">
                <div
                  class="progress-fill"
                  :style="{ width: uvDownloadProgress.percent + '%' }"
                />
              </div>
            </div>
            <p
              v-else-if="uvInstalling"
              class="progress-hint"
            >
              正在安装 UV Python 包管理器...
            </p>

            <!-- Error Message -->
            <div
              v-if="installError"
              class="error-message"
            >
              <i class="fa-solid fa-exclamation-triangle" />
              <span>{{ installError }}</span>
              <button
                class="retry-button"
                @click="checkStatus"
              >
                重试
              </button>
            </div>

            <!-- Success Message -->
            <div
              v-if="allDepsReady && !installError"
              class="success-message"
            >
              <i class="fa-solid fa-check-circle" />
              <span>所有依赖已就绪！</span>
            </div>

            <!-- Actions -->
            <div class="actions">
              <button
                class="refresh-btn"
                :disabled="isChecking"
                @click="checkStatus"
              >
                <i :class="isChecking ? 'fa-solid fa-spinner fa-spin' : 'fa-solid fa-rotate'" />
                刷新状态
              </button>
              <button
                v-if="allDepsReady"
                class="next-btn"
                @click="goToStep2"
              >
                下一步
                <i class="fa-solid fa-arrow-right" />
              </button>
            </div>
          </template>
        </template>

        <!-- Step 2: Marketplace -->
        <template v-if="currentStep === 2">
          <div class="env-setup">
            <p>在插件商店添加插件源，浏览和安装插件。</p>
            <div class="actions">
              <button
                class="back-btn"
                @click="goToStep1"
              >
                <i class="fa-solid fa-arrow-left" />
                返回
              </button>
              <button
                class="browse-btn"
                @click="openPluginStore"
              >
                <i class="fa-solid fa-store" />
                浏览插件商店
              </button>
              <button
                class="next-btn"
                @click="goToStep3"
              >
                下一步
                <i class="fa-solid fa-arrow-right" />
              </button>
            </div>
          </div>
        </template>

        <!-- Step 3: Environment Variables -->
        <template v-if="currentStep === 3">
          <div class="env-setup">
            <div class="env-icon">
              <i class="fa-solid fa-key" />
            </div>
            <h3>环境变量设置</h3>
            <p class="env-description">
              请配置 Claude API 所需的环境变量
            </p>

            <div class="env-form">
              <!-- ANTHROPIC_AUTH_TOKEN -->
              <div class="env-item">
                <label>
                  ANTHROPIC_AUTH_TOKEN
                  <span class="required">*</span>
                </label>
                <input
                  v-model="envVars.ANTHROPIC_AUTH_TOKEN"
                  type="password"
                  placeholder="sk-ant-..."
                >
              </div>

              <!-- ANTHROPIC_MODEL -->
              <div class="env-item">
                <label>
                  ANTHROPIC_MODEL
                  <span class="required">*</span>
                </label>
                <input
                  v-model="envVars.ANTHROPIC_MODEL"
                  type="text"
                  :placeholder="ENV_DEFAULTS.ANTHROPIC_MODEL"
                >
              </div>

              <!-- ANTHROPIC_BASE_URL -->
              <div class="env-item">
                <label>
                  ANTHROPIC_BASE_URL
                  <span class="required">*</span>
                </label>
                <input
                  v-model="envVars.ANTHROPIC_BASE_URL"
                  type="text"
                  placeholder="https://api.anthropic.com"
                >
              </div>
            </div>

            <!-- Success Message -->
            <div
              v-if="allEnvVarsReady"
              class="success-message"
            >
              <i class="fa-solid fa-check-circle" />
              <span>所有环境变量已填写</span>
            </div>

            <div class="actions">
              <button
                class="back-btn"
                @click="goBackToStep2"
              >
                <i class="fa-solid fa-arrow-left" />
                上一步
              </button>
              <button
                class="save-btn"
                :disabled="!allEnvVarsReady"
                @click="saveEnvVars"
              >
                <i class="fa-solid fa-check" />
                完成
              </button>
            </div>
          </div>
        </template>
      </div>
    </template>
  </div>
</template>

<style lang="less" scoped>
.setup-guide-window {
  height: 100%;
  background: #f8f8f8;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  position: relative;
}

// Plugin Store Overlay
.plugin-store-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: #f8f8f8;
  z-index: 100;
  display: flex;
  flex-direction: column;

  .plugin-store-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
    color: white;

    h2 {
      margin: 0;
      font-size: 16px;
      font-weight: 600;
    }

    .close-btn {
      width: 32px;
      height: 32px;
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.1);
      border: none;
      color: white;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all 0.2s;

      &:hover {
        background: rgba(255, 255, 255, 0.2);
      }

      i {
        font-size: 16px;
      }
    }
  }

  .plugin-store-content {
    flex: 1;
    overflow: hidden;
  }
}

.header {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 20px 24px;
  background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
  color: white;

  .logo {
    width: 48px;
    height: 48px;
    filter: drop-shadow(0 2px 8px rgba(124, 110, 184, 0.4));
  }

  .header-text {
    flex: 1;

    h1 {
      font-size: 18px;
      font-weight: 600;
      margin: 0 0 4px 0;
    }

    .subtitle {
      font-size: 12px;
      color: rgba(255, 255, 255, 0.7);
      margin: 0;
    }
  }

  .step-indicator {
    display: flex;
    align-items: center;
    gap: 8px;

    span:not(.step-line) {
      width: 28px;
      height: 28px;
      border-radius: 50%;
      background: rgba(255, 255, 255, 0.2);
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 12px;
      font-weight: 600;
      transition: all 0.3s;

      &.active {
        background: #7c6eb8;
        box-shadow: 0 0 12px rgba(124, 110, 184, 0.5);
      }

      &.completed {
        background: #4caf50;
      }
    }

    .step-line {
      width: 24px;
      height: 2px;
      background: rgba(255, 255, 255, 0.2);
      transition: all 0.3s;

      &.completed {
        background: #4caf50;
      }
    }
  }
}

.main-section {
  flex: 1;
  padding: 24px;
  overflow-y: auto;
}

.checking {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: #7c6eb8;
  font-size: 16px;
  padding: 40px 0;

  i {
    font-size: 20px;
  }
}

.status-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 16px;
  background: white;
  border-radius: 10px;
  border: 1px solid #e8e8e8;
  transition: all 0.2s;

  &.ready {
    border-color: rgba(76, 175, 80, 0.3);
    background: rgba(76, 175, 80, 0.05);

    .status-icon {
      color: #4caf50;
    }
  }

  &.installing {
    border-color: rgba(124, 110, 184, 0.3);
    background: rgba(124, 110, 184, 0.05);

    .status-icon {
      color: #7c6eb8;
    }
  }

  &.optional {
    opacity: 0.85;

    .optional-badge {
      font-size: 12px;
      color: #999;
      margin-left: 4px;
    }
  }

  .status-icon {
    font-size: 18px;
    color: #999;
    width: 24px;
    text-align: center;
  }

  .status-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;

    .status-name {
      font-size: 14px;
      font-weight: 500;
      color: #333;
    }

    .status-version,
    .status-desc {
      font-size: 12px;
      color: #888;
    }
  }

  .status-label {
    font-size: 12px;
    color: #888;
    padding: 4px 10px;
    background: #f0f0f0;
    border-radius: 12px;

    &.ready {
      color: #4caf50;
      background: rgba(76, 175, 80, 0.1);
    }

    &.installing {
      color: #7c6eb8;
      background: rgba(124, 110, 184, 0.1);
    }
  }

  .install-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    background: #7c6eb8;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.2s;

    &:hover {
      background: #6a5ca0;
    }

    i {
      font-size: 11px;
    }
  }
}

.progress-hint {
  margin: 16px 0 0 0;
  font-size: 13px;
  color: #888;
  text-align: center;
}

.download-progress {
  margin: 16px 0 0 0;
  padding: 12px 16px;
  background: rgba(124, 110, 184, 0.05);
  border: 1px solid rgba(124, 110, 184, 0.2);
  border-radius: 8px;

  .progress-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;

    .progress-message {
      font-size: 13px;
      color: #666;
    }

    .progress-percent {
      font-size: 14px;
      font-weight: 600;
      color: #7c6eb8;
    }
  }

  .progress-bar {
    height: 8px;
    background: #e8e8e8;
    border-radius: 4px;
    overflow: hidden;

    .progress-fill {
      height: 100%;
      background: linear-gradient(90deg, #7c6eb8 0%, #9b8fd4 100%);
      border-radius: 4px;
      transition: width 0.2s ease-out;
    }
  }

  .progress-size {
    margin-top: 6px;
    font-size: 12px;
    color: #888;
    text-align: center;
  }
}

.error-message {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin-top: 20px;
  padding: 12px 16px;
  background: rgba(244, 67, 54, 0.1);
  border: 1px solid rgba(244, 67, 54, 0.3);
  border-radius: 8px;
  color: #f44336;
  font-size: 13px;
  flex-wrap: wrap;

  .retry-button {
    margin-left: 8px;
    padding: 4px 12px;
    background: rgba(244, 67, 54, 0.2);
    border: 1px solid rgba(244, 67, 54, 0.4);
    border-radius: 4px;
    color: #f44336;
    cursor: pointer;
    font-size: 12px;

    &:hover {
      background: rgba(244, 67, 54, 0.3);
    }
  }
}

.success-message {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  margin-top: 20px;
  padding: 16px;
  background: rgba(76, 175, 80, 0.1);
  border: 1px solid rgba(76, 175, 80, 0.3);
  border-radius: 8px;
  color: #4caf50;
  font-size: 14px;

  i {
    font-size: 18px;
  }
}

.actions {
  margin-top: 24px;
  display: flex;
  justify-content: center;
  gap: 12px;

  .refresh-btn,
  .back-btn,
  .retry-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    background: #f0f0f0;
    color: #333;
    border: 1px solid #ddd;
    border-radius: 8px;
    font-size: 13px;
    cursor: pointer;
    transition: all 0.2s;

    &:hover:not(:disabled) {
      background: #e8e8e8;
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }

  .next-btn,
  .browse-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 24px;
    background: #7c6eb8;
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;

    &:hover {
      background: #6a5ca0;
    }
  }
}

// Step 3: Environment Variables
.env-setup {
  text-align: center;
  padding: 20px 0;

  .env-icon {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background: rgba(124, 110, 184, 0.1);
    display: flex;
    align-items: center;
    justify-content: center;
    margin: 0 auto 16px;

    i {
      font-size: 28px;
      color: #7c6eb8;
    }
  }

  h3 {
    font-size: 18px;
    font-weight: 600;
    color: #333;
    margin: 0 0 8px 0;
  }

  .env-description {
    font-size: 14px;
    color: #666;
    margin: 0 0 24px 0;
  }
}

.env-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 400px;
  margin: 0 auto;
  text-align: left;

  .env-item {
    label {
      display: block;
      margin-bottom: 6px;
      font-size: 13px;
      font-weight: 500;
      color: #444;
      font-family: 'SF Mono', 'Monaco', 'Inconsolata', monospace;

      .required {
        color: #ef4444;
        margin-left: 2px;
      }
    }

    input {
      width: 100%;
      padding: 10px 12px;
      border: 1px solid #ddd;
      border-radius: 8px;
      font-size: 14px;
      background: white;
      transition: all 0.2s;

      &:focus {
        outline: none;
        border-color: #7c6eb8;
        box-shadow: 0 0 0 3px rgba(124, 110, 184, 0.1);
      }

      &::placeholder {
        color: #aaa;
      }
    }
  }
}

.save-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 24px;
  background: #4caf50;
  color: white;
  border: none;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;

  &:hover:not(:disabled) {
    background: #43a047;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}
</style>
