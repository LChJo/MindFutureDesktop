<script setup lang="ts">
import type { WorkLog } from '@/types/coworks-persistence'

defineProps<{
  workLogs: WorkLog[]
}>()

// Format relative time
function formatTime(timestamp: number): string {
  const date = new Date(timestamp)
  return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
}
</script>

<template>
  <div class="work-panel">
    <div
      v-if="workLogs.length === 0"
      class="empty-state"
    >
      <span class="empty-text">暂无工作日志</span>
    </div>

    <div
      v-else
      class="log-list"
    >
      <div
        v-for="log in workLogs"
        :key="log.id"
        class="log-item"
      >
        <span class="log-time">{{ formatTime(log.timestamp) }}</span>
        <span class="log-action">{{ log.action }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.work-panel {
  flex: 1;
  overflow-y: auto;
}

.empty-state {
  padding: 12px;
  text-align: center;
}

.empty-text {
  font-size: 12px;
  color: #8B8680;
}

.log-list {
  padding: 8px 0;
}

.log-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 12px;
  font-size: 12px;
}

.log-time {
  flex-shrink: 0;
  color: #8B8680;
  font-family: monospace;
}

.log-action {
  color: #10b981;
  line-height: 1.4;
}
</style>
