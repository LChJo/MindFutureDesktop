<script setup lang="ts">
import { ref, computed, nextTick, watch, onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { useBotSession } from '@/composables/claudeBot/useBotSession'
import type { BotDefinition } from '@/types/claude-bot'

interface Attachment {
  id: string
  name: string
  path: string
  size: number
  type: string
}

const props = defineProps<{
  bot: BotDefinition
}>()

const botSession = useBotSession()
const inputText = ref('')
const messagesContainer = ref<HTMLDivElement | null>(null)
const inputRef = ref<HTMLTextAreaElement | null>(null)
const inputAreaRef = ref<HTMLDivElement | null>(null)

// Attachment state
const attachments = ref<Attachment[]>([])
const isDragging = ref(false)
const MAX_ATTACHMENTS = 5

// Drag-drop listeners
let unlistenDrop: UnlistenFn | null = null
let unlistenHover: UnlistenFn | null = null
let unlistenLeave: UnlistenFn | null = null

onMounted(async () => {
  await botSession.ensureConversationLoaded(props.bot.id)
  scrollToBottom()
  inputRef.value?.focus()

  unlistenHover = await listen<{ paths: string[]; position: { x: number; y: number } }>(
    'tauri://drag-over',
    (event) => {
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
      if (!inputAreaRef.value) return
      const rect = inputAreaRef.value.getBoundingClientRect()
      const { x, y } = event.payload.position
      if (x < rect.left || x > rect.right || y < rect.top || y > rect.bottom) return
      if (botSession.isLoading(props.bot.id)) return

      const remainingSlots = MAX_ATTACHMENTS - attachments.value.length
      const pathsToAdd = event.payload.paths.slice(0, remainingSlots)

      for (const filePath of pathsToAdd) {
        const fileName = filePath.split(/[/\\]/).pop() || filePath
        attachments.value = [
          ...attachments.value,
          {
            id: `${Date.now()}-${Math.random().toString(36).substring(2, 9)}`,
            name: fileName,
            path: filePath,
            size: 0,
            type: getFileType(fileName),
          },
        ]
      }
    }
  )
})

onUnmounted(() => {
  unlistenDrop?.()
  unlistenHover?.()
  unlistenLeave?.()
})

const messages = computed(() => botSession.getMessages(props.bot.id))

const visibleMessages = computed(() =>
  messages.value.filter(m => !m.isSystem)
)

watch(messages, () => {
  nextTick(() => scrollToBottom())
}, { deep: true })

function scrollToBottom() {
  const el = messagesContainer.value
  if (el) {
    el.scrollTop = el.scrollHeight
  }
}

async function handleSend() {
  const text = inputText.value.trim()
  if (!text) return
  const currentAttachments = attachments.value.map(a => ({ path: a.path, name: a.name }))
  inputText.value = ''
  attachments.value = []
  await botSession.sendMessage(
    props.bot,
    text,
    currentAttachments.length > 0 ? currentAttachments : undefined,
  )
  nextTick(() => scrollToBottom())
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    handleSend()
  }
}

