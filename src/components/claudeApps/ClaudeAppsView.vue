<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useClaudeApps } from '@/composables/claudeApps'
import { useDynamicFunctionStore } from '@/stores/dynamicFunctionStore'
import ChatPanel from '@/components/coworks/ChatPanel.vue'
import ClaudeAppsList from './ClaudeAppsList.vue'
import type { Attachment } from '@/components/coworks/ChatPanel.vue'

defineProps<{
  windowId?: string
  windowData?: Record<string, unknown>
}>()

const claude = useClaudeApps()
const dfStore = useDynamicFunctionStore()

// New App dialog state
const showNewAppDialog = ref(false)
const newAppDescription = ref('')
const newAppInputRef = ref<HTMLTextAreaElement | null>(null)

onMounted(async () => {
  await claude.initialize()
})

function openNewAppDialog() {
  newAppDescription.value = ''
  showNewAppDialog.value = true
  // Focus input after DOM update
  setTimeout(() => newAppInputRef.value?.focus(), 50)
}

function closeNewAppDialog() {
  showNewAppDialog.value = false
  newAppDescription.value = ''
}

function confirmNewApp() {
  const desc = newAppDescription.value.trim()
  if (!desc) return
  showNewAppDialog.value = false
  newAppDescription.value = ''
  claude.createNewApp(desc)
}

function handleDialogKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    confirmNewApp()
  }
  if (e.key === 'Escape') {
    closeNewAppDialog()
  }
}

function handleSend(content: string, attachments: Attachment[]) {
  claude.sendMessage(content, attachments.length > 0 ? attachments : undefined)
}

function handleCancel() {
  claude.cancelRequest()
}

function handleClearError() {
  claude.clearError()
}

async function handleToggleBackend(appName: string) {
  const current = dfStore.getBackendStatus(appName)
  try {
    if (current.status === 'running') {
      await dfStore.stopBackend(appName)
    } else {
      await dfStore.startBackend(appName)
    }
  } catch (e) {
    console.error('Backend toggle failed:', e)
  }
}

const emptyHint = '选择左侧应用开始对话，或点击 + 创建新应用'
</script>

<template>
  <div class="claude-apps-view">
    <!-- Left panel: App list -->
    <div class="left-panel">
      <div class="panel-header">
        <div class="panel-title">
          <i class="fa-solid fa-cubes" />
          <span>Claude 轻应用</span>
        </div>
        <button
          class="new-btn"
          title="新建应用"
          @click="openNewAppDialog"
        >
          <i class="fa-solid fa-plus" />
        </button>
      </div>
      <ClaudeAppsList
        :apps="claude.apps.value"
        :conversations="claude.conversations.value"
        :selected-app-name="claude.selectedAppName.value"
        :selected-conversation-id="claude.currentConversationId.value"
        :backend-statuses="dfStore.backendStatuses"
        @select-app="claude.selectApp"
        @select-conversation="claude.loadConversation"
        @open-app="claude.openApp"
        @reveal-app="claude.revealInExplorer"
        @delete-app="(app) => claude.deleteApp(app.name)"
        @delete-conversation="claude.deleteConversation"
        @toggle-backend="handleToggleBackend"
      />
    </div>

    <!-- Right panel: Chat -->
    <div class="right-panel">
      <ChatPanel
        v-if="claude.currentConversationId.value"
        :messages="claude.messages.value"
        :is-loading="claude.isLoading.value"
        :error="claude.error.value"
        :model="claude.model.value"
        :empty-hint="emptyHint"
        @send="handleSend"
        @cancel="handleCancel"
        @clear="handleClearError"
        @update:model="claude.model.value = $event"
      />
      <div
        v-else
        class="empty-placeholder"
      >
        <i class="fa-solid fa-cubes" />
        <span>选择左侧应用开始对话，或点击 + 创建新应用</span>
      </div>
    </div>

    <!-- New App Dialog -->
    <Teleport to="body">
      <div
        v-if="showNewAppDialog"
        class="dialog-overlay"
        @click.self="closeNewAppDialog"
      >
        <div class="dialog-box">
          <div class="dialog-header">
            <i class="fa-solid fa-cubes" />
            <span>创建新应用</span>
          </div>
          <div class="dialog-body">
            <label class="dialog-label">描述你想创建的应用</label>
            <textarea
              ref="newAppInputRef"
              v-model="newAppDescription"
              class="dialog-input"
              placeholder="例如：一个简单的天气查询应用，支持搜索城市并显示天气信息..."
              rows="4"
              @keydown="handleDialogKeydown"
            />
          </div>
          <div class="dialog-footer">
            <button
              class="dialog-btn cancel"
              @click="closeNewAppDialog"
            >
              取消
            </button>
            <button
              class="dialog-btn confirm"
              :disabled="!newAppDescription.trim()"
              @click="confirmNewApp"
            >
              创建
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style lang="less" scoped>
.claude-apps-view {
  width: 100%;
  height: 100%;
  display: flex;
  background: #FAF9F7;
}

.left-panel {
  width: 260px;
  min-width: 260px;
  display: flex;
  flex-direction: column;
  border-right: 1px solid #E8E5E0;
  background: #FFFFFF;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 12px;
  border-bottom: 1px solid #E8E5E0;
}

.panel-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: #2D2B2A;

  i {
    color: #C4704B;
    font-size: 14px;
  }
}

.new-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(196, 112, 75, 0.10);
  border: none;
  border-radius: 6px;
  cursor: pointer;
  color: #C4704B;
  transition: all 0.15s;

  &:hover {
    background: rgba(196, 112, 75, 0.18);
  }

  i {
    font-size: 12px;
  }
}

.right-panel {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.empty-placeholder {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: #B0ACA7;
  font-size: 14px;

  i {
    font-size: 32px;
    opacity: 0.5;
  }
}

// Dialog styles
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
}

.dialog-box {
  width: 480px;
  max-width: 90vw;
  background: #FFFFFF;
  border: 1px solid #E8E5E0;
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.12);
}

.dialog-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 16px 20px;
  border-bottom: 1px solid #E8E5E0;
  font-size: 16px;
  font-weight: 600;
  color: #2D2B2A;

  i {
    color: #C4704B;
  }
}

.dialog-body {
  padding: 16px 20px;
}

.dialog-label {
  display: block;
  font-size: 13px;
  color: #8B8680;
  margin-bottom: 8px;
}

.dialog-input {
  width: 100%;
  background: #FAF9F7;
  border: 1px solid #E8E5E0;
  border-radius: 8px;
  padding: 12px;
  color: #2D2B2A;
  font-size: 14px;
  font-family: inherit;
  resize: none;
  line-height: 1.5;

  &:focus {
    outline: none;
    border-color: rgba(196, 112, 75, 0.5);
  }

  &::placeholder {
    color: #B0ACA7;
  }
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 20px;
  border-top: 1px solid #E8E5E0;
}

.dialog-btn {
  padding: 8px 20px;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;

  &.cancel {
    background: transparent;
    border: 1px solid #E8E5E0;
    color: #2D2B2A;

    &:hover {
      background: rgba(0, 0, 0, 0.03);
    }
  }

  &.confirm {
    background: #C4704B;
    border: none;
    color: white;

    &:hover:not(:disabled) {
      background: #A85A38;
    }

    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }
}
</style>
