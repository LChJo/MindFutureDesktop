<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
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
  nodejsInstalling,

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
  claudeCodeError,

  // Environment variables state
  envVars,

  // Computed
  allDepsReady,
  allEnvVarsReady,

  // Methods
  setupProgressListener,
  cleanupProgressListener,
  checkStatus,
  installUv,
  installClaudeCode,
  saveEnvVars,
  loadEnvVars,
} = useSetupGuide()

const props = defineProps<{
  skipToEnvPage?: boolean
}>()

const emit = defineEmits<{
  (e: 'complete'): void
}>()

// Page State (1: 依赖检查, 2: 环境变量)
const currentPage = ref(1)
const installSuccess = ref(false)

// Check Node.js, Git and Claude Code status on mount
onMounted(async () => {
  await setupProgressListener()

  // If skipToEnvPage is set, skip dependency check and go directly to env vars page
  if (props.skipToEnvPage) {
    // Mark all dependencies as ready since we already checked them in App.vue
    nodejsReady.value = true
    gitReady.value = true
    claudeCodeReady.value = true
    currentPage.value = 2
    await loadEnvVars()
    return
  }

  await checkStatus()
  await loadEnvVars()
})

// Cleanup on unmount
onUnmounted(() => {
  cleanupProgressListener()
})

// Page navigation
const goToPage2 = () => {
  currentPage.value = 2
}

const goBackToPage1 = () => {
  currentPage.value = 1
}

// Complete setup
const completeSetup = () => {
  saveEnvVars()
  installSuccess.value = true
  setTimeout(() => {
    emit('complete')
  }, 500)
}
</script>

