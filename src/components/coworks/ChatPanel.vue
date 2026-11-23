<script setup lang="ts">
import { ref, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import type { ChatMessage as ChatMessageType } from '@/types/coworks'
import type { BuiltinApp } from '@/types/coworks-persistence'
import ChatMessage from './ChatMessage.vue'
import ModelSelect from './ModelSelect.vue'

// Attachment interface
export interface Attachment {
  id: string
  name: string
  path: string
  size: number
  type: string
}

const MAX_ATTACHMENTS = 5

const props = withDefaults(defineProps<{
  messages: ChatMessageType[]
  isLoading: boolean
  error: string | null
  model: string
  activeApp?: BuiltinApp | null
  emptyHint?: string
}>(), {
  emptyHint: '输入你想要执行的操作开始对话',
})

const emit = defineEmits<{
  (e: 'send', message: string, attachments: Attachment[]): void
  (e: 'cancel'): void
  (e: 'clear'): void
  (e: 'clearApp'): void
  (e: 'update:model', model: string): void
}>()

const inputText = ref('')
const messagesContainer = ref<HTMLElement | null>(null)
const attachments = ref<Attachment[]>([])
const isDragging = ref(false)
const inputAreaRef = ref<HTMLElement | null>(null)

// Track task result status for completion/error indicators
const resultStatus = ref<'idle' | 'completed' | 'error'>('idle')
const showErrorDetail = ref(false)

// Detect loading → done/error transition
watch(() => props.isLoading, (loading, wasLoading) => {
  if (wasLoading && !loading) {
    resultStatus.value = props.error ? 'error' : 'completed'
    showErrorDetail.value = false
  }
})

// Reset status when new messages are added (user sends a new message)
watch(() => props.messages.length, (len, prevLen) => {
  if (len > (prevLen ?? 0)) {
    resultStatus.value = 'idle'
    showErrorDetail.value = false
  }
})

// Tauri drag-drop event listeners
let unlistenDrop: UnlistenFn | null = null
let unlistenHover: UnlistenFn | null = null
let unlistenLeave: UnlistenFn | null = null

onMounted(async () => {
  // Listen for Tauri drag-drop events
  unlistenHover = await listen<{ paths: string[]; position: { x: number; y: number } }>(
    'tauri://drag-over',
    (event) => {
      // Check if hovering over input area
      if (inputAreaRef.value) {
        const rect = inputAreaRef.value.getBoundingClientRect()
        const { x, y } = event.payload.position
        if (x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom) {
          isDragging.value = true
        }
      }
    }
  )

  unlistenLeave = await listen('tauri://drag-leave', () => {
    isDragging.value = false
  })

  unlistenDrop = await listen<{ paths: string[]; position: { x: number; y: number } }>(
    'tauri://drag-drop',
    (event) => {
      isDragging.value = false

      // Check if dropped on input area
      if (!inputAreaRef.value) return
      const rect = inputAreaRef.value.getBoundingClientRect()
      const { x, y } = event.payload.position
      if (x < rect.left || x > rect.right || y < rect.top || y > rect.bottom) return

      if (props.isLoading) return

      const remainingSlots = MAX_ATTACHMENTS - attachments.value.length
      const pathsToAdd = event.payload.paths.slice(0, remainingSlots)

      for (const filePath of pathsToAdd) {
        const fileName = filePath.split(/[/\\]/).pop() || filePath
        const attachment: Attachment = {
          id: `${Date.now()}-${Math.random().toString(36).substring(2, 9)}`,
          name: fileName,
          path: filePath,
          size: 0, // Size not available from Tauri drag-drop
          type: getFileType(fileName),
        }
        attachments.value.push(attachment)
      }
    }
  )
})

onUnmounted(() => {
  unlistenDrop?.()
  unlistenHover?.()
  unlistenLeave?.()
})

// Auto-scroll to bottom when messages change (new messages or loading a chat)
watch(
  () => props.messages,
  () => {
    nextTick(() => {
      if (messagesContainer.value) {
        messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
      }
    })
  },
  { immediate: true, deep: false }
)

// Also scroll when message count changes
watch(
  () => props.messages.length,
  () => {
    nextTick(() => {
      if (messagesContainer.value) {
        messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
      }
    })
  }
)

// Also scroll when content updates (streaming)
watch(
  () => props.messages[props.messages.length - 1]?.content,
  () => {
    nextTick(() => {
      if (messagesContainer.value) {
        messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
      }
    })
  }
)

function handleSend() {
  if (!inputText.value.trim() || props.isLoading) return
  emit('send', inputText.value, [...attachments.value])
  inputText.value = ''
  attachments.value = []
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    handleSend()
  }
}

