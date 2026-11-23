<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useBotStore } from '@/stores/botStore'
import { useBotSession } from '@/composables/claudeBot/useBotSession'
import { useBotChannels } from '@/composables/claudeBot/useBotChannels'
import ChannelConfigDialog from './ChannelConfigDialog.vue'
import ScheduleConfigDialog from './ScheduleConfigDialog.vue'
import BotEditDialog from './BotEditDialog.vue'
import ChannelStatusBadge from './ChannelStatusBadge.vue'
import type { BotDefinition } from '@/types/claude-bot'

const botStore = useBotStore()
const contextMenuBot = ref<string | null>(null)
const contextMenuPos = ref({ x: 0, y: 0 })
const channelDialogBotId = ref<string | null>(null)
const scheduleDialogBotId = ref<string | null>(null)
const editDialogBotId = ref<string | null>(null)

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

function handleContextMenu(e: MouseEvent, botId: string) {
  e.preventDefault()
  e.stopPropagation()
  contextMenuBot.value = botId
  contextMenuPos.value = { x: e.clientX, y: e.clientY }
}

function closeContextMenu() {
  contextMenuBot.value = null
}

async function handleDelete(botId: string) {
  closeContextMenu()
  const botSession = useBotSession()
  await botSession.stopSession(botId)
  await botStore.deleteBot(botId)
}

async function handleToggleStatus(bot: BotDefinition) {
  closeContextMenu()
  const channels = useBotChannels()
  if (bot.status !== 'stopped') {
    // If the bot is actively working, stop the sidecar session first so its
    // 'done' callback cannot race and overwrite the 'stopped' status we set below.
    if (bot.status === 'working') {
      const botSession = useBotSession()
      await botSession.stopSession(bot.id)
    }
    await channels.stopListening(bot.id)
    await botStore.updateBotStatus(bot.id, 'stopped')
  } else {
    await botStore.updateBotStatus(bot.id, 'idle')
    const updatedBot = botStore.bots.find(b => b.id === bot.id)
    if (updatedBot) {
      const hasEnabled = await channels.hasEnabledChannels(bot.id)
      if (hasEnabled) {
        channels.startListening(updatedBot).catch(e =>
          console.error('[BotList] startListening failed:', e)
        )
      }
    }
  }
}

function handleBotClick(botId: string) {
  botStore.selectBotForInteraction(botId)
}

function openChannelConfig(botId: string) {
  closeContextMenu()
  channelDialogBotId.value = botId
}

function closeChannelDialog() {
  channelDialogBotId.value = null
}

function openScheduleConfig(botId: string) {
  closeContextMenu()
  scheduleDialogBotId.value = botId
}

function closeScheduleDialog() {
  scheduleDialogBotId.value = null
}

function openEditDialog(botId: string) {
  closeContextMenu()
  editDialogBotId.value = botId
}

function closeEditDialog() {
  editDialogBotId.value = null
}

onMounted(() => {
  for (const bot of botStore.bots) {
    botStore.refreshChannelStatus(bot.id)
  }
})

function formatTime(ts?: number): string {
  if (!ts) return '从未'
  const d = new Date(ts)
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}
</script>

