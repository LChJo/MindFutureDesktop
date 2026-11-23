<script setup lang="ts">
import { computed } from 'vue'
import { useAutoUpdate } from '@/composables/useAutoUpdate'

const {
  showModal,
  modalPhase,
  lastCheckResult,
  downloadProgress,
  error,
  confirmUpdate,
  confirmInstall,
  cancelUpdate,
} = useAutoUpdate()

function formatSize(bytes: number): string {
  if (bytes <= 0) return ''
  const mb = bytes / (1024 * 1024)
  return `${mb.toFixed(1)} MB`
}

const downloadedSize = computed(() => formatSize(downloadProgress.value.downloaded))
const totalSize = computed(() => formatSize(downloadProgress.value.total))
const progressPercent = computed(() => Math.round(downloadProgress.value.percent))

function handleOverlayClick(e: MouseEvent) {
  if (e.target === e.currentTarget && modalPhase.value !== 'downloading') {
    cancelUpdate()
  }
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="showModal"
      class="update-overlay"
      @click="handleOverlayClick"
    >
      <div
        class="update-modal"
        @click.stop
      >
        <!-- Header -->
        <div class="modal-header">
          <div class="header-icon">
            <i
              v-if="modalPhase === 'confirm'"
              class="fa-solid fa-arrow-up-from-bracket"
            />
            <i
              v-else-if="modalPhase === 'downloading'"
              class="fa-solid fa-cloud-arrow-down"
            />
            <i
              v-else
              class="fa-solid fa-circle-check"
            />
          </div>
          <h3>
            <template v-if="modalPhase === 'confirm'">
              发现新版本
            </template>
            <template v-else-if="modalPhase === 'downloading'">
              正在下载更新
            </template>
            <template v-else>
              下载完成
            </template>
          </h3>
          <button
            v-if="modalPhase !== 'downloading'"
            class="close-btn"
            @click="cancelUpdate"
          >
            &times;
          </button>
        </div>

        <!-- Body: Confirm -->
        <div
          v-if="modalPhase === 'confirm'"
          class="modal-body"
        >
          <div class="version-info">
            <span class="version-badge current">{{ lastCheckResult?.current_version }}</span>
            <i class="fa-solid fa-arrow-right version-arrow" />
            <span class="version-badge new">{{ lastCheckResult?.remote_version }}</span>
          </div>

          <div
            v-if="lastCheckResult?.download_size"
            class="meta-row"
          >
            <i class="fa-solid fa-hard-drive" />
            <span>{{ formatSize(lastCheckResult.download_size) }}</span>
          </div>

          <div
            v-if="lastCheckResult?.release_notes"
            class="release-notes"
          >
            <div class="notes-label">
              更新日志
            </div>
            <div class="notes-content">
              {{ lastCheckResult.release_notes }}
            </div>
          </div>

          <div
            v-if="error"
            class="error-message"
          >
            <i class="fa-solid fa-triangle-exclamation" />
            {{ error }}
          </div>
        </div>

        <!-- Body: Downloading -->
        <div
          v-else-if="modalPhase === 'downloading'"
          class="modal-body"
        >
          <div class="progress-section">
            <div class="progress-bar-track">
              <div
                class="progress-bar-fill"
                :style="{ width: `${progressPercent}%` }"
              />
            </div>
            <div class="progress-details">
              <span>{{ progressPercent }}%</span>
              <span v-if="totalSize">{{ downloadedSize }} / {{ totalSize }}</span>
            </div>
          </div>
        </div>

        <!-- Body: Downloaded -->
        <div
          v-else
          class="modal-body"
        >
          <div class="done-message">
            <i class="fa-solid fa-circle-check done-icon" />
            <p>更新已下载完成，安装将关闭当前应用。</p>
          </div>

          <div
            v-if="error"
            class="error-message"
          >
            <i class="fa-solid fa-triangle-exclamation" />
            {{ error }}
          </div>
        </div>

        <!-- Footer -->
        <div class="modal-footer">
          <template v-if="modalPhase === 'confirm'">
            <button
              class="btn btn-secondary"
              @click="cancelUpdate"
            >
              取消
            </button>
            <button
              class="btn btn-primary"
              @click="confirmUpdate"
            >
              下载并安装
            </button>
          </template>

          <template v-else-if="modalPhase === 'downloading'">
            <span class="downloading-hint">请勿关闭应用...</span>
          </template>

          <template v-else>
            <button
              class="btn btn-secondary"
              @click="cancelUpdate"
            >
              稍后安装
            </button>
            <button
              class="btn btn-primary"
              @click="confirmInstall"
            >
              立即安装
            </button>
          </template>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped lang="less">
.update-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
}

