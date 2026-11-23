/**
 * Bot Session composable
 * Manages sidecar sessions for individual bots, reusing the CoworksSidecarManager
 */

import { reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useSettingsStore } from '@/stores/settingsStore'
import { useBotStore } from '@/stores/botStore'
import { useBotChannels } from '@/composables/claudeBot/useBotChannels'
import { clearFeishuAckReaction } from '@/composables/claudeBot/feishuReactions'
import type { BotDefinition, BotChatMessage, BotToolUsage } from '@/types/claude-bot'

interface SidecarOutput {
  type: string
  sessionId?: string
  seq?: number
  sdkSessionId?: string
  stage?: string
  content?: string
  toolId?: string
  toolName?: string
  input?: Record<string, unknown>
  summary?: string
  message?: string
}

interface BotSession {
  botId: string
  sidecarId: string
  messages: BotChatMessage[]
  isLoading: boolean
  error: string | null
  sidecarActive: boolean
  initialized: boolean
  sdkSessionId?: string
  unlistenOutput?: UnlistenFn
  unlistenReady?: UnlistenFn
  unlistenStderr?: UnlistenFn
}

/** 注销 session 上的 sidecar 事件监听器（幂等） */
function cleanupListeners(session: BotSession): void {
  session.unlistenOutput?.()
  session.unlistenReady?.()
  session.unlistenStderr?.()
}

// Active sessions keyed by botId
const botSessions = reactive<Map<string, BotSession>>(new Map())

// Module-level conversation load tracking — shared across all useBotSession() instances
// to prevent redundant disk loads that overwrite in-flight messages
const conversationLoaded = new Set<string>()

// Per-message streaming state
const streamingStates = new Map<string, { content: string; toolUsages: BotToolUsage[] }>()
const lastOutputSeqs = new Map<string, number>()

// Turn counters for memory compression
const turnCounters = new Map<string, number>()
const COMPRESSION_EVERY_N_TURNS = 8

// ── 防阻塞机制 ────────────────────────────────────────────────────────────────

// 机制 1：无活动看门狗
// 跟踪每个 bot 最后一次 sidecar 有效输出的时间
const lastActivityTime = new Map<string, number>()
const INACTIVITY_TIMEOUT_MS = 15 * 60_000  // 15 分钟无有效进展才视为卡死

// 机制 2（已移除）：会话总时长限制会误杀复杂长时任务，由机制 1 看门狗覆盖卡死场景即可

// 机制 3：飞书"思考中"进度消息 timer handle（botId → timer）
const thinkingTimerHandles = new Map<string, ReturnType<typeof setTimeout>>()
const THINKING_NOTIFY_DELAY_MS = 60_000  // 1 分钟后发进度消息
/** 清除某个 bot 会话关联的所有 timer（watchdog 外），在 done/error 时调用 */
function clearSessionTimers(botId: string, _assistantMsgId: string): void {
  const thinkingHandle = thinkingTimerHandles.get(botId)
  if (thinkingHandle !== undefined) {
    clearTimeout(thinkingHandle)
    thinkingTimerHandles.delete(botId)
  }
}

function logBotSession(level: 'debug' | 'info' | 'warn' | 'error', message: string): void {
  invoke('app_log', {
    scope: 'bot_session',
    level,
    message,
  }).catch(() => {})
}

function resolveAssistantMessage(session: BotSession, output: SidecarOutput): BotChatMessage | undefined {
  if (output.sessionId) {
    const matched = session.messages.find(msg => msg.id === output.sessionId)
    if (matched?.role === 'assistant') {
      return matched
    }
    if (matched) {
      console.warn(`[useBotSession] handleSidecarOutput skipped: sessionId=${output.sessionId} resolved_non_assistant role=${matched.role} type=${output.type}`)
      return undefined
    }
  }

  const lastMsg = session.messages[session.messages.length - 1]
  if (lastMsg?.role === 'assistant') {
    return lastMsg
  }

  console.warn(`[useBotSession] handleSidecarOutput skipped: sessionId=${output.sessionId ?? 'none'} lastMsg=${lastMsg?.role ?? 'none'} messages=${session.messages.length} type=${output.type}`)
  return undefined
}

