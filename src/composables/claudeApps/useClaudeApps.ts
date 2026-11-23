/**
 * Claude Apps facade composable
 * Orchestrates app list, conversation history, and sidecar sessions
 *
 * Model: one app = one conversation (for developing/maintaining that app)
 * Each app has a per-app system prompt derived from its manifest
 */

import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { homeDir, join } from '@tauri-apps/api/path'
import { useSession } from '@/composables/coworks/useSession'
import { useClaudeAppsHistory } from './useClaudeAppsHistory'
import { useDesktopStore } from '@/stores/desktopStore'
import type { DynamicFunctionInfo } from '@/stores/dynamicFunctionStore'
import type { ChatMessage } from '@/types/coworks'
import type { Attachment } from '@/components/coworks/ChatPanel.vue'

// Cache for dynamic-functions base dir
let cachedDfBaseDir: string | null = null

async function getDfBaseDir(): Promise<string> {
  if (cachedDfBaseDir) return cachedDfBaseDir
  const home = await homeDir()
  cachedDfBaseDir = await join(home, '.MoontonClaudeDesktop', 'dynamic-functions')
  return cachedDfBaseDir
}

/** Build a per-app system prompt from app manifest info */
function buildAppSystemPrompt(app: DynamicFunctionInfo): string {
  return `你是一个 Claude Apps 开发助手，当前正在维护应用「${app.displayName}」。

当前应用信息：
- 名称: ${app.name}
- 显示名: ${app.displayName}
- 版本: ${app.version}
- 描述: ${app.description || '无'}
- 图标: ${app.icon}
- 后端: ${app.backendEnabled ? '已启用' : '未启用'}
- 路径: ${app.path}

应用文件结构：
- manifest.json: 应用元数据
- frontend/index.html: 前端入口
- frontend/app.js: 前端逻辑
- frontend/style.css: 样式
${app.backendEnabled ? '- backend/server.js: Node.js HTTP 服务（读取 PORT 和 HOST 环境变量）' : ''}

iframe 内可通过 window.mcd 对象调用宿主 API：
${app.backendEnabled ? '- mcd.backend(path, options) - 请求后端 API\n' : ''}- mcd.invoke('mcd:readFile', {path}) - 读取函数目录内文件
- mcd.invoke('mcd:writeFile', {path, content}) - 写入函数目录内文件
- mcd.setTitle(title) - 设置窗口标题
- mcd.notify(message, level) - 显示通知
- mcd.openExternal(url) - 在系统默认浏览器中打开URL（仅 http/https）
- mcd.exec(command, args, options) - 执行系统命令（需 manifest 声明 "exec": true）
  command: 可执行程序名（如 'python'）
  args: 参数数组（如 ['script.py', '--flag']）
  options: { cwd?: string, env?: Record<string,string>, background?: boolean }
  同步模式（默认）返回 { stdout, stderr, exitCode }
  后台模式（background:true）返回 { pid }
- mcd.kill(pid) - 终止 mcd.exec 启动的后台进程（需 exec capability）

请根据用户的需求修改和完善此应用。工作目录已设为应用根目录，可直接读写文件。

【重要】请始终使用中文回复用户，包括所有解释、说明和输出内容。`
}

