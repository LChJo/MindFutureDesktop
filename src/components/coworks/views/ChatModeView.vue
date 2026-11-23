<script setup lang="ts">
import { computed } from 'vue'
import ChatPanel from '../ChatPanel.vue'
import ChatHistoryList from '../ChatHistoryList.vue'
import { useChat } from '@/composables/coworks'

const chat = useChat()

// Handle chat send
async function handleChatSend(message: string, attachments: any[] = []) {
  const attachmentInfos = attachments.map(a => ({ path: a.path, name: a.name }))
  await chat.sendMessage(message, attachmentInfos.length > 0 ? attachmentInfos : undefined)
}

// Handle create new chat
async function handleNewChat() {
  await chat.createNewChat()
}

// Handle load chat
async function handleLoadChat(id: string) {
  await chat.loadChat(id)
}

// Handle delete chat
async function handleDeleteChat(id: string) {
  await chat.deleteChat(id)
}

// Empty state hint based on whether there are chats
const emptyHint = computed(() => {
  if (chat.chatHistories.value.length === 0) {
    return '输入你想问的问题开始对话'
  }
  return '选择一个历史对话或开始新对话'
})
</script>

<template>
  <div class="chat-mode-view">
    <!-- Left panel: Chat history -->
    <div class="left-panel">
      <div class="panel-header">
        <button
          class="new-chat-btn"
          @click="handleNewChat"
        >
          + 新建对话
        </button>
      </div>
      <div class="history-container">
        <ChatHistoryList
          :histories="chat.chatHistories.value"
          :current-id="chat.contextId.value"
          @select="handleLoadChat"
          @delete="handleDeleteChat"
        />
      </div>
    </div>

    <!-- Main panel: Chat -->
    <div class="main-panel">
      <ChatPanel
        v-if="chat.contextId.value"
        :messages="chat.messages.value"
        :is-loading="chat.isLoading.value"
        :error="chat.error.value"
        :model="chat.model.value"
        :empty-hint="emptyHint"
        @send="handleChatSend"
        @cancel="chat.cancelRequest"
        @clear="chat.clearError"
        @update:model="chat.model.value = $event"
      />
      <div
        v-else
        class="empty-placeholder"
      >
        <i class="fa-solid fa-comments" />
        <span>选择一个历史对话或点击上方按钮新建对话</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.chat-mode-view {
  display: flex;
  height: 100%;
  width: 100%;
  background: #FAF9F7;
}

.left-panel {
  width: 200px;
  display: flex;
  flex-direction: column;
  border-right: 1px solid #E8E5E0;
  background: #FFFFFF;
}

.panel-header {
  padding: 12px;
  border-bottom: 1px solid #E8E5E0;
}

.new-chat-btn {
  width: 100%;
  padding: 10px 16px;
  background: rgba(196, 112, 75, 0.10);
  border: 1px solid rgba(196, 112, 75, 0.2);
  border-radius: 8px;
  color: #C4704B;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.new-chat-btn:hover {
  background: rgba(196, 112, 75, 0.18);
}

.history-container {
  flex: 1;
  overflow-y: auto;
}

.main-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
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
}

.empty-placeholder i {
  font-size: 32px;
  opacity: 0.5;
}
</style>
