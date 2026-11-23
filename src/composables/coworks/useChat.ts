/**
 * Chat composable for Coworks
 * Facade over session management for each mode
 * Handles chat state and session management
 */

import { ref, computed, watch } from 'vue'
import { useSession } from './useSession'
import { useChatHistory } from './useChatHistory'
import { useTasks } from './useTasks'
import { useWorkspace } from './useWorkspace'
import { useSettingsStore } from '@/stores/settingsStore'
import type { AppMode } from '@/types/coworks'
import type { TaskSkillInfo } from '@/types/coworks-persistence'

// Per-mode context state (preserved across mode switches)
interface ModeContext {
  contextId: string
  workDir: string
}

// Singleton state for mode contexts
const modeContexts = ref<Record<AppMode, ModeContext>>({
  chat: { contextId: '', workDir: '' },
  code: { contextId: '', workDir: '' },
  cowork: { contextId: '', workDir: '' },
  claudeapps: { contextId: '', workDir: '' },
})

// Singleton state for current mode and model
const currentMode = ref<AppMode>('chat')
const currentModel = ref<string>('auto')

export function useChat() {
  const session = useSession()
  const chatHistory = useChatHistory()
  const tasks = useTasks()
  const workspace = useWorkspace()

  // Use singleton mode and model
  const mode = currentMode
  const model = currentModel

  // Computed: context ID for current mode
  const contextId = computed({
    get: () => modeContexts.value[mode.value].contextId,
    set: (value: string) => {
      modeContexts.value[mode.value].contextId = value
    },
  })

  // Computed: work directory for current mode
  const workDir = computed({
    get: () => modeContexts.value[mode.value].workDir,
    set: (value: string) => {
      modeContexts.value[mode.value].workDir = value
    },
  })

  // Computed: messages from current session
  const messages = computed(() => {
    return session.currentSession.value?.messages || []
  })

  // Computed: loading state from current session
  const isLoading = computed(() => {
    return session.currentSession.value?.isLoading || false
  })

  // Computed: error from current session
  const error = computed({
    get: () => session.currentSession.value?.error || null,
    set: (value: string | null) => {
      if (session.currentSession.value) {
        session.currentSession.value.error = value
      }
    },
  })

  // Initialize composables
  async function initialize() {
    await chatHistory.initialize()
    await tasks.initialize()
    await workspace.initialize()

    // Apply default model from settings (if not already changed by user)
    const settingsStore = useSettingsStore()
    const configDefault = settingsStore.coworksModelConfig?.defaultModel
    if (configDefault && model.value === 'auto') {
      model.value = configDefault
    }
  }

  // Switch mode
  async function setMode(newMode: AppMode) {
    mode.value = newMode

    // Restore session for the mode if it has a context
    const modeContext = modeContexts.value[newMode]
    if (modeContext.contextId) {
      session.switchToSession(newMode, modeContext.contextId)
    } else {
      // Clear current session if no context for this mode
      session.clearCurrentSession()
    }
  }

  // ==========================================
  // Chat Mode Functions
  // ==========================================

  // Create new chat and set as current
  async function createNewChat() {
    const id = await chatHistory.createChat()
    contextId.value = id
    const s = session.getOrCreateSession('chat', id, model.value, '')
    // Link session messages to chatHistory messages (same array reference)
    const chatRef = chatHistory.currentChat.value
    if (chatRef) {
      s.messages = chatRef.messages
      // Register completion callback for background persistence
      s.onComplete = () => chatHistory.saveChatRef(chatRef)
    }
    return id
  }

  // Load existing chat
  async function loadChat(id: string) {
    const chat = await chatHistory.loadChat(id)
    if (chat) {
      contextId.value = id
      // Restore persisted model choice
      if (chat.model) {
        model.value = chat.model
      }
      const s = session.getOrCreateSession('chat', id, model.value, '')
      // Don't replace messages if session is actively streaming —
      // in-memory messages contain pending user + assistant that disk doesn't have
      if (!s.isLoading) {
        s.messages = chat.messages
        s.onComplete = () => chatHistory.saveChatRef(chat)
      }
    }
  }

  // Delete chat
  async function deleteChat(id: string) {
    await chatHistory.deleteChat(id)
    if (contextId.value === id) {
      contextId.value = ''
      session.clearMessages()
    }
  }

  // ==========================================
  // Code Mode Functions
  // ==========================================

  // Set project path for code mode
  async function setProject(projectPath: string) {
    contextId.value = projectPath
    workDir.value = projectPath
    // Restore persisted model choice for this project
    const savedModel = workspace.getProjectModel(projectPath)
    if (savedModel) {
      model.value = savedModel
    }
    session.getOrCreateSession('code', projectPath, model.value, projectPath)
    await workspace.setCurrentWorkspace(projectPath, 'code')
  }

  // ==========================================
  // Cowork Mode Functions
  // ==========================================

  // Create new task
  async function createNewTask(
    name: string,
    taskWorkDir: string,
    appId?: string,
    skillInfo?: TaskSkillInfo
  ) {
    const id = await tasks.createTask(name, taskWorkDir, appId, skillInfo)
    contextId.value = id
    workDir.value = taskWorkDir
    const s = session.getOrCreateSession('cowork', id, model.value, taskWorkDir)
    // Link session messages to task messages (same array reference)
    const taskRef = tasks.currentTask.value
    if (taskRef) {
      s.messages = taskRef.messages
      // Register completion callback for background persistence
      s.onComplete = () => tasks.saveTaskRef(taskRef)
    }
    await workspace.setCurrentWorkspace(taskWorkDir, 'cowork')
    return id
  }

  // Load existing task
  async function loadTask(id: string) {
    const task = await tasks.loadTask(id)
    if (task) {
      contextId.value = id
      workDir.value = task.workDir
      // Restore persisted model choice
      if (task.model) {
        model.value = task.model
      }
      const s = session.getOrCreateSession('cowork', id, model.value, task.workDir)
      // Don't replace messages if session is actively streaming
      if (!s.isLoading) {
        s.messages = task.messages
        s.onComplete = () => tasks.saveTaskRef(task)
      }
      await workspace.setCurrentWorkspace(task.workDir, 'cowork')
    }
  }

  // Delete task
  async function deleteTask(id: string) {
    await tasks.deleteTask(id)
    if (contextId.value === id) {
      contextId.value = ''
      workDir.value = ''
      session.clearMessages()
    }
  }

  // ==========================================
  // Common Functions
  // ==========================================

  // Send message
  async function sendMessage(content: string, attachments?: { path: string; name: string }[]) {
    // Ensure we have a context
    if (!contextId.value) {
      if (mode.value === 'chat') {
        await createNewChat()
      } else {
        error.value = '请先选择项目或创建任务'
        return
      }
    }

    await session.sendMessage(content, attachments)
    // Persistence is handled by session.onComplete callback when streaming completes
  }

  // Cancel current request
  async function cancelRequest() {
    await session.cancelRequest()
  }

  // Clear messages
  function clearMessages() {
    session.clearMessages()
  }

  // Clear error
  function clearError() {
    if (session.currentSession.value) {
      session.currentSession.value.error = null
    }
  }

  // Watch model changes - update current session and persist to context
  watch(model, (newModel) => {
    if (session.currentSession.value) {
      session.currentSession.value.model = newModel
    }
    // Persist model choice to current context
    if (mode.value === 'chat' && chatHistory.currentChat.value) {
      chatHistory.currentChat.value.model = newModel
    } else if (mode.value === 'cowork' && tasks.currentTask.value) {
      tasks.currentTask.value.model = newModel
    } else if (mode.value === 'code' && contextId.value) {
      workspace.setProjectModel(contextId.value, newModel)
    }
  })

  return {
    // State
    mode,
    model,
    contextId,
    workDir,
    messages,
    isLoading,
    error,

    // Chat mode
    chatHistories: chatHistory.chatHistories,
    currentChat: chatHistory.currentChat,
    createNewChat,
    loadChat,
    deleteChat,

    // Code mode
    recentProjects: workspace.recentProjects,

    // Cowork mode
    tasks: tasks.tasks,
    activeTasks: tasks.activeTasks,
    currentTask: tasks.currentTask,
    relatedFiles: tasks.relatedFiles,
    workLogs: tasks.workLogs,
    createNewTask,
    loadTask,
    deleteTask,
    getTaskDisplayName: tasks.getTaskDisplayName,

    // Workspace helpers
    recentWorkDirs: workspace.recentWorkDirs,
    getWorkspaceName: workspace.getWorkspaceName,

    // Actions
    initialize,
    setMode,
    setProject,
    sendMessage,
    cancelRequest,
    clearMessages,
    clearError,

    // Session management (only close chat/code/cowork sessions, not claudeapps)
    closeCoworksSessions: async () => {
      await session.closeSessionsByMode(['chat', 'code', 'cowork'])
    },

    // File change tracking (for file tree auto-refresh)
    lastFileChange: session.lastFileChange,
    // Generated file from text output
    generatedFile: session.generatedFile,
  }
}
