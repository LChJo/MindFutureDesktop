<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import BotSprite from './BotSprite.vue'
import type { BotDefinition } from '@/types/claude-bot'
import { useBotStore } from '@/stores/botStore'

const props = defineProps<{
  bots: BotDefinition[]
}>()

const emit = defineEmits<{
  'bot-click': [botId: string]
}>()

// Bot positions - randomly assigned and occasionally wandering
const botPositions = ref<Record<string, { x: number; y: number }>>({})
const areaRef = ref<HTMLDivElement | null>(null)
let wanderTimer: ReturnType<typeof setInterval> | null = null

const botStore = useBotStore()

function isChannelDisconnected(botId: string): boolean {
  const statuses = botStore.channelStatuses[botId]
  if (!statuses || statuses.length === 0) return false
  return statuses.some(s => s.state === 'disconnected')
}

// Minimum distance between bot centers to avoid overlap (sprite size ~64px + gap)
const MIN_BOT_DISTANCE = 80

const activeBots = computed(() => {
  return props.bots.filter(b => b.status !== 'stopped')
})

const stoppedBots = computed(() => {
  return props.bots.filter(b => b.status === 'stopped')
})

function distanceBetween(a: { x: number; y: number }, b: { x: number; y: number }): number {
  return Math.sqrt((a.x - b.x) ** 2 + (a.y - b.y) ** 2)
}

function hasCollision(pos: { x: number; y: number }, excludeBotId: string, positions: Record<string, { x: number; y: number }>): boolean {
  for (const [id, other] of Object.entries(positions)) {
    if (id === excludeBotId) continue
    if (distanceBetween(pos, other) < MIN_BOT_DISTANCE) return true
  }
  return false
}

function findNonOverlappingPosition(
  botId: string,
  candidate: { x: number; y: number },
  positions: Record<string, { x: number; y: number }>,
  maxX: number,
  maxY: number,
): { x: number; y: number } {
  if (!hasCollision(candidate, botId, positions)) return candidate

  const offsets = [
    { x: MIN_BOT_DISTANCE, y: 0 }, { x: -MIN_BOT_DISTANCE, y: 0 },
    { x: 0, y: MIN_BOT_DISTANCE }, { x: 0, y: -MIN_BOT_DISTANCE },
    { x: MIN_BOT_DISTANCE, y: MIN_BOT_DISTANCE }, { x: -MIN_BOT_DISTANCE, y: -MIN_BOT_DISTANCE },
  ]
  for (const off of offsets) {
    const nudged = {
      x: Math.max(20, Math.min(maxX, candidate.x + off.x)),
      y: Math.max(20, Math.min(maxY, candidate.y + off.y)),
    }
    if (!hasCollision(nudged, botId, positions)) return nudged
  }

  return positions[botId] ?? candidate
}

function ensurePosition(botId: string) {
  if (!botPositions.value[botId]) {
    const area = areaRef.value
    const maxX = area ? area.clientWidth - 100 : 400
    const maxY = area ? area.clientHeight - 120 : 300

    const candidate = {
      x: 80 + Math.random() * Math.max(100, maxX - 80),
      y: 60 + Math.random() * Math.max(60, maxY - 60),
    }
    const pos = findNonOverlappingPosition(botId, candidate, botPositions.value, maxX, maxY)

    botPositions.value = {
      ...botPositions.value,
      [botId]: pos,
    }
  }
}

function wanderBots() {
  const area = areaRef.value
  if (!area) return

  const maxX = area.clientWidth - 100
  const maxY = area.clientHeight - 120

  const updated = { ...botPositions.value }
  for (const bot of activeBots.value) {
    if (bot.status === 'idle' || bot.status === 'paused') {
      const current = updated[bot.id]
      if (!current) continue
      const dx = (Math.random() - 0.5) * 60
      const dy = (Math.random() - 0.5) * 40
      const candidate = {
        x: Math.max(20, Math.min(maxX, current.x + dx)),
        y: Math.max(20, Math.min(maxY, current.y + dy)),
      }
      updated[bot.id] = findNonOverlappingPosition(bot.id, candidate, updated, maxX, maxY)
    }
  }
  botPositions.value = updated
}

onMounted(() => {
  for (const bot of props.bots) {
    ensurePosition(bot.id)
  }

  wanderTimer = setInterval(() => {
    wanderBots()
  }, 3000 + Math.random() * 5000)
})