async function handleAttachClick() {
  if (botSession.isLoading(props.bot.id)) return
  const remainingSlots = MAX_ATTACHMENTS - attachments.value.length
  if (remainingSlots <= 0) return

  try {
    const selected = await open({
      multiple: true,
      title: '选择附件文件',
    })
    if (!selected) return

    const paths = Array.isArray(selected) ? selected : [selected]
    const pathsToAdd = paths.slice(0, remainingSlots)

    for (const filePath of pathsToAdd) {
      const fileName = filePath.split(/[/\\]/).pop() || filePath
      attachments.value = [
        ...attachments.value,
        {
          id: `${Date.now()}-${Math.random().toString(36).substring(2, 9)}`,
          name: fileName,
          path: filePath,
          size: 0,
          type: getFileType(fileName),
        },
      ]
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
    image: 'fa-solid fa-image',
    document: 'fa-solid fa-file-pdf',
    text: 'fa-solid fa-file-lines',
    code: 'fa-solid fa-file-code',
    audio: 'fa-solid fa-music',
    video: 'fa-solid fa-film',
    file: 'fa-solid fa-paperclip',
  }
  return iconMap[type] || 'fa-solid fa-paperclip'
}
</script>

<template>
  <div class="bot-chat-panel">
    <!-- Messages -->
    <div
      ref="messagesContainer"
      class="messages-area"
    >
      <div
        v-if="visibleMessages.length === 0"
        class="empty-chat"
      >
        <i class="fa-solid fa-comments" />
        <span>向 {{ bot.name }} 发送消息</span>
      </div>

      <div
        v-for="msg in visibleMessages"
        :key="msg.id"
        class="message"
        :class="msg.role"
      >
        <div class="message-header">
          <span class="role-badge">
            <i :class="msg.role === 'user' ? 'fa-solid fa-user' : 'fa-solid fa-frog'" />
            {{ msg.role === 'user' ? '你' : bot.name }}
          </span>
          <span class="timestamp">
            {{ new Date(msg.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) }}
          </span>
        </div>
        <div class="message-content">
          <pre v-if="msg.content">{{ msg.content }}</pre>
          <span
            v-else
            class="typing-indicator"
          >
            <span class="dot" /><span class="dot" /><span class="dot" />
          </span>
        </div>
        <!-- Tool usages -->
        <div
          v-if="msg.toolUsages?.length"
          class="tool-usages"
        >
          <div
            v-for="tool in msg.toolUsages"
            :key="tool.id"
            class="tool-item"
          >
            <i class="fa-solid fa-wrench" />
            <span class="tool-name">{{ tool.toolName }}</span>
            <span
              class="tool-status"
              :class="tool.status"
            >{{ tool.status }}</span>
            <span
              v-if="tool.summary"
              class="tool-summary"
            >{{ tool.summary }}</span>
          </div>
        </div>
      </div>
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
        <i
          :class="getAttachmentIcon(attachment.type)"
          class="attachment-icon"
        />
        <span class="attachment-name">{{ attachment.name }}</span>
        <button
          class="attachment-remove"
          @click="removeAttachment(attachment.id)"
        >
          <i class="fa-solid fa-xmark" />
        </button>
      </div>
    </div>

    <!-- Input area -->
    <div
      ref="inputAreaRef"
      class="input-area"
      :class="{ 'drag-over': isDragging }"
    >
      <!-- Drag overlay -->
      <div
        v-if="isDragging"
        class="drag-overlay"
      >
        <i class="fa-solid fa-cloud-arrow-up" />
        <span>拖放文件到此处</span>
      </div>

      <button
        class="attach-btn"
        :disabled="botSession.isLoading(bot.id) || attachments.length >= MAX_ATTACHMENTS"
        title="附加文件"
        @click="handleAttachClick"
      >
        <i class="fa-solid fa-paperclip" />
      </button>
      <textarea
        ref="inputRef"
        v-model="inputText"
        rows="1"
        placeholder="输入消息..."
        :disabled="botSession.isLoading(bot.id)"
        @keydown="handleKeydown"
      />
      <button
        class="send-btn"
        :disabled="!inputText.trim() || botSession.isLoading(bot.id)"
        @click="handleSend"
      >
        <i
          v-if="botSession.isLoading(bot.id)"
          class="fa-solid fa-spinner fa-spin"
        />
        <i
          v-else
          class="fa-solid fa-paper-plane"
        />
      </button>
    </div>

    <!-- Error display -->
    <div
      v-if="botSession.getError(bot.id)"
      class="error-bar"
    >
      <i class="fa-solid fa-exclamation-triangle" />
      {{ botSession.getError(bot.id) }}
    </div>
  </div>
</template>

<style scoped lang="less">
.bot-chat-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #0d1117;
}

.messages-area {
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.empty-chat {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 100%;
  color: #6e7681;
  font-size: 13px;

  i {
    font-size: 24px;
    opacity: 0.5;
  }
}

.message {
  &.user {
    .message-content {
      background: rgba(88, 166, 255, 0.08);
      border-left: 2px solid #58a6ff;
    }
  }

  &.assistant {
    .message-content {
      background: #161b22;
      border-left: 2px solid #3fb950;
    }
  }
}

.message-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
  font-size: 11px;
}

.role-badge {
  display: flex;
  align-items: center;
  gap: 4px;
  color: #8b949e;
  font-weight: 600;

  i {
    font-size: 10px;
  }
}

