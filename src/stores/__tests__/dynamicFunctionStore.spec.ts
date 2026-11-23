/**
 * Unit tests for dynamicFunctionStore.
 * Tests loadFunctions, deleteFunction, startBackend/stopBackend
 * (status transitions), resetBackendStatus, and getBackendStatus getter.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

// Mock Tauri APIs
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

describe('dynamicFunctionStore', () => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let useDynamicFunctionStore: any
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let mockInvoke: any

  beforeEach(async () => {
    setActivePinia(createPinia())
    vi.clearAllMocks()

    const coreMod = await import('@tauri-apps/api/core')
    mockInvoke = coreMod.invoke

    const mod = await import('@/stores/dynamicFunctionStore')
    useDynamicFunctionStore = mod.useDynamicFunctionStore
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  // ─── Default state ───────────────────────────────────────────────

  describe('default state', () => {
    it('should start with empty functions array', () => {
      const store = useDynamicFunctionStore()
      expect(store.functions).toEqual([])
    })

    it('should start with loading false', () => {
      const store = useDynamicFunctionStore()
      expect(store.loading).toBe(false)
    })

    it('should start with empty backendStatuses', () => {
      const store = useDynamicFunctionStore()
      expect(store.backendStatuses).toEqual({})
    })
  })

  // ─── getBackendStatus getter ────────────────────────────────────

  describe('getBackendStatus', () => {
    it('should return default status for unknown function name', () => {
      const store = useDynamicFunctionStore()
      const status = store.getBackendStatus('unknown-fn')
      expect(status).toEqual({
        status: 'stopped',
        port: null,
        error: null
      })
    })

    it('should return actual status when set', () => {
      const store = useDynamicFunctionStore()
      store.backendStatuses = {
        'my-fn': { status: 'running', port: 3000, error: null }
      }
      const status = store.getBackendStatus('my-fn')
      expect(status.status).toBe('running')
      expect(status.port).toBe(3000)
    })
  })

  // ─── loadFunctions ──────────────────────────────────────────────

  describe('loadFunctions', () => {
    it('should set loading to true during load', async () => {
      const store = useDynamicFunctionStore()
      let loadingDuringCall = false
      mockInvoke.mockImplementation(() => {
        loadingDuringCall = store.loading
        return Promise.resolve([])
      })
      await store.loadFunctions()
      expect(loadingDuringCall).toBe(true)
    })

    it('should set loading to false after successful load', async () => {
      const store = useDynamicFunctionStore()
      mockInvoke.mockResolvedValue([])
      await store.loadFunctions()
      expect(store.loading).toBe(false)
    })

    it('should populate functions from invoke result', async () => {
      const store = useDynamicFunctionStore()
      const mockFunctions = [
        {
          name: 'fn1',
          displayName: 'Function 1',
          version: '1.0.0',
          description: 'Test function',
          icon: 'fa-solid fa-star',
          backendEnabled: false,
          path: '/path/to/fn1'
        },
        {
          name: 'fn2',
          displayName: 'Function 2',
          version: '0.1.0',
          description: 'Another function',
          icon: 'fa-solid fa-cube',
          backendEnabled: true,
          path: '/path/to/fn2'
        }
      ]
      mockInvoke.mockResolvedValue(mockFunctions)
      await store.loadFunctions()
      expect(store.functions).toEqual(mockFunctions)
      expect(store.functions.length).toBe(2)
    })

    it('should call invoke with df_list_functions', async () => {
      const store = useDynamicFunctionStore()
      mockInvoke.mockResolvedValue([])
      await store.loadFunctions()
      expect(mockInvoke).toHaveBeenCalledWith('df_list_functions')
    })

    it('should set loading to false even on error', async () => {
      const store = useDynamicFunctionStore()
      mockInvoke.mockRejectedValue(new Error('load failed'))
      await store.loadFunctions()
      expect(store.loading).toBe(false)
    })

    it('should not throw on invoke failure', async () => {
      const store = useDynamicFunctionStore()
      mockInvoke.mockRejectedValue(new Error('network error'))
      await expect(store.loadFunctions()).resolves.toBeUndefined()
    })

    it('should keep existing functions on load failure', async () => {
      const store = useDynamicFunctionStore()
      store.functions = [{ name: 'existing', displayName: 'Existing', version: '1.0', description: '', icon: '', backendEnabled: false, path: '' }]
      mockInvoke.mockRejectedValue(new Error('fail'))
      await store.loadFunctions()
      // Functions should remain unchanged on error
      expect(store.functions.length).toBe(1)
      expect(store.functions[0].name).toBe('existing')
    })
  })

  // ─── deleteFunction ─────────────────────────────────────────────

  describe('deleteFunction', () => {
    it('should call invoke with df_delete_function', async () => {
      const store = useDynamicFunctionStore()
      store.functions = [{ name: 'del-fn', displayName: 'Delete', version: '1.0', description: '', icon: '', backendEnabled: false, path: '' }]
      mockInvoke.mockResolvedValue(undefined)
      await store.deleteFunction('del-fn')
      expect(mockInvoke).toHaveBeenCalledWith('df_delete_function', { name: 'del-fn' })
    })

    it('should remove function from list after deletion', async () => {
      const store = useDynamicFunctionStore()
      store.functions = [
        { name: 'keep', displayName: 'Keep', version: '1.0', description: '', icon: '', backendEnabled: false, path: '' },
        { name: 'remove', displayName: 'Remove', version: '1.0', description: '', icon: '', backendEnabled: false, path: '' }
      ]
      mockInvoke.mockResolvedValue(undefined)
      await store.deleteFunction('remove')
      expect(store.functions.length).toBe(1)
      expect(store.functions[0].name).toBe('keep')
    })

    it('should reset backend status for deleted function', async () => {
      const store = useDynamicFunctionStore()
      store.functions = [{ name: 'fn', displayName: 'Fn', version: '1.0', description: '', icon: '', backendEnabled: true, path: '' }]
      store.backendStatuses = { fn: { status: 'running', port: 3000, error: null } }
      mockInvoke.mockResolvedValue(undefined)
      await store.deleteFunction('fn')
      expect(store.backendStatuses['fn']).toBeUndefined()
    })

    it('should throw on invoke failure', async () => {
      const store = useDynamicFunctionStore()
      store.functions = [{ name: 'fail-fn', displayName: 'Fail', version: '1.0', description: '', icon: '', backendEnabled: false, path: '' }]
      mockInvoke.mockRejectedValue(new Error('delete error'))
      await expect(store.deleteFunction('fail-fn')).rejects.toThrow('delete error')
    })

    it('should not remove function from list on invoke failure', async () => {
      const store = useDynamicFunctionStore()
      store.functions = [{ name: 'safe', displayName: 'Safe', version: '1.0', description: '', icon: '', backendEnabled: false, path: '' }]
      mockInvoke.mockRejectedValue(new Error('fail'))
      try { await store.deleteFunction('safe') } catch { /* expected */ }
      expect(store.functions.length).toBe(1)
    })
  })

  // ─── startBackend ───────────────────────────────────────────────

  describe('startBackend', () => {
    it('should transition status: stopped -> starting -> running', async () => {
      const store = useDynamicFunctionStore()
      const statusLog: string[] = []

      mockInvoke.mockImplementation(() => {
        statusLog.push(store.getBackendStatus('my-app').status)
        return Promise.resolve(3001)
      })

      await store.startBackend('my-app')
      statusLog.push(store.getBackendStatus('my-app').status)

      expect(statusLog).toEqual(['starting', 'running'])
    })

    it('should set port on successful start', async () => {
      const store = useDynamicFunctionStore()
      mockInvoke.mockResolvedValue(8080)
      const port = await store.startBackend('port-test')
      expect(port).toBe(8080)
      expect(store.getBackendStatus('port-test').port).toBe(8080)
    })

    it('should transition to error status on failure', async () => {
      const store = useDynamicFunctionStore()
      mockInvoke.mockRejectedValue(new Error('start failed'))
      try { await store.startBackend('err-fn') } catch { /* expected */ }
      const status = store.getBackendStatus('err-fn')
      expect(status.status).toBe('error')
      expect(status.error).toBe('start failed')
      expect(status.port).toBeNull()
    })

    it('should call invoke with df_start_backend', async () => {
      const store = useDynamicFunctionStore()
      mockInvoke.mockResolvedValue(3000)
      await store.startBackend('invoke-test')
      expect(mockInvoke).toHaveBeenCalledWith('df_start_backend', { name: 'invoke-test' })
    })

    it('should throw on invoke failure', async () => {
      const store = useDynamicFunctionStore()
      mockInvoke.mockRejectedValue(new Error('boom'))
      await expect(store.startBackend('fail')).rejects.toThrow('boom')
    })

    it('should handle non-Error rejection', async () => {
      const store = useDynamicFunctionStore()
      mockInvoke.mockRejectedValue('string error')
      try { await store.startBackend('str-err') } catch { /* expected */ }
      expect(store.getBackendStatus('str-err').error).toBe('string error')
    })
  })

  // ─── stopBackend ────────────────────────────────────────────────

  describe('stopBackend', () => {
    it('should transition status: running -> stopping -> stopped', async () => {
      const store = useDynamicFunctionStore()
      // Pre-set to running
      store.backendStatuses = {
        'stop-test': { status: 'running', port: 3000, error: null }
      }

      const statusLog: string[] = []
      mockInvoke.mockImplementation(() => {
        statusLog.push(store.getBackendStatus('stop-test').status)
        return Promise.resolve(undefined)
      })

      await store.stopBackend('stop-test')
      statusLog.push(store.getBackendStatus('stop-test').status)

      expect(statusLog).toEqual(['stopping', 'stopped'])
    })

    it('should clear port and error on successful stop', async () => {
      const store = useDynamicFunctionStore()
      store.backendStatuses = {
        'clear-test': { status: 'running', port: 8080, error: null }
      }
      mockInvoke.mockResolvedValue(undefined)
      await store.stopBackend('clear-test')
      const status = store.getBackendStatus('clear-test')
      expect(status.port).toBeNull()
      expect(status.error).toBeNull()
    })

    it('should transition to error status on stop failure', async () => {
      const store = useDynamicFunctionStore()
      store.backendStatuses = {
        'stop-err': { status: 'running', port: 3000, error: null }
      }
      mockInvoke.mockRejectedValue(new Error('stop failed'))
      try { await store.stopBackend('stop-err') } catch { /* expected */ }
      const status = store.getBackendStatus('stop-err')
      expect(status.status).toBe('error')
      expect(status.error).toBe('stop failed')
    })

    it('should call invoke with df_stop_backend', async () => {
      const store = useDynamicFunctionStore()
      store.backendStatuses = {
        'invoke-stop': { status: 'running', port: 3000, error: null }
      }
      mockInvoke.mockResolvedValue(undefined)
      await store.stopBackend('invoke-stop')
      expect(mockInvoke).toHaveBeenCalledWith('df_stop_backend', { name: 'invoke-stop' })
    })

    it('should throw on invoke failure', async () => {
      const store = useDynamicFunctionStore()
      store.backendStatuses = {
        'throw-stop': { status: 'running', port: 3000, error: null }
      }
      mockInvoke.mockRejectedValue(new Error('kaboom'))
      await expect(store.stopBackend('throw-stop')).rejects.toThrow('kaboom')
    })
  })

  // ─── resetBackendStatus ─────────────────────────────────────────

  describe('resetBackendStatus', () => {
    it('should remove status entry for the function', () => {
      const store = useDynamicFunctionStore()
      store.backendStatuses = {
        fn1: { status: 'running', port: 3000, error: null },
        fn2: { status: 'stopped', port: null, error: null }
      }
      store.resetBackendStatus('fn1')
      expect(store.backendStatuses['fn1']).toBeUndefined()
      // fn2 should remain
      expect(store.backendStatuses['fn2']).toBeDefined()
    })

    it('should return default status after reset', () => {
      const store = useDynamicFunctionStore()
      store.backendStatuses = {
        fn: { status: 'error', port: null, error: 'some error' }
      }
      store.resetBackendStatus('fn')
      const status = store.getBackendStatus('fn')
      expect(status).toEqual({ status: 'stopped', port: null, error: null })
    })

    it('should handle resetting nonexistent function gracefully', () => {
      const store = useDynamicFunctionStore()
      expect(() => store.resetBackendStatus('nonexistent')).not.toThrow()
    })
  })

  // ─── Immutability of backendStatuses ────────────────────────────

  describe('immutable state updates', () => {
    it('startBackend should create new backendStatuses object (not mutate)', async () => {
      const store = useDynamicFunctionStore()
      const original = store.backendStatuses
      mockInvoke.mockResolvedValue(3000)
      await store.startBackend('imm-test')
      expect(store.backendStatuses).not.toBe(original)
    })

    it('stopBackend should create new backendStatuses object', async () => {
      const store = useDynamicFunctionStore()
      store.backendStatuses = { fn: { status: 'running', port: 3000, error: null } }
      const original = store.backendStatuses
      mockInvoke.mockResolvedValue(undefined)
      await store.stopBackend('fn')
      expect(store.backendStatuses).not.toBe(original)
    })

    it('resetBackendStatus should create new backendStatuses object', () => {
      const store = useDynamicFunctionStore()
      store.backendStatuses = { fn: { status: 'running', port: 3000, error: null } }
      const original = store.backendStatuses
      store.resetBackendStatus('fn')
      expect(store.backendStatuses).not.toBe(original)
    })
  })
})