// Attachment handling - use Tauri dialog to get full file paths
async function handleAttachClick() {
  if (props.isLoading) return

  const remainingSlots = MAX_ATTACHMENTS - attachments.value.length
  if (remainingSlots <= 0) return

  try {
    const selected = await open({
      multiple: true,
      title: '选择附件文件',
    })

    if (!selected) return

    // Handle both single and multiple selection
    const paths = Array.isArray(selected) ? selected : [selected]
    const pathsToAdd = paths.slice(0, remainingSlots)

    for (const filePath of pathsToAdd) {
      const fileName = filePath.split(/[/\\]/).pop() || filePath
      const attachment: Attachment = {
        id: `${Date.now()}-${Math.random().toString(36).substring(2, 9)}`,
        name: fileName,
        path: filePath,
        size: 0, // Size not available from dialog
        type: getFileType(fileName),
      }
      attachments.value.push(attachment)
    }
  } catch (error) {
    console.error('Failed to open file dialog:', error)
  }
}

function removeAttachment(id: string) {
  attachments.value = attachments.value.filter(a => a.id !== id)
}

function getFileType(fileName: string): string {
  const ext = fileName.split('.').pop()?.toLowerCase() || ''
  const typeMap: Record<string, string> = {
    png: 'image', jpg: 'image', jpeg: 'image', gif: 'image', webp: 'image', svg: 'image',
    pdf: 'document', doc: 'document', docx: 'document', txt: 'text', md: 'text',
    js: 'code', ts: 'code', py: 'code', json: 'code', html: 'code', css: 'code',
    mp3: 'audio', wav: 'audio', mp4: 'video', mov: 'video',
  }
  return typeMap[ext] || 'file'
}