.timestamp {
  color: #6e7681;
}

.message-content {
  padding: 10px 12px;
  border-radius: 6px;
  font-size: 13px;
  line-height: 1.5;
  color: #e6edf3;

  pre {
    margin: 0;
    white-space: pre-wrap;
    word-wrap: break-word;
    font-family: inherit;
    font-size: inherit;
    color: inherit;
  }
}

.typing-indicator {
  display: inline-flex;
  gap: 4px;
  padding: 4px 0;

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #6e7681;
    animation: typing-bounce 1.4s ease-in-out infinite;

    &:nth-child(2) { animation-delay: 0.2s; }
    &:nth-child(3) { animation-delay: 0.4s; }
  }
}

@keyframes typing-bounce {
  0%, 60%, 100% { transform: translateY(0); }
  30% { transform: translateY(-4px); }
}

.tool-usages {
  margin-top: 6px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.tool-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  background: #010409;
  border-radius: 4px;
  font-size: 11px;
  color: #8b949e;

  .tool-name {
    color: #e6edf3;
    font-weight: 500;
  }

  .tool-status {
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 10px;

    &.running { background: rgba(63, 185, 80, 0.1); color: #3fb950; }
    &.completed { background: rgba(88, 166, 255, 0.1); color: #58a6ff; }
    &.error { background: rgba(248, 81, 73, 0.1); color: #f85149; }
    &.pending { background: #21262d; color: #8b949e; }
  }

  .tool-summary {
    color: #6e7681;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 200px;
  }
}

.attachments-preview {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 8px 16px 0;
  background: #161b22;
}

.attachment-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  background: #21262d;
  border: 1px solid #30363d;
  border-radius: 6px;
  font-size: 11px;
  color: #e6edf3;
  max-width: 200px;
}

.attachment-icon {
  color: #58a6ff;
  font-size: 12px;
  flex-shrink: 0;
}

.attachment-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.attachment-remove {
  background: none;
  border: none;
  color: #6e7681;
  cursor: pointer;
  padding: 0 2px;
  font-size: 11px;
  flex-shrink: 0;

  &:hover {
    color: #f85149;
  }
}

.input-area {
  display: flex;
  gap: 8px;
  padding: 12px 16px;
  border-top: 1px solid #30363d;
  background: #161b22;
  position: relative;
  align-items: flex-end;

  &.drag-over {
    border-color: #58a6ff;
    background: rgba(88, 166, 255, 0.04);
  }

  textarea {
    flex: 1;
    box-sizing: border-box;
    padding: 7px 12px;
    background: #0d1117;
    border: 1px solid #30363d;
    border-radius: 8px;
    color: #e6edf3;
    font-size: 13px;
    line-height: 20px;
    font-family: inherit;
    resize: none;
    min-height: 36px;
    max-height: 120px;

    &:focus {
      outline: none;
      border-color: #58a6ff;
    }

    &::placeholder {
      color: #6e7681;
    }

    &:disabled {
      opacity: 0.6;
    }
  }
}

.drag-overlay {
  position: absolute;
  inset: 0;
  background: rgba(88, 166, 255, 0.08);
  border: 2px dashed #58a6ff;
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  z-index: 10;
  color: #58a6ff;
  font-size: 12px;
  font-weight: 600;

  i {
    font-size: 20px;
  }
}

.attach-btn {
  width: 36px;
  height: 36px;
  background: #21262d;
  border: 1px solid #30363d;
  border-radius: 8px;
  color: #8b949e;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  transition: all 0.2s;
  flex-shrink: 0;

  &:hover:not(:disabled) {
    background: #30363d;
    color: #58a6ff;
  }

  &:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
}

.send-btn {
  width: 36px;
  height: 36px;
  background: #238636;
  border: 1px solid #2ea043;
  border-radius: 8px;
  color: #ffffff;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  transition: background 0.2s;
  flex-shrink: 0;

  &:hover:not(:disabled) {
    background: #2ea043;
  }

  &:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
}

.error-bar {
  padding: 8px 16px;
  background: rgba(248, 81, 73, 0.1);
  color: #f85149;
  font-size: 12px;
  display: flex;
  align-items: center;
  gap: 6px;
  border-top: 1px solid rgba(248, 81, 73, 0.2);
}
</style>