<template>
  <div class="setup-guide-overlay">
    <div class="setup-guide-content">
      <!-- Logo and Title - Horizontal Layout -->
      <div class="header">
        <div class="header-content">
          <img
            src="/icons/icon.png"
            alt="Moonton Claude Desktop"
            class="logo"
          >
          <div class="header-text">
            <h1>Moonton Claude Desktop</h1>
            <p class="subtitle">
              {{ currentPage === 1 ? '检查和安装运行环境依赖' : '配置 Claude API 环境变量' }}
            </p>
          </div>
        </div>
        <!-- Step Indicator -->
        <div class="step-indicator">
          <span :class="{ active: currentPage === 1, completed: currentPage > 1 }">1</span>
          <span
            class="step-line"
            :class="{ completed: currentPage > 1 }"
          />
          <span :class="{ active: currentPage === 2 }">2</span>
        </div>
      </div>

      <!-- Main Content -->
      <div class="main-section">
        <!-- Success Message -->
        <div
          v-if="installSuccess"
          class="success-message"
        >
          <i class="fa-solid fa-check-circle" />
          <span>配置完成！正在进入桌面...</span>
        </div>

        <!-- Page 1: Dependencies Check -->
        <template v-else-if="currentPage === 1">
          <div
            v-if="isChecking"
            class="checking"
          >
            <i class="fa-solid fa-spinner fa-spin" />
            <span>{{ setupStep || '正在初始化...' }}</span>
          </div>

          <template v-else>
            <p class="requirement-text">
              <i class="fa-solid fa-info-circle" />
              本应用需要以下依赖才能正常使用
            </p>

            <!-- Status Grid (2x2) -->
            <div class="status-grid">
              <!-- Node.js Status -->
              <div
                class="status-item"
                :class="{ ready: nodejsReady, installing: nodejsInstalling }"
              >
                <span class="status-icon">
                  <i
                    v-if="nodejsInstalling"
                    class="fa-solid fa-spinner fa-spin"
                  />
                  <i
                    v-else-if="nodejsReady"
                    class="fa-solid fa-check-circle"
                  />
                  <i
                    v-else
                    class="fa-solid fa-circle"
                  />
                </span>
                <span class="status-text">
                  Node.js {{ nodejsVersion ? `v${nodejsVersion}` : '' }}
                  <span class="status-label">
                    {{ nodejsInstalling ? '释放中...' : nodejsReady ? '已就绪' : '未安装' }}
                  </span>
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
                  Git {{ gitVersion ? `v${gitVersion}` : '' }}
                  <span class="status-label">
                    {{ gitDownloading ? '下载中...' : gitReady ? '已就绪' : '待安装' }}
                  </span>
                </span>
              </div>

              <!-- UV Status -->
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
                  UV <span class="optional-badge">（可选）</span>{{ uvVersion ? ` v${uvVersion}` : '' }}
                  <span class="status-label">
                    {{ uvInstalling ? '安装中...' : uvReady ? '已就绪' : '待安装' }}
                  </span>
                </span>
                <button
                  v-if="!uvReady && !uvInstalling"
                  class="mini-button"
                  @click="installUv"
                >
                  <i class="fa-solid fa-download" />
                  安装
                </button>
              </div>

              <!-- Claude Code Status -->
              <div
                class="status-item"
                :class="{ installing: claudeCodeInstalling, ready: claudeCodeReady }"
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
                  Claude Code
                  <span class="status-label">
                    {{ claudeCodeInstalling ? '安装中...' : claudeCodeReady ? '已安装' : '待安装' }}
                  </span>
                </span>
              </div>
            </div>

            <!-- Install Button -->
            <button
              v-if="!claudeCodeReady"
              class="install-button"
              :disabled="!nodejsReady || !gitReady || claudeCodeInstalling || nodejsInstalling || gitDownloading || uvInstalling"
              @click="installClaudeCode"
            >
              <i
                v-if="claudeCodeInstalling"
                class="fa-solid fa-spinner fa-spin"
              />
              <i
                v-else
                class="fa-solid fa-download"
              />
              <span>{{ claudeCodeInstalling ? '安装中...' : '安装 Claude Code' }}</span>
            </button>

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
              正在下载 Git 便携版（约60MB）...
            </p>
            <!-- UV install progress -->
            <div
              v-if="uvInstalling && uvDownloadProgress"
              class="download-progress"
            >
              <div class="progress-header">
                <span class="progress-message">{{ uvDownloadProgress.message }}</span>
                <span class="progress-percent">{{ uvDownloadProgress.percent.toFixed(0) }}%</span>
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

            <!-- Claude Code Error Details -->
            <div
              v-if="claudeCodeError && !claudeCodeReady && !installError"
              class="error-detail"
            >
              <i class="fa-solid fa-info-circle" />
              <span>{{ claudeCodeError }}</span>
            </div>

            <!-- Success & Next Button -->
            <div
              v-if="allDepsReady && !installError"
              class="page-actions"
            >
              <div class="success-hint">
                <i class="fa-solid fa-check-circle" />
                <span>所有依赖已就绪！</span>
              </div>
              <button
                class="next-btn"
                @click="goToPage2"
              >
                下一步
                <i class="fa-solid fa-arrow-right" />
              </button>
            </div>
          </template>
        </template>

        <!-- Page 2: Environment Variables -->
        <template v-else-if="currentPage === 2">
          <div class="env-setup-section">
            <div class="env-icon">
              <i class="fa-solid fa-key" />
            </div>
            <h3>配置 Claude API 环境变量</h3>
            <p class="env-description">
              请填写以下必填环境变量以完成配置
            </p>

            <div class="env-form">
              <div class="env-item">
                <label>ANTHROPIC_AUTH_TOKEN <span class="required">*</span></label>
                <input
                  v-model="envVars.ANTHROPIC_AUTH_TOKEN"
                  type="password"
                  placeholder="sk-ant-..."
                >
              </div>

              <div class="env-item">
                <label>ANTHROPIC_MODEL <span class="required">*</span></label>
                <input
                  v-model="envVars.ANTHROPIC_MODEL"
                  type="text"
                  :placeholder="ENV_DEFAULTS.ANTHROPIC_MODEL"
                >
              </div>

              <div class="env-item">
                <label>ANTHROPIC_BASE_URL <span class="required">*</span></label>
                <input
                  v-model="envVars.ANTHROPIC_BASE_URL"
                  type="text"
                  placeholder="https://api.anthropic.com"
                >
              </div>
            </div>

            <div class="page-actions">
              <button
                class="back-btn"
                @click="goBackToPage1"
              >
                <i class="fa-solid fa-arrow-left" />
                上一步
              </button>
              <button
                class="complete-btn"
                :disabled="!allEnvVarsReady"
                @click="completeSetup"
              >
                完成
                <i class="fa-solid fa-check" />
              </button>
            </div>
          </div>
        </template>
      </div>

      <!-- Footer -->
      <div class="footer">
        <p>Powered by Tauri + Vue 3 + Claude Code</p>
      </div>
    </div>
  </div>
