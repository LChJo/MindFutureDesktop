<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useBotStore } from '@/stores/botStore'
import { useWindowStore } from '@/stores/windowStore'
import { useDesktopStore } from '@/stores/desktopStore'
import BotCreationForm from './BotCreationForm.vue'
import BotList from './BotList.vue'
import BotActivityArea from './BotActivityArea.vue'
import BotInteractionPanel from './BotInteractionPanel.vue'

defineProps<{
  windowId?: string
  windowData?: Record<string, unknown>
}>()

const botStore = useBotStore()
const windowStore = useWindowStore()
const desktopStore = useDesktopStore()
const showCreationForm = ref(false)

onMounted(async () => {
  await botStore.loadBots()
})

function handleAddBot() {
  showCreationForm.value = true
}

function handleCancelCreate() {
  showCreationForm.value = false
}

function handleBotCreated() {
  showCreationForm.value = false
}

function handleBotClick(botId: string) {
  botStore.selectBotForInteraction(botId)
}

function handleCloseInteraction() {
  botStore.selectBotForInteraction(null)
}

function openModelSettings() {
  const settingsIcon = desktopStore.icons.find(icon => icon.type === 'settings') ?? {
    id: 'settings',
    name: '设置',
    icon: 'fa-solid fa-gear',
    position: { x: 0, y: 0 },
    type: 'settings' as const
  }
  windowStore.openWindow(settingsIcon, { initialTab: 'coworks' })
}
</script>

<template>
  <div class="claude-bot">
    <!-- Left Panel -->
    <div class="left-panel">
      <div class="panel-header">
        <i class="fa-solid fa-frog" />
        <span>Claude Bot</span>
      </div>

      <!-- Add Bot Button -->
      <div class="section">
        <button
          class="add-bot-btn"
          @click="handleAddBot"
        >
          <i class="fa-solid fa-plus" />
          <span>Add Bot</span>
        </button>
      </div>

      <!-- Creation Form -->
      <BotCreationForm
        v-if="showCreationForm"
        @created="handleBotCreated"
        @cancel="handleCancelCreate"
      />

      <!-- Bot List -->
      <div class="section bot-list-section">
        <div class="section-title">
          机器人 ({{ botStore.bots.length }})
        </div>
        <BotList />
      </div>

      <!-- Model Settings -->
      <div class="web-server-control">
        <button
          class="server-btn"
          @click="openModelSettings"
        >
          <i class="fa-solid fa-sliders" />
          配置模型
        </button>
      </div>
    </div>

    <!-- Right Panel -->
    <div class="right-panel">
      <BotActivityArea
        :bots="botStore.bots"
        @bot-click="handleBotClick"
      />
    </div>

    <!-- Interaction Panel Overlay -->
    <BotInteractionPanel
      v-if="botStore.interactionBot"
      :bot="botStore.interactionBot"
      @close="handleCloseInteraction"
    />
  </div>
</template>

<style scoped lang="less">
.claude-bot {
  display: flex;
  width: 100%;
  height: 100%;
  background: #0d1117;
  color: #e6edf3;
  position: relative;
  overflow: hidden;
}

.left-panel {
  width: 300px;
  min-width: 300px;
  border-right: 1px solid #30363d;
  display: flex;
  flex-direction: column;
  background: #161b22;
  overflow-y: auto;
}

.panel-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 16px;
  font-size: 16px;
  font-weight: 600;
  border-bottom: 1px solid #30363d;
  color: #58a6ff;

  i {
    font-size: 18px;
  }
}

.section {
  padding: 12px 16px;
  border-bottom: 1px solid #30363d;
}

.section-title {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: #8b949e;
  margin-bottom: 8px;
}

.add-bot-btn {
  width: 100%;
  padding: 10px 12px;
  background: #238636;
  border: 1px solid #2ea043;
  border-radius: 8px;
  color: #ffffff;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  transition: all 0.2s;

  &:hover {
    background: #2ea043;
  }
}

.bot-list-section {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}

.web-server-control {
  padding: 12px 16px;
  border-top: 1px solid #30363d;
}

.server-btn {
  width: 100%;
  padding: 8px 12px;
  background: #21262d;
  border: 1px solid #30363d;
  border-radius: 6px;
  color: #e6edf3;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  font-size: 12px;
  transition: background 0.2s;

  &:hover {
    background: #30363d;
  }
}

.coming-soon-toast {
  margin-top: 8px;
  padding: 6px 12px;
  background: #30363d;
  border: 1px solid #484f58;
  border-radius: 6px;
  font-size: 12px;
  color: #8b949e;
  text-align: center;
}

.toast-enter-active,
.toast-leave-active {
  transition: opacity 0.3s ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
}

.right-panel {
  flex: 1;
  min-width: 0;
  position: relative;
}
</style>
