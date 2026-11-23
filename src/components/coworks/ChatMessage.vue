<script setup lang="ts">
import { computed } from 'vue'
import { marked } from 'marked'
import type { ChatMessage } from '@/types/coworks'
import ToolUsage from './ToolUsage.vue'

const props = defineProps<{
  message: ChatMessage
}>()

// Configure marked for safe rendering
marked.setOptions({
  breaks: true,
  gfm: true,
})

const renderedContent = computed(() => {
  if (!props.message.content) return ''
  return marked.parse(props.message.content) as string
})

const timeString = computed(() => {
  const date = new Date(props.message.timestamp)
  return date.toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
  })
})
</script>

<template>
  <div
    class="chat-message"
    :class="message.role"
  >
    <div class="message-header">
      <span class="role-label">
        {{ message.role === 'user' ? '你' : 'Claude' }}
      </span>
      <span class="timestamp">{{ timeString }}</span>
    </div>
    <div
      class="message-content"
      v-html="renderedContent"
    />
    <div
      v-if="message.toolUsages?.length"
      class="tool-usages"
    >
      <ToolUsage
        v-for="usage in message.toolUsages"
        :key="usage.id"
        :usage="usage"
      />
    </div>
  </div>
</template>

<style scoped>
.chat-message {
  padding: 12px 16px;
  border-radius: 8px;
  margin-bottom: 12px;
}

.chat-message.user {
  background: #F0EDE8;
  margin-left: 24px;
}

.chat-message.assistant {
  background: #FFFFFF;
  margin-right: 24px;
}

.message-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.role-label {
  font-weight: 600;
  font-size: 13px;
}

.user .role-label {
  color: #C4704B;
}

.assistant .role-label {
  color: #2D2B2A;
}

.timestamp {
  font-size: 11px;
  color: #8B8680;
}

.message-content {
  font-size: 14px;
  line-height: 1.6;
}

.message-content :deep(p) {
  margin: 0 0 8px 0;
}

.message-content :deep(p:last-child) {
  margin-bottom: 0;
}

.message-content :deep(code) {
  background: #F5F3F0;
  padding: 2px 6px;
  border-radius: 4px;
  font-family: 'Fira Code', monospace;
  font-size: 13px;
  color: #C4704B;
}

.message-content :deep(pre) {
  background: #F5F3F0;
  padding: 12px;
  border-radius: 6px;
  overflow-x: auto;
  margin: 8px 0;
}

.message-content :deep(pre code) {
  background: none;
  padding: 0;
}

.message-content :deep(ul),
.message-content :deep(ol) {
  margin: 8px 0;
  padding-left: 20px;
}

.message-content :deep(li) {
  margin: 4px 0;
}

.tool-usages {
  margin-top: 12px;
}
</style>