onUnmounted(() => {
  if (wanderTimer) {
    clearInterval(wanderTimer)
  }
})

function getBotPosition(botId: string) {
  ensurePosition(botId)
  return botPositions.value[botId]!
}
</script>

<template>
  <div
    ref="areaRef"
    class="bot-activity-area"
  >
    <!-- Grid background -->
    <div class="grid-bg" />

    <!-- Title -->
    <div class="area-title">
      <i class="fa-solid fa-microchip" />
      Bot Activity
    </div>

    <!-- Active bots wandering -->
    <div
      v-for="bot in activeBots"
      :key="bot.id"
      class="bot-wrapper"
      :style="{
        left: getBotPosition(bot.id).x + 'px',
        top: getBotPosition(bot.id).y + 'px',
      }"
      @click="emit('bot-click', bot.id)"
    >
      <BotSprite
        :bot="bot"
        :size="64"
        :channel-disconnected="isChannelDisconnected(bot.id)"
      />
    </div>

    <!-- Stopped bots in a row at bottom -->
    <div
      v-if="stoppedBots.length > 0"
      class="stopped-zone"
    >
      <div class="stopped-label">
        已停止
      </div>
      <div class="stopped-bots">
        <div
          v-for="bot in stoppedBots"
          :key="bot.id"
          class="stopped-bot-item"
          @click="emit('bot-click', bot.id)"
        >
          <BotSprite
            :bot="bot"
            :size="48"
          />
        </div>
      </div>
    </div>

    <!-- Empty state -->
    <div
      v-if="bots.length === 0"
      class="empty-activity"
    >
      <div class="empty-icon">
        <i class="fa-solid fa-frog" />
      </div>
      <div class="empty-text">
        创建机器人开始使用
      </div>
      <div class="empty-hint">
        点击「Add Bot」开始
      </div>
    </div>

    <!-- Stats bar -->
    <div class="stats-bar">
      <span class="stat">
        <span class="stat-dot active" />
        {{ activeBots.length }} 活跃
      </span>
      <span class="stat">
        <span class="stat-dot stopped" />
        {{ stoppedBots.length }} 已停止
      </span>
      <span class="stat">
        <i class="fa-solid fa-frog" />
        {{ bots.length }} 总计
      </span>
    </div>
  </div>
</template>

<style scoped lang="less">
.bot-activity-area {
  width: 100%;
  height: 100%;
  position: relative;
  overflow: hidden;
  background: #010409;
}

.grid-bg {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(rgba(48, 54, 61, 0.4) 1px, transparent 1px),
    linear-gradient(90deg, rgba(48, 54, 61, 0.4) 1px, transparent 1px);
  background-size: 40px 40px;
  pointer-events: none;
}

.area-title {
  position: absolute;
  top: 12px;
  right: 16px;
  font-size: 11px;
  color: #30363d;
  text-transform: uppercase;
  letter-spacing: 2px;
  display: flex;
  align-items: center;
  gap: 6px;
  pointer-events: none;
}

.bot-wrapper {
  position: absolute;
  transition: left 2s ease-in-out, top 2s ease-in-out;
  z-index: 1;
}

.stopped-zone {
  position: absolute;
  bottom: 48px;
  left: 0;
  right: 0;
  padding: 8px 16px;
  border-top: 1px dashed #30363d;
}

.stopped-label {
  font-size: 10px;
  color: #6e7681;
  text-transform: uppercase;
  letter-spacing: 1px;
  margin-bottom: 8px;
}

.stopped-bots {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.stopped-bot-item {
  cursor: pointer;
}

.empty-activity {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  pointer-events: none;
}

.empty-icon {
  font-size: 48px;
  color: #21262d;
  opacity: 0.5;
}

.empty-text {
  font-size: 16px;
  color: #6e7681;
}

.empty-hint {
  font-size: 12px;
  color: #484f58;
}

.stats-bar {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 36px;
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 0 16px;
  background: rgba(1, 4, 9, 0.9);
  border-top: 1px solid #30363d;
  font-size: 11px;
  color: #8b949e;
}

.stat {
  display: flex;
  align-items: center;
  gap: 5px;
}

.stat-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;

  &.active {
    background: #3fb950;
  }

  &.stopped {
    background: #d29922;
  }
}
</style>
