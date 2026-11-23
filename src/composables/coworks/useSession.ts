/**
 * Session composable for Coworks
 * Manages chat sessions per mode + context combination
 * Each session maintains its own message history and sidecar process
 */

import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { homeDir } from '@tauri-apps/api/path'
import type { AppMode, ChatMessage, ToolUsage, SidecarOutput } from '@/types/coworks'
import { useSettingsStore } from '@/stores/settingsStore'
import { useTasks } from './useTasks'
import { useChatHistory } from './useChatHistory'
import { useWorkspace } from './useWorkspace'

// Cache for home directory
let cachedHomeDir: string | null = null

async function getDefaultWorkDir(): Promise<string> {
  if (cachedHomeDir) return cachedHomeDir
  try {
    cachedHomeDir = await homeDir()
    return cachedHomeDir
  } catch {
    // Fallback for development
    return 'C:\\'
  }
}

export interface Session {
  id: string
  mode: AppMode
  contextId: string // chatId for chat, projectPath for code, taskId for cowork
  workDir: string // Working directory (empty for chat mode)
  model: string
  messages: ChatMessage[]
  isLoading: boolean
  error: string | null
  sidecarActive: boolean
  initialized: boolean
  unlistenOutput?: UnlistenFn
  unlistenReady?: UnlistenFn
  unlistenStderr?: UnlistenFn
  onSessionStarted?: (sdkSessionId: string) => void
  onComplete?: () => void | Promise<void> // Called when streaming completes (done/error/cancelled)
  sdkSessionId?: string // For modes that manage their own sdkSessionId (e.g., claudeapps)
  systemPrompt?: string // Custom system prompt (overrides mode default in sidecar)
}