/** System prompt for creating a brand new app */
function buildNewAppSystemPrompt(dfBaseDir: string): string {
  return `你是一个 Claude Apps 开发助手，帮助用户创建全新的应用。

请在当前工作目录（${dfBaseDir}）下创建应用。每个应用是一个子目录，包含：

manifest.json 模板：
{
  "name": "app-name",
  "displayName": "应用显示名",
  "version": "1.0.0",
  "description": "应用描述",
  "icon": "fa-solid fa-cube",
  "window": { "width": 800, "height": 600, "minWidth": 400, "minHeight": 300 },
  "capabilities": { "network": false, "localStorage": false, "exec": false },
  "backend": { "enabled": false }
}

默认创建纯前端应用，不要生成 backend 目录。

文件结构：
- {app-name}/manifest.json: 应用元数据
- {app-name}/frontend/index.html: 前端入口
- {app-name}/frontend/app.js: 前端逻辑
- {app-name}/frontend/style.css: 样式

iframe 内可通过 window.mcd 对象调用宿主 API：
- mcd.invoke('mcd:readFile', {path}) - 读取函数目录内文件
- mcd.invoke('mcd:writeFile', {path, content}) - 写入函数目录内文件
- mcd.setTitle(title) - 设置窗口标题
- mcd.notify(message, level) - 显示通知
- mcd.openExternal(url) - 在系统默认浏览器中打开URL（仅 http/https）
- mcd.exec(command, args, options) - 执行系统命令（需 manifest 声明 "exec": true）
  command: 可执行程序名（如 'python'）
  args: 参数数组（如 ['script.py', '--flag']）
  options: { cwd?: string, env?: Record<string,string>, background?: boolean }
  同步模式（默认）返回 { stdout, stderr, exitCode }
  后台模式（background:true）返回 { pid }
- mcd.kill(pid) - 终止 mcd.exec 启动的后台进程（需 exec capability）

请根据用户的描述生成完整、可运行的应用代码。目录名用英文小写加连字符。

【重要】请始终使用中文回复用户，包括所有解释、说明和输出内容。`
}

// Singleton state
const apps = ref<DynamicFunctionInfo[]>([])
const selectedAppName = ref<string | null>(null)
const model = ref<string>('auto')
const initialized = ref(false)

