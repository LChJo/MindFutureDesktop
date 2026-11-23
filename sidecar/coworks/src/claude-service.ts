import { query, type SDKAssistantMessage, type SDKResultMessage, type SDKSystemMessage, type Query } from '@anthropic-ai/claude-agent-sdk'
import { existsSync, mkdirSync, readFileSync } from 'fs'
import { join } from 'path'
import type { OutputMessage, CoworkMode, PluginInfo, AttachmentInfo } from './types.js'
import { buildOptions } from './mode-config.js'

// 日志前缀
const LOG_PREFIX = '[claude-service]'

// 日志工具函数
function log(level: 'INFO' | 'DEBUG' | 'WARN' | 'ERROR', ...args: unknown[]): void {
  const timestamp = new Date().toISOString()
  console.error(`${timestamp} ${LOG_PREFIX} [${level}]`, ...args)
}

// Active sessions for cancellation
const activeSessions = new Map<string, Query>()
const outputSeqs = new Map<string, number>()

function nextOutputSeq(sessionId: string): number {
  const seq = (outputSeqs.get(sessionId) ?? 0) + 1
  outputSeqs.set(sessionId, seq)
  return seq
}

function emit(message: OutputMessage): void {
  const payload = 'sessionId' in message
    ? { ...message, seq: nextOutputSeq(message.sessionId) }
    : message
  const json = JSON.stringify(payload)
  // 输出到 stdout（给前端）
  console.log(json)
  // text_delta 只打摘要，避免高频日志淹没 stderr
  if (payload.type === 'text_delta') {
    const content = (payload as { content?: string }).content ?? ''
    log('DEBUG', `[Sidecar OUT] text_delta (+${content.length} chars)`)
  } else {
    log('INFO', `[Sidecar OUT] ${json}`)
  }
}

function normalizeResumePrompt(text: string): string {
  return text
    .replace(/^\[[^\]]+\]\s+[^\r\n:]{1,200}:\s*/u, '')
    .replace(/\r\n/g, '\n')
    .trim()
}

function isResumePromptMatch(userText: string, finalPrompt: string): boolean {
  const rawUser = userText.replace(/\r\n/g, '\n').trim()
  const rawPrompt = finalPrompt.replace(/\r\n/g, '\n').trim()
  const normalizedUser = normalizeResumePrompt(userText)
  const normalizedPrompt = normalizeResumePrompt(finalPrompt)

  if (!rawUser || !rawPrompt) return false
  if (rawUser === rawPrompt) return true
  if (!normalizedUser || !normalizedPrompt) return false
  return normalizedUser === normalizedPrompt
}

function loadAssistantHistory(workDir: string): string[] {
  try {
    const conversationPath = join(workDir, 'conversation.json')
    if (!existsSync(conversationPath)) return []
    const raw = readFileSync(conversationPath, 'utf8')
    const messages = JSON.parse(raw) as Array<{ role?: string; content?: string }>
    const assistantHistory: string[] = []
    for (let i = messages.length - 1; i >= 0; i -= 1) {
      const message = messages[messages.length - 1 - i]
      if (message?.role === 'assistant' && typeof message.content === 'string' && message.content.trim()) {
        assistantHistory.push(message.content.trim())
      }
    }
    return assistantHistory
  } catch (error) {
    log('WARN', `[SDK] Failed to load conversation.json for resume matching: ${error}`)
  }
  return []
}

function collectAssistantText(content: Array<{ type: string; text?: string }>): string {
  return content
    .filter((block) => block.type === 'text' && typeof block.text === 'string')
    .map((block) => block.text ?? '')
    .join('')
    .trim()
}