function getAttachmentIcon(type: string): string {
  const iconMap: Record<string, string> = {
    image: '🖼️',
    document: '📄',
    text: '📝',
    code: '📜',
    audio: '🎵',
    video: '🎬',
    file: '📎',
  }
  return iconMap[type] || '📎'
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
</script>

<template>
  <div class="chat-panel">
    <!-- Active app context bar -->
    <div
      v-if="activeApp"
      class="app-context-bar"
    >
      <span class="app-icon">{{ activeApp.icon }}</span>
      <span class="app-name">{{ activeApp.name }}</span>
      <button
        class="clear-app-btn"
        @click="emit('clearApp')"
      >
        清除
      </button>
    </div>

    <!-- Messages area -->
    <div
      ref="messagesContainer"
      class="messages-container"
    >
      <div
        v-if="messages.length === 0"
        class="empty-state"
      >
        <div class="empty-icon">
          💬
        </div>
        <div class="empty-title">
          开始对话
        </div>
        <div class="empty-subtitle">
          {{ props.emptyHint }}
        </div>
      </div>

      <ChatMessage
        v-for="message in messages"
        :key="message.id"
        :message="message"
      />

      <div
        v-if="isLoading"
        class="loading-indicator"
      >
        <span class="loading-dot" />
        <span class="loading-dot" />
        <span class="loading-dot" />
      </div>

      <!-- Completion status -->
      <div
        v-if="resultStatus === 'completed' && !isLoading"
        class="status-bar completed"
      >
        <span class="status-icon-text">&#10003;</span>
        <span>任务已完成</span>
      </div>
    </div>

    <!-- Error display -->
    <div
      v-if="resultStatus === 'error' && !isLoading && error"
      class="status-bar error"
    >
      <div class="status-bar-main">
        <span class="status-icon-text">&#10007;</span>
        <span class="status-bar-message">任务出错</span>
        <button
          class="status-bar-toggle"
          @click="showErrorDetail = !showErrorDetail"
        >
          {{ showErrorDetail ? '收起' : '详情' }}
        </button>
        <button
          class="status-bar-clear"
          @click="emit('clear')"
        >
          清除
        </button>
      </div>
      <div
        v-if="showErrorDetail"
        class="status-bar-detail"
      >
        {{ error }}
      </div>
    </div>

    <!-- Input area -->
    <div
      ref="inputAreaRef"
      class="input-area"
      :class="{ 'drag-over': isDragging }"
    >
      <!-- Drop overlay -->
      <div
        v-if="isDragging"
        class="drop-overlay"
      >
        <span class="drop-icon">📎</span>
        <span class="drop-text">拖放文件添加附件</span>
      </div>

      <!-- Attachments preview -->
      <div
        v-if="attachments.length > 0"
        class="attachments-preview"
      >
        <div
          v-for="attachment in attachments"
          :key="attachment.id"
          class="attachment-item"
          :title="attachment.path"
        >
          <span class="attachment-icon">{{ getAttachmentIcon(getFileType(attachment.name)) }}</span>
          <span class="attachment-name">{{ attachment.name }}</span>
          <span class="attachment-size">{{ formatFileSize(attachment.size) }}</span>
          <button
            class="attachment-remove"
            @click="removeAttachment(attachment.id)"
          >
            ×
          </button>
        </div>
      </div>

      <textarea
        v-model="inputText"
        placeholder="输入消息... (Enter 发送, Shift+Enter 换行)"
        :disabled="isLoading"
        @keydown="handleKeydown"
      />
      <div class="input-bottom">
        <div class="input-left">
          <ModelSelect
            :model="model"
            @update:model="emit('update:model', $event)"
          />
          <!-- Attachment button -->
          <button
            class="attach-btn"
            :disabled="attachments.length >= MAX_ATTACHMENTS || isLoading"
            :title="attachments.length >= MAX_ATTACHMENTS ? '已达到最大附件数量' : '添加附件'"
            @click="handleAttachClick"
          >
            <span class="attach-icon">📎</span>
            <span
              v-if="attachments.length > 0"
              class="attach-count"
            >{{ attachments.length }}/{{ MAX_ATTACHMENTS }}</span>
          </button>
        </div>
        <div class="input-actions">
          <button
            v-if="isLoading"
            class="cancel-btn"
            @click="emit('cancel')"
          >
            取消
          </button>
          <button
            v-else
            class="send-btn"
            :disabled="!inputText.trim()"
            @click="handleSend"
          >
            发送
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.chat-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #FAF9F7;
}

/* Active app context bar */
.app-context-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  background: rgba(196, 112, 75, 0.08);
  border-bottom: 1px solid rgba(196, 112, 75, 0.2);
}

.app-context-bar .app-icon {
  font-size: 16px;
}

.app-context-bar .app-name {
  flex: 1;
  font-size: 13px;
  font-weight: 500;
  color: #C4704B;
}

.clear-app-btn {
  padding: 4px 8px;
  background: transparent;
  border: 1px solid rgba(196, 112, 75, 0.3);
  border-radius: 4px;
  color: #C4704B;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
}

.clear-app-btn:hover {
  background: rgba(196, 112, 75, 0.10);
}

.messages-container {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  text-align: center;
  color: #8B8680;
}

.empty-icon {
  font-size: 48px;
  margin-bottom: 16px;
}

.empty-title {
  font-size: 18px;
  font-weight: 600;
  margin-bottom: 8px;
  color: #2D2B2A;
}

.empty-subtitle {
  font-size: 14px;
  line-height: 1.6;
}

.loading-indicator {
  display: flex;
  gap: 4px;
  padding: 12px 16px;
}

.loading-dot {
  width: 8px;
  height: 8px;
  background: #C4704B;
  border-radius: 50%;
  animation: bounce 1.4s infinite ease-in-out both;
}

.loading-dot:nth-child(1) {
  animation-delay: -0.32s;
}

.loading-dot:nth-child(2) {
  animation-delay: -0.16s;
}

@keyframes bounce {
  0%, 80%, 100% {
    transform: scale(0);
  }
  40% {
    transform: scale(1);
  }
}