export function useClaudeApps() {
  const session = useSession()
  const history = useClaudeAppsHistory()
  const desktopStore = useDesktopStore()

  // Snapshot of apps before sending (for detecting new apps)
  let appsBeforeSend: string[] = []

  // Claude Apps own session reference — isolates from Coworks' currentSessionKey changes
  const appSession = computed(() => {
    const convId = history.currentConversationId.value
    if (!convId) return null
    return session.sessions.value.get(`claudeapps::${convId}`) ?? null
  })

  // Messages from the Claude Apps session (not affected by Coworks mode switch)
  const messages = computed<ChatMessage[]>(() => {
    return appSession.value?.messages || []
  })

  const isLoading = computed(() => {
    return appSession.value?.isLoading || false
  })

  const error = computed({
    get: () => appSession.value?.error || null,
    set: (value: string | null) => {
      if (appSession.value) {
        appSession.value.error = value
      }
    },
  })

  const selectedApp = computed(() => {
    if (!selectedAppName.value) return null
    return apps.value.find(a => a.name === selectedAppName.value) ?? null
  })

  async function initialize() {
    if (initialized.value) return
    await loadApps()
    await history.initialize()
    initialized.value = true
  }

  async function loadApps() {
    try {
      apps.value = await invoke<DynamicFunctionInfo[]>('df_list_functions')
    } catch (e) {
      console.error('[useClaudeApps] Failed to list functions:', e)
    }
  }

  /** Select an existing app — load its single conversation */
  async function selectApp(appName: string) {
    selectedAppName.value = appName

    try {
      // Each app has at most one conversation
      const appConvs = history.getConversationsForApp(appName)
      if (appConvs.length > 0) {
        await loadConversation(appConvs[0].id)
      } else {
        // No conversation yet — create one for this app
        await createAppConversation(appName)
      }
    } catch (e) {
      console.error('[useClaudeApps] Failed to select app:', e)
      session.clearCurrentSession()
      history.clearCurrentConversation()
    }
  }

  /** Load an existing conversation into the session */
  async function loadConversation(conversationId: string) {
    const conv = await history.loadConversation(conversationId)
    if (!conv) return

    if (conv.appName) {
      selectedAppName.value = conv.appName
    }

    const workDir = await getWorkDir(conv.appName)
    const systemPrompt = await getSystemPrompt(conv.appName)

    const s = session.getOrCreateSession(
      'claudeapps',
      conv.id,
      model.value,
      workDir,
      false // Don't change global currentSessionKey
    )

    s.messages = conv.messages
    s.systemPrompt = systemPrompt

    if (conv.sdkSessionId) {
      s.sdkSessionId = conv.sdkSessionId
    }
    s.onSessionStarted = (sdkSessionId: string) => {
      history.setSdkSessionId(sdkSessionId)
      s.sdkSessionId = sdkSessionId
    }
  }

  /** Create a conversation for an existing app */
  async function createAppConversation(appName: string) {
    selectedAppName.value = appName
    const id = await history.createConversation(appName)

    const workDir = await getWorkDir(appName)
    const systemPrompt = await getSystemPrompt(appName)

    const s = session.getOrCreateSession(
      'claudeapps',
      id,
      model.value,
      workDir,
      false // Don't change global currentSessionKey
    )

    if (history.currentConversation.value) {
      s.messages = history.currentConversation.value.messages
    }
    s.systemPrompt = systemPrompt

    s.onSessionStarted = (sdkSessionId: string) => {
      history.setSdkSessionId(sdkSessionId)
      s.sdkSessionId = sdkSessionId
    }

    return id
  }

  /**
   * Create a new app conversation from a description (via dialog).
   * Creates an unlinked conversation, sends the description as the first message.
   * The app will be auto-linked once sidecar generates it.
   */
  async function createNewApp(description: string) {
    selectedAppName.value = null

    const baseDir = await getDfBaseDir()
    const systemPrompt = buildNewAppSystemPrompt(baseDir)

    const id = await history.createConversation(null)

    const s = session.getOrCreateSession(
      'claudeapps',
      id,
      model.value,
      baseDir,
      false // Don't change global currentSessionKey
    )

    if (history.currentConversation.value) {
      s.messages = history.currentConversation.value.messages
    }
    s.systemPrompt = systemPrompt

    s.onSessionStarted = (sdkSessionId: string) => {
      history.setSdkSessionId(sdkSessionId)
      s.sdkSessionId = sdkSessionId
    }

    // Snapshot and send the description as first message
    appsBeforeSend = apps.value.map(a => a.name)
    // Send directly to this session by key (avoids switching global currentSessionKey)
    const sessionKey = `claudeapps::${id}`
    await session.sendMessage(description, undefined, sessionKey)
  }

  /** Send a message in the current conversation */
  async function sendMessage(content: string, attachments?: Attachment[]) {
    const convId = history.currentConversationId.value
    if (!convId) {
      console.error('[useClaudeApps] No conversation loaded')
      return
    }

    appsBeforeSend = apps.value.map(a => a.name)

    // Send directly to this session by key (avoids switching global currentSessionKey)
    const sessionKey = `claudeapps::${convId}`
    const sessionAttachments = attachments?.map(a => ({ path: a.path, name: a.name }))
    await session.sendMessage(content, sessionAttachments, sessionKey)
  }

  // Watch isLoading to detect completion and check for new apps
  watch(isLoading, async (loading, wasLoading) => {
    if (wasLoading && !loading) {
      // Save conversation
      if (history.currentConversation.value && appSession.value) {
        history.currentConversation.value.messages = appSession.value.messages
        await history.saveCurrentConversation()
      }

      // Refresh apps list and detect new ones
      const prevNames = new Set(appsBeforeSend)
      await loadApps()

      const newApps = apps.value.filter(a => !prevNames.has(a.name))
      if (newApps.length > 0) {
        // Auto-link conversation to first new app if unlinked
        if (history.currentConversation.value && !history.currentConversation.value.appName) {
          history.linkToApp(newApps[0].name)
          selectedAppName.value = newApps[0].name

          // Update the session's workDir and systemPrompt for the newly linked app
          if (appSession.value) {
            const newWorkDir = await getWorkDir(newApps[0].name)
            const newPrompt = await getSystemPrompt(newApps[0].name)
            appSession.value.workDir = newWorkDir
            appSession.value.systemPrompt = newPrompt
            // workDir 已变更，旧 SDK session 无法跨目录 resume，清除以便下条消息创建新 session
            appSession.value.sdkSessionId = undefined
          }

          await history.saveCurrentConversation()
        }

        // Add desktop icons for new apps
        for (const newApp of newApps) {
          desktopStore.addDynamicFunction(
            newApp.name,
            newApp.displayName,
            newApp.icon,
            newApp.backendEnabled
          )
        }
      } else if (selectedApp.value) {
        // Existing app may have been updated — refresh its system prompt
        if (appSession.value) {
          appSession.value.systemPrompt = buildAppSystemPrompt(selectedApp.value)
        }
      }
    }
  })

  async function deleteApp(appName: string) {
    const app = apps.value.find(a => a.name === appName)
    const displayName = app?.displayName || appName
    if (!confirm(`确定要删除应用「${displayName}」吗？此操作不可撤销。`)) {
      return
    }

    try {
      // Close any open window for this app so the iframe releases file handles
      const { useWindowStore } = await import('@/stores/windowStore')
      const windowStore = useWindowStore()
      const appWindow = windowStore.windows.find(
        w => w.data?.functionName === appName
      )
      if (appWindow) {
        windowStore.closeWindow(appWindow.id)
        // Brief delay to let Vue tear down the iframe / release handles
        await new Promise(resolve => setTimeout(resolve, 200))
      }

      await invoke('df_delete_function', { name: appName })
      desktopStore.removeDynamicFunction(appName)

      const appConvs = history.getConversationsForApp(appName)
      for (const conv of appConvs) {
        await history.deleteConversation(conv.id)
      }

      if (selectedAppName.value === appName) {
        selectedAppName.value = null
        session.clearCurrentSession()
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      console.error('[useClaudeApps] Failed to delete app:', e)
      error.value = `删除应用失败: ${msg}`
    }

    await loadApps()
  }

  async function deleteConversation(conversationId: string) {
    const isCurrentConv = history.currentConversationId.value === conversationId
    await history.deleteConversation(conversationId)

    if (isCurrentConv) {
      session.clearCurrentSession()
    }
  }

  async function openApp(func: DynamicFunctionInfo) {
    const { useWindowStore } = await import('@/stores/windowStore')
    const windowStore = useWindowStore()

    const existingIcon = desktopStore.icons.find(
      i => i.type === 'dynamic-function' && i.dynamicFunctionInfo?.functionName === func.name
    )

    if (existingIcon) {
      windowStore.openWindow(existingIcon, {
        functionName: func.name,
        functionPath: func.path,
        backendPort: null,
      })
    } else {
      const result = desktopStore.addDynamicFunction(
        func.name,
        func.displayName,
        func.icon,
        func.backendEnabled
      )
      if (result.icon) {
        windowStore.openWindow(result.icon, {
          functionName: func.name,
          functionPath: func.path,
          backendPort: null,
        })
      }
    }
  }

  async function revealInExplorer(func: DynamicFunctionInfo) {
    try {
      await invoke('reveal_in_explorer', { path: func.path })
    } catch (e) {
      console.error('[useClaudeApps] Failed to reveal in explorer:', e)
    }
  }

  function cancelRequest() {
    const convId = history.currentConversationId.value
    if (!convId) return
    // Cancel directly by session key (avoids switching global currentSessionKey)
    const sessionKey = `claudeapps::${convId}`
    session.cancelRequest(sessionKey)
  }

  function clearError() {
    if (appSession.value) {
      appSession.value.error = null
    }
  }

  // Watch model changes
  watch(model, (newModel) => {
    if (appSession.value) {
      appSession.value.model = newModel
    }
  })

  // Helper: get workDir for an app (or base dir for new apps)
  async function getWorkDir(appName: string | null): Promise<string> {
    const baseDir = await getDfBaseDir()
    if (appName) {
      return await join(baseDir, appName)
    }
    return baseDir
  }

  // Helper: get system prompt for an app (or generic for new apps)
  async function getSystemPrompt(appName: string | null): Promise<string> {
    if (appName) {
      const app = apps.value.find(a => a.name === appName)
      if (app) {
        return buildAppSystemPrompt(app)
      }
    }
    const baseDir = await getDfBaseDir()
    return buildNewAppSystemPrompt(baseDir)
  }

  return {
    // State
    apps,
    selectedAppName,
    selectedApp,
    model,
    messages,
    isLoading,
    error,

    // History
    conversations: history.conversations,
    currentConversationId: history.currentConversationId,
    currentConversation: history.currentConversation,
    getConversationsForApp: history.getConversationsForApp,
    getUnlinkedConversations: history.getUnlinkedConversations,

    // Actions
    initialize,
    loadApps,
    selectApp,
    loadConversation,
    createNewApp,
    sendMessage,
    cancelRequest,
    clearError,
    deleteApp,
    deleteConversation,
    openApp,
    revealInExplorer,
  }
}