</template>

<style lang="less" scoped>
.setup-guide-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #1a1a2e 100%);
  z-index: 99999;
  display: flex;
  align-items: center;
  justify-content: center;

  &::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: radial-gradient(circle at 50% 50%, rgba(124, 110, 184, 0.1) 0%, transparent 50%);
    pointer-events: none;
  }
}

.setup-guide-content {
  position: relative;
  text-align: center;
  color: #e0e0e0;
  max-width: 480px;
  padding: 32px;
}

.header {
  margin-bottom: 24px;

  .header-content {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 16px;
    margin-bottom: 16px;
  }

  .logo {
    width: 56px;
    height: 56px;
    filter: drop-shadow(0 4px 12px rgba(124, 110, 184, 0.4));
    flex-shrink: 0;
  }

  .header-text {
    text-align: left;

    h1 {
      font-size: 22px;
      font-weight: 600;
      color: #fff;
      margin: 0 0 4px 0;
      letter-spacing: 0.5px;
    }

    .subtitle {
      font-size: 13px;
      color: #888;
      margin: 0;
    }
  }

  .step-indicator {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;

    span:not(.step-line) {
      width: 24px;
      height: 24px;
      border-radius: 50%;
      background: rgba(255, 255, 255, 0.2);
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 11px;
      font-weight: 600;
      color: #fff;
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
      width: 20px;
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
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 16px;
  padding: 24px;
  backdrop-filter: blur(10px);
}

.checking {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: #7c6eb8;
  font-size: 16px;
  padding: 20px 0;

  i {
    font-size: 20px;
  }
}

.requirement-text {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: #aaa;
  font-size: 13px;
  margin: 0 0 16px 0;

  i {
    color: #7c6eb8;
  }
}

// 2x2 Grid layout for status items
.status-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  margin-bottom: 20px;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  background: rgba(0, 0, 0, 0.2);
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.05);
  transition: all 0.3s ease;

  &.ready {
    border-color: rgba(76, 175, 80, 0.3);
    background: rgba(76, 175, 80, 0.1);

    .status-icon {
      color: #4caf50;
    }

    .status-label {
      color: #4caf50;
    }
  }

  &.installing {
    border-color: rgba(124, 110, 184, 0.3);
    background: rgba(124, 110, 184, 0.1);

    .status-icon {
      color: #7c6eb8;
    }

    .status-label {
      color: #7c6eb8;
    }
  }

  &.optional {
    opacity: 0.85;

    .optional-badge {
      font-size: 11px;
      color: #999;
      margin-left: 4px;
    }
  }

  .status-icon {
    font-size: 16px;
    color: #666;
    width: 20px;
    text-align: center;
    flex-shrink: 0;
  }

  .status-text {
    flex: 1;
    text-align: left;
    font-size: 12px;
    color: #e0e0e0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .status-label {
    font-size: 11px;
    color: #888;
  }
}

.mini-button {
  padding: 4px 10px;
  background: rgba(124, 110, 184, 0.2);
  border: 1px solid rgba(124, 110, 184, 0.4);
  border-radius: 4px;
  color: #7c6eb8;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s ease;

  &:hover {
    background: rgba(124, 110, 184, 0.3);
  }
}