.update-modal {
  width: 440px;
  max-width: 90%;
  background: #21252b;
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  overflow: hidden;
  color: #abb2bf;
}

.modal-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 18px 20px;
  border-bottom: 1px solid #181a1f;

  .header-icon {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    background: rgba(124, 110, 184, 0.15);
    display: flex;
    align-items: center;
    justify-content: center;
    color: #7c6eb8;
    font-size: 14px;
    flex-shrink: 0;
  }

  h3 {
    flex: 1;
    font-size: 15px;
    font-weight: 600;
    color: #e0e0e0;
    margin: 0;
  }

  .close-btn {
    background: none;
    border: none;
    color: #5c6370;
    font-size: 20px;
    cursor: pointer;
    padding: 0 4px;
    line-height: 1;

    &:hover {
      color: #abb2bf;
    }
  }
}

.modal-body {
  padding: 20px;
}

.version-info {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin-bottom: 16px;

  .version-badge {
    padding: 4px 12px;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 500;
    font-family: 'Consolas', 'Courier New', monospace;

    &.current {
      background: #3e4451;
      color: #5c6370;
    }

    &.new {
      background: rgba(124, 110, 184, 0.2);
      color: #a99dd6;
    }
  }

  .version-arrow {
    color: #5c6370;
    font-size: 12px;
  }
}

.meta-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: #5c6370;
  margin-bottom: 16px;

  i {
    font-size: 12px;
  }
}

.release-notes {
  background: #282c34;
  border-radius: 8px;
  padding: 12px 14px;

  .notes-label {
    font-size: 12px;
    color: #5c6370;
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .notes-content {
    font-size: 13px;
    line-height: 1.6;
    color: #abb2bf;
    white-space: pre-wrap;
    max-height: 200px;
    overflow-y: auto;
  }
}

.progress-section {
  .progress-bar-track {
    height: 8px;
    background: #3e4451;
    border-radius: 4px;
    overflow: hidden;
    margin-bottom: 10px;
  }

  .progress-bar-fill {
    height: 100%;
    background: linear-gradient(90deg, #7c6eb8, #a99dd6);
    border-radius: 4px;
    transition: width 0.3s ease;
  }

  .progress-details {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
    color: #5c6370;
  }
}

.done-message {
  text-align: center;
  padding: 8px 0;

  .done-icon {
    font-size: 36px;
    color: #98c379;
    margin-bottom: 12px;
  }

  p {
    font-size: 14px;
    color: #abb2bf;
    margin: 0;
  }
}

.error-message {
  margin-top: 12px;
  padding: 10px 12px;
  background: rgba(224, 108, 117, 0.1);
  border-radius: 6px;
  font-size: 13px;
  color: #e06c75;
  display: flex;
  align-items: flex-start;
  gap: 8px;

  i {
    margin-top: 2px;
    flex-shrink: 0;
  }
}

.modal-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  padding: 14px 20px;
  border-top: 1px solid #181a1f;

  .downloading-hint {
    font-size: 13px;
    color: #5c6370;
    font-style: italic;
  }
}

.btn {
  padding: 8px 18px;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  border: none;
  transition: background 0.15s;

  &.btn-primary {
    background: #7c6eb8;
    color: #fff;

    &:hover {
      background: #6a5ca0;
    }
  }

  &.btn-secondary {
    background: transparent;
    border: 1px solid #3e4451;
    color: #abb2bf;

    &:hover {
      background: rgba(255, 255, 255, 0.05);
    }
  }
}
</style>
