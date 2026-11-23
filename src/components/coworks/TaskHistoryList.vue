<script setup lang="ts">
import { ask } from '@tauri-apps/plugin-dialog'
import type { TaskIndexItem } from '@/types/coworks-persistence'

defineProps<{
  tasks: TaskIndexItem[]
  currentId: string
  getDisplayName: (task: TaskIndexItem) => string
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

// Get status icon
function getStatusIcon(status: string): string {
  switch (status) {
    case 'active': return '🔵'
    case 'completed': return '✅'
    case 'archived': return '📦'
    default: return '📋'
  }
}

// Handle delete with confirmation
async function handleDelete(id: string) {
  const confirmed = await ask('确定要删除这个任务吗？', {
    title: '删除确认',
    kind: 'warning',
  })
  if (confirmed) {
    emit('delete', id)
  }
}
</script>

<template>
  <div class="task-history-list">
    <div
      v-if="tasks.length === 0"
      class="empty-state"
    >
      <span class="empty-text">暂无任务历史</span>
    </div>

    <div
      v-for="task in tasks"
      :key="task.id"
      class="task-item"
      :class="{ active: currentId === task.id }"
      @click="emit('select', task.id)"
    >
      <div class="task-content">
        <div class="task-header">
          <span class="status-icon">{{ getStatusIcon(task.status) }}</span>
          <span class="task-name">{{ task.name }}</span>
        </div>
        <div class="task-meta">
          <span class="task-dir">{{ task.workDir.split(/[/\\]/).pop() }}</span>
          <span class="task-time">{{ formatTime(task.updatedAt) }}</span>
        </div>
      </div>
      <button
        class="delete-btn"
        title="删除"
        @click.stop="handleDelete(task.id)"
      >
        ×
      </button>
    </div>
  </div>
</template>

<style scoped>
.task-history-list {
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

.task-item {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  cursor: pointer;
  transition: all 0.15s;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
}

.task-item:hover {
  background: rgba(0, 0, 0, 0.03);
}

.task-item.active {
  background: rgba(16, 185, 129, 0.10);
}

.task-content {
  flex: 1;
  min-width: 0;
}

.task-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}

.status-icon {
  font-size: 10px;
}

.task-name {
  font-size: 13px;
  color: #2D2B2A;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.task-meta {
  display: flex;
  gap: 8px;
  font-size: 11px;
  color: #8B8680;
}

.task-dir {
  color: #10b981;
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

.task-item:hover .delete-btn {
  display: block;
}

.delete-btn:hover {
  opacity: 1;
}
</style>
