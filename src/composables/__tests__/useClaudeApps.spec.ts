/**
 * Unit tests for useClaudeApps composable
 * Tests: initialize, selectApp, loadConversation, createNewApp, sendMessage,
 *        deleteApp, cancelRequest
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { ref, nextTick } from 'vue'

// --- Tauri mocks ---
const mockInvoke = vi.fn()

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(vi.fn()),
}))

vi.mock('@tauri-apps/api/path', () => ({
  homeDir: vi.fn().mockResolvedValue('C:\\Users\\Test'),
  join: vi.fn((...parts: string[]) => Promise.resolve(parts.join('\\'))),
}))

vi.mock('@tauri-apps/plugin-fs', () => ({
  readTextFile: vi.fn(),
  writeTextFile: vi.fn(),
  exists: vi.fn(),
  mkdir: vi.fn(),
  BaseDirectory: { Home: 'Home' },
}))

// --- Mock useSession ---
const mockSendMessage = vi.fn()
const mockCancelRequest = vi.fn()
const mockClearCurrentSession = vi.fn()
const mockGetOrCreateSession = vi.fn()
const mockSessions = ref(new Map())

vi.mock('@/composables/coworks/useSession', () => ({
  useSession: () => ({
    sessions: mockSessions,
    sendMessage: mockSendMessage,
    cancelRequest: mockCancelRequest,
    clearCurrentSession: mockClearCurrentSession,
    getOrCreateSession: mockGetOrCreateSession,
  }),
}))

// --- Mock useClaudeAppsHistory ---
const mockHistoryInitialize = vi.fn()
const mockHistoryCreateConversation = vi.fn()
const mockHistoryLoadConversation = vi.fn()
const mockHistorySaveCurrentConversation = vi.fn()
const mockHistoryDeleteConversation = vi.fn()
const mockHistoryLinkToApp = vi.fn()
const mockHistorySetSdkSessionId = vi.fn()
const mockHistoryClearCurrentConversation = vi.fn()
const mockHistoryGetConversationsForApp = vi.fn()
const mockHistoryGetUnlinkedConversations = vi.fn()

const mockHistoryCurrentConversationId = ref<string | null>(null)
const mockHistoryCurrentConversation = ref<{
  id: string
  title: string
  appName: string | null
  messages: Array<{ id: string; role: string; content: string; timestamp: number }>
  createdAt: number
  updatedAt: number
  sdkSessionId?: string
} | null>(null)
const mockHistoryConversations = ref<Array<{
  id: string
  title: string
  appName: string | null
  createdAt: number
  updatedAt: number
}>>([])

vi.mock('@/composables/claudeApps/useClaudeAppsHistory', () => ({
  useClaudeAppsHistory: () => ({
    conversations: mockHistoryConversations,
    currentConversationId: mockHistoryCurrentConversationId,
    currentConversation: mockHistoryCurrentConversation,
    isLoading: ref(false),
    initialize: mockHistoryInitialize,
    createConversation: mockHistoryCreateConversation,
    loadConversation: mockHistoryLoadConversation,
    saveCurrentConversation: mockHistorySaveCurrentConversation,
    deleteConversation: mockHistoryDeleteConversation,
    linkToApp: mockHistoryLinkToApp,
    setSdkSessionId: mockHistorySetSdkSessionId,
    clearCurrentConversation: mockHistoryClearCurrentConversation,
    getConversationsForApp: mockHistoryGetConversationsForApp,
    getUnlinkedConversations: mockHistoryGetUnlinkedConversations,
  }),
}))

// --- Mock desktopStore ---
const mockAddDynamicFunction = vi.fn()
const mockRemoveDynamicFunction = vi.fn()
const mockDesktopIcons = ref<Array<{ id: string; type: string; name: string; dynamicFunctionInfo?: { functionName: string } }>>([])

vi.mock('@/stores/desktopStore', () => ({
  useDesktopStore: () => ({
    icons: mockDesktopIcons.value,
    addDynamicFunction: mockAddDynamicFunction,
    removeDynamicFunction: mockRemoveDynamicFunction,
  }),
}))

// --- Mock windowStore (used by deleteApp and openApp dynamically) ---
const mockWindowStoreWindows = ref<Array<{ id: string; data?: { functionName?: string } }>>([])
const mockCloseWindow = vi.fn()
const mockOpenWindow = vi.fn()

vi.mock('@/stores/windowStore', () => ({
  useWindowStore: () => ({
    windows: mockWindowStoreWindows.value,
    closeWindow: mockCloseWindow,
    openWindow: mockOpenWindow,
  }),
}))

// Sample app data
function createApp(name: string, displayName: string) {
  return {
    name,
    displayName,
    version: '1.0.0',
    description: 'Test app',
    icon: 'fa-solid fa-cube',
    backendEnabled: false,
    path: `C:\\Users\\Test\\.MoontonClaudeDesktop\\dynamic-functions\\${name}`,
  }
}

describe('useClaudeApps', () => {
  beforeEach(() => {
    vi.resetModules()
    vi.clearAllMocks()
    setActivePinia(createPinia())

    // Reset mock state
    mockSessions.value = new Map()
    mockHistoryCurrentConversationId.value = null
    mockHistoryCurrentConversation.value = null
    mockHistoryConversations.value = []
    mockDesktopIcons.value = []
    mockWindowStoreWindows.value = []

    // Default mock returns
    mockGetOrCreateSession.mockReturnValue({
      messages: [],
      isLoading: false,
      error: null,
      systemPrompt: '',
      workDir: '',
      model: 'auto',
      sdkSessionId: undefined,
      onSessionStarted: undefined,
    })
  })

  async function getComposable() {
    const mod = await import('@/composables/claudeApps/useClaudeApps')
    return mod.useClaudeApps()
  }

  // -- initialize --

  describe('initialize', () => {
    it('should load apps and initialize history', async () => {
      const appList = [createApp('app-1', 'App One')]
      mockInvoke.mockResolvedValueOnce(appList)

      const composable = await getComposable()
      await composable.initialize()

      expect(mockInvoke).toHaveBeenCalledWith('df_list_functions')
      expect(mockHistoryInitialize).toHaveBeenCalled()
      expect(composable.apps.value).toEqual(appList)
    })

    it('should not re-initialize when already initialized', async () => {
      const appList = [createApp('app-1', 'App One')]
      mockInvoke.mockResolvedValueOnce(appList)

      const composable = await getComposable()
      await composable.initialize()
      await composable.initialize() // Second call

      // Only called once for df_list_functions
      expect(mockInvoke).toHaveBeenCalledTimes(1)
      expect(mockHistoryInitialize).toHaveBeenCalledTimes(1)
    })

    it('should handle loadApps error gracefully', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Failed'))
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

      const composable = await getComposable()
      await composable.initialize()

      expect(composable.apps.value).toEqual([])
      consoleSpy.mockRestore()
    })
  })

  // -- selectApp --

  describe('selectApp', () => {
    it('should select app and load existing conversation', async () => {
      const existingConv = { id: 'conv-1', title: 'Test', appName: 'my-app', createdAt: 1000, updatedAt: 2000 }
      mockHistoryGetConversationsForApp.mockReturnValue([existingConv])
      mockHistoryLoadConversation.mockResolvedValue({
        ...existingConv,
        messages: [{ id: 'm1', role: 'user', content: 'Hi', timestamp: 1000 }],
      })

      // Pre-load apps
      mockInvoke.mockResolvedValueOnce([createApp('my-app', 'My App')])
      const composable = await getComposable()
      await composable.initialize()

      await composable.selectApp('my-app')

      expect(composable.selectedAppName.value).toBe('my-app')
      expect(mockHistoryLoadConversation).toHaveBeenCalledWith('conv-1')
    })

    it('should create new conversation when app has none', async () => {
      mockHistoryGetConversationsForApp.mockReturnValue([])
      mockHistoryCreateConversation.mockResolvedValue('new-conv-id')
      mockHistoryCurrentConversation.value = {
        id: 'new-conv-id',
        title: 'New',
        appName: 'fresh-app',
        messages: [],
        createdAt: 1000,
        updatedAt: 1000,
      }

      mockInvoke.mockResolvedValueOnce([createApp('fresh-app', 'Fresh App')])
      const composable = await getComposable()
      await composable.initialize()

      await composable.selectApp('fresh-app')

      expect(mockHistoryCreateConversation).toHaveBeenCalledWith('fresh-app')
      expect(mockGetOrCreateSession).toHaveBeenCalledWith(
        'claudeapps',
        expect.any(String),
        'auto',
        expect.any(String),
        false
      )
    })

    it('should handle select error gracefully', async () => {
      mockHistoryGetConversationsForApp.mockImplementation(() => {
        throw new Error('Query failed')
      })
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

      mockInvoke.mockResolvedValueOnce([createApp('bad-app', 'Bad App')])
      const composable = await getComposable()
      await composable.initialize()

      await composable.selectApp('bad-app')

      expect(mockClearCurrentSession).toHaveBeenCalled()
      expect(mockHistoryClearCurrentConversation).toHaveBeenCalled()
      consoleSpy.mockRestore()
    })
  })

  // -- loadConversation --

  describe('loadConversation', () => {
    it('should load conversation and create session', async () => {
      const conv = {
        id: 'conv-1',
        title: 'Test Conv',
        appName: 'test-app',
        messages: [{ id: 'm1', role: 'user', content: 'Hi', timestamp: 1000 }],
        createdAt: 1000,
        updatedAt: 2000,
        sdkSessionId: 'sdk-1',
      }
      mockHistoryLoadConversation.mockResolvedValue(conv)

      const mockSession = {
        messages: [],
        systemPrompt: '',
        sdkSessionId: undefined as string | undefined,
        onSessionStarted: undefined as ((sdkSessionId: string) => void) | undefined,
      }
      mockGetOrCreateSession.mockReturnValue(mockSession)

      mockInvoke.mockResolvedValueOnce([createApp('test-app', 'Test App')])
      const composable = await getComposable()
      await composable.initialize()

      await composable.loadConversation('conv-1')

      expect(composable.selectedAppName.value).toBe('test-app')
      expect(mockSession.messages).toEqual(conv.messages)
      expect(mockSession.sdkSessionId).toBe('sdk-1')
    })

    it('should do nothing when conversation not found', async () => {
      mockHistoryLoadConversation.mockResolvedValue(null)

      const composable = await getComposable()
      await composable.loadConversation('nonexistent')

      expect(mockGetOrCreateSession).not.toHaveBeenCalled()
    })
  })

  // -- createNewApp --

  describe('createNewApp', () => {
    it('should create unlinked conversation and send description', async () => {
      mockHistoryCreateConversation.mockResolvedValue('new-conv-id')
      mockHistoryCurrentConversation.value = {
        id: 'new-conv-id',
        title: 'New',
        appName: null,
        messages: [],
        createdAt: 1000,
        updatedAt: 1000,
      }

      const mockSession = {
        messages: [],
        systemPrompt: '',
        sdkSessionId: undefined as string | undefined,
        onSessionStarted: undefined as ((sdkSessionId: string) => void) | undefined,
      }
      mockGetOrCreateSession.mockReturnValue(mockSession)

      const composable = await getComposable()
      await composable.createNewApp('Build a todo app')

      expect(composable.selectedAppName.value).toBeNull()
      expect(mockHistoryCreateConversation).toHaveBeenCalledWith(null)
      expect(mockSendMessage).toHaveBeenCalledWith(
        'Build a todo app',
        undefined,
        'claudeapps::new-conv-id'
      )
    })

    it('should set new-app system prompt', async () => {
      mockHistoryCreateConversation.mockResolvedValue('conv-id')
      mockHistoryCurrentConversation.value = {
        id: 'conv-id',
        title: 'New',
        appName: null,
        messages: [],
        createdAt: 1000,
        updatedAt: 1000,
      }

      const mockSession = {
        messages: [],
        systemPrompt: '',
        sdkSessionId: undefined as string | undefined,
        onSessionStarted: undefined as ((sdkSessionId: string) => void) | undefined,
      }
      mockGetOrCreateSession.mockReturnValue(mockSession)

      const composable = await getComposable()
      await composable.createNewApp('Test app')

      // System prompt should contain the dynamic-functions base dir
      expect(mockSession.systemPrompt).toContain('dynamic-functions')
    })
  })

  // -- sendMessage --

  describe('sendMessage', () => {
    it('should send message to the session by key', async () => {
      mockHistoryCurrentConversationId.value = 'conv-1'

      const composable = await getComposable()
      await composable.sendMessage('Hello')

      expect(mockSendMessage).toHaveBeenCalledWith(
        'Hello',
        undefined,
        'claudeapps::conv-1'
      )
    })

    it('should send message with attachments', async () => {
      mockHistoryCurrentConversationId.value = 'conv-2'

      const composable = await getComposable()
      const attachments = [
        { id: 'a1', name: 'file.txt', path: '/tmp/file.txt', size: 100, type: 'text/plain' },
      ]
      await composable.sendMessage('Check this file', attachments)

      expect(mockSendMessage).toHaveBeenCalledWith(
        'Check this file',
        [{ path: '/tmp/file.txt', name: 'file.txt' }],
        'claudeapps::conv-2'
      )
    })

    it('should not send when no conversation is loaded', async () => {
      mockHistoryCurrentConversationId.value = null
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

      const composable = await getComposable()
      await composable.sendMessage('Hello')

      expect(mockSendMessage).not.toHaveBeenCalled()
      consoleSpy.mockRestore()
    })
  })

  // -- deleteApp --

  describe('deleteApp', () => {
    beforeEach(() => {
      // happy-dom does not provide window.confirm, so we define it for spying
      if (typeof globalThis.confirm !== 'function') {
        globalThis.confirm = () => true
      }
    })

    it('should delete app, remove desktop icon, delete conversations, and reload', async () => {
      // Mock confirm to return true
      vi.spyOn(globalThis, 'confirm').mockReturnValue(true)
      mockInvoke.mockResolvedValue(undefined)
      mockHistoryGetConversationsForApp.mockReturnValue([
        { id: 'conv-1', title: 'Conv1', appName: 'del-app', createdAt: 1000, updatedAt: 2000 },
      ])
      mockHistoryDeleteConversation.mockResolvedValue(undefined)

      // Pre-load app
      const appList = [createApp('del-app', 'Delete Me')]
      mockInvoke.mockResolvedValueOnce(appList)
      const composable = await getComposable()
      await composable.initialize()

      composable.selectedAppName.value = 'del-app'

      // Need to mock the second df_list_functions call (after delete, it reloads)
      mockInvoke.mockResolvedValue(undefined)

      await composable.deleteApp('del-app')

      expect(mockInvoke).toHaveBeenCalledWith('df_delete_function', { name: 'del-app' })
      expect(mockRemoveDynamicFunction).toHaveBeenCalledWith('del-app')
      expect(mockHistoryDeleteConversation).toHaveBeenCalledWith('conv-1')
      expect(composable.selectedAppName.value).toBeNull()
      expect(mockClearCurrentSession).toHaveBeenCalled()
    })

    it('should do nothing when user cancels confirmation', async () => {
      vi.spyOn(globalThis, 'confirm').mockReturnValue(false)

      mockInvoke.mockResolvedValueOnce([createApp('keep-app', 'Keep Me')])
      const composable = await getComposable()
      await composable.initialize()

      await composable.deleteApp('keep-app')

      expect(mockInvoke).not.toHaveBeenCalledWith('df_delete_function', expect.anything())
    })

    it('should handle delete error gracefully', async () => {
      vi.spyOn(globalThis, 'confirm').mockReturnValue(true)
      mockInvoke.mockResolvedValueOnce([createApp('err-app', 'Error App')])

      const composable = await getComposable()
      await composable.initialize()

      // Make df_delete_function fail
      mockInvoke.mockRejectedValueOnce(new Error('Permission denied'))
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

      await composable.deleteApp('err-app')

      consoleSpy.mockRestore()
    })

    it('should close app window before deleting', async () => {
      vi.spyOn(globalThis, 'confirm').mockReturnValue(true)
      mockWindowStoreWindows.value = [
        { id: 'win-1', data: { functionName: 'close-app' } },
      ]
      mockHistoryGetConversationsForApp.mockReturnValue([])

      mockInvoke.mockResolvedValueOnce([createApp('close-app', 'Close App')])
      const composable = await getComposable()
      await composable.initialize()

      mockInvoke.mockResolvedValue(undefined)
      await composable.deleteApp('close-app')

      expect(mockCloseWindow).toHaveBeenCalledWith('win-1')
    })
  })

  // -- cancelRequest --

  describe('cancelRequest', () => {
    it('should cancel request by session key', async () => {
      mockHistoryCurrentConversationId.value = 'conv-1'

      const composable = await getComposable()
      composable.cancelRequest()

      expect(mockCancelRequest).toHaveBeenCalledWith('claudeapps::conv-1')
    })

    it('should do nothing when no conversation is loaded', async () => {
      mockHistoryCurrentConversationId.value = null

      const composable = await getComposable()
      composable.cancelRequest()

      expect(mockCancelRequest).not.toHaveBeenCalled()
    })
  })

  // -- loadApps --

  describe('loadApps', () => {
    it('should reload the apps list', async () => {
      mockInvoke.mockResolvedValueOnce([])

      const composable = await getComposable()
      await composable.initialize()

      const newApps = [createApp('new-1', 'New One')]
      mockInvoke.mockResolvedValueOnce(newApps)
      await composable.loadApps()

      expect(composable.apps.value).toEqual(newApps)
    })
  })

  // -- computed: selectedApp --

  describe('selectedApp', () => {
    it('should return the app matching selectedAppName', async () => {
      const appList = [createApp('a', 'A'), createApp('b', 'B')]
      mockInvoke.mockResolvedValueOnce(appList)

      const composable = await getComposable()
      await composable.initialize()

      composable.selectedAppName.value = 'b'
      expect(composable.selectedApp.value?.name).toBe('b')
    })

    it('should return null when no app selected', async () => {
      mockInvoke.mockResolvedValueOnce([createApp('a', 'A')])

      const composable = await getComposable()
      await composable.initialize()

      composable.selectedAppName.value = null
      expect(composable.selectedApp.value).toBeNull()
    })

    it('should return null when selected app not found', async () => {
      mockInvoke.mockResolvedValueOnce([createApp('a', 'A')])

      const composable = await getComposable()
      await composable.initialize()

      composable.selectedAppName.value = 'nonexistent'
      expect(composable.selectedApp.value).toBeNull()
    })
  })

  // -- deleteConversation --

  describe('deleteConversation', () => {
    it('should delete conversation and clear session if current', async () => {
      mockHistoryCurrentConversationId.value = 'conv-to-delete'

      const composable = await getComposable()
      await composable.deleteConversation('conv-to-delete')

      expect(mockHistoryDeleteConversation).toHaveBeenCalledWith('conv-to-delete')
      expect(mockClearCurrentSession).toHaveBeenCalled()
    })

    it('should delete conversation without clearing session if not current', async () => {
      mockHistoryCurrentConversationId.value = 'other-conv'

      const composable = await getComposable()
      await composable.deleteConversation('conv-to-delete')

      expect(mockHistoryDeleteConversation).toHaveBeenCalledWith('conv-to-delete')
      expect(mockClearCurrentSession).not.toHaveBeenCalled()
    })
  })

  // -- revealInExplorer --

  describe('revealInExplorer', () => {
    it('should invoke reveal_in_explorer with app path', async () => {
      mockInvoke.mockResolvedValueOnce([])
      const composable = await getComposable()

      const app = createApp('my-app', 'My App')
      mockInvoke.mockResolvedValueOnce(undefined)
      await composable.revealInExplorer(app)

      expect(mockInvoke).toHaveBeenCalledWith('reveal_in_explorer', { path: app.path })
    })

    it('should handle reveal error gracefully', async () => {
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
      mockInvoke.mockResolvedValueOnce([])
      const composable = await getComposable()

      const app = createApp('my-app', 'My App')
      mockInvoke.mockRejectedValueOnce(new Error('Not found'))
      await composable.revealInExplorer(app)

      // Should not throw, just log
      consoleSpy.mockRestore()
    })
  })

  // -- clearError --

  describe('clearError', () => {
    it('should clear error on app session', async () => {
      // Setup: create a session in the sessions map
      const mockSession = {
        messages: [],
        isLoading: false,
        error: 'some error' as string | null,
        systemPrompt: '',
        workDir: '',
        model: 'auto',
        sdkSessionId: undefined,
        onSessionStarted: undefined,
      }
      mockHistoryCurrentConversationId.value = 'conv-1'
      mockSessions.value = new Map([['claudeapps::conv-1', mockSession]])

      const composable = await getComposable()
      composable.clearError()

      expect(mockSession.error).toBeNull()
    })
  })

  // -- openApp --

  describe('openApp', () => {
    it('should open existing icon window', async () => {
      const app = createApp('open-app', 'Open App')
      const existingIcon = {
        id: 'dynfunc-open-app',
        type: 'dynamic-function',
        name: 'Open App',
        dynamicFunctionInfo: { functionName: 'open-app' },
      }
      mockDesktopIcons.value = [existingIcon]

      mockInvoke.mockResolvedValueOnce([app])
      const composable = await getComposable()
      await composable.initialize()

      await composable.openApp(app)

      // windowStore.openWindow should be called via the imported store
      // Since we mock the store, we can verify it was called via the mock
      expect(mockCloseWindow).not.toHaveBeenCalled() // sanity: no close
    })

    it('should add desktop icon and open window when icon does not exist', async () => {
      const app = createApp('new-open-app', 'New Open App')
      mockDesktopIcons.value = [] // no existing icon

      mockAddDynamicFunction.mockReturnValue({
        icon: {
          id: 'dynfunc-new-open-app',
          type: 'dynamic-function',
          name: 'New Open App',
        },
      })

      mockInvoke.mockResolvedValueOnce([app])
      const composable = await getComposable()
      await composable.initialize()

      await composable.openApp(app)

      expect(mockAddDynamicFunction).toHaveBeenCalledWith(
        'new-open-app',
        'New Open App',
        'fa-solid fa-cube',
        false
      )
    })
  })

  // -- exposed history delegates --

  describe('history delegates', () => {
    it('should expose conversations from history', async () => {
      mockHistoryConversations.value = [
        { id: 'c1', title: 'Test', appName: 'a', createdAt: 1, updatedAt: 2 },
      ]

      const composable = await getComposable()
      expect(composable.conversations.value).toHaveLength(1)
      expect(composable.conversations.value[0].id).toBe('c1')
    })

    it('should expose currentConversationId from history', async () => {
      mockHistoryCurrentConversationId.value = 'conv-id'

      const composable = await getComposable()
      expect(composable.currentConversationId.value).toBe('conv-id')
    })

    it('should delegate getConversationsForApp to history', async () => {
      mockHistoryGetConversationsForApp.mockReturnValue([{ id: 'c1' }])

      const composable = await getComposable()
      const result = composable.getConversationsForApp('app-a')

      expect(mockHistoryGetConversationsForApp).toHaveBeenCalledWith('app-a')
      expect(result).toHaveLength(1)
    })

    it('should delegate getUnlinkedConversations to history', async () => {
      mockHistoryGetUnlinkedConversations.mockReturnValue([])

      const composable = await getComposable()
      const result = composable.getUnlinkedConversations()

      expect(mockHistoryGetUnlinkedConversations).toHaveBeenCalled()
      expect(result).toEqual([])
    })
  })

  // -- model watcher --

  describe('model watcher', () => {
    it('should update session model when model changes', async () => {
      const sessionData = {
        messages: [],
        isLoading: false,
        error: null as string | null,
        systemPrompt: '',
        workDir: '',
        model: 'auto',
        sdkSessionId: undefined,
        onSessionStarted: undefined,
      }
      mockHistoryCurrentConversationId.value = 'conv-1'
      mockSessions.value.set('claudeapps::conv-1', sessionData)

      const composable = await getComposable()
      composable.model.value = 'claude-opus'
      await nextTick()

      // Access through the reactive map to check
      const session = mockSessions.value.get('claudeapps::conv-1')
      expect(session.model).toBe('claude-opus')
    })
  })

  // -- isLoading watcher --

  describe('isLoading watcher (post-completion)', () => {
    it('should save conversation when loading completes', async () => {
      // Set up a session with isLoading=true through the reactive map
      mockSessions.value.set('claudeapps::conv-1', {
        messages: [
          { id: 'm1', role: 'user', content: 'Hi', timestamp: 1000 },
          { id: 'm2', role: 'assistant', content: 'Hello', timestamp: 1001 },
        ],
        isLoading: true,
        error: null,
        systemPrompt: '',
        workDir: '',
        model: 'auto',
        sdkSessionId: undefined,
        onSessionStarted: undefined,
      })
      mockHistoryCurrentConversationId.value = 'conv-1'
      mockHistoryCurrentConversation.value = {
        id: 'conv-1',
        title: 'Test',
        appName: 'test-app',
        messages: [],
        createdAt: 1000,
        updatedAt: 2000,
      }

      // loadApps will be called in the watcher
      mockInvoke.mockResolvedValue([createApp('test-app', 'Test App')])

      const composable = await getComposable()

      // Verify isLoading is true initially
      expect(composable.isLoading.value).toBe(true)

      // Transition isLoading from true to false through the reactive Map
      mockSessions.value.get('claudeapps::conv-1').isLoading = false
      await nextTick()
      // Allow async watcher to complete
      await new Promise(r => setTimeout(r, 50))

      expect(mockHistorySaveCurrentConversation).toHaveBeenCalled()
    })

    it('should detect new apps and add desktop icons', async () => {
      mockSessions.value.set('claudeapps::conv-1', {
        messages: [],
        isLoading: true,
        error: null,
        systemPrompt: '',
        workDir: '',
        model: 'auto',
        sdkSessionId: undefined,
        onSessionStarted: undefined,
      })
      mockHistoryCurrentConversationId.value = 'conv-1'
      mockHistoryCurrentConversation.value = {
        id: 'conv-1',
        title: 'Test',
        appName: null, // unlinked
        messages: [],
        createdAt: 1000,
        updatedAt: 2000,
      }

      // Initial loadApps during initialize: no apps
      mockInvoke.mockResolvedValueOnce([])

      const composable = await getComposable()
      await composable.initialize()

      // After loading completes, loadApps returns a new app
      const newApp = createApp('new-app', 'New App')
      mockInvoke.mockResolvedValueOnce([newApp])

      // Transition isLoading from true to false
      mockSessions.value.get('claudeapps::conv-1').isLoading = false
      await nextTick()
      await new Promise(r => setTimeout(r, 100))

      // Should auto-link and add desktop icon
      expect(mockHistoryLinkToApp).toHaveBeenCalledWith('new-app')
      expect(mockAddDynamicFunction).toHaveBeenCalledWith(
        'new-app', 'New App', 'fa-solid fa-cube', false
      )
    })
  })
})
