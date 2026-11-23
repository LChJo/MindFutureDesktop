import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type {
  DevToolsProject,
  SessionSummary,
  SessionDetail,
} from '@/types/devtools'

export interface DevToolsState {
  projects: DevToolsProject[]
  sessions: SessionSummary[]
  selectedProjectId: string | null
  selectedSessionId: string | null
  activeSession: SessionDetail | null
  isLoadingProjects: boolean
  isLoadingSessions: boolean
  isLoadingSession: boolean
  isWatching: boolean
  error: string | null
  searchQuery: string
  sortBy: 'recent' | 'tokens' | 'cost' | 'name'
}

export const useDevtoolsStore = defineStore('devtools', {
  state: (): DevToolsState => ({
    projects: [],
    sessions: [],
    selectedProjectId: null,
    selectedSessionId: null,
    activeSession: null,
    isLoadingProjects: false,
    isLoadingSessions: false,
    isLoadingSession: false,
    isWatching: false,
    error: null,
    searchQuery: '',
    sortBy: 'recent',
  }),

  getters: {
    filteredProjects(state): DevToolsProject[] {
      const query = state.searchQuery.toLowerCase()
      const filtered = query
        ? state.projects.filter(
            (p) =>
              p.name.toLowerCase().includes(query) ||
              p.path.toLowerCase().includes(query)
          )
        : state.projects

      switch (state.sortBy) {
        case 'name':
          return [...filtered].sort((a, b) => a.name.localeCompare(b.name))
        case 'tokens':
          return filtered // Already sorted by backend
        case 'cost':
          return filtered
        default:
          return filtered // Already sorted by most_recent from backend
      }
    },

    selectedProject(state): DevToolsProject | null {
      return (
        state.projects.find((p) => p.id === state.selectedProjectId) ?? null
      )
    },

    totalCost(state): number {
      return state.sessions.reduce((sum, s) => sum + s.metrics.cost_usd, 0)
    },

    totalTokens(state): number {
      return state.sessions.reduce((sum, s) => sum + s.metrics.total_tokens, 0)
    },
  },

  actions: {
    async loadProjects() {
      this.isLoadingProjects = true
      this.error = null
      try {
        this.projects = await invoke<DevToolsProject[]>(
          'devtools_list_projects'
        )
      } catch (e) {
        this.error = `Failed to load projects: ${e}`
      } finally {
        this.isLoadingProjects = false
      }
    },

    async selectProject(projectId: string) {
      this.selectedProjectId = projectId
      this.selectedSessionId = null
      this.activeSession = null
      this.isLoadingSessions = true
      this.error = null
      try {
        this.sessions = await invoke<SessionSummary[]>(
          'devtools_list_sessions',
          { projectId }
        )
      } catch (e) {
        this.error = `Failed to load sessions: ${e}`
      } finally {
        this.isLoadingSessions = false
      }
    },

    async selectSession(sessionId: string, projectId?: string) {
      const resolvedProjectId = projectId ?? this.selectedProjectId
      if (!resolvedProjectId) return
      this.selectedSessionId = sessionId
      this.isLoadingSession = true
      this.error = null
      try {
        this.activeSession = await invoke<SessionDetail>(
          'devtools_get_session',
          {
            projectId: resolvedProjectId,
            sessionId,
          }
        )
      } catch (e) {
        this.error = `Failed to load session: ${e}`
      } finally {
        this.isLoadingSession = false
      }
    },

    async startWatching() {
      if (this.isWatching) return
      try {
        await invoke('devtools_start_watching')
        this.isWatching = true
      } catch (e) {
        this.error = `Failed to start file watcher: ${e}`
      }
    },

    async stopWatching() {
      if (!this.isWatching) return
      try {
        await invoke('devtools_stop_watching')
        this.isWatching = false
      } catch (e) {
        this.error = `Failed to stop file watcher: ${e}`
      }
    },

    async handleFileChange(path: string) {
      // Only reload session detail if the changed file matches the active session
      if (
        this.activeSession &&
        this.selectedProjectId &&
        this.selectedSessionId
      ) {
        const fileName = path.replace(/\\/g, '/').split('/').pop() ?? ''
        const stem = fileName.replace(/\.jsonl$/, '')
        if (stem === this.selectedSessionId) {
          await this.selectSession(this.selectedSessionId)
        }
      }
      // Refresh session list (lightweight quick_scan)
      if (this.selectedProjectId) {
        try {
          this.sessions = await invoke<SessionSummary[]>(
            'devtools_list_sessions',
            { projectId: this.selectedProjectId }
          )
        } catch (e) {
          this.error = `Failed to refresh sessions: ${e}`
        }
      }
    },

    clearError() {
      this.error = null
    },
  },
})
