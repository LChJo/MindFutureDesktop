/**
 * Bot Channels composable
 * Manages inbound message listening and outbound broadcasting for bot channels.
 */

import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useBotStore } from '@/stores/botStore'
import { useBotSession } from '@/composables/claudeBot/useBotSession'
import { useSettingsStore } from '@/stores/settingsStore'
import { clearFeishuAckReaction, FEISHU_PROCESSING_REACTION } from '@/composables/claudeBot/feishuReactions'
import type { BotDefinition, ChannelIncomingMessage } from '@/types/claude-bot'

// Track active listeners per bot
const activeListeners = new Map<string, UnlistenFn>()
const listeningBots = ref<Set<string>>(new Set())
const startingListeners = new Map<string, Promise<void>>()

// Periodic channel status pollers (15s interval) per bot
const statusPollers = new Map<string, ReturnType<typeof setInterval>>()

// Message queue for buffering inbound messages while bot is busy
interface QueuedMessage {
  content: string   // Already prefixed: "[platform] sender: text"
  timestamp: number
  sourceChatId?: string
  attachments?: { path: string; name: string }[]
  /** Feishu message_id (for reaction management when bot drains the queue) */
  messageId?: string
  /** StatusFlashOfInspiration reaction_id to remove after the bot replies */
  reactionId?: string
}

const messageQueue = new Map<string, QueuedMessage[]>()
const MAX_QUEUE_SIZE = 50
const MESSAGE_TTL_MS = 10 * 60_000  // 10 minutes
const FEISHU_QUEUED_NOTICE = '⏳ 已收到，当前正在处理其他消息，稍后回复。'

// Track the source chat_id for the message currently being processed per bot
const activeSourceChatId = new Map<string, string>()
// Track the Feishu message_id and StatusFlashOfInspiration reaction_id for the active message
const activeSourceMessageId = new Map<string, string>()
const activeSourceReactionId = new Map<string, string>()

// Per-bot serialization lock: prevents concurrent filter→reaction→send races when
// multiple Feishu messages arrive during the ~1-3s Haiku filter API call.
const filterLocks = new Map<string, Promise<void>>()

// Generation counter: incremented each time startListening is called for a bot.
// Callbacks captured by a previous listener generation check this and abort if
// stopListening has been called since they were scheduled (prevents stale
// Promise-chain callbacks from executing after the listener is torn down).
const listenerGenerations = new Map<string, number>()

// Pending scheduled message per bot (at most one, newer overwrites older)
interface PendingScheduled {
  message: string
  chatId?: string
}
const pendingScheduledMessages = new Map<string, PendingScheduled>()

function logBotChannels(level: 'debug' | 'info' | 'warn' | 'error', message: string): void {
  invoke('app_log', {
    scope: 'bot_channels',
    level,
    message,
  }).catch(() => {})
}

async function sendPlatformMessage(
  botId: string,
  platform: string,
  message: string,
  targetChatId?: string,
): Promise<void> {
  await invoke('channel_send_platform_message', {
    botId,
    platform,
    message,
    targetChatId: targetChatId || null,
  })
}

function getQueueLength(botId: string): number {
  return messageQueue.get(botId)?.length ?? 0
}

function enqueueMessage(
  botId: string,
  content: string,
  sourceChatId?: string,
  attachments?: { path: string; name: string }[],
  messageId?: string,
  reactionId?: string,
): void {
  const queue = messageQueue.get(botId) ?? []
  queue.push({ content, timestamp: Date.now(), sourceChatId, attachments, messageId, reactionId })
  if (queue.length > MAX_QUEUE_SIZE) {
    const dropped = queue.shift()!
    // Remove orphaned StatusFlashOfInspiration reaction for the dropped message
    if (dropped.reactionId && dropped.messageId) {
      invoke('feishu_remove_reaction', {
        botId,
        messageId: dropped.messageId,
        reactionId: dropped.reactionId,
      }).catch(() => {})
    }
    if (dropped.messageId) {
      clearFeishuAckReaction(botId, dropped.messageId).catch(() => {})
    }
    console.warn(`[BotChannels] Queue overflow for bot ${botId}, dropped oldest message`)
    logBotChannels(
      'warn',
      `queue_overflow bot=${botId} dropped_msg_id=${dropped.messageId ?? 'none'} source_chat=${dropped.sourceChatId ?? 'none'} queue_len_after_drop=${queue.length}`,
    )
  }
  messageQueue.set(botId, queue)
}