function shouldProcessOutput(output: SidecarOutput): boolean {
  if (!output.sessionId || typeof output.seq !== 'number') {
    return true
  }

  const lastSeq = lastOutputSeqs.get(output.sessionId) ?? 0
  if (output.seq <= lastSeq) {
    return false
  }

  lastOutputSeqs.set(output.sessionId, output.seq)
  return true
}

function releaseActiveSourceAndDrain(botId: string, reason: string): void {
  const channels = useBotChannels()
  const sourceMessageId = channels.getActiveSourceMessageId(botId)
  const sourceReactionId = channels.getActiveSourceReactionId(botId)
  const sourceChatId = channels.getActiveSourceChatId(botId)
  const queueLen = channels.getQueueLength(botId)

  if (sourceMessageId && sourceReactionId) {
    invoke('feishu_remove_reaction', {
      botId,
      messageId: sourceMessageId,
      reactionId: sourceReactionId,
    }).catch(e => console.error('[reaction] remove StatusFlashOfInspiration after failure failed:', e))
  }
  if (sourceMessageId) {
    clearFeishuAckReaction(botId, sourceMessageId)
      .catch(e => console.warn('[reaction] clear acknowledgment after failure failed:', e))
  }

  channels.clearActiveSource(botId)
  logBotSession(
    'warn',
    `recover_after_failure bot=${botId} reason=${reason} source_chat=${sourceChatId ?? 'none'} source_msg_id=${sourceMessageId ?? 'none'} queue_len=${queueLen}`,
  )
  channels.drainQueue(botId)
  channels.drainScheduled(botId)
}

/** 模块级看门狗：每 30 秒检查所有处于 loading 状态的 session */
setInterval(() => {
  const now = Date.now()
  for (const [botId, session] of botSessions) {
    if (!session.isLoading) continue
    const lastActive = lastActivityTime.get(botId) ?? now
    if (now - lastActive < INACTIVITY_TIMEOUT_MS) continue

    // 超过无活动阈值，强制重置
    const lastMsg = session.messages[session.messages.length - 1]
    if (lastMsg) {
      streamingStates.delete(lastMsg.id)
      lastOutputSeqs.delete(lastMsg.id)
    }
    session.isLoading = false
    lastActivityTime.delete(botId)
    // 保留 session.sdkSessionId — 下一条消息会尝试 resume，
    // sidecar 的 resume-failed 自动 fresh-retry 逻辑会在失败时降级为新会话

    invoke('coworks_kill_sidecar', { sessionId: session.sidecarId }).catch(() => {})
    // 注销旧监听器，防止 spawnSidecar 重新注册后出现双监听器导致消息重复
    cleanupListeners(session)
    session.sidecarActive = false
    session.initialized = false

    // 清理 thinking timer（timeoutHandle 由各 sendMessage 内的 setTimeout 自行维护）
    const thinkingHandle = thinkingTimerHandles.get(botId)
    if (thinkingHandle !== undefined) {
      clearTimeout(thinkingHandle)
      thinkingTimerHandles.delete(botId)
    }

    // Set error status BEFORE draining: drainQueue → sendMessage sets 'working',
    // so if we set 'error' after, the bot would be stuck in isLoading=true + status='error'.
    useBotStore().updateBotStatus(botId, 'error', '会话无响应，已自动重置')
    logBotSession(
      'warn',
      `watchdog_reset bot=${botId} last_active_age_ms=${now - lastActive} queue_len=${useBotChannels().getQueueLength(botId)}`,
    )
    releaseActiveSourceAndDrain(botId, 'watchdog_timeout')
  }
}, 30_000)

