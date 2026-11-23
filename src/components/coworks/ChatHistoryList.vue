<script setup lang="ts">
import { ask } from '@tauri-apps/plugin-dialog'
import type { ChatHistoryIndexItem } from '@/types/coworks-persistence'

defineProps<{
  histories: ChatHistoryIndexItem[]
  currentId: string
}>()

const emit = defineEmits<{
  (e: 'select', id: string): void
  (e: 'delete', id: string): void
}>()

// Format relative time
function formatTime(timestamp: number): string {
  const diff = Date.now() - timestamp
  const minutes = Math.floor(diff / 60000)
  const hours = Math.floor(diff / 3600000)
  const days = Math.floor(diff / 86400000)

  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes}分钟前`
  if (hours < 24) return `${hours}小时前`
  if (days < 7) return `${days}天前`

  return new Date(timestamp).toLocaleDateString('zh-CN')
}

// Handle delete with confirmation
async function handleDelete(id: string) {
  const confirmed = await ask('确定要删除这个对话吗？', {
    title: '删除确认',
    kind: 'warning',
  })
  if (confirmed) {
    emit('delete', id)
  }
}
</script>

<template>
  <div class="chat-history-list">
    <div
      v-if="histories.length === 0"
      class="empty-state"
    >
      <span class="empty-text">暂无对话历史</span>
    </div>

    <div
      v-for="history in histories"
      :key="history.id"
      class="history-item"
      :class="{ active: currentId === history.id }"
      @click="emit('select', history.id)"
    >
      <div class="history-content">
        <div class="history-title">
          {{ history.title }}
        </div>
        <div class="history-meta">
          <span class="history-time">{{ formatTime(history.updatedAt) }}</span>
          <span class="history-count">{{ history.messageCount }} 条消息</span>
        </div>
      </div>
      <button
        class="delete-btn"
        title="删除"
        @click.stop="handleDelete(history.id)"
      >
        ×
      </button>
    </div>
  </div>
</template>

<style scoped>
.chat-history-list {
  padding: 8px 0;
}

.empty-state {
  padding: 20px 12px;
  text-align: center;
}

.empty-text {
  font-size: 12px;
  color: #8B8680;
}

.history-item {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  cursor: pointer;
  transition: all 0.15s;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
}

.history-item:hover {
  background: rgba(0, 0, 0, 0.03);
}

.history-item.active {
  background: rgba(196, 112, 75, 0.08);
}

.history-content {
  flex: 1;
  min-width: 0;
}

.history-title {
  font-size: 13px;
  color: #2D2B2A;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-bottom: 4px;
}

.history-meta {
  display: flex;
  gap: 8px;
  font-size: 11px;
  color: #8B8680;
}

.delete-btn {
  display: none;
  padding: 4px 8px;
  background: transparent;
  border: none;
  color: #D94F4F;
  font-size: 16px;
  cursor: pointer;
  opacity: 0.7;
  transition: opacity 0.15s;
}

.history-item:hover .delete-btn {
  display: block;
}

.delete-btn:hover {
  opacity: 1;
}
</style>
