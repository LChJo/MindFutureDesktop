<script setup lang="ts">
import { ref } from 'vue'
import BotChatPanel from './BotChatPanel.vue'
import BotOutputViewer from './BotOutputViewer.vue'
import type { BotDefinition } from '@/types/claude-bot'

defineProps<{
  bot: BotDefinition
}>()

const emit = defineEmits<{
  close: []
}>()

const activeTab = ref<'chat' | 'output'>('chat')

const statusColors: Record<string, string> = {
  idle: '#8b949e',
  working: '#3fb950',
  error: '#f85149',
  stopped: '#d29922',
  paused: '#d29922',
}

const statusLabels: Record<string, string> = {
  idle: '空闲',
  working: '工作中',
  error: '错误',
  stopped: '已停止',
  paused: '已暂停',
}
</script>

<template>
  <div
    class="interaction-overlay"
    @click.self="emit('close')"
  >
    <div class="interaction-panel">
      <!-- Header -->
      <div class="panel-header">
        <div class="bot-identity">
          <span
            class="status-dot"
            :style="{ background: statusColors[bot.status] }"
          />
          <span class="bot-name">{{ bot.name }}</span>
          <span class="bot-status">{{ statusLabels[bot.status] || bot.status }}</span>
        </div>

        <div class="header-actions">
          <!-- Tab buttons -->
          <div class="tab-buttons">
            <button
              class="tab-btn"
              :class="{ active: activeTab === 'chat' }"
              @click="activeTab = 'chat'"
            >
              <i class="fa-solid fa-comments" />
              <span>对话</span>
            </button>
            <button
              class="tab-btn"
              :class="{ active: activeTab === 'output' }"
              @click="activeTab = 'output'"
            >
              <i class="fa-solid fa-display" />
              <span>输出</span>
            </button>
          </div>
          <button
            class="close-btn"
            @click="emit('close')"
          >
            <i class="fa-solid fa-xmark" />
          </button>
        </div>
      </div>

      <!-- Content: Tab-based -->
      <div class="panel-content">
        <div
          v-show="activeTab === 'chat'"
          class="tab-pane"
        >
          <BotChatPanel :bot="bot" />
        </div>
        <div
          v-show="activeTab === 'output'"
          class="tab-pane"
        >
          <BotOutputViewer :bot="bot" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped lang="less">
.interaction-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
}

.interaction-panel {
  width: 95%;
  height: 92%;
  max-width: 1200px;
  max-height: 800px;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid #30363d;
  background: #0d1117;
}

.bot-identity {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.bot-name {
  font-size: 14px;
  font-weight: 600;
  color: #e6edf3;
}

.bot-status {
  font-size: 11px;
  color: #8b949e;
  text-transform: uppercase;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.tab-buttons {
  display: flex;
  gap: 2px;
  background: #010409;
  border-radius: 6px;
  padding: 2px;
}

.tab-btn {
  padding: 6px 12px;
  background: transparent;
  border: none;
  border-radius: 4px;
  color: #8b949e;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  transition: all 0.15s;

  &:hover {
    color: #e6edf3;
    background: #21262d;
  }

  &.active {
    color: #e6edf3;
    background: #21262d;
  }
}

.close-btn {
  width: 28px;
  height: 28px;
  background: none;
  border: 1px solid transparent;
  border-radius: 6px;
  color: #8b949e;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;

  &:hover {
    background: #21262d;
    color: #e6edf3;
    border-color: #30363d;
  }
}

.panel-content {
  flex: 1;
  min-height: 0;
  display: flex;
  overflow: hidden;
}

.tab-pane {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>
