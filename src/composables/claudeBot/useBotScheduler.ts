/**
 * Bot Scheduler composable
 * Manages frontend timers for sending scheduled messages to bots.
 * Fixed-time checks use a Rust backend heartbeat event ("scheduler-tick")
 * instead of setInterval, ensuring reliable timing on macOS where
 * WKWebView throttles JS timers in the background.
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useBotStore } from '@/stores/botStore'
import { useBotSession } from '@/composables/claudeBot/useBotSession'
import { useBotChannels } from '@/composables/claudeBot/useBotChannels'
import { migrateScheduleConfig } from '@/types/claude-bot'
import type { BotDefinition, BotScheduleConfig } from '@/types/claude-bot'

type TimerHandle = ReturnType<typeof setInterval>

interface ActiveSchedule {
  intervalTimers: TimerHandle[]
  fixedTimeUnlisten?: UnlistenFn
}

const activeSchedules = new Map<string, ActiveSchedule>()

function getCurrentHHmm(): string {
  const now = new Date()
  const h = String(now.getHours()).padStart(2, '0')
  const m = String(now.getMinutes()).padStart(2, '0')
  return `${h}:${m}`
}

// Track last fired time to avoid duplicate sends within the same minute
const lastFiredTime = new Map<string, string>()

export function useBotScheduler() {
  async function startSchedule(bot: BotDefinition, config: BotScheduleConfig): Promise<void> {
    stopSchedule(bot.id)

    if (!config.intervalEnabled && !config.fixedTimeEnabled) return

    const schedule: ActiveSchedule = { intervalTimers: [] }

    // 固定间隔（多条）
    if (config.intervalEnabled && config.intervals.length > 0) {
      for (const entry of config.intervals) {
        if (entry.intervalMinutes <= 0) continue
        const message = entry.message || config.message
        if (!message.trim()) continue
        const ms = entry.intervalMinutes * 60_000
        const timer = setInterval(() => {
          sendScheduledMessage(bot.id, message, entry.chatId)
        }, ms)
        schedule.intervalTimers.push(timer)
      }
    }

    // 指定时间：监听 Rust 后端心跳事件（不受 WebView 节流影响）
    if (config.fixedTimeEnabled && config.fixedTimes.length) {
      schedule.fixedTimeUnlisten = await listen('scheduler-tick', () => {
        const now = getCurrentHHmm()
        if (lastFiredTime.get(bot.id) === now) return

        const today = new Date().getDay()
        const matchedEntry = config.fixedTimes.find(entry => {
          if (entry.time !== now) return false
          const days = entry.repeatDays
          return !days?.length || days.length === 7 || days.includes(today)
        })

        if (matchedEntry) {
          const message = matchedEntry.message || config.message
          if (message.trim()) {
            lastFiredTime.set(bot.id, now)
            sendScheduledMessage(bot.id, message, matchedEntry.chatId)
          }
        }
      })
    }

    if (schedule.intervalTimers.length > 0 || schedule.fixedTimeUnlisten) {
      activeSchedules.set(bot.id, schedule)
    }
  }

  function stopSchedule(botId: string): void {
    const schedule = activeSchedules.get(botId)
    if (schedule) {
      for (const timer of schedule.intervalTimers) {
        clearInterval(timer)
      }
      schedule.fixedTimeUnlisten?.()
      activeSchedules.delete(botId)
    }
    lastFiredTime.delete(botId)
  }

  function isScheduleActive(botId: string): boolean {
    return activeSchedules.has(botId)
  }

  async function initSchedules(): Promise<void> {
    const botStore = useBotStore()
    for (const bot of botStore.bots) {
      if (bot.scheduleConfig) {
        const config = migrateScheduleConfig(bot.scheduleConfig)
        if (config.intervalEnabled || config.fixedTimeEnabled) {
          await startSchedule(bot, config)
        }
      }
    }
  }

  function stopAll(): void {
    for (const botId of activeSchedules.keys()) {
      stopSchedule(botId)
    }
  }

  return {
    startSchedule,
    stopSchedule,
    isScheduleActive,
    initSchedules,
    stopAll,
  }
}

function sendScheduledMessage(botId: string, message: string, chatId?: string): void {
  const botStore = useBotStore()
  const bot = botStore.bots.find(b => b.id === botId)
  if (!bot || bot.status === 'stopped') return

  const channels = useBotChannels()

  // Bot 正在处理消息，将定时消息暂存，等 done 后自动发送
  if (bot.status === 'working') {
    channels.enqueueScheduled(botId, message, chatId)
    return
  }

  // Bot 空闲，直接发送
  if (chatId) {
    channels.setActiveSourceChatId(botId, chatId)
  }

  const session = useBotSession()
  session.sendMessage(bot, message).catch(e => {
    console.error(`[useBotScheduler] Failed to send scheduled message for ${botId}:`, e)
  })
}