/* Status bar (completion / error) */
.status-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  font-size: 13px;
  flex-shrink: 0;
}

.status-bar.completed {
  color: #10b981;
  border-radius: 6px;
  margin: 8px 0;
}

.status-bar.error {
  flex-direction: column;
  align-items: stretch;
  background: rgba(217, 79, 79, 0.06);
  color: #D94F4F;
  border-top: 1px solid rgba(217, 79, 79, 0.15);
}

.status-icon-text {
  font-weight: 700;
  font-size: 14px;
}

.status-bar-main {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-bar-message {
  flex: 1;
  font-weight: 500;
}

.status-bar-toggle,
.status-bar-clear {
  background: transparent;
  border: 1px solid currentColor;
  color: inherit;
  padding: 2px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.15s;
}

.status-bar-toggle:hover,
.status-bar-clear:hover {
  background: rgba(217, 79, 79, 0.08);
}

.status-bar-detail {
  margin-top: 6px;
  padding: 8px;
  background: rgba(217, 79, 79, 0.04);
  border-radius: 4px;
  font-size: 12px;
  font-family: monospace;
  word-break: break-all;
  max-height: 120px;
  overflow-y: auto;
  color: #8B4040;
}

.input-area {
  border-top: 1px solid #E8E5E0;
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  position: relative;
}

.input-area.drag-over {
  background: rgba(196, 112, 75, 0.04);
}

.drop-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  background: rgba(250, 249, 247, 0.95);
  border: 2px dashed rgba(196, 112, 75, 0.4);
  border-radius: 8px;
  z-index: 10;
  pointer-events: none;
}

.drop-icon {
  font-size: 24px;
}

.drop-text {
  font-size: 13px;
  color: #C4704B;
}

.input-area textarea {
  width: 100%;
  background: #FFFFFF;
  border: 1px solid #E8E5E0;
  border-radius: 8px;
  padding: 12px;
  color: #2D2B2A;
  font-size: 14px;
  resize: none;
  min-height: 60px;
  max-height: 120px;
  font-family: inherit;
}

.input-area textarea:focus {
  outline: none;
  border-color: rgba(196, 112, 75, 0.5);
}

.input-area textarea::placeholder {
  color: #B0ACA7;
}

.input-area textarea:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.input-bottom {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.input-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.input-bottom :deep(.model-select) {
  background: transparent;
}

.input-bottom :deep(select) {
  background: #FFFFFF;
  font-size: 12px;
  padding: 4px 8px;
}

.attach-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  background: transparent;
  border: 1px solid #E8E5E0;
  border-radius: 4px;
  color: #2D2B2A;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
}

.attach-btn:hover:not(:disabled) {
  background: rgba(196, 112, 75, 0.06);
  border-color: rgba(196, 112, 75, 0.3);
}

.attach-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.attach-icon {
  font-size: 14px;
}

.attach-count {
  font-size: 11px;
  color: #C4704B;
}

.attachments-preview {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-bottom: 6px;
}

.attachment-item {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 6px;
  background: rgba(196, 112, 75, 0.06);
  border: 1px solid rgba(196, 112, 75, 0.15);
  border-radius: 4px;
  font-size: 11px;
}

.attachment-icon {
  font-size: 12px;
}

.attachment-name {
  color: #2D2B2A;
  max-width: 100px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.attachment-size {
  color: #8B8680;
  font-size: 10px;
}

.attachment-remove {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  padding: 0;
  background: transparent;
  border: none;
  border-radius: 50%;
  color: #8B8680;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
}

.attachment-remove:hover {
  background: rgba(217, 79, 79, 0.08);
  color: #D94F4F;
}

.input-actions {
  display: flex;
  align-items: center;
}

.send-btn,
.cancel-btn {
  padding: 8px 20px;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.send-btn {
  background: #C4704B;
  border: none;
  color: #FFFFFF;
}

.send-btn:hover:not(:disabled) {
  background: #A85A38;
}

.send-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.cancel-btn {
  background: transparent;
  border: 1px solid #D94F4F;
  color: #D94F4F;
}

.cancel-btn:hover {
  background: rgba(217, 79, 79, 0.06);
}
</style>