<template>
  <div
    class="bot-list"
    @click="closeContextMenu"
  >
    <div
      v-if="botStore.failedBots.length > 0"
      class="config-error-banner"
    >
      <i class="fa-solid fa-triangle-exclamation" />
      以下机器人配置文件损坏，无法加载：
      <span
        v-for="(b, i) in botStore.failedBots"
        :key="b.id"
        class="failed-name"
      >{{ b.name }}<template v-if="i < botStore.failedBots.length - 1">、</template></span>
      <span class="failed-hint">（请检查 ~/.MoontonClaudeDesktop/bots/&lt;id&gt;/config.json）</span>
    </div>

    <div
      v-if="botStore.bots.length === 0 && botStore.failedBots.length === 0"
      class="empty-state"
    >
      <i class="fa-solid fa-frog" />
      <span>暂无机器人</span>
    </div>

    <div
      v-for="bot in botStore.bots"
      :key="bot.id"
      class="bot-item"
      :class="{ active: botStore.interactionBotId === bot.id }"
      @click="handleContextMenu($event, bot.id)"
      @contextmenu="handleContextMenu($event, bot.id)"
    >
      <div class="bot-trigger-icon">
        <i class="fa-solid fa-frog" />
      </div>
      <div class="bot-info">
        <div class="bot-name-row">
          <span class="bot-name">{{ bot.name }}</span>
          <ChannelStatusBadge
            v-if="botStore.channelStatuses[bot.id]?.length"
            :statuses="botStore.channelStatuses[bot.id]"
          />
        </div>
        <div class="bot-meta">
          <span
            class="status-dot"
            :style="{ background: statusColors[bot.status] }"
          />
          <span class="status-text">{{ statusLabels[bot.status] || bot.status }}</span>
          <span class="bot-time">{{ formatTime(bot.lastActiveAt) }}</span>
        </div>
      </div>
    </div>

    <!-- Context Menu -->
    <Teleport to="body">
      <div
        v-if="contextMenuBot"
        class="context-menu"
        :style="{ left: contextMenuPos.x + 'px', top: contextMenuPos.y + 'px' }"
      >
        <button @click="handleBotClick(contextMenuBot!); closeContextMenu()">
          <i class="fa-solid fa-comments" />
          聊天
        </button>
        <button @click="openEditDialog(contextMenuBot!)">
          <i class="fa-solid fa-pen-to-square" />
          编辑
        </button>
        <div class="menu-divider" />
        <button
          @click="handleToggleStatus(botStore.bots.find(b => b.id === contextMenuBot)!)"
        >
          <i :class="botStore.bots.find(b => b.id === contextMenuBot)?.status === 'stopped' ? 'fa-solid fa-play' : 'fa-solid fa-stop'" />
          {{ botStore.bots.find(b => b.id === contextMenuBot)?.status === 'stopped' ? '启动' : '停止' }}
        </button>
        <button @click="openChannelConfig(contextMenuBot!)">
          <i class="fa-solid fa-tower-broadcast" />
          频道
        </button>
        <button @click="openScheduleConfig(contextMenuBot!)">
          <i class="fa-solid fa-clock" />
          定时
        </button>
        <div class="menu-divider" />
        <button
          class="danger"
          @click="handleDelete(contextMenuBot!)"
        >
          <i class="fa-solid fa-trash" />
          删除
        </button>
        <div class="menu-divider" />
        <button @click="closeContextMenu">
          <i class="fa-solid fa-xmark" />
          取消
        </button>
      </div>
    </Teleport>

    <!-- Channel Config Dialog -->
    <ChannelConfigDialog
      v-if="channelDialogBotId"
      :bot-id="channelDialogBotId"
      :visible="!!channelDialogBotId"
      @close="closeChannelDialog"
    />

    <!-- Schedule Config Dialog -->
    <ScheduleConfigDialog
      v-if="scheduleDialogBotId"
      :bot-id="scheduleDialogBotId"
      :visible="!!scheduleDialogBotId"
      @close="closeScheduleDialog"
    />

    <!-- Edit Dialog -->
    <BotEditDialog
      v-if="editDialogBotId"
      :bot-id="editDialogBotId"
      :visible="!!editDialogBotId"
      @close="closeEditDialog"
    />
  </div>
</template>

<style scoped lang="less">
.bot-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.config-error-banner {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
  padding: 8px 10px;
  margin-bottom: 4px;
  border-radius: 6px;
  background: rgba(210, 153, 34, 0.1);
  border: 1px solid rgba(210, 153, 34, 0.3);
  font-size: 11px;
  color: #d29922;
  line-height: 1.5;

  i {
    flex-shrink: 0;
  }

  .failed-name {
    font-weight: 600;
    background: rgba(210, 153, 34, 0.15);
    padding: 0 4px;
    border-radius: 3px;
  }

  .failed-hint {
    color: rgba(210, 153, 34, 0.7);
    font-size: 10px;
  }
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 24px;
  color: #6e7681;
  font-size: 12px;

  i {
    font-size: 24px;
    opacity: 0.5;
  }
}

.bot-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.15s;

  &:hover {
    background: #21262d;
  }

  &.active {
    background: rgba(88, 166, 255, 0.1);
  }
}

.bot-trigger-icon {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  background: #0d1117;
  font-size: 12px;
  color: #8b949e;
  flex-shrink: 0;
}

.bot-info {
  flex: 1;
  min-width: 0;
}

.bot-name-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.bot-name {
  font-size: 13px;
  font-weight: 500;
  color: #e6edf3;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.bot-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 2px;
  font-size: 11px;
  color: #8b949e;
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}

.bot-time {
  margin-left: auto;
}

.context-menu {
  position: fixed;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 8px;
  padding: 4px;
  z-index: 10000;
  min-width: 140px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);

  .menu-divider {
    height: 1px;
    background: #30363d;
    margin: 4px 0;
  }

  button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    background: none;
    border: none;
    color: #e6edf3;
    cursor: pointer;
    font-size: 12px;
    border-radius: 4px;
    text-align: left;

    &:hover {
      background: #21262d;
    }

    &.danger {
      color: #f85149;

      &:hover {
        background: rgba(248, 81, 73, 0.1);
      }
    }

    i {
      width: 16px;
      text-align: center;
    }
  }
}
</style>
