/**
 * Unit tests for useClaudeAppsHistory composable
 * Tests: CRUD operations, linking, queries, sorting
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

// --- Tauri mocks ---
const mockInvoke = vi.fn()

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}))

vi.mock('@tauri-apps/plugin-fs', () => ({
  readTextFile: vi.fn(),
  writeTextFile: vi.fn(),
  exists: vi.fn(),
  mkdir: vi.fn(),
  BaseDirectory: { Home: 'Home' },
}))

describe('useClaudeAppsHistory', () => {
  beforeEach(() => {
    vi.resetModules()
    vi.clearAllMocks()
    setActivePinia(createPinia())
  })

  async function getComposable() {
    const mod = await import('@/composables/claudeApps/useClaudeAppsHistory')
    return mod.useClaudeAppsHistory()
  }

  // -- initialize --

  describe('initialize', () => {
    it('should load conversation index via invoke', async () => {
      const indexItems = [
        { id: 'conv-1', title: 'Test 1', appName: 'app-a', createdAt: 1000, updatedAt: 2000 },
        { id: 'conv-2', title: 'Test 2', createdAt: 1500, updatedAt: 2500 },
      ]
      mockInvoke.mockResolvedValueOnce(indexItems)

      const history = await getComposable()
      await history.initialize()

      expect(mockInvoke).toHaveBeenCalledWith('claude_apps_list_conversations')
      expect(history.conversations.value).toHaveLength(2)

      // Verify appName normalization (undefined -> null)
      const conv2 = history.conversations.value.find((c: { id: string }) => c.id === 'conv-2')
      expect(conv2?.appName).toBeNull()
    })

    it('should handle initialize error gracefully', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Load failed'))
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

      const history = await getComposable()
      await history.initialize()

      expect(history.conversations.value).toHaveLength(0)
      consoleSpy.mockRestore()
    })
  })

  // -- createConversation --

  describe('createConversation', () => {
    it('should create a conversation linked to an app', async () => {
      mockInvoke.mockResolvedValueOnce(undefined) // claude_apps_save_conversation

      const history = await getComposable()
      const id = await history.createConversation('my-app')

      expect(id).toBeTruthy()
      expect(typeof id).toBe('string')
      expect(mockInvoke).toHaveBeenCalledWith('claude_apps_save_conversation', {
        conversation: expect.objectContaining({
          id,
          title: expect.any(String),
          appName: 'my-app',
          messages: [],
        }),
      })

      expect(history.currentConversationId.value).toBe(id)
      expect(history.currentConversation.value?.appName).toBe('my-app')
    })

    it('should create an unlinked conversation (null appName)', async () => {
      mockInvoke.mockResolvedValueOnce(undefined)

      const history = await getComposable()
      await history.createConversation(null)

      expect(mockInvoke).toHaveBeenCalledWith('claude_apps_save_conversation', {
        conversation: expect.objectContaining({
          appName: undefined, // null becomes undefined for Rust
        }),
      })
      expect(history.currentConversation.value?.appName).toBeNull()
      expect(history.conversations.value).toHaveLength(1)
    })

    it('should add conversation to the list', async () => {
      mockInvoke.mockResolvedValue(undefined)

      const history = await getComposable()
      await history.createConversation('app-1')
      await history.createConversation('app-2')

      // conversations is sorted by updatedAt desc, so the latest is first
      expect(history.conversations.value).toHaveLength(2)
    })
  })

  // -- loadConversation --

  describe('loadConversation', () => {
    it('should load a conversation by id', async () => {
      const rustConv = {
        id: 'conv-1',
        title: 'Hello',
        appName: 'test-app',
        messages: [{ id: 'm1', role: 'user', content: 'Hello', timestamp: 1000 }],
        createdAt: 1000,
        updatedAt: 2000,
        sdkSessionId: 'sdk-123',
      }
      mockInvoke.mockResolvedValueOnce(rustConv)

      const history = await getComposable()
      const result = await history.loadConversation('conv-1')

      expect(mockInvoke).toHaveBeenCalledWith('claude_apps_load_conversation', { id: 'conv-1' })
      expect(result).toBeTruthy()
      expect(result?.appName).toBe('test-app')
      expect(result?.sdkSessionId).toBe('sdk-123')
      expect(history.currentConversationId.value).toBe('conv-1')
      expect(history.currentConversation.value).toEqual(result)
    })

    it('should normalize undefined appName to null', async () => {
      const rustConv = {
        id: 'conv-2',
        title: 'No App',
        messages: [],
        createdAt: 1000,
        updatedAt: 2000,
      }
      mockInvoke.mockResolvedValueOnce(rustConv)

      const history = await getComposable()
      const result = await history.loadConversation('conv-2')

      expect(result?.appName).toBeNull()
    })

    it('should return null on error', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Not found'))
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

      const history = await getComposable()
      const result = await history.loadConversation('nonexistent')

      expect(result).toBeNull()
      consoleSpy.mockRestore()
    })

    it('should set isLoading during load and reset afterwards', async () => {
      let resolveLoad: (v: unknown) => void
      mockInvoke.mockReturnValueOnce(
        new Promise((resolve) => { resolveLoad = resolve })
      )

      const history = await getComposable()
      const promise = history.loadConversation('conv-1')

      expect(history.isLoading.value).toBe(true)

      resolveLoad!({
        id: 'conv-1',
        title: 'Test',
        messages: [],
        createdAt: 1000,
        updatedAt: 2000,
      })
      await promise

      expect(history.isLoading.value).toBe(false)
    })
  })

  // -- saveCurrentConversation --

  describe('saveCurrentConversation', () => {
    it('should save current conversation and update title from first user message', async () => {
      mockInvoke.mockResolvedValue(undefined)

      const history = await getComposable()
      // Create a conversation first
      await history.createConversation('app-x')

      // Add messages
      history.currentConversation.value!.messages = [
        { id: 'm1', role: 'user' as const, content: 'Build a calculator', timestamp: 1000 },
        { id: 'm2', role: 'assistant' as const, content: 'Sure!', timestamp: 1001 },
      ]

      await history.saveCurrentConversation()

      // Should invoke with the updated title from the first user message
      const saveCall = mockInvoke.mock.calls.find(
        (call: unknown[]) => call[0] === 'claude_apps_save_conversation' &&
          (call[1] as Record<string, unknown>)?.conversation &&
          ((call[1] as Record<string, Record<string, unknown>>).conversation?.title === 'Build a calculator')
      )
      expect(saveCall).toBeTruthy()
    })

    it('should use default title when no user messages', async () => {
      mockInvoke.mockResolvedValue(undefined)

      const history = await getComposable()
      await history.createConversation('app-x')
      history.currentConversation.value!.messages = []

      await history.saveCurrentConversation()

      // The title generation returns default title when no user messages
      const lastSaveCall = mockInvoke.mock.calls.filter(
        (call: unknown[]) => call[0] === 'claude_apps_save_conversation'
      ).pop()
      expect((lastSaveCall?.[1] as Record<string, Record<string, unknown>>)?.conversation?.title).toBeTruthy()
    })

    it('should truncate long titles to 50 characters', async () => {
      mockInvoke.mockResolvedValue(undefined)

      const history = await getComposable()
      await history.createConversation('app-x')

      const longContent = 'A'.repeat(100)
      history.currentConversation.value!.messages = [
        { id: 'm1', role: 'user' as const, content: longContent, timestamp: 1000 },
      ]

      await history.saveCurrentConversation()

      const lastSaveCall = mockInvoke.mock.calls.filter(
        (call: unknown[]) => call[0] === 'claude_apps_save_conversation'
      ).pop()
      const savedTitle = (lastSaveCall?.[1] as Record<string, Record<string, unknown>>)?.conversation?.title
      expect(savedTitle).toBe('A'.repeat(47) + '...')
    })

    it('should do nothing when no current conversation', async () => {
      const history = await getComposable()
      await history.saveCurrentConversation()

      // No invoke calls for save
      expect(mockInvoke).not.toHaveBeenCalledWith('claude_apps_save_conversation', expect.anything())
    })

    it('should update index item', async () => {
      mockInvoke.mockResolvedValue(undefined)

      const history = await getComposable()
      const id = await history.createConversation('app-x')

      history.currentConversation.value!.messages = [
        { id: 'm1', role: 'user' as const, content: 'New title text', timestamp: 1000 },
      ]

      await history.saveCurrentConversation()

      const indexItem = history.conversations.value.find((c: { id: string }) => c.id === id)
      expect(indexItem?.title).toBe('New title text')
    })
  })

  // -- deleteConversation --

  describe('deleteConversation', () => {
    it('should delete conversation from list and invoke backend', async () => {
      mockInvoke.mockResolvedValue(undefined)

      const history = await getComposable()
      const id = await history.createConversation('app-x')

      expect(history.conversations.value).toHaveLength(1)

      await history.deleteConversation(id)

      expect(mockInvoke).toHaveBeenCalledWith('claude_apps_delete_conversation', { id })
      expect(history.conversations.value).toHaveLength(0)
    })

    it('should clear current conversation if deleting the current one', async () => {
      mockInvoke.mockResolvedValue(undefined)

      const history = await getComposable()
      const id = await history.createConversation('app-x')
      expect(history.currentConversationId.value).toBe(id)

      await history.deleteConversation(id)

      expect(history.currentConversationId.value).toBeNull()
      expect(history.currentConversation.value).toBeNull()
    })

    it('should not clear current conversation if deleting a different one', async () => {
      mockInvoke.mockResolvedValue(undefined)

      const history = await getComposable()
      const id1 = await history.createConversation('app-1')
      const id2 = await history.createConversation('app-2')

      // Current is id2 (the latest createConversation sets it)
      expect(history.currentConversationId.value).toBe(id2)

      await history.deleteConversation(id1)

      expect(history.currentConversationId.value).toBe(id2)
    })
  })

  // -- linkToApp --

  describe('linkToApp', () => {
    it('should link current conversation to an app', async () => {
      mockInvoke.mockResolvedValue(undefined)

      const history = await getComposable()
      await history.createConversation(null)

      expect(history.currentConversation.value?.appName).toBeNull()

      history.linkToApp('new-app')

      expect(history.currentConversation.value?.appName).toBe('new-app')
      // sdkSessionId should be cleared when linking to new app
      expect(history.currentConversation.value?.sdkSessionId).toBeUndefined()
    })

    it('should update index item appName', async () => {
      mockInvoke.mockResolvedValue(undefined)

      const history = await getComposable()
      const id = await history.createConversation(null)

      history.linkToApp('my-app')

      const indexItem = history.conversations.value.find((c: { id: string }) => c.id === id)
      expect(indexItem?.appName).toBe('my-app')
    })

    it('should do nothing when no current conversation', async () => {
      const history = await getComposable()
      // No current conversation
      history.linkToApp('some-app')
      // Should not throw
    })
  })

  // -- setSdkSessionId --

  describe('setSdkSessionId', () => {
    it('should set SDK session ID on current conversation', async () => {
      mockInvoke.mockResolvedValue(undefined)

      const history = await getComposable()
      await history.createConversation('app-x')

      history.setSdkSessionId('sdk-abc-123')

      expect(history.currentConversation.value?.sdkSessionId).toBe('sdk-abc-123')
    })

    it('should do nothing when no current conversation', async () => {
      const history = await getComposable()
      history.setSdkSessionId('sdk-abc')
      // Should not throw
    })
  })

  // -- getConversationsForApp --

  describe('getConversationsForApp', () => {
    it('should return only conversations for specified app', async () => {
      mockInvoke.mockResolvedValue(undefined)

      const history = await getComposable()
      await history.createConversation('app-a')
      await history.createConversation('app-b')
      await history.createConversation('app-a')

      const appAConvs = history.getConversationsForApp('app-a')
      expect(appAConvs).toHaveLength(2)
      expect(appAConvs.every((c: { appName: string | null }) => c.appName === 'app-a')).toBe(true)
    })

    it('should return empty array when no conversations match', async () => {
      const history = await getComposable()
      const result = history.getConversationsForApp('nonexistent')
      expect(result).toEqual([])
    })
  })

  // -- getUnlinkedConversations --

  describe('getUnlinkedConversations', () => {
    it('should return only conversations without an app', async () => {
      mockInvoke.mockResolvedValue(undefined)

      const history = await getComposable()
      await history.createConversation('app-a')
      await history.createConversation(null)
      await history.createConversation(null)

      const unlinked = history.getUnlinkedConversations()
      expect(unlinked).toHaveLength(2)
      expect(unlinked.every((c: { appName: string | null }) => c.appName === null)).toBe(true)
    })
  })

  // -- sortedConversations --

  describe('sortedConversations (sorting)', () => {
    it('should sort conversations by updatedAt descending (newest first)', async () => {
      // Initialize with pre-existing items
      const indexItems = [
        { id: 'old', title: 'Old', appName: 'a', createdAt: 100, updatedAt: 100 },
        { id: 'new', title: 'New', appName: 'b', createdAt: 200, updatedAt: 300 },
        { id: 'mid', title: 'Mid', createdAt: 150, updatedAt: 200 },
      ]
      mockInvoke.mockResolvedValueOnce(indexItems)

      const history = await getComposable()
      await history.initialize()

      const sorted = history.conversations.value
      expect(sorted[0].id).toBe('new')
      expect(sorted[1].id).toBe('mid')
      expect(sorted[2].id).toBe('old')
    })
  })

  // -- clearCurrentConversation --

  describe('clearCurrentConversation', () => {
    it('should clear current conversation state', async () => {
      mockInvoke.mockResolvedValue(undefined)

      const history = await getComposable()
      await history.createConversation('app-x')

      expect(history.currentConversationId.value).not.toBeNull()

      history.clearCurrentConversation()

      expect(history.currentConversationId.value).toBeNull()
      expect(history.currentConversation.value).toBeNull()
    })
  })
})
