/**
 * Unit tests for settingsStore.
 * Tests default state, environment variables, LLM router config,
 * Claude aliases, coworks model config, and keybindings.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {}
  return {
    getItem: vi.fn((key: string) => store[key] ?? null),
    setItem: vi.fn((key: string, value: string) => { store[key] = value }),
    removeItem: vi.fn((key: string) => { delete store[key] }),
    clear: vi.fn(() => { store = {} }),
  }
})()

Object.defineProperty(globalThis, 'localStorage', { value: localStorageMock, writable: true })

// Mock Tauri APIs
vi.mock('@tauri-apps/plugin-fs', () => ({
  readTextFile: vi.fn(),
  writeTextFile: vi.fn(),
  exists: vi.fn().mockResolvedValue(false),
  mkdir: vi.fn(),
  BaseDirectory: { Home: 'Home' },
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}))

describe('settingsStore', () => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let useSettingsStore: any
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let mockWriteTextFile: any
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let mockExists: any
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let mockReadTextFile: any
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let mockInvoke: any

  beforeEach(async () => {
    setActivePinia(createPinia())
    localStorageMock.clear()
    vi.clearAllMocks()

    // Re-import mocks
    const fsMod = await import('@tauri-apps/plugin-fs')
    mockWriteTextFile = fsMod.writeTextFile
    mockExists = fsMod.exists
    mockReadTextFile = fsMod.readTextFile
    const coreMod = await import('@tauri-apps/api/core')
    mockInvoke = coreMod.invoke

    // Default: ensureConfigDir succeeds
    mockExists.mockResolvedValue(true)
    mockWriteTextFile.mockResolvedValue(undefined)

    const mod = await import('@/stores/settingsStore')
    useSettingsStore = mod.useSettingsStore
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  // ─── Default state ───────────────────────────────────────────────

  describe('default state', () => {
    it('should have correct default values', () => {
      const store = useSettingsStore()
      expect(store.terminalFontSize).toBe(14)
      expect(store.defaultCli).toBe('powershell')
      expect(store.defaultWindowWidth).toBe(1000)
      expect(store.defaultWindowHeight).toBe(600)
      expect(store.environmentVariables).toEqual({})
      expect(store.claudeAliases).toEqual([])
    })

    it('should have default keybindings', () => {
      const store = useSettingsStore()
      expect(store.keybindings['ctrl+alt+t']).toBe('openTerminal')
      expect(store.keybindings['ctrl+w']).toBe('closeWindow')
      expect(store.keybindings['alt+tab']).toBe('switchWindow')
    })

    it('should have default LLM router config', () => {
      const store = useSettingsStore()
      expect(store.llmRouterConfig.selectedProviderId).toBe('claude')
      expect(store.llmRouterConfig.thinkingLevel).toBe('medium')
    })

    it('should have default coworks model config', () => {
      const store = useSettingsStore()
      expect(store.coworksModelConfig.defaultModel).toBe('auto')
      expect(store.coworksModelConfig.providers).toEqual([])
      expect(store.coworksModelConfig.maxTurns).toBe(30)
      expect(store.coworksModelConfig.streamingEnabled).toBe(true)
    })
  })

  // ─── setEnvironmentVariable / removeEnvironmentVariable ─────────

  describe('environment variables', () => {
    it('should set an environment variable', async () => {
      const store = useSettingsStore()
      await store.setEnvironmentVariable('API_KEY', 'abc123')
      expect(store.environmentVariables['API_KEY']).toBe('abc123')
    })

    it('should persist after setting env var', async () => {
      const store = useSettingsStore()
      await store.setEnvironmentVariable('KEY', 'VAL')
      expect(mockWriteTextFile).toHaveBeenCalled()
    })

    it('should call invoke to update shell env', async () => {
      const store = useSettingsStore()
      await store.setEnvironmentVariable('MY_VAR', 'my_value')
      expect(mockInvoke).toHaveBeenCalledWith('update_shell_env', { key: 'MY_VAR', value: 'my_value' })
    })

    it('should remove an environment variable', async () => {
      const store = useSettingsStore()
      await store.setEnvironmentVariable('REMOVE_ME', 'val')
      await store.removeEnvironmentVariable('REMOVE_ME')
      expect(store.environmentVariables['REMOVE_ME']).toBeUndefined()
    })

    it('should call invoke with null value when removing env var', async () => {
      const store = useSettingsStore()
      await store.setEnvironmentVariable('DEL', 'x')
      vi.clearAllMocks()
      await store.removeEnvironmentVariable('DEL')
      expect(mockInvoke).toHaveBeenCalledWith('update_shell_env', { key: 'DEL', value: null })
    })

    it('should handle invoke failure gracefully', async () => {
      const store = useSettingsStore()
      mockInvoke.mockRejectedValue(new Error('shell update failed'))
      // Should not throw even if invoke fails
      await expect(store.setEnvironmentVariable('K', 'V')).resolves.toBeUndefined()
      // Variable should still be set locally
      expect(store.environmentVariables['K']).toBe('V')
    })
  })

  // ─── LLM Router Config ─────────────────────────────────────────

  describe('LLM router config', () => {
    it('setLlmRouterConfig should update selectedProviderId', async () => {
      const store = useSettingsStore()
      await store.setLlmRouterConfig({ selectedProviderId: 'openai' })
      expect(store.llmRouterConfig.selectedProviderId).toBe('openai')
    })

    it('setLlmRouterConfig should merge modelProviders', async () => {
      const store = useSettingsStore()
      await store.setLlmRouterConfig({ modelProviders: { openai: 'gpt-4o' } })
      expect(store.llmRouterConfig.modelProviders.openai).toBe('gpt-4o')
      // Claude should still exist
      expect(store.llmRouterConfig.modelProviders.claude).toBe('Sonnet')
    })

    it('setLlmRouterConfig should update thinkingLevel', async () => {
      const store = useSettingsStore()
      await store.setLlmRouterConfig({ thinkingLevel: 'high' })
      expect(store.llmRouterConfig.thinkingLevel).toBe('high')
    })

    it('setLlmRouterProvider should set provider and model', async () => {
      const store = useSettingsStore()
      await store.setLlmRouterProvider('gemini', 'gemini-2.0-flash')
      expect(store.llmRouterConfig.selectedProviderId).toBe('gemini')
      expect(store.llmRouterConfig.modelProviders.gemini).toBe('gemini-2.0-flash')
    })

    it('setLlmRouterProvider without model should only change provider', async () => {
      const store = useSettingsStore()
      const originalModel = store.llmRouterConfig.modelProviders.claude
      await store.setLlmRouterProvider('claude')
      expect(store.llmRouterConfig.selectedProviderId).toBe('claude')
      expect(store.llmRouterConfig.modelProviders.claude).toBe(originalModel)
    })

    it('setLlmRouterModel should update model for specific provider', async () => {
      const store = useSettingsStore()
      await store.setLlmRouterModel('claude', 'Opus')
      expect(store.llmRouterConfig.modelProviders.claude).toBe('Opus')
      // selectedProviderId should not change
      expect(store.llmRouterConfig.selectedProviderId).toBe('claude')
    })
  })

  // ─── Claude Aliases ─────────────────────────────────────────────

  describe('Claude aliases', () => {
    it('should add a new alias with generated id and timestamps', async () => {
      const store = useSettingsStore()
      const alias = await store.addClaudeAlias({
        name: 'claude-opus',
        description: 'Opus model',
        anthropicModel: 'claude-opus-4-20250514',
        anthropicAuthToken: '',
        anthropicBaseUrl: '',
        customEnvVars: {}
      })
      expect(alias.id).toMatch(/^alias-\d+$/)
      expect(alias.name).toBe('claude-opus')
      expect(alias.createdAt).toBeGreaterThan(0)
      expect(alias.updatedAt).toBe(alias.createdAt)
    })

    it('should persist after adding alias', async () => {
      const store = useSettingsStore()
      await store.addClaudeAlias({
        name: 'test',
        description: '',
        anthropicModel: '',
        anthropicAuthToken: '',
        anthropicBaseUrl: '',
        customEnvVars: {}
      })
      expect(mockWriteTextFile).toHaveBeenCalled()
    })

    it('should append alias without replacing existing ones', async () => {
      const store = useSettingsStore()
      await store.addClaudeAlias({
        name: 'a1',
        description: '',
        anthropicModel: '',
        anthropicAuthToken: '',
        anthropicBaseUrl: '',
        customEnvVars: {}
      })
      await store.addClaudeAlias({
        name: 'a2',
        description: '',
        anthropicModel: '',
        anthropicAuthToken: '',
        anthropicBaseUrl: '',
        customEnvVars: {}
      })
      expect(store.claudeAliases.length).toBe(2)
    })

    it('should update an existing alias', async () => {
      const store = useSettingsStore()
      const alias = await store.addClaudeAlias({
        name: 'update-me',
        description: 'old',
        anthropicModel: '',
        anthropicAuthToken: '',
        anthropicBaseUrl: '',
        customEnvVars: {}
      })
      await store.updateClaudeAlias(alias.id, { description: 'new' })
      const updated = store.claudeAliases.find((a: { id: string }) => a.id === alias.id)
      expect(updated?.description).toBe('new')
      expect(updated?.updatedAt).toBeGreaterThanOrEqual(alias.updatedAt)
    })

    it('should not modify other aliases when updating one', async () => {
      const store = useSettingsStore()
      // Use unique timestamps to avoid ID collision
      const mockNow = vi.spyOn(Date, 'now')
      mockNow.mockReturnValueOnce(1000)
      const a1 = await store.addClaudeAlias({
        name: 'keep',
        description: 'original',
        anthropicModel: '',
        anthropicAuthToken: '',
        anthropicBaseUrl: '',
        customEnvVars: {}
      })
      mockNow.mockReturnValueOnce(2000)
      const a2 = await store.addClaudeAlias({
        name: 'change',
        description: 'old',
        anthropicModel: '',
        anthropicAuthToken: '',
        anthropicBaseUrl: '',
        customEnvVars: {}
      })
      mockNow.mockRestore()
      expect(a1.id).not.toBe(a2.id)
      await store.updateClaudeAlias(a2.id, { description: 'new' })
      const kept = store.claudeAliases.find((a: { id: string }) => a.id === a1.id)
      expect(kept?.description).toBe('original')
    })

    it('should remove an alias by id', async () => {
      const store = useSettingsStore()
      const alias = await store.addClaudeAlias({
        name: 'remove-me',
        description: '',
        anthropicModel: '',
        anthropicAuthToken: '',
        anthropicBaseUrl: '',
        customEnvVars: {}
      })
      await store.removeClaudeAlias(alias.id)
      expect(store.claudeAliases.length).toBe(0)
    })

    it('should do nothing when removing nonexistent alias id', async () => {
      const store = useSettingsStore()
      await store.addClaudeAlias({
        name: 'stay',
        description: '',
        anthropicModel: '',
        anthropicAuthToken: '',
        anthropicBaseUrl: '',
        customEnvVars: {}
      })
      await store.removeClaudeAlias('alias-nonexistent')
      expect(store.claudeAliases.length).toBe(1)
    })
  })

  // ─── Coworks Model Config ──────────────────────────────────────

  describe('coworks model config', () => {
    it('should set default model', async () => {
      const store = useSettingsStore()
      await store.setCoworksDefaultModel('custom-provider-1')
      expect(store.coworksModelConfig.defaultModel).toBe('custom-provider-1')
    })

    it('should update config settings', async () => {
      const store = useSettingsStore()
      await store.setCoworksConfig({ maxTurns: 50, streamingEnabled: false })
      expect(store.coworksModelConfig.maxTurns).toBe(50)
      expect(store.coworksModelConfig.streamingEnabled).toBe(false)
    })

    it('should partially update config', async () => {
      const store = useSettingsStore()
      await store.setCoworksConfig({ maxTurns: 10 })
      expect(store.coworksModelConfig.maxTurns).toBe(10)
      // streamingEnabled should remain unchanged
      expect(store.coworksModelConfig.streamingEnabled).toBe(true)
    })

    it('should add a coworks provider', async () => {
      const store = useSettingsStore()
      const provider = await store.addCoworksProvider({
        name: 'MyProvider',
        model: 'my-model-v1',
        baseUrl: 'https://api.example.com',
        apiKey: 'sk-test'
      })
      expect(provider.id).toMatch(/^custom-\d+$/)
      expect(provider.name).toBe('MyProvider')
      expect(store.coworksModelConfig.providers.length).toBe(1)
    })

    it('should update a coworks provider', async () => {
      const store = useSettingsStore()
      const provider = await store.addCoworksProvider({
        name: 'OldName',
        model: 'old-model',
        baseUrl: '',
        apiKey: ''
      })
      await store.updateCoworksProvider(provider.id, { name: 'NewName', model: 'new-model' })
      const updated = store.coworksModelConfig.providers.find((p: { id: string }) => p.id === provider.id)
      expect(updated?.name).toBe('NewName')
      expect(updated?.model).toBe('new-model')
    })

    it('should remove a coworks provider', async () => {
      const store = useSettingsStore()
      const provider = await store.addCoworksProvider({
        name: 'RemoveMe',
        model: 'model',
        baseUrl: '',
        apiKey: ''
      })
      await store.removeCoworksProvider(provider.id)
      expect(store.coworksModelConfig.providers.length).toBe(0)
    })

    it('should reset defaultModel to auto when removed provider was default', async () => {
      const store = useSettingsStore()
      const provider = await store.addCoworksProvider({
        name: 'Default',
        model: 'model',
        baseUrl: '',
        apiKey: ''
      })
      await store.setCoworksDefaultModel(provider.id)
      await store.removeCoworksProvider(provider.id)
      expect(store.coworksModelConfig.defaultModel).toBe('auto')
    })

    it('should not reset defaultModel when a different provider is removed', async () => {
      const store = useSettingsStore()
      // Use unique timestamps to avoid ID collision
      const mockNow = vi.spyOn(Date, 'now')
      mockNow.mockReturnValueOnce(3000)
      const p1 = await store.addCoworksProvider({
        name: 'Keep',
        model: 'm1',
        baseUrl: '',
        apiKey: ''
      })
      mockNow.mockReturnValueOnce(4000)
      const p2 = await store.addCoworksProvider({
        name: 'Remove',
        model: 'm2',
        baseUrl: '',
        apiKey: ''
      })
      mockNow.mockRestore()
      expect(p1.id).not.toBe(p2.id)
      await store.setCoworksDefaultModel(p1.id)
      await store.removeCoworksProvider(p2.id)
      expect(store.coworksModelConfig.defaultModel).toBe(p1.id)
    })
  })

  // ─── Keybindings ────────────────────────────────────────────────

  describe('keybindings', () => {
    it('should set a new keybinding', () => {
      const store = useSettingsStore()
      store.setKeybinding('ctrl+shift+n', 'newTerminal')
      expect(store.keybindings['ctrl+shift+n']).toBe('newTerminal')
    })

    it('should overwrite an existing keybinding', () => {
      const store = useSettingsStore()
      store.setKeybinding('ctrl+w', 'newAction')
      expect(store.keybindings['ctrl+w']).toBe('newAction')
    })

    it('should persist after setting keybinding', async () => {
      const store = useSettingsStore()
      vi.clearAllMocks()
      store.setKeybinding('ctrl+n', 'action')
      // setKeybinding calls save() without await; wait for the async save to complete
      await vi.waitFor(() => {
        expect(mockWriteTextFile).toHaveBeenCalled()
      })
    })
  })

  // ─── save / load ────────────────────────────────────────────────

  describe('save and load', () => {
    it('should save to file via Tauri plugin-fs', async () => {
      const store = useSettingsStore()
      await store.save()
      expect(mockWriteTextFile).toHaveBeenCalledWith(
        expect.stringContaining('settings.json'),
        expect.any(String),
        expect.objectContaining({ baseDir: 'Home' })
      )
    })

    it('should fall back to localStorage if file save fails', async () => {
      const store = useSettingsStore()
      mockExists.mockResolvedValue(true)
      mockWriteTextFile.mockRejectedValue(new Error('write failed'))
      await store.save()
      expect(localStorageMock.setItem).toHaveBeenCalledWith('mcd-settings', expect.any(String))
    })

    it('should load from file when it exists', async () => {
      const store = useSettingsStore()
      const savedState = { ...store.$state, terminalFontSize: 18 }
      mockExists.mockResolvedValue(true)
      mockReadTextFile.mockResolvedValue(JSON.stringify(savedState))
      await store.load()
      expect(store.terminalFontSize).toBe(18)
    })

    it('should migrate from localStorage when file does not exist', async () => {
      const store = useSettingsStore()
      mockExists.mockResolvedValue(false)
      localStorageMock.getItem.mockReturnValue(JSON.stringify({ terminalFontSize: 20 }))
      await store.load()
      expect(store.terminalFontSize).toBe(20)
      // Should also save to file for migration
      expect(mockWriteTextFile).toHaveBeenCalled()
    })

    it('should fall back to localStorage when file read fails', async () => {
      const store = useSettingsStore()
      mockExists.mockResolvedValue(true)
      mockReadTextFile.mockRejectedValue(new Error('read error'))
      localStorageMock.getItem.mockReturnValue(JSON.stringify({ terminalFontSize: 16 }))
      await store.load()
      expect(store.terminalFontSize).toBe(16)
    })

    it('should handle missing localStorage gracefully', async () => {
      const store = useSettingsStore()
      mockExists.mockResolvedValue(false)
      localStorageMock.getItem.mockReturnValue(null as unknown as string)
      await expect(store.load()).resolves.toBeUndefined()
      // Should still have defaults
      expect(store.terminalFontSize).toBe(14)
    })

    it('should handle corrupted localStorage gracefully', () => {
      const store = useSettingsStore()
      localStorageMock.getItem.mockReturnValue('NOT_JSON')
      expect(() => store.loadFromLocalStorage()).not.toThrow()
    })
  })

  // ─── ensureConfigDir ────────────────────────────────────────────

  describe('ensureConfigDir', () => {
    it('should create directory if it does not exist', async () => {
      const store = useSettingsStore()
      const mockMkdir = (await import('@tauri-apps/plugin-fs')).mkdir
      mockExists.mockResolvedValue(false)
      await store.ensureConfigDir()
      expect(mockMkdir).toHaveBeenCalledWith(
        expect.stringContaining('config'),
        expect.objectContaining({ baseDir: 'Home', recursive: true })
      )
    })

    it('should not create directory if it already exists', async () => {
      const store = useSettingsStore()
      const mockMkdir = (await import('@tauri-apps/plugin-fs')).mkdir
      mockExists.mockResolvedValue(true)
      vi.clearAllMocks()
      await store.ensureConfigDir()
      expect(mockMkdir).not.toHaveBeenCalled()
    })
  })
})
