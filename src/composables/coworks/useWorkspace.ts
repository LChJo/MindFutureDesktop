/**
 * Workspace composable for Coworks
 * Manages workspace list and recent usage
 * Only persists workspace paths to disk (not message history)
 */

import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { AppMode } from '@/types/coworks'

export interface Workspace {
  path: string
  mode: AppMode
  lastUsed: number
}

// Rust API types
interface RustRecentWorkspace {
  path: string
  name: string
  lastOpened: number
}

interface RustCodeProjectSession {
  projectPath: string
  sdkSessionId: string
  updatedAt: number
  model?: string
}

// Singleton state
const workspaces = ref<Workspace[]>([])
const currentWorkspace = ref<Workspace | null>(null)

// Project sessions for Code mode (projectPath -> sdkSessionId)
const projectSessions = ref<Map<string, string>>(new Map())

// Project models for Code mode (projectPath -> model)
const projectModels = ref<Map<string, string>>(new Map())

// Maximum recent workspaces to keep
const MAX_RECENT_WORKSPACES = 10

export function useWorkspace() {
  // Recent workspaces, sorted by last used time
  const recentWorkspaces = computed(() => {
    return [...workspaces.value]
      .sort((a, b) => b.lastUsed - a.lastUsed)
      .slice(0, MAX_RECENT_WORKSPACES)
  })

  // Recent cowork mode workspaces
  const recentWorkDirs = computed(() => {
    return recentWorkspaces.value.filter(w => w.mode === 'cowork')
  })

  // Recent code mode projects
  const recentProjects = computed(() => {
    return recentWorkspaces.value.filter(w => w.mode === 'code')
  })

  // Initialize workspaces from storage
  async function initialize() {
    try {
      // Load cowork mode workspaces
      const coworkItems = await invoke<RustRecentWorkspace[]>('coworks_get_recent_workspaces', { mode: 'cowork' })
      const codeItems = await invoke<RustRecentWorkspace[]>('coworks_get_recent_workspaces', { mode: 'code' })

      workspaces.value = [
        ...coworkItems.map(item => ({
          path: item.path,
          mode: 'cowork' as AppMode,
          lastUsed: item.lastOpened,
        })),
        ...codeItems.map(item => ({
          path: item.path,
          mode: 'code' as AppMode,
          lastUsed: item.lastOpened,
        })),
      ]

      // Load project sessions for Code mode
      try {
        const sessions = await invoke<RustCodeProjectSession[]>('coworks_list_project_sessions')
        sessions.forEach(s => {
          projectSessions.value.set(s.projectPath, s.sdkSessionId)
          if (s.model) {
            projectModels.value.set(s.projectPath, s.model)
          }
        })
      } catch (e) {
        console.error('Failed to load project sessions:', e)
      }
    } catch (e) {
      console.error('Failed to load workspaces:', e)
    }
  }

  // Set current workspace
  async function setCurrentWorkspace(path: string, mode: AppMode) {
    // Find existing or create new
    let workspace = workspaces.value.find(w => w.path === path && w.mode === mode)

    if (workspace) {
      // Update last used time
      workspace.lastUsed = Date.now()
    } else {
      // Create new workspace
      workspace = {
        path,
        mode,
        lastUsed: Date.now(),
      }
      workspaces.value.push(workspace)
    }

    currentWorkspace.value = workspace

    // Persist to storage
    await saveWorkspaces(mode)
  }

  // Remove a workspace from history
  async function removeWorkspace(path: string, mode: AppMode) {
    const index = workspaces.value.findIndex(w => w.path === path && w.mode === mode)
    if (index !== -1) {
      workspaces.value.splice(index, 1)
      await saveWorkspaces(mode)
    }

    // Clear current if it was the removed one
    if (currentWorkspace.value?.path === path && currentWorkspace.value?.mode === mode) {
      currentWorkspace.value = null
    }
  }

  // Save workspaces to storage for a specific mode
  async function saveWorkspaces(mode: AppMode) {
    try {
      // Get workspaces for this mode
      const modeWorkspaces = workspaces.value
        .filter(w => w.mode === mode)
        .sort((a, b) => b.lastUsed - a.lastUsed)
        .slice(0, MAX_RECENT_WORKSPACES)

      // Convert to Rust format
      const rustWorkspaces: RustRecentWorkspace[] = modeWorkspaces.map(w => ({
        path: w.path,
        name: w.path.split(/[/\\]/).pop() || w.path,
        lastOpened: w.lastUsed,
      }))

      await invoke('coworks_save_recent_workspaces', {
        mode,
        workspaces: rustWorkspaces
      })
    } catch (e) {
      console.error('Failed to save workspaces:', e)
    }
  }

  // Clear all workspaces
  async function clearWorkspaces() {
    workspaces.value = []
    currentWorkspace.value = null
    await saveWorkspaces('cowork')
    await saveWorkspaces('code')
  }

  // Get display name for a workspace path
  function getWorkspaceName(path: string): string {
    return path.split(/[/\\]/).pop() || path
  }

  // Get SDK session ID for a project (Code mode)
  function getProjectSdkSessionId(projectPath: string): string | undefined {
    return projectSessions.value.get(projectPath)
  }

  // Set SDK session ID for a project (Code mode)
  async function setProjectSdkSessionId(projectPath: string, sdkSessionId: string) {
    projectSessions.value.set(projectPath, sdkSessionId)
    // Persist to storage (include model if available)
    const model = projectModels.value.get(projectPath)
    try {
      await invoke('coworks_save_project_session', {
        projectPath,
        sdkSessionId,
        model: model || null,
      })
    } catch (e) {
      console.error('Failed to save project session:', e)
    }
  }

  // Get model for a project (Code mode)
  function getProjectModel(projectPath: string): string | undefined {
    return projectModels.value.get(projectPath)
  }

  // Set model for a project (Code mode)
  async function setProjectModel(projectPath: string, model: string) {
    projectModels.value.set(projectPath, model)
    // Persist to storage (include sdkSessionId if available)
    const sdkSessionId = projectSessions.value.get(projectPath) || ''
    try {
      await invoke('coworks_save_project_session', {
        projectPath,
        sdkSessionId,
        model,
      })
    } catch (e) {
      console.error('Failed to save project model:', e)
    }
  }

  return {
    workspaces,
    currentWorkspace,
    recentWorkspaces,
    recentWorkDirs,
    recentProjects,
    initialize,
    setCurrentWorkspace,
    removeWorkspace,
    clearWorkspaces,
    getWorkspaceName,
    getProjectSdkSessionId,
    setProjectSdkSessionId,
    getProjectModel,
    setProjectModel,
  }
}
