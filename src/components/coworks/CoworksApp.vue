<script setup lang="ts">
import { onMounted } from 'vue'
import ModeSwitch from './ModeSwitch.vue'
import ChatModeView from './views/ChatModeView.vue'
import CodeModeView from './views/CodeModeView.vue'
import CoworkModeView from './views/CoworkModeView.vue'
import { useChat } from '@/composables/coworks'
import type { AppMode } from '@/types/coworks'

const chat = useChat()

// Initialize on mount - sessions survive window close as module-level singletons
onMounted(async () => {
  await chat.initialize()

  // Restore previous mode/context if session is still alive (window was closed and reopened)
  if (chat.contextId.value) {
    await chat.setMode(chat.mode.value)
  }
})

// Handle mode change
function handleModeChange(mode: AppMode) {
  chat.setMode(mode)
}
</script>

<template>
  <div class="coworks-app-container">
    <!-- Sidebar: Logo + Mode Switch -->
    <aside class="coworks-sidebar">
      <div class="sidebar-header">
        <h1 class="app-title">
          Cowork
        </h1>
      </div>
      <div class="mode-section">
        <ModeSwitch
          :mode="chat.mode.value"
          @update:mode="handleModeChange"
        />
      </div>
    </aside>

    <!-- Main content based on mode -->
    <main class="main-content">
      <ChatModeView v-if="chat.mode.value === 'chat'" />
      <CodeModeView v-else-if="chat.mode.value === 'code'" />
      <CoworkModeView v-else-if="chat.mode.value === 'cowork'" />
    </main>
  </div>
</template>

<style scoped>
.coworks-app-container {
  display: flex;
  height: 100%;
  width: 100%;
  background: #FAF9F7;
  color: #2D2B2A;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen,
    Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
}

/* Sidebar */
.coworks-sidebar {
  display: flex;
  flex-direction: column;
  width: 80px;
  background: #FFFFFF;
  border-right: 1px solid #E8E5E0;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px 8px;
  border-bottom: 1px solid #E8E5E0;
}

.app-title {
  font-size: 14px;
  font-weight: 600;
  background: linear-gradient(135deg, #C4704B, #A85A38);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.mode-section {
  padding: 12px 8px;
}

.mode-section :deep(.mode-switch) {
  flex-direction: column;
  gap: 4px;
}

.mode-section :deep(.mode-btn) {
  flex-direction: column;
  gap: 2px;
  padding: 8px 4px;
  width: 100%;
}

.mode-section :deep(.mode-label) {
  font-size: 10px;
}

/* Main content */
.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>