function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).substring(2, 11)}`
}

export function useBotSession() {
  function getSession(botId: string): BotSession | undefined {
    return botSessions.get(botId)
  }

  function getMessages(botId: string): BotChatMessage[] {
    return botSessions.get(botId)?.messages ?? []
  }

  function isLoading(botId: string): boolean {
    return botSessions.get(botId)?.isLoading ?? false
  }

  function getError(botId: string): string | null {
    return botSessions.get(botId)?.error ?? null
  }

  // Load conversation from disk if not loaded yet
  async function ensureConversationLoaded(botId: string) {
    if (conversationLoaded.has(botId)) return
    // Mark immediately BEFORE the async load to prevent concurrent calls
    // from different useBotSession() instances triggering parallel loads
    conversationLoaded.add(botId)

    const session = ensureSession(botId)
    try {
      const raw: string = await invoke('bot_load_conversation', { botId })
      const messages: BotChatMessage[] = JSON.parse(raw)
      // Only assign if session has no in-flight messages (defensive guard)
      if (!session.isLoading) {
        session.messages = messages
      }
    } catch (e) {
      console.error(`[useBotSession] Failed to load conversation for ${botId}:`, e)
    }
  }

  // Save conversation to disk
  async function saveConversation(botId: string) {
    const session = botSessions.get(botId)
    if (!session) return
    try {
      await invoke('bot_save_conversation', {
        botId,
        conversation: JSON.stringify(session.messages),
      })
    } catch (e) {
      console.error(`[useBotSession] Failed to save conversation for ${botId}:`, e)
    }
  }

  function ensureSession(botId: string): BotSession {
    const existing = botSessions.get(botId)
    if (existing) return existing

    const session: BotSession = {
      botId,
      sidecarId: generateId(),
      messages: [],
      isLoading: false,
      error: null,
      sidecarActive: false,
      initialized: false,
    }
    botSessions.set(botId, session)
    return botSessions.get(botId)!
  }

  function handleSidecarOutput(output: SidecarOutput, session: BotSession) {
    if (!shouldProcessOutput(output)) {
      return
    }

    // session_started 不需要 assistant 消息，提前处理避免被 resolveAssistantMessage 误拦截
    // （SDK 首个事件到达时 session.messages 中尚无 assistant 占位符）
    if (output.type === 'session_started') {
      if (output.sdkSessionId) {
        session.sdkSessionId = output.sdkSessionId
      }
      lastActivityTime.set(session.botId, Date.now())
      return
    }

    if (output.type === 'progress') {
      lastActivityTime.set(session.botId, Date.now())
      return
    }

    const assistantMsg = resolveAssistantMessage(session, output)
    if (!assistantMsg) {
      return
    }

    // 机制 1：任何有效输出都刷新无活动时间戳
    lastActivityTime.set(session.botId, Date.now())

    switch (output.type) {
      case 'text_delta': {
        const streaming = streamingStates.get(assistantMsg.id)
        if (!streaming) {
          console.warn(`[useBotSession] handleSidecarOutput skipped: no streaming state for ${assistantMsg.id}, type=${output.type}`)
          return
        }
        if (output.content) {
          streaming.content += output.content
          assistantMsg.content = streaming.content
        }
        break
      }

      case 'tool_use': {
        const streaming = streamingStates.get(assistantMsg.id)
        if (!streaming) {
          console.warn(`[useBotSession] handleSidecarOutput skipped: no streaming state for ${assistantMsg.id}, type=${output.type}`)
          return
        }
        const toolUsage: BotToolUsage = {
          id: output.toolId ?? generateId(),
          toolName: output.toolName ?? 'unknown',
          toolId: output.toolId ?? '',
          input: output.input,
          status: 'running',
        }
        streaming.toolUsages.push(toolUsage)
        assistantMsg.toolUsages = [...streaming.toolUsages]
        break
      }

      case 'tool_result': {
        const streaming = streamingStates.get(assistantMsg.id)
        if (!streaming) {
          console.warn(`[useBotSession] handleSidecarOutput skipped: no streaming state for ${assistantMsg.id}, type=${output.type}`)
          return
        }
        const tool = streaming.toolUsages.find(t => t.toolId === output.toolId)
        if (tool) {
          tool.status = 'completed'
          tool.summary = output.summary
          assistantMsg.toolUsages = [...streaming.toolUsages]
        }
        break
      }

      case 'done': {
        session.isLoading = false
        streamingStates.delete(assistantMsg.id)
        lastOutputSeqs.delete(assistantMsg.id)
        // 清理防阻塞 timers
        clearSessionTimers(session.botId, assistantMsg.id)
        lastActivityTime.delete(session.botId)
        // Auto-save conversation
        saveConversation(session.botId)
        // Update bot status
        const botStore = useBotStore()
        botStore.updateBotStatus(session.botId, 'idle')

        // Channel broadcast: forward bot reply to the source chat
        // 系统消息（记忆压缩等内部指令）的回复不向频道广播，避免将"记忆更新"等发送给用户
        const finalContent = assistantMsg.content
        const channels = useBotChannels()
        const targetChatId = channels.getActiveSourceChatId(session.botId)
        if (!assistantMsg.isSystem) {
          if (finalContent) {
            channels.broadcastMessage(session.botId, finalContent, targetChatId)
              .catch(e => console.error('[channel] broadcast failed:', e))
          } else if (targetChatId) {
            // 模型未产生任何文本回复时，通知飞书用户，避免用户以为 Bot 无响应
            channels.broadcastMessage(
              session.botId,
              '⚠️ Bot 处理完毕，但未生成文字回复。可能是模型配置问题，请稍后重试。',
              targetChatId,
            ).catch(e => console.error('[channel] fallback broadcast failed:', e))
          }
        }
        logBotSession(
          finalContent ? 'info' : 'warn',
          `turn_output_resolved bot=${session.botId} session_msg_id=${assistantMsg.id} output_session_id=${output.sessionId ?? 'none'} final_chars=${finalContent.length} target_chat=${targetChatId ?? 'none'} used_fallback=${(!finalContent && !!targetChatId).toString()}`,
        )

        // Remove StatusFlashOfInspiration reaction after reply.
        const sourceMessageId = channels.getActiveSourceMessageId(session.botId)
        const sourceReactionId = channels.getActiveSourceReactionId(session.botId)
        if (sourceMessageId && sourceReactionId) {
          invoke('feishu_remove_reaction', {
            botId: session.botId,
            messageId: sourceMessageId,
            reactionId: sourceReactionId,
          }).catch(e => console.error('[reaction] remove StatusFlashOfInspiration failed:', e))
        }
        if (sourceMessageId) {
          clearFeishuAckReaction(session.botId, sourceMessageId)
            .catch(e => console.warn('[reaction] clear acknowledgment after reply failed:', e))
        }

        channels.clearActiveSource(session.botId)
        logBotSession(
          'info',
          `turn_done bot=${session.botId} target_chat=${targetChatId ?? 'none'} source_msg_id=${sourceMessageId ?? 'none'} queue_len=${channels.getQueueLength(session.botId)}`,
        )

        // Drain queued channel messages that arrived while bot was busy
        channels.drainQueue(session.botId)
        // Drain scheduled message that was deferred because bot was busy
        channels.drainScheduled(session.botId)

        // Memory compression: every N turns, send a compression instruction
        const count = (turnCounters.get(session.botId) ?? 0) + 1
        turnCounters.set(session.botId, count)
        if (count % COMPRESSION_EVERY_N_TURNS === 0) {
          const botId = session.botId
          setTimeout(() => {
            const s = botSessions.get(botId)
            if (!s || s.isLoading) return
            const bot = useBotStore().bots.find(b => b.id === botId)
            if (!bot) return
            sendMessage(bot, [
              '【系统维护】请整理 CLAUDE.md 记忆：',
              '1. 读取 CLAUDE.md（不存在则按模板创建）',
              '2. 将近期对话中有长期价值的信息归入对应章节：',
              '   - 用户画像：沟通偏好、专业背景',
              '   - 工具与工作流偏好：工具链、代码风格',
              '   - 关键知识：项目约定、领域知识',
              '   - 错误规避：纠正过的错误、踩过的坑',
              '3. 更新「近期摘要」：保留最近 8 条重要记录（日期 + 一句话要点），删除过时或冗余内容',
              '4. 全文控制在 200 行以内，精简最旧或最低价值的条目',
              '5. 每条记忆保留 1-2 句上下文，不要过度压缩',
              '完成后回复"记忆更新"。',
            ].join('\n'), undefined, { isSystem: true })
          }, 2000)
        }
        break
      }

      case 'error':
        session.isLoading = false
        session.error = output.message ?? 'Unknown error'
        streamingStates.delete(assistantMsg.id)
        lastOutputSeqs.delete(assistantMsg.id)
        // 清理防阻塞 timers
        clearSessionTimers(session.botId, assistantMsg.id)
        lastActivityTime.delete(session.botId)
        useBotStore().updateBotStatus(session.botId, 'error', session.error ?? undefined)
        releaseActiveSourceAndDrain(session.botId, 'sidecar_error')
        break
    }
  }

  async function spawnSidecar(session: BotSession): Promise<void> {
    if (session.sidecarActive) return

    // 防御性注销：确保不会残留旧监听器（如看门狗重置后重新启动）
    cleanupListeners(session)

    const settingsStore = useSettingsStore()
    const botStore = useBotStore()
    const envVars = { ...(settingsStore.environmentVariables || {}) }

    // Apply custom provider env vars if the bot uses a custom provider
    const bot = botStore.bots.find(b => b.id === session.botId)
    const coworksConfig = settingsStore.coworksModelConfig
    if (bot && coworksConfig?.providers?.length) {
      const provider = coworksConfig.providers.find(p => p.id === bot.model)
      if (provider) {
        if (provider.baseUrl) envVars['ANTHROPIC_BASE_URL'] = provider.baseUrl
        if (provider.apiKey) envVars['ANTHROPIC_AUTH_TOKEN'] = provider.apiKey
        if (provider.model) envVars['ANTHROPIC_MODEL'] = provider.model
      }
    }

    session.unlistenOutput = await listen<string>(
      `coworks-sidecar-output-${session.sidecarId}`,
      (event) => {
        const line = event.payload
        if (!line.trim()) return
        try {
          const output: SidecarOutput = JSON.parse(line)
          const currentSession = botSessions.get(session.botId)
          if (currentSession) {
            handleSidecarOutput(output, currentSession)
          }
        } catch (e) {
          console.error('[useBotSession] Failed to parse output:', e)
        }
      }
    )

    session.unlistenReady = await listen<void>(
      `coworks-sidecar-ready-${session.sidecarId}`,
      () => {
        const s = botSessions.get(session.botId)
        if (s) s.initialized = true
      }
    )

    session.unlistenStderr = await listen<string>(
      `coworks-sidecar-stderr-${session.sidecarId}`,
      () => {
        // 有 stderr 输出说明 sidecar 仍在运行（例如 SDK 历史回放期间只走 stderr）
        // 刷新看门狗时间戳，防止回放长历史时被误判为卡死
        const s = botSessions.get(session.botId)
        if (s?.isLoading) {
          lastActivityTime.set(session.botId, Date.now())
        }
      }
    )

    await invoke('coworks_spawn_sidecar', {
      sessionId: session.sidecarId,
      env: Object.keys(envVars).length > 0 ? envVars : null,
    })
    session.sidecarActive = true

    // Wait for sidecar to emit { type: 'ready' } via stdout (detected by Rust)
    const SIDECAR_READY_TIMEOUT_MS = 15_000
    try {
      await new Promise<void>((resolve, reject) => {
        const check = setInterval(() => {
          if (session.initialized) { clearInterval(check); resolve() }
        }, 50)
        setTimeout(() => {
          clearInterval(check)
          if (!session.initialized) {
            reject(new Error(`Sidecar 启动超时 (${SIDECAR_READY_TIMEOUT_MS / 1000}s)`))
          } else {
            resolve()
          }
        }, SIDECAR_READY_TIMEOUT_MS)
      })
      session.initialized = true
    } catch (e) {
      // Clean up the failed sidecar: kill process, remove listeners, reset state
      await invoke('coworks_kill_sidecar', { sessionId: session.sidecarId }).catch(() => {})
      cleanupListeners(session)
      session.sidecarActive = false
      session.initialized = false
      throw e
    }
  }

  async function sendMessage(
    bot: BotDefinition,
    content: string,
    attachments?: { path: string; name: string }[],
    options?: { isSystem?: boolean },
  ) {
    const session = ensureSession(bot.id)
    if (session.isLoading) return

    // 在第一个 await 前立即标记 loading 并更新状态，防止 drainQueue + drainScheduled 竞态：
    // 两者都在 done 回调中同步调用 sendMessage，若不在第一个 await 前设置 isLoading，
    // 两次调用都能通过上方的守卫检查，导致 sidecar 收到两条 start 消息，
    // 所有 text_delta 叠加到同一个 streaming state，最终输出内容重复。
    session.isLoading = true
    session.error = null
    useBotStore().updateBotStatus(bot.id, 'working')

    await ensureConversationLoaded(bot.id)

    // Add user message
    const userMsg: BotChatMessage = {
      id: generateId(),
      role: 'user',
      content: content.trim(),
      timestamp: Date.now(),
      isSystem: options?.isSystem,
    }
    session.messages.push(userMsg)

    // Add assistant placeholder
    const assistantMsg: BotChatMessage = {
      id: generateId(),
      role: 'assistant',
      content: '',
      timestamp: Date.now(),
      toolUsages: [],
      isSystem: options?.isSystem,
    }
    session.messages.push(assistantMsg)
    streamingStates.set(assistantMsg.id, { content: '', toolUsages: [] })

    try {
      // Spawn sidecar if needed
      await spawnSidecar(session)

      // Resolve model name: for custom providers, use the actual model name
      let resolvedModel = bot.model
      const coworksModelConfig = useSettingsStore().coworksModelConfig
      if (coworksModelConfig?.providers?.length) {
        const matchedProvider = coworksModelConfig.providers.find(p => p.id === bot.model)
        if (matchedProvider) {
          resolvedModel = matchedProvider.model
        }
      }

      // Send start message
      const startMsg = JSON.stringify({
        type: 'start',
        sessionId: assistantMsg.id,
        workDir: bot.workDir,
        model: resolvedModel === 'auto' ? 'auto' : resolvedModel,
        prompt: content.trim(),
        mode: 'code',
        sdkSessionId: session.sdkSessionId,
        systemPrompt: bot.systemPrompt,
        usePreset: true,
        attachments: attachments ?? [],
      })
      await invoke('coworks_send_to_sidecar', {
        sessionId: session.sidecarId,
        message: startMsg,
      })

      // 机制 3：1 分钟后向飞书发送"思考中"进度消息
      const thinkingHandle = setTimeout(() => {
        if (!session.isLoading) return
        const ch = useBotChannels()
        const thinkingTarget = ch.getActiveSourceChatId(bot.id)
        ch.broadcastMessage(bot.id, '⏳ 正在处理中，请稍候...', thinkingTarget)
          .catch(() => {})
      }, THINKING_NOTIFY_DELAY_MS)
      thinkingTimerHandles.set(bot.id, thinkingHandle)
    } catch (e: unknown) {
      session.error = e instanceof Error ? e.message : 'Failed to send message'
      session.isLoading = false
      streamingStates.delete(assistantMsg.id)
      lastOutputSeqs.delete(assistantMsg.id)
      clearSessionTimers(bot.id, assistantMsg.id)
      lastActivityTime.delete(bot.id)
      useBotStore().updateBotStatus(bot.id, 'error', session.error ?? undefined)
      releaseActiveSourceAndDrain(bot.id, 'send_message_failure')
    }
  }

  async function stopSession(botId: string) {
    const session = botSessions.get(botId)
    if (!session) return

    try {
      if (session.sidecarActive) {
        await invoke('coworks_kill_sidecar', { sessionId: session.sidecarId })
      }
    } catch (e) {
      console.error(`[useBotSession] Failed to kill sidecar for ${botId}:`, e)
    }

    // 清理防阻塞 timers
    const lastMsg = session.messages[session.messages.length - 1]
    if (lastMsg) clearSessionTimers(botId, lastMsg.id)
    lastActivityTime.delete(botId)

    cleanupListeners(session)
    session.sidecarActive = false
    session.initialized = false
    session.isLoading = false
    session.messages.forEach(msg => lastOutputSeqs.delete(msg.id))
    botSessions.delete(botId)
    conversationLoaded.delete(botId)  // allow reload on next start
  }

  return {
    getSession,
    getMessages,
    isLoading,
    getError,
    ensureConversationLoaded,
    saveConversation,
    sendMessage,
    stopSession,
  }
}