function isReplayAssistantMatch(assistantText: string, previousAssistantText?: string): boolean {
  if (!assistantText || !previousAssistantText) return false
  const current = assistantText.replace(/\s+/g, ' ').trim()
  const previous = previousAssistantText.replace(/\s+/g, ' ').trim()
  if (!current || !previous) return false
  if (current === previous) return true

  // Very short replies like "2" or "OK" are common real outputs; prefix matching
  // would falsely treat "26" as a replay of "2".
  if (current.length < 8 || previous.length < 8) return false

  return current.startsWith(previous) || previous.startsWith(current)
}

export async function startSession(
  sessionId: string,
  workDir: string,
  model: string,
  prompt: string,
  mode: CoworkMode,
  systemPrompt?: string,
  plugin?: PluginInfo,
  attachments?: AttachmentInfo[],
  sdkSessionId?: string,
  usePreset?: boolean
): Promise<void> {
  log('INFO', '========== 会话开始 ==========')
  log('INFO', `SessionId: ${sessionId}`)
  log('INFO', `Mode: ${mode}`)
  log('INFO', `WorkDir: ${workDir}`)
  log('INFO', `Model: ${model}`)
  log('INFO', `Prompt: ${prompt.substring(0, 100)}${prompt.length > 100 ? '...' : ''}`)
  if (attachments && attachments.length > 0) {
    log('INFO', `Attachments: ${attachments.map(a => a.name).join(', ')}`)
  }
  if (systemPrompt) {
    log('INFO', `CustomSystemPrompt: ${systemPrompt.substring(0, 50)}...`)
  }
  if (usePreset) {
    log('INFO', `UsePreset: true`)
  }
  if (plugin) {
    log('INFO', `Plugin: ${plugin.path}`)
    log('INFO', `SkillName: ${plugin.skillName}`)
    log('INFO', `SkillDescription: ${plugin.skillDescription}`)
  }

  // Ensure workDir exists (avoid SDK spawn ENOENT when cwd is missing)
  if (workDir && !existsSync(workDir)) {
    log('WARN', `WorkDir 不存在，自动创建: ${workDir}`)
    try {
      mkdirSync(workDir, { recursive: true })
    } catch (e) {
      log('WARN', `创建 WorkDir 失败: ${e}，回退到 process.cwd()`)
      workDir = process.cwd()
    }
  }

  // Cancel any existing session with the same ID
  await cancelSession(sessionId)

  try {
    // Build options based on mode
    const options = buildOptions(mode, workDir, model, systemPrompt, plugin, usePreset)

    // If we have a previous SDK session ID, use resume option to restore context
    if (sdkSessionId) {
      options.resume = sdkSessionId
      log('INFO', `Resuming SDK session: ${sdkSessionId}`)
    }

    // Build final prompt with attachments
    let finalPrompt = prompt
    if (attachments && attachments.length > 0) {
      const attachmentList = attachments
        .map((a) => `- ${a.name}: ${a.path}`)
        .join('\n')
      finalPrompt = `[附件文件]\n${attachmentList}\n\n请先使用 Read 工具读取附件内容，然后回答用户的问题。\n\n${prompt}`
    }

    // Timeout constants
    const FIRST_MESSAGE_TIMEOUT_MS = 90_000       // 90s for fresh sessions
    const RESUME_FIRST_MESSAGE_TIMEOUT_MS = 30_000 // 30s for resume (fail fast)
    const RESUME_REPLAY_STALL_TIMEOUT_MS = 45_000
    const PROGRESS_HEARTBEAT_INTERVAL_MS = 15_000

    const previousAssistantHistory = sdkSessionId ? loadAssistantHistory(workDir) : []
    if (previousAssistantHistory.length > 0) {
      log('DEBUG', `[SDK] Loaded ${previousAssistantHistory.length} previous assistant message(s) for resume matching`)
    }

    // Run query, with automatic retry on resume failure
    const runQuery = async (opts: typeof options): Promise<void> => {
      log('DEBUG', `[Sidecar SDK] prompt=${finalPrompt.substring(0, 100)}${finalPrompt.length > 100 ? '...' : ''}`)

      const response = query({ prompt: finalPrompt, options: opts })
      activeSessions.set(sessionId, response)
      log('INFO', `Query 已启动，等待响应... (resume=${opts.resume || 'none'})`)

      // Emit early session_started so frontend refreshes watchdog activity timestamp.
      // sdkSessionId is intentionally empty — frontend guards with `if (output.sdkSessionId)`
      // so this won't overwrite a valid persisted session ID.
      emit({ type: 'session_started', sessionId, sdkSessionId: '' })

      // First-message timeout: abort SDK if no message within threshold
      const timeoutMs = opts.resume
        ? RESUME_FIRST_MESSAGE_TIMEOUT_MS
        : FIRST_MESSAGE_TIMEOUT_MS
      let firstMessageReceived = false
      let timedOut = false
      const timeoutId = setTimeout(() => {
        if (!firstMessageReceived) {
          timedOut = true
          log('ERROR', `SDK 首消息超时 (${timeoutMs / 1000}s)，中止查询`)
          response.interrupt().catch(() => {})
        }
      }, timeoutMs)

      let messageCount = 0
      let capturedSdkSessionId: string | undefined
      let firstMessageAt = 0
      let resumeStallTimerId: ReturnType<typeof setTimeout> | null = null
      let lastProgressEmitAt = 0
      // 跟踪当前 agentic turn 已 emit 的文本内容，防止 SDK 多次 emit 同一 assistant
      // 消息（先发 [text]、再发 [text, tool_use]）导致文本内容重复。
      // 同时兼容某些模型只在 result.result 中给出最终文本的场景。
      let emittedText = ''
      // Resume 模式下，SDK 会重播历史对话。skipHistoryAssistant=true 时跳过历史 assistant
      // 消息，直到在 user 消息中检测到当前 prompt 为止，防止历史内容被叠加到新回复中。
      let skipHistoryAssistant = !!opts.resume
      let skippedReplayAssistantCount = 0
      let replayAssistantIndex = 0

      const emitProgress = (stage: string, message?: string, force = false): void => {
        const now = Date.now()
        if (!force && now - lastProgressEmitAt < PROGRESS_HEARTBEAT_INTERVAL_MS) {
          return
        }
        lastProgressEmitAt = now
        emit({ type: 'progress', sessionId, stage, message })
      }

      const resetResumeStallTimer = (): void => {
        if (resumeStallTimerId !== null) {
          clearTimeout(resumeStallTimerId)
        }
        resumeStallTimerId = setTimeout(() => {
          if (!skipHistoryAssistant) return
          emitProgress(
            'resume_stalled',
            `Resume replay exceeded ${Math.round(RESUME_REPLAY_STALL_TIMEOUT_MS / 1000)}s without reaching the new turn`,
            true,
          )
          response.interrupt().catch(() => {})
        }, RESUME_REPLAY_STALL_TIMEOUT_MS)
      }

      try {
        for await (const message of response) {
          if (!firstMessageReceived) {
            firstMessageReceived = true
            firstMessageAt = Date.now()
            clearTimeout(timeoutId)
            if (opts.resume && skipHistoryAssistant) resetResumeStallTimer()
            log('INFO', `SDK 首消息已到达 (耗时 <${timeoutMs / 1000}s)`)
          }
          messageCount++

          // Extract session_id from SDK message
          const msgObj = message as Record<string, unknown>
          const msgSessionId = msgObj.session_id as string | undefined

          // Capture session_id and emit session_started with real ID
          if (msgSessionId && !capturedSdkSessionId) {
            capturedSdkSessionId = msgSessionId
            emit({
              type: 'session_started',
              sessionId,
              sdkSessionId: capturedSdkSessionId,
            })
          }

          // Handle different message types
          if (message.type === 'system') {
            const sysMsg = message as SDKSystemMessage
            log('INFO', `[SDK] System message - Model: ${sysMsg.model}`)
            log('DEBUG', `[SDK] Available tools: ${sysMsg.tools?.join(', ') || '(none)'}`)
            log('DEBUG', `[SDK] Available skills: ${sysMsg.skills?.join(', ') || '(none)'}`)
            log('DEBUG', `[SDK] Loaded plugins: ${JSON.stringify(sysMsg.plugins ?? [])}`)
            log('DEBUG', `[SDK] Slash commands: ${sysMsg.slash_commands?.join(', ') || '(none)'}`)
            continue
          }

          if (message.type === 'user') {
            log('DEBUG', '[SDK] User message (跳过)')
            emittedText = ''  // 新一轮 turn 开始，重置文本计数

            // Resume 模式：检查是否匹配当前 prompt，若匹配则停止跳过历史
            if (skipHistoryAssistant) {
              resetResumeStallTimer()
              emitProgress('resume_replay', 'Replaying resume history before the current turn')
              const userMsg = message as Record<string, unknown>
              const msgContent = userMsg.message as Record<string, unknown> | undefined
              const contentArr = Array.isArray(msgContent?.content)
                ? (msgContent!.content as Array<{ type: string; text?: string }>)
                : null
              const userText: string = contentArr
                ? contentArr
                    .filter((b) => b.type === 'text' && b.text)
                    .map((b) => b.text!)
                    .join('')
                : (typeof msgContent?.content === 'string' ? (msgContent.content as string) : '')
              if (isResumePromptMatch(userText, finalPrompt)) {
                skipHistoryAssistant = false
                if (resumeStallTimerId !== null) { clearTimeout(resumeStallTimerId); resumeStallTimerId = null }
                log('DEBUG', '[SDK] Detected new query user message, resume history skip ended')
                emitProgress('resume_ready', 'Resume replay reached the current turn', true)
              }
            }
            continue
          }

          if (message.type === 'assistant') {
            const assistantMsg = message as SDKAssistantMessage
            log('DEBUG', `[SDK] Assistant message, blocks: ${assistantMsg.message.content.length}`)

            // Resume 模式下跳过历史重播的 assistant 消息
            if (skipHistoryAssistant) {
              resetResumeStallTimer()
              const assistantText = collectAssistantText(assistantMsg.message.content)
              const expectedReplayAssistant = previousAssistantHistory[replayAssistantIndex]

              if (expectedReplayAssistant && isReplayAssistantMatch(assistantText, expectedReplayAssistant)) {
                skippedReplayAssistantCount += 1
                replayAssistantIndex += 1
                emitProgress('resume_replay', `Skipping replayed assistant message #${skippedReplayAssistantCount}`)
                log('DEBUG', '[SDK] Skipping replayed assistant message (matched previous assistant)')
                continue
              }

              if (previousAssistantHistory.length === 0) {
                emitProgress('resume_failed', 'Resume replay could not be matched against local history', true)
                throw new Error('Resume replay could not be matched against local history')
              }

              if (replayAssistantIndex > 0 && replayAssistantIndex < previousAssistantHistory.length) {
                emitProgress('resume_failed', 'Resume replay diverged from local history before reaching current turn', true)
                throw new Error('Resume replay diverged from local history before reaching current turn')
              }

              skipHistoryAssistant = false
              if (resumeStallTimerId !== null) { clearTimeout(resumeStallTimerId); resumeStallTimerId = null }
              log('DEBUG', '[SDK] Resume replay ended on assistant message')
              emitProgress('resume_ready', 'Resume replay reached the current turn via assistant message', true)
            }

            for (const block of assistantMsg.message.content) {
              if (block.type === 'text') {
                const newContent = block.text.startsWith(emittedText)
                  ? block.text.slice(emittedText.length)
                  : block.text
                if (newContent) {
                  log('DEBUG', `[SDK] Text block, length: ${block.text.length} (+${newContent.length} new)`)
                  emit({
                    type: 'text_delta',
                    sessionId,
                    content: newContent,
                  })
                  emittedText = block.text
                } else {
                  log('DEBUG', `[SDK] Text block, length: ${block.text.length} (skipped, already emitted)`)
                }
              } else if (block.type === 'tool_use') {
                log('INFO', `[SDK] Tool use: ${block.name}`)
                log('DEBUG', `[SDK] Tool input:`, JSON.stringify(block.input).substring(0, 200))
                emit({
                  type: 'tool_use',
                  sessionId,
                  toolId: block.id,
                  toolName: block.name,
                  input: block.input as Record<string, unknown>,
                })
              }
            }
          }

          if (message.type === 'result') {
            const resultMsg = message as SDKResultMessage
            if (skipHistoryAssistant) {
              emitProgress('resume_failed', 'Resume replay finished without reaching the current prompt', true)
              log('WARN', '[SDK] Resume history skip was still active at result — prompt match may have failed')
              throw new Error('Resume replay completed without reaching current prompt')
            }
            if (resultMsg.subtype === 'success' && typeof resultMsg.result === 'string' && resultMsg.result) {
              if (resultMsg.result.startsWith(emittedText)) {
                const newContent = resultMsg.result.slice(emittedText.length)
                if (newContent) {
                  log('INFO', `[SDK] Result text补发: total=${resultMsg.result.length} (+${newContent.length} new)`)
                  emit({
                    type: 'text_delta',
                    sessionId,
                    content: newContent,
                  })
                }
              } else if (!emittedText) {
                log('INFO', `[SDK] Result text直出: total=${resultMsg.result.length}`)
                emit({
                  type: 'text_delta',
                  sessionId,
                  content: resultMsg.result,
                })
              } else {
                log('WARN', `[SDK] Result text differs from streamed text: streamed=${emittedText.length}, result=${resultMsg.result.length}`)
              }
              emittedText = resultMsg.result
            }
            log('INFO', '[SDK] Result message, 查询完成')
            break
          }
        }
      } finally {
        clearTimeout(timeoutId)
        if (resumeStallTimerId !== null) clearTimeout(resumeStallTimerId)
      }

      // If the loop exited due to timeout (interrupt resolved cleanly), throw
      if (timedOut) {
        throw new Error(`SDK 首消息超时 (${timeoutMs / 1000}s)`)
      }

      // If resume replay was still active when the loop ended (stall interrupt or
      // stream finished without reaching the current prompt), trigger retry
      if (skipHistoryAssistant) {
        throw new Error('Resume replay did not reach current prompt (stall or interrupt)')
      }

      log('INFO', `会话完成，共处理 ${messageCount} 条消息`)
    }

    try {
      await runQuery(options)
    } catch (resumeError) {
      // If resume was used and failed, retry without resume (stale session fallback)
      if (options.resume) {
        const errMsg = resumeError instanceof Error ? resumeError.message : String(resumeError)
        log('WARN', `Resume 失败 (${errMsg})，将以新会话重试...`)
        activeSessions.delete(sessionId)
        const { resume: _, ...freshOptions } = options
        await runQuery(freshOptions)
      } else {
        throw resumeError
      }
    }

    emit({ type: 'done', sessionId })
  } catch (error) {
    log('ERROR', '会话出错:', error instanceof Error ? error.message : String(error))
    emit({
      type: 'error',
      sessionId,
      message: error instanceof Error ? error.message : String(error),
    })
  } finally {
    activeSessions.delete(sessionId)
    outputSeqs.delete(sessionId)
    log('INFO', '========== 会话结束 ==========')
  }
}

export async function cancelSession(sessionId: string): Promise<void> {
  const response = activeSessions.get(sessionId)
  if (response) {
    log('INFO', `取消会话: ${sessionId}`)
    try {
      await response.interrupt()
      log('INFO', '会话已中断')
    } catch (error) {
      log('WARN', '中断会话时出错:', error)
    }
    activeSessions.delete(sessionId)
  }
}