.install-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  background: linear-gradient(135deg, #7c6eb8 0%, #5a4f8c 100%);
  color: #fff;
  border: none;
  padding: 14px 32px;
  font-size: 16px;
  font-weight: 500;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.3s ease;

  &:hover:not(:disabled) {
    background: linear-gradient(135deg, #8d7ec9 0%, #6b5f9d 100%);
    transform: translateY(-2px);
    box-shadow: 0 4px 20px rgba(124, 110, 184, 0.4);
  }

  &:active:not(:disabled) {
    transform: translateY(0);
  }

  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  i {
    font-size: 18px;
  }
}

.progress-hint {
  margin: 16px 0 0 0;
  font-size: 13px;
  color: #888;
}

.download-progress {
  margin: 16px 0 0 0;
  padding: 12px 16px;
  background: rgba(124, 110, 184, 0.15);
  border: 1px solid rgba(124, 110, 184, 0.3);
  border-radius: 8px;

  .progress-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;

    .progress-message {
      font-size: 13px;
      color: #bbb;
    }

    .progress-percent {
      font-size: 14px;
      font-weight: 600;
      color: #9b8fd4;
    }
  }

  .progress-bar {
    height: 8px;
    background: rgba(255, 255, 255, 0.15);
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
    color: #aaa;
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

.error-detail {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin-top: 16px;
  padding: 12px 16px;
  background: rgba(255, 152, 0, 0.1);
  border: 1px solid rgba(255, 152, 0, 0.3);
  border-radius: 8px;
  color: #ff9800;
  font-size: 12px;
  text-align: left;

  i {
    margin-top: 2px;
  }
}

.success-message {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 20px;
  color: #4caf50;
  font-size: 18px;

  i {
    font-size: 24px;
  }
}

.footer {
  margin-top: 20px;

  p {
    font-size: 11px;
    color: #555;
    margin: 0;
  }
}

// Environment Variables Setup Styles
.env-setup-section {
  margin-top: 24px;
  padding: 20px;
  background: rgba(124, 110, 184, 0.1);
  border: 1px solid rgba(124, 110, 184, 0.3);
  border-radius: 12px;

  .env-title {
    display: flex;
    align-items: center;
    gap: 10px;
    color: #7c6eb8;
    font-size: 16px;
    font-weight: 500;
    margin: 0 0 8px 0;

    i {
      font-size: 18px;
    }
  }

  .env-description {
    color: #888;
    font-size: 13px;
    margin: 0 0 20px 0;
  }

  .env-form {
    display: flex;
    flex-direction: column;
    gap: 16px;

    .env-item {
      text-align: left;

      label {
        display: block;
        margin-bottom: 6px;
        font-size: 13px;
        color: #aaa;

        .required {
          color: #ef4444;
        }
      }

      input {
        width: 100%;
        padding: 10px 12px;
        background: rgba(0, 0, 0, 0.3);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 8px;
        font-size: 14px;
        color: #e0e0e0;
        box-sizing: border-box;

        &::placeholder {
          color: #666;
        }

        &:focus {
          outline: none;
          border-color: #7c6eb8;
          background: rgba(0, 0, 0, 0.4);
        }
      }
    }
  }

  .env-icon {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background: rgba(124, 110, 184, 0.2);
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
    color: #fff;
    margin: 0 0 8px 0;
  }
}

// Page Actions
.page-actions {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin-top: 24px;
  flex-wrap: wrap;
}

// Success hint
.success-hint {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 12px 16px;
  background: rgba(76, 175, 80, 0.1);
  border: 1px solid rgba(76, 175, 80, 0.3);
  border-radius: 8px;
  color: #4caf50;
  font-size: 14px;
  margin-bottom: 16px;

  i {
    font-size: 18px;
  }
}

// Navigation Buttons
.back-btn,
.retry-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 20px;
  background: rgba(255, 255, 255, 0.1);
  color: #e0e0e0;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;

  &:hover {
    background: rgba(255, 255, 255, 0.15);
  }
}

.next-btn {
  display: inline-flex;
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

  &:hover:not(:disabled) {
    background: #6a5ca0;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.complete-btn {
  display: inline-flex;
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

  &:hover {
    background: #43a047;
  }
}

.install-btn {
  display: inline-flex;
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
</style>