export function useBotChannels() {
  /**
   * Start listening for inbound channel messages for a bot.
   * Incoming messages are automatically forwarded to the bot as user messages.
   */
  async function startListening(bot: BotDefinition): Promise<void> {
    if (activeListeners.has(bot.id)) return
    const inFlight = startingListeners.get(bot.id)
    if (inFlight) return inFlight

    const startPromise = (async () => {
      const botStore = useBotStore()
      const botSession = useBotSession()

      // Increment generation so any in-flight callbacks from a previous listener
      // (i.e. messages queued in filterLocks after stopListening) will self-abort.
      const generation = (listenerGenerations.get(bot.id) ?? 0) + 1
      listenerGenerations.set(bot.id, generation)

      // Start channel listeners in Rust
      await botStore.startChannels(bot.id)

      // Listen for inbound messages from Rust
      const unlisten = await listen<ChannelIncomingMessage>(
        `channel-incoming-${bot.id}`,
        (event) => {
          // Serialize per-bot: chain onto any in-flight filter operation so concurrent
          // Feishu messages don't race on activeSourceReactionId / sendMessage.
          const prevLock = filterLocks.get(bot.id) ?? Promise.resolve()
          let releaseLock!: () => void
          const myLock = new Promise<void>(resolve => { releaseLock = resolve })
          filterLocks.set(bot.id, myLock)

          prevLock.then(async () => {
            // Abort if stopListening was called after this callback was scheduled.
            if (listenerGenerations.get(bot.id) !== generation) {
              releaseLock()
              return
            }
            try {
              const msg = event.payload
              logBotChannels(
                'info',
                `inbound_received bot=${bot.id} platform=${msg.platform} msg_id=${msg.messageId ?? 'none'} source_chat=${msg.channelId} sender=${msg.sender} queue_len=${getQueueLength(bot.id)}`,
              )
              const currentBot = botStore.bots.find(b => b.id === bot.id)
              if (!currentBot || currentBot.status === 'stopped') {
                // Bot is stopped: clean up the Status_PrivateMessage reaction added by Rust
                if (msg.platform === 'feishu' && msg.messageId) {
                  await clearFeishuAckReaction(bot.id, msg.messageId)
                    .catch(e => console.warn('[reaction] clear on stopped bot failed:', e))
                }
                logBotChannels(
                  'warn',
                  `inbound_dropped_stopped bot=${bot.id} platform=${msg.platform} msg_id=${msg.messageId ?? 'none'} source_chat=${msg.channelId}`,
                )
                return
              }

              // 1. Message filter (Feishu only, when filter is enabled)
              const filter = currentBot.messageFilter
              if (filter?.enabled && filter.prompt.trim() && msg.platform === 'feishu') {
                // Build env vars matching the bot's provider (same logic as useBotSession:spawnSidecar)
                const settingsStore = useSettingsStore()
                const filterEnv: Record<string, string> = { ...(settingsStore.environmentVariables ?? {}) }
                // Always remove ANTHROPIC_MODEL so the filter's own model selection is not overridden.
                delete filterEnv['ANTHROPIC_MODEL']
                const coworksConfig = settingsStore.coworksModelConfig
                if (coworksConfig?.providers?.length) {
                  const provider = coworksConfig.providers.find(
                    (p: { id: string; baseUrl?: string; apiKey?: string; model?: string }) =>
                      p.id === currentBot.model,
                  )
                  if (provider) {
                    if (provider.baseUrl) filterEnv['ANTHROPIC_BASE_URL'] = provider.baseUrl
                    if (provider.apiKey) filterEnv['ANTHROPIC_AUTH_TOKEN'] = provider.apiKey
                    // Do NOT set ANTHROPIC_MODEL: filter uses filter.model (Haiku by default),
                    // and this env var would override the model passed into query().
                  }
                }

                const pass = await invoke<boolean>('bot_filter_message', {
                  content: msg.content,
                  systemPrompt: filter.prompt,
                  model: filter.model || 'Haiku',
                  env: Object.keys(filterEnv).length > 0 ? filterEnv : null,
                }).catch(() => true)  // fail-open

                if (!pass) {
                  // Filter blocked: quietly remove Status_PrivateMessage acknowledgment reaction
                  if (msg.messageId) {
                    await clearFeishuAckReaction(bot.id, msg.messageId)
                      .catch(e => console.warn('[reaction] clear after filter block failed:', e))
                  }
                  logBotChannels(
                    'info',
                    `inbound_filtered bot=${bot.id} platform=${msg.platform} msg_id=${msg.messageId ?? 'none'} source_chat=${msg.channelId}`,
                  )
                  return
                }
              }

              // 2. Message accepted: swap Status_PrivateMessage → StatusFlashOfInspiration for Feishu,
              // regardless of whether the pre-filter is enabled.
              let processingReactionId: string | undefined
              if (msg.platform === 'feishu' && msg.messageId) {
                await clearFeishuAckReaction(bot.id, msg.messageId)
                  .catch(e => console.warn('[reaction] clear before StatusFlashOfInspiration failed:', e))
                processingReactionId = await invoke<string>('feishu_add_reaction', {
                  botId: bot.id,
                  messageId: msg.messageId,
                  emojiType: FEISHU_PROCESSING_REACTION,
                }).catch(e => {
                  console.warn('[reaction] add StatusFlashOfInspiration failed:', e)
                  return undefined
                })
              }

              // 3. Route to bot session or queue
              const prefixed = `[${msg.platform}] ${msg.sender}: ${msg.content}`
              const attachments = msg.attachments?.length ? msg.attachments : undefined

              if (currentBot.status === 'working') {
                enqueueMessage(bot.id, prefixed, msg.channelId, attachments, msg.messageId, processingReactionId)
                const queueLen = getQueueLength(bot.id)
                logBotChannels(
                  'info',
                  `inbound_queued bot=${bot.id} platform=${msg.platform} msg_id=${msg.messageId ?? 'none'} source_chat=${msg.channelId} queue_len=${queueLen}`,
                )
                if (msg.platform === 'feishu' && msg.channelId) {
                  sendPlatformMessage(bot.id, 'feishu', FEISHU_QUEUED_NOTICE, msg.channelId)
                    .then(() => {
                      logBotChannels(
                        'info',
                        `queued_notice_sent bot=${bot.id} platform=feishu msg_id=${msg.messageId ?? 'none'} source_chat=${msg.channelId} queue_len=${queueLen}`,
                      )
                    })
                    .catch(e => {
                      console.error('[useBotChannels] Failed to send queued notice:', e)
                      logBotChannels(
                        'warn',
                        `queued_notice_failed bot=${bot.id} platform=feishu msg_id=${msg.messageId ?? 'none'} source_chat=${msg.channelId} error=${String(e)}`,
                      )
                    })
                }
              } else {
                activeSourceChatId.set(bot.id, msg.channelId)
                if (msg.messageId) activeSourceMessageId.set(bot.id, msg.messageId)
                if (processingReactionId) activeSourceReactionId.set(bot.id, processingReactionId)
                logBotChannels(
                  'info',
                  `inbound_dispatch bot=${bot.id} platform=${msg.platform} msg_id=${msg.messageId ?? 'none'} source_chat=${msg.channelId} mode=direct`,
                )
                botSession.sendMessage(currentBot, prefixed, attachments)
              }
            } finally {
              releaseLock()
              if (filterLocks.get(bot.id) === myLock) filterLocks.delete(bot.id)
            }
          }).catch(err => {
            console.error('[useBotChannels] inbound handler failed:', err)
            releaseLock()
            if (filterLocks.get(bot.id) === myLock) filterLocks.delete(bot.id)
          })
        },
      )

      activeListeners.set(bot.id, unlisten)
      listeningBots.value = new Set([...listeningBots.value, bot.id])

      // Start periodic status polling so the UI can detect disconnections
      const poller = setInterval(() => {
        useBotStore().refreshChannelStatus(bot.id).catch(() => {})
      }, 15_000)
      statusPollers.set(bot.id, poller)
    })()

    startingListeners.set(bot.id, startPromise)
    try {
      await startPromise
    } finally {
      if (startingListeners.get(bot.id) === startPromise) {
        startingListeners.delete(bot.id)
      }
    }
  }

  /**
   * Stop listening for inbound channel messages for a bot.
   */
  async function stopListening(botId: string): Promise<void> {
    // Invalidate any in-flight filterLock callbacks captured by the current generation.
    listenerGenerations.set(botId, (listenerGenerations.get(botId) ?? 0) + 1)

    const unlisten = activeListeners.get(botId)
    if (unlisten) {
      unlisten()
      activeListeners.delete(botId)
    }

    // Clear status poller
    const poller = statusPollers.get(botId)
    if (poller !== undefined) {
      clearInterval(poller)
      statusPollers.delete(botId)
    }

    const botStore = useBotStore()
    await botStore.stopChannels(botId)

    // Clean up orphaned StatusFlashOfInspiration reactions before discarding the queue
    const queue = messageQueue.get(botId)
    if (queue) {
      queue.forEach(m => {
        if (m.reactionId && m.messageId) {
          invoke('feishu_remove_reaction', { botId, messageId: m.messageId, reactionId: m.reactionId })
            .catch(() => {})
        }
        if (m.messageId) {
          clearFeishuAckReaction(botId, m.messageId).catch(() => {})
        }
      })
    }
    messageQueue.delete(botId)

    // Clean up the active source reaction (message currently being processed, if any)
    const activeMsgId = activeSourceMessageId.get(botId)
    const activeRxnId = activeSourceReactionId.get(botId)
    if (activeMsgId && activeRxnId) {
      invoke('feishu_remove_reaction', { botId, messageId: activeMsgId, reactionId: activeRxnId })
        .catch(() => {})
    }
    if (activeMsgId) {
      clearFeishuAckReaction(botId, activeMsgId).catch(() => {})
    }

    activeSourceChatId.delete(botId)
    activeSourceMessageId.delete(botId)
    activeSourceReactionId.delete(botId)
    filterLocks.delete(botId)
    startingListeners.delete(botId)

    const next = new Set(listeningBots.value)
    next.delete(botId)
    listeningBots.value = next
  }

  /**
   * Restart listening: stop then start, used after config changes.
   */
  async function restartListening(bot: BotDefinition): Promise<void> {
    await stopListening(bot.id)
    await startListening(bot)
  }

  /**
   * Broadcast a bot's reply to all enabled channels.
   * `targetChatId` is an optional dynamic target (e.g. Feishu source group chat_id).
   */
  async function broadcastMessage(botId: string, message: string, targetChatId?: string): Promise<void> {
    if (!message.trim()) return

    try {
      await invoke('channel_send_message', {
        botId,
        message,
        targetChatId: targetChatId || null,
      })
      logBotChannels('info', `broadcast_sent bot=${botId} target_chat=${targetChatId ?? 'default'}`)
    } catch (e) {
      console.error(`[useBotChannels] Broadcast failed for bot ${botId}, targetChat: ${targetChatId}:`, e)
    }
  }

  /**
   * Check if a bot has any enabled channels configured.
   */
  async function hasEnabledChannels(botId: string): Promise<boolean> {
    const botStore = useBotStore()
    const config = await botStore.loadChannelConfig(botId)
    return !!(
      config.telegram?.enabled ||
      config.discord?.enabled ||
      config.feishu?.enabled
    )
  }

  function isListening(botId: string): boolean {
    return listeningBots.value.has(botId)
  }

  /**
   * Get the source chat_id for the message currently being processed by a bot.
   */
  function getActiveSourceChatId(botId: string): string | undefined {
    return activeSourceChatId.get(botId)
  }

  /**
   * Clear the active source chat_id, message_id and reaction_id after reply is sent.
   */
  function clearActiveSource(botId: string): void {
    activeSourceChatId.delete(botId)
    activeSourceMessageId.delete(botId)
    activeSourceReactionId.delete(botId)
  }

  function getActiveSourceMessageId(botId: string): string | undefined {
    return activeSourceMessageId.get(botId)
  }

  function getActiveSourceReactionId(botId: string): string | undefined {
    return activeSourceReactionId.get(botId)
  }

  /**
   * Manually set the target chat_id for the next outbound reply.
   * Used by the scheduler to route scheduled messages to a specific group.
   */
  function setActiveSourceChatId(botId: string, chatId: string): void {
    activeSourceChatId.set(botId, chatId)
  }

  /**
   * Drain queued channel messages after bot finishes processing.
   * Filters expired messages (>10min), batches remaining into one send.
   */
  function drainQueue(botId: string): void {
    const queue = messageQueue.get(botId)
    if (!queue || queue.length === 0) return

    const now = Date.now()
    const expiredMsgs = queue.filter(m => now - m.timestamp >= MESSAGE_TTL_MS)
    const allValid = queue.filter(m => now - m.timestamp < MESSAGE_TTL_MS)

    // Remove StatusFlashOfInspiration reactions for expired messages — they will never be replied to
    expiredMsgs.forEach(m => {
      if (m.reactionId && m.messageId) {
        invoke('feishu_remove_reaction', { botId, messageId: m.messageId, reactionId: m.reactionId })
          .catch(() => {})
      }
      if (m.messageId) {
        clearFeishuAckReaction(botId, m.messageId).catch(() => {})
      }
    })
    if (expiredMsgs.length > 0) {
      logBotChannels(
        'warn',
        `queue_expired bot=${botId} expired=${expiredMsgs.length} remaining_before=${queue.length}`,
      )
    }

    if (allValid.length === 0) {
      messageQueue.delete(botId)
      logBotChannels(
        'info',
        `queue_drained_empty bot=${botId} expired=${expiredMsgs.length}`,
      )
      return
    }

    const currentBot = useBotStore().bots.find(b => b.id === botId)
    if (!currentBot || currentBot.status === 'stopped') {
      logBotChannels(
        'warn',
        `queue_drain_skipped bot=${botId} reason=${!currentBot ? 'missing_bot' : 'stopped'} queue_len=${allValid.length}`,
      )
      return
    }

    // 只处理与第一条消息相同 chatId 的消息，其余留在队列等下次 drain
    const targetChatId = allValid[0].sourceChatId
    const toProcess = allValid.filter(m => m.sourceChatId === targetChatId)
    const remaining = allValid.filter(m => m.sourceChatId !== targetChatId)

    if (remaining.length > 0) {
      messageQueue.set(botId, remaining)
    } else {
      messageQueue.delete(botId)
    }
    logBotChannels(
      'info',
      `queue_drain_start bot=${botId} batch=${toProcess.length} remaining=${remaining.length} target_chat=${targetChatId ?? 'none'} expired=${expiredMsgs.length}`,
    )

    if (targetChatId) {
      activeSourceChatId.set(botId, targetChatId)
    }
    const firstMsg = toProcess[0]
    if (firstMsg.messageId) activeSourceMessageId.set(botId, firstMsg.messageId)
    if (firstMsg.reactionId) activeSourceReactionId.set(botId, firstMsg.reactionId)

    // Remove StatusFlashOfInspiration reactions for batched messages [1..n]: they're merged into a single
    // bot response, so only the first message's reaction is tracked for removal.
    toProcess.slice(1).forEach(m => {
      if (m.reactionId && m.messageId) {
        invoke('feishu_remove_reaction', { botId, messageId: m.messageId, reactionId: m.reactionId })
          .catch(() => {})
      }
      if (m.messageId) {
        clearFeishuAckReaction(botId, m.messageId).catch(() => {})
      }
    })

    const allAttachments = toProcess.flatMap(m => m.attachments ?? [])
    const attachments = allAttachments.length > 0 ? allAttachments : undefined

    const expiredNotice = expiredMsgs.length > 0
      ? `[系统提示] 有 ${expiredMsgs.length} 条消息因超时已自动忽略\n`
      : ''

    const botSession = useBotSession()
    if (toProcess.length === 1) {
      logBotChannels(
        'info',
        `queue_dispatch bot=${botId} mode=single msg_id=${firstMsg.messageId ?? 'none'} target_chat=${targetChatId ?? 'none'}`,
      )
      botSession.sendMessage(currentBot, expiredNotice + toProcess[0].content, attachments)
    } else {
      const header = `[系统提示] 以下是等待处理的 ${toProcess.length} 条消息：\n`
      const batched = expiredNotice + header + toProcess.map(m => m.content).join('\n')
      logBotChannels(
        'info',
        `queue_dispatch bot=${botId} mode=batch batch=${toProcess.length} first_msg_id=${firstMsg.messageId ?? 'none'} target_chat=${targetChatId ?? 'none'}`,
      )
      botSession.sendMessage(currentBot, batched, attachments)
    }
  }

  /**
   * Store a scheduled message to be sent after the bot finishes its current task.
   * Newer calls overwrite older pending messages for the same bot.
   */
  function enqueueScheduled(botId: string, message: string, chatId?: string): void {
    pendingScheduledMessages.set(botId, { message, chatId })
  }

  /**
   * Send any pending scheduled message for a bot.
   * Called from useBotSession after the 'done' event, after drainQueue().
   */
  function drainScheduled(botId: string): void {
    const pending = pendingScheduledMessages.get(botId)
    if (!pending) return

    pendingScheduledMessages.delete(botId)

    const currentBot = useBotStore().bots.find(b => b.id === botId)
    if (!currentBot || currentBot.status === 'stopped') return

    if (pending.chatId) {
      activeSourceChatId.set(botId, pending.chatId)
    }

    useBotSession().sendMessage(currentBot, pending.message)
      .catch(e => console.error(`[useBotChannels] drainScheduled failed for ${botId}:`, e))
  }

  return {
    startListening,
    stopListening,
    restartListening,
    broadcastMessage,
    drainQueue,
    drainScheduled,
    enqueueScheduled,
    hasEnabledChannels,
    isListening,
    getActiveSourceChatId,
    setActiveSourceChatId,
    clearActiveSource,
    getActiveSourceMessageId,
    getActiveSourceReactionId,
    getQueueLength,
    listeningBots,
  }
}
