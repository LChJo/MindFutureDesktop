<script setup lang="ts">
import { useChat } from '@/composables/coworks'

const chat = useChat()

// Get folder name from path
function getFolderName(path: string): string {
  return path.split(/[/\\]/).pop() || path
}
</script>

<template>
  <div class="context-panel">
    <div
      v-if="!chat.workDir.value"
      class="empty-state"
    >
      <span class="empty-text">未选择项目</span>
    </div>

    <div
      v-else
      class="context-content"
    >
      <!-- Project name -->
      <div class="context-section">
        <div class="section-label">
          项目
        </div>
        <div class="project-name">
          {{ getFolderName(chat.workDir.value) }}
        </div>
      </div>

      <!-- Project path -->
      <div class="context-section">
        <div class="section-label">
          路径
        </div>
        <div class="path-value">
          {{ chat.workDir.value }}
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.context-panel {
  padding: 12px;
}

.empty-state {
  text-align: center;
  padding: 8px;
}

.empty-text {
  font-size: 12px;
  color: #8B8680;
}

.context-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.context-section {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.section-label {
  font-size: 11px;
  font-weight: 600;
  color: #8B8680;
  text-transform: uppercase;
}

.project-name {
  font-size: 13px;
  color: #2D2B2A;
  font-weight: 500;
}

.path-value {
  font-size: 11px;
  color: #8B8680;
  word-break: break-all;
}
</style>
