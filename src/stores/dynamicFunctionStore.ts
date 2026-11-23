import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'

export type BackendStatusKind = 'stopped' | 'starting' | 'running' | 'stopping' | 'error'

export interface BackendStatus {
  status: BackendStatusKind
  port: number | null
  error: string | null
}

export interface DynamicFunctionInfo {
  name: string
  displayName: string
  version: string
  description: string
  icon: string
  backendEnabled: boolean
  path: string
}

export interface DynamicFunctionState {
  functions: DynamicFunctionInfo[]
  loading: boolean
  backendStatuses: Record<string, BackendStatus>
}

const DEFAULT_BACKEND_STATUS: BackendStatus = {
  status: 'stopped',
  port: null,
  error: null
}

export const useDynamicFunctionStore = defineStore('dynamicFunction', {
  state: (): DynamicFunctionState => ({
    functions: [],
    loading: false,
    backendStatuses: {}
  }),

  getters: {
    getBackendStatus: (state) => {
      return (name: string): BackendStatus =>
        state.backendStatuses[name] ?? DEFAULT_BACKEND_STATUS
    }
  },

  actions: {
    async loadFunctions() {
      this.loading = true
      try {
        this.functions = await invoke<DynamicFunctionInfo[]>('df_list_functions')
      } catch (e) {
        console.error('Failed to list dynamic functions:', e)
      } finally {
        this.loading = false
      }
    },

    async deleteFunction(name: string) {
      try {
        await invoke('df_delete_function', { name })
        this.functions = this.functions.filter(f => f.name !== name)
        this.resetBackendStatus(name)
      } catch (e) {
        console.error('Failed to delete function:', e)
        throw e
      }
    },

    async startBackend(name: string): Promise<number> {
      this.backendStatuses = {
        ...this.backendStatuses,
        [name]: { status: 'starting', port: null, error: null }
      }
      try {
        const port = await invoke<number>('df_start_backend', { name })
        this.backendStatuses = {
          ...this.backendStatuses,
          [name]: { status: 'running', port, error: null }
        }
        return port
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e)
        this.backendStatuses = {
          ...this.backendStatuses,
          [name]: { status: 'error', port: null, error: msg }
        }
        throw e
      }
    },

    async stopBackend(name: string) {
      this.backendStatuses = {
        ...this.backendStatuses,
        [name]: { ...this.getBackendStatus(name), status: 'stopping', error: null }
      }
      try {
        await invoke('df_stop_backend', { name })
        this.backendStatuses = {
          ...this.backendStatuses,
          [name]: { status: 'stopped', port: null, error: null }
        }
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e)
        this.backendStatuses = {
          ...this.backendStatuses,
          [name]: { status: 'error', port: null, error: msg }
        }
        throw e
      }
    },

    resetBackendStatus(name: string) {
      const { [name]: _, ...rest } = this.backendStatuses
      this.backendStatuses = rest
    }
  }
})