// Generate unique ID
function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).substring(2, 11)}`
}

// Session key for Map
function getSessionKey(mode: AppMode, contextId: string): string {
  return `${mode}::${contextId}`
}

// Singleton state
const sessions = ref<Map<string, Session>>(new Map())
const currentSessionKey = ref<string | null>(null)
// Track file changes for auto-refresh (timestamp of last file-modifying tool)
const lastFileChange = ref<number>(0)

// Track generated/mentioned files from text output
export interface GeneratedFile {
  path: string
  name: string
  timestamp: number
}
const generatedFile = ref<GeneratedFile | null>(null)

// Extract file path from text content
function extractFilePath(text: string): string | null {
  // Match Windows paths like C:\path\to\file.ext or I:\AI\temp\file.png
  // Also match paths in backticks like `I:\AI\temp\画图\generated.png`
  const windowsPathRegex = /`?([A-Za-z]:\\[^`\n\r]+\.[a-zA-Z0-9]+)`?/g
  // Match Unix paths like /path/to/file.ext
  const unixPathRegex = /`?(\/[^`\s\n\r]+\.[a-zA-Z0-9]+)`?/g

  // Try Windows path first
  const windowsMatch = text.match(windowsPathRegex)
  if (windowsMatch) {
    // Get the last match (most recent file mentioned)
    const lastPath = windowsMatch[windowsMatch.length - 1]
    // Remove backticks if present
    return lastPath.replace(/`/g, '')
  }

  // Try Unix path
  const unixMatch = text.match(unixPathRegex)
  if (unixMatch) {
    const lastPath = unixMatch[unixMatch.length - 1]
    return lastPath.replace(/`/g, '')
  }

  return null
}

// 每个消息的流式状态（按 sessionId/messageId 索引）
interface StreamingState {
  content: string
  toolUsages: ToolUsage[]
}
const streamingStates = new Map<string, StreamingState>()

export function useSession() {
  // Get current session
  const currentSession = computed<Session | null>(() => {
    if (!currentSessionKey.value) return null
    return sessions.value.get(currentSessionKey.value) || null
  })

  // Get or create a session for the given mode and context
  // Pass setCurrent=false to avoid changing the global currentSessionKey (used by Claude Apps)
  function getOrCreateSession(
    mode: AppMode,
    contextId: string,
    model: string,
    workDir: string = '',
    setCurrent: boolean = true
  ): Session {
    const key = getSessionKey(mode, contextId)
    const existingSession = sessions.value.get(key)

    if (existingSession) {
      // Update model if changed
      if (existingSession.model !== model) {
        existingSession.model = model
      }

      // Update workDir if changed
      if (existingSession.workDir !== workDir) {
        existingSession.workDir = workDir
      }

      if (setCurrent) {
        currentSessionKey.value = key
      }
      return existingSession
    }

    // Create new session
    const newSession: Session = {
      id: generateId(),
      mode,
      contextId,
      workDir,
      model,
      messages: [],
      isLoading: false,
      error: null,
      sidecarActive: false,
      initialized: false,
    }
    sessions.value.set(key, newSession)
    if (setCurrent) {
      currentSessionKey.value = key
    }
    // Return the reactive proxy from the Map, not the raw object.
    // This ensures callers (e.g. loadChat doing s.messages = chat.messages)
    // mutate the same object that currentSession.value reads from.
    return sessions.value.get(key)!
  }

  // Switch to a session
  function switchToSession(mode: AppMode, contextId: string) {
    const key = getSessionKey(mode, contextId)
    if (sessions.value.has(key)) {
      currentSessionKey.value = key
    }
  }

  // Clear current session (without destroying it)
  function clearCurrentSession() {
    currentSessionKey.value = null
  }

  // Set messages for current session (for loading from history)
  function setMessages(messages: ChatMessage[]) {
    const session = currentSession.value
    if (session) {
      session.messages = messages
    }
  }

  // Send a message in the current session (or a specific session by key)
  async function sendMessage(content: string, attachments?: { path: string; name: string }[], targetSessionKey?: string) {
    const session = targetSessionKey
      ? sessions.value.get(targetSessionKey) ?? null
      : currentSession.value
    if (!session) {
      console.error('[useSession] No current session')
      return
    }

    if (!content.trim() || session.isLoading) {
      return
    }

    // Add user message
    const userMessage: ChatMessage = {
      id: generateId(),
      role: 'user',
      content: content.trim(),
      timestamp: Date.now(),
    }
    session.messages.push(userMessage)

    // Reset state
    session.isLoading = true
    session.error = null

    // Add placeholder for assistant message
    const assistantMessage: ChatMessage = {
      id: generateId(),
      role: 'assistant',
      content: '',
      timestamp: Date.now(),
      toolUsages: [],
    }
    session.messages.push(assistantMessage)

    // 初始化该消息的流式状态
    streamingStates.set(assistantMessage.id, {
      content: '',
      toolUsages: [],
    })

    try {
      // Spawn sidecar if not already running
      if (!session.sidecarActive) {
        // Setup event listeners
        // Capture the session key (not the session object) to avoid stale closure references
        const sessionKey = getSessionKey(session.mode, session.contextId)
        session.unlistenOutput = await listen<string>(
          `coworks-sidecar-output-${session.id}`,
          (event) => {
            const line = event.payload
            if (!line.trim()) return

            try {
              const output: SidecarOutput = JSON.parse(line)
              // 动态查找 session，避免闭包引用过期
              const currentSess = sessions.value.get(sessionKey)
              if (currentSess) {
                handleSidecarOutput(output, currentSess)
              }
            } catch (e) {
              console.error('[useSession] Failed to parse sidecar output:', e, line)
            }
          }
        )

        session.unlistenReady = await listen<void>(
          `coworks-sidecar-ready-${session.id}`,
          () => {
            const currentSess = sessions.value.get(sessionKey)
            if (currentSess) {
              currentSess.initialized = true
            }
          }
        )

        // Listen for stderr (dev mode only, emitted by Rust in debug build)
        // Log sidecar IN/OUT and SDK call parameters as expandable objects
        session.unlistenStderr = await listen<string>(
          `coworks-sidecar-stderr-${session.id}`,
          (event) => {
            const line = event.payload
            // [Sidecar IN] - Message from frontend to sidecar
            if (line.includes('[Sidecar IN]')) {
              const jsonMatch = line.match(/\[Sidecar IN\]\s*(.+)$/)
              if (jsonMatch) {
                try {
                  const data = JSON.parse(jsonMatch[1])
                  console.log('%c[Sidecar IN]', 'color: #2196F3; font-weight: bold', data)
                } catch {
                  console.log('%c[Sidecar IN]', 'color: #2196F3; font-weight: bold', jsonMatch[1])
                }
              }
            }
            // [Sidecar SDK] - Parameters passed to Claude SDK
            else if (line.includes('[Sidecar SDK]')) {
              const jsonMatch = line.match(/\[Sidecar SDK\]\s*(.+)$/)
              if (jsonMatch) {
                try {
                  const data = JSON.parse(jsonMatch[1])
                  console.log('%c[Sidecar SDK]', 'color: #9C27B0; font-weight: bold', data)
                } catch {
                  console.log('%c[Sidecar SDK]', 'color: #9C27B0; font-weight: bold', jsonMatch[1])
                }
              }
            }
            // [Sidecar OUT] - Response from SDK
            else if (line.includes('[Sidecar OUT]')) {
              const jsonMatch = line.match(/\[Sidecar OUT\]\s*(.+)$/)
              if (jsonMatch) {
                try {
                  const data = JSON.parse(jsonMatch[1])
                  console.log('%c[Sidecar OUT]', 'color: #4CAF50; font-weight: bold', data)
                } catch {
                  console.log('%c[Sidecar OUT]', 'color: #4CAF50; font-weight: bold', jsonMatch[1])
                }
              }
            }
            // [SDK MSG] - Debug info about SDK messages
            else if (line.includes('[SDK MSG]') || line.includes('[EMIT]') || line.includes('[SDK]')) {
              console.log('%c[Sidecar DEBUG]', 'color: #FF9800; font-weight: bold', line)
            }
          }
        )

        // Get environment variables for sidecar (including ANTHROPIC_AUTH_TOKEN)
        const settingsStore = useSettingsStore()
        const envVars = { ...(settingsStore.environmentVariables || {}) }

        // Apply custom Coworks provider env vars if using a custom provider
        const coworksConfig = settingsStore.coworksModelConfig
        if (coworksConfig?.providers?.length) {
          const provider = coworksConfig.providers.find(p => p.id === session.model)
          if (provider) {
            if (provider.baseUrl) envVars['ANTHROPIC_BASE_URL'] = provider.baseUrl
            if (provider.apiKey) envVars['ANTHROPIC_AUTH_TOKEN'] = provider.apiKey
            if (provider.model) envVars['ANTHROPIC_MODEL'] = provider.model
          }
        }

        console.log('[useSession] Spawning sidecar with env vars:', Object.keys(envVars))

        // Spawn the sidecar via Rust with environment variables
        await invoke('coworks_spawn_sidecar', {
          sessionId: session.id,
          env: Object.keys(envVars).length > 0 ? envVars : null
        })
        session.sidecarActive = true

        // Wait for ready signal
        await new Promise<void>((resolve) => {
          const checkReady = setInterval(() => {
            if (session.initialized) {
              clearInterval(checkReady)
              resolve()
            }
          }, 50)
          // Timeout after 5 seconds
          setTimeout(() => {
            clearInterval(checkReady)
            resolve()
          }, 5000)
        })
      }

      // Mark as initialized after sidecar is ready
      session.initialized = true

      // Determine workDir based on mode
      // Chat mode: empty workDir (sidecar will use process.cwd())
      // Code/Cowork/ClaudeApps mode: use session.workDir or fallback to home directory
      let effectiveWorkDir = ''
      if (session.mode !== 'chat') {
        effectiveWorkDir = session.workDir
        if (!effectiveWorkDir || effectiveWorkDir === '.') {
          effectiveWorkDir = await getDefaultWorkDir()
        }
      }

      // Get plugin info if in cowork mode with a skill-based task
      let plugin: { path: string; skillName: string; skillDescription: string } | undefined
      const tasksComposable = useTasks()
      if (session.mode === 'cowork') {
        const currentTask = tasksComposable.currentTask.value
        if (currentTask?.skillInfo) {
          plugin = {
            path: currentTask.skillInfo.pluginPath,
            skillName: currentTask.skillInfo.skillName,
            skillDescription: currentTask.skillInfo.skillDescription, // Use skill description as systemPrompt
          }
        }
      }

      // Get SDK session ID for conversation continuity
      let sdkSessionId: string | undefined
      if (session.sdkSessionId) {
        // Use session-level sdkSessionId (e.g., claudeapps mode)
        sdkSessionId = session.sdkSessionId
        console.log('[useSession] Session-level sdkSessionId:', sdkSessionId)
      } else if (session.mode === 'chat') {
        const chatHistory = useChatHistory()
        sdkSessionId = chatHistory.currentChat.value?.sdkSessionId
        console.log('[useSession] Chat mode - retrieved sdkSessionId:', sdkSessionId)
      } else if (session.mode === 'cowork') {
        sdkSessionId = tasksComposable.currentTask.value?.sdkSessionId
        console.log('[useSession] Cowork mode - retrieved sdkSessionId:', sdkSessionId)
      } else if (session.mode === 'code') {
        const workspace = useWorkspace()
        sdkSessionId = workspace.getProjectSdkSessionId(session.workDir)
        console.log('[useSession] Code mode - retrieved sdkSessionId:', sdkSessionId)
      }
      console.log('[useSession] Will send with sdkSessionId:', sdkSessionId || '(none - new session)')

      // Resolve model name: for custom providers, use the actual model name instead of provider ID
      let resolvedModel = session.model
      const coworksModelConfig = useSettingsStore().coworksModelConfig
      if (coworksModelConfig?.providers?.length) {
        const matchedProvider = coworksModelConfig.providers.find(p => p.id === session.model)
        if (matchedProvider) {
          resolvedModel = matchedProvider.model
        }
      }

      // Send start message (sidecar expects 'start' with prompt and mode)
      const startMsg = JSON.stringify({
        type: 'start',
        sessionId: assistantMessage.id,
        workDir: effectiveWorkDir,
        model: resolvedModel,
        prompt: content.trim(),
        mode: session.mode,
        plugin,
        attachments: attachments || [],
        sdkSessionId,
        ...(session.systemPrompt ? { systemPrompt: session.systemPrompt } : {}),
      })
      await invoke('coworks_send_to_sidecar', {
        sessionId: session.id,
        message: startMsg,
      })
    } catch (e: unknown) {
      console.error('[useSession] Error:', e)
      session.error = e instanceof Error ? e.message : 'Failed to send message'
      session.isLoading = false

      // 清理流式状态
      streamingStates.delete(assistantMessage.id)

      // Remove placeholder message on error
      const idx = session.messages.findIndex((m) => m.id === assistantMessage.id)
      if (idx !== -1) {
        session.messages.splice(idx, 1)
      }
    }
  }

  // Handle sidecar output
  function handleSidecarOutput(output: SidecarOutput, session: Session) {
    // 处理无需 sessionId 的消息类型
    switch (output.type) {
      case 'ready':
        session.initialized = true
        return

      case 'pong':
        return

      case 'session_started':
        // Save SDK session ID for conversation continuity
        // Guard: sidecar first emits an empty sdkSessionId (watchdog keepalive),
        // then emits the real one. Only save when non-empty to avoid clearing a
        // previously persisted session ID.
        console.log('[useSession] Received session_started:', output.sdkSessionId, 'mode:', session.mode)
        if (output.sdkSessionId) {
          if (session.onSessionStarted) {
            session.onSessionStarted(output.sdkSessionId)
            console.log('[useSession] Called onSessionStarted callback')
          } else if (session.mode === 'chat') {
            const chatHistory = useChatHistory()
            chatHistory.setSdkSessionId(output.sdkSessionId)
            console.log('[useSession] Saved sdkSessionId to chatHistory')
          } else if (session.mode === 'cowork') {
            const tasksComposable = useTasks()
            tasksComposable.setSdkSessionId(output.sdkSessionId)
            console.log('[useSession] Saved sdkSessionId to task')
          } else if (session.mode === 'code') {
            const workspace = useWorkspace()
            workspace.setProjectSdkSessionId(session.workDir, output.sdkSessionId)
            console.log('[useSession] Saved sdkSessionId to project:', session.workDir)
          }
        }
        return

      case 'progress':
        return
    }

    // 以下消息类型需要 sessionId 来定位消息
    const messageId = output.sessionId
    if (!messageId) {
      console.warn('[useSession] Missing sessionId in output:', output.type)
      return
    }

    // 通过 sessionId（即 messageId）查找消息
    const messageIndex = session.messages.findIndex((m) => m.id === messageId)
    if (messageIndex === -1) {
      console.warn('[useSession] Message not found:', messageId, 'in session', session.id)
      return
    }

    const message = session.messages[messageIndex]

    // 获取或创建该消息的流式状态
    let state = streamingStates.get(messageId)
    if (!state) {
      state = { content: '', toolUsages: [] }
      streamingStates.set(messageId, state)
    }

    switch (output.type) {
      case 'text_delta':
        state.content += output.content || ''
        session.messages[messageIndex] = {
          ...message,
          content: state.content,
        }
        // Extract file path from text content
        const filePath = extractFilePath(state.content)
        if (filePath) {
          const fileName = filePath.split(/[/\\]/).pop() || filePath
          generatedFile.value = {
            path: filePath,
            name: fileName,
            timestamp: Date.now(),
          }
        }
        break

      case 'tool_use':
        const toolUsage: ToolUsage = {
          id: output.toolId!,
          toolName: output.toolName!,
          toolId: output.toolId!,
          input: output.input,
          status: 'running',
        }
        state.toolUsages.push(toolUsage)
        session.messages[messageIndex] = {
          ...message,
          toolUsages: [...state.toolUsages],
        }
        break

      case 'tool_result':
        const existingTool = state.toolUsages.find((t) => t.toolId === output.toolId)
        if (existingTool) {
          existingTool.summary = output.summary
          existingTool.status = 'completed'
          session.messages[messageIndex] = {
            ...session.messages[messageIndex],
            toolUsages: [...state.toolUsages],
          }
          // Track file-modifying tool completions for auto-refresh
          const fileModifyingTools = ['Write', 'Edit', 'Bash', 'NotebookEdit']
          if (fileModifyingTools.includes(existingTool.toolName)) {
            lastFileChange.value = Date.now()
          }
        }
        break

      case 'done':
        session.isLoading = false
        streamingStates.delete(messageId)
        if (session.onComplete) session.onComplete()
        break

      case 'error':
        session.error = output.message || 'Unknown error'
        session.isLoading = false
        streamingStates.delete(messageId)
        if (session.onComplete) session.onComplete()
        break

      case 'cancelled':
        session.isLoading = false
        streamingStates.delete(messageId)
        if (session.onComplete) session.onComplete()
        break
    }
  }

  // Cancel current request (or a specific session by key)
  async function cancelRequest(targetSessionKey?: string) {
    const session = targetSessionKey
      ? sessions.value.get(targetSessionKey) ?? null
      : currentSession.value
    if (!session?.sidecarActive) return

    // Get the last assistant message's ID as the sessionId to cancel
    const lastAssistantMsg = [...session.messages].reverse().find(m => m.role === 'assistant')
    const cancelSessionId = lastAssistantMsg?.id || session.id

    try {
      await invoke('coworks_send_to_sidecar', {
        sessionId: session.id,
        message: JSON.stringify({ type: 'cancel', sessionId: cancelSessionId }),
      })
    } catch (e) {
      console.error('Failed to cancel:', e)
    }
    session.isLoading = false
  }

  // Clear messages in current session
  function clearMessages() {
    const session = currentSession.value
    if (session) {
      session.messages = []
      session.error = null
    }
  }

  // Close a session and its sidecar
  async function closeSession(mode: AppMode, contextId: string) {
    const key = getSessionKey(mode, contextId)
    const session = sessions.value.get(key)

    if (session) {
      // Cleanup event listeners
      if (session.unlistenOutput) {
        session.unlistenOutput()
      }
      if (session.unlistenReady) {
        session.unlistenReady()
      }
      if (session.unlistenStderr) {
        session.unlistenStderr()
      }

      if (session.sidecarActive) {
        try {
          await invoke('coworks_send_to_sidecar', {
            sessionId: session.id,
            message: JSON.stringify({ type: 'close' }),
          })
          await invoke('coworks_kill_sidecar', { sessionId: session.id })
        } catch (e) {
          console.error('Failed to close sidecar:', e)
        }
      }
    }

    sessions.value.delete(key)

    if (currentSessionKey.value === key) {
      currentSessionKey.value = null
    }
  }

  // Close all sessions (called when app closes)
  async function closeAllSessions() {
    for (const [, session] of sessions.value) {
      // Cleanup event listeners
      if (session.unlistenOutput) {
        session.unlistenOutput()
      }
      if (session.unlistenReady) {
        session.unlistenReady()
      }
      if (session.unlistenStderr) {
        session.unlistenStderr()
      }

      if (session.sidecarActive) {
        try {
          await invoke('coworks_kill_sidecar', { sessionId: session.id })
        } catch (e) {
          console.error('Failed to kill sidecar:', e)
        }
      }
    }
    sessions.value.clear()
    currentSessionKey.value = null
  }

  // Close sessions for specific modes only (preserves sessions of other modes)
  async function closeSessionsByMode(modes: AppMode[]) {
    const modeSet = new Set(modes)
    for (const [key, s] of sessions.value) {
      if (!modeSet.has(s.mode)) continue

      if (s.unlistenOutput) s.unlistenOutput()
      if (s.unlistenReady) s.unlistenReady()
      if (s.unlistenStderr) s.unlistenStderr()

      if (s.sidecarActive) {
        try {
          await invoke('coworks_kill_sidecar', { sessionId: s.id })
        } catch (e) {
          console.error('Failed to kill sidecar:', e)
        }
      }

      sessions.value.delete(key)
    }

    // Clear currentSessionKey only if the pointed session was deleted
    if (currentSessionKey.value && !sessions.value.has(currentSessionKey.value)) {
      currentSessionKey.value = null
    }
  }

  return {
    sessions,
    currentSession,
    lastFileChange,
    generatedFile,
    getOrCreateSession,
    switchToSession,
    clearCurrentSession,
    setMessages,
    sendMessage,
    cancelRequest,
    clearMessages,
    closeSession,
    closeAllSessions,
    closeSessionsByMode,
  }
}
