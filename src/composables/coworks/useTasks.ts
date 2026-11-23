/**
 * Tasks composable for Coworks
 * Manages tasks for Cowork mode
 * Persists to ~/.coworks/tasks/
 */

import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ChatMessage } from '@/types/coworks'
import type { CoworksTask, TaskIndexItem, RelatedFile, TaskSkillInfo } from '@/types/coworks-persistence'

// Rust API types
interface RustTaskIndexEntry {
  id: string
  name: string
  workDir: string
  status: string
  createdAt: number
  updatedAt: number
}

interface RustCoworksTask {
  id: string
  name: string
  appId: string | null
  workDir: string
  messages: ChatMessage[]
  relatedFiles: RelatedFile[]
  workLogs: { id: string; action: string; timestamp: number; details?: string }[]
  createdAt: number
  updatedAt: number
  status: string
  skillInfo?: TaskSkillInfo | null
  sdkSessionId?: string | null
  model?: string | null
}

// Generate unique ID
function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
}

// Get directory name from path
function getDirName(path: string): string {
  return path.split(/[/\\]/).pop() || path
}

// Singleton state
const tasks = ref<TaskIndexItem[]>([])
const currentTaskId = ref<string | null>(null)
const currentTask = ref<CoworksTask | null>(null)
const isLoading = ref(false)

export function useTasks() {
  // Sorted tasks (newest first)
  const sortedTasks = computed(() => {
    return [...tasks.value].sort((a, b) => b.updatedAt - a.updatedAt)
  })

  // Active tasks
  const activeTasks = computed(() => {
    return sortedTasks.value.filter(t => t.status === 'active')
  })

  // Initialize from storage
  async function initialize() {
    try {
      const items = await invoke<RustTaskIndexEntry[]>('coworks_list_tasks')
      tasks.value = items.map(item => ({
        id: item.id,
        name: item.name,
        workDir: item.workDir,
        status: item.status as 'active' | 'completed' | 'archived',
        createdAt: item.createdAt,
        updatedAt: item.updatedAt,
        messageCount: 0,
      }))
    } catch (e) {
      console.error('[useTasks] Failed to load index:', e)
    }
  }

  // Create a new task
  async function createTask(
    name: string,
    workDir: string,
    appId?: string,
    skillInfo?: TaskSkillInfo
  ): Promise<string> {
    const id = generateId()
    const now = Date.now()

    const task: RustCoworksTask = {
      id,
      name,
      appId: appId || null,
      workDir,
      messages: [],
      relatedFiles: [],
      workLogs: [],
      createdAt: now,
      updatedAt: now,
      status: 'active',
      skillInfo: skillInfo || null,
    }

    // Save task (this also updates the index)
    await invoke('coworks_save_task', { task })

    // Update local state
    tasks.value.push({
      id,
      name,
      appId,
      workDir,
      status: 'active',
      createdAt: now,
      updatedAt: now,
      messageCount: 0,
    })

    // Set as current
    currentTaskId.value = id
    currentTask.value = {
      ...task,
      appId,
      status: 'active',
      skillInfo,
      sdkSessionId: undefined,
      model: undefined,
    }

    return id
  }

  // Load a task by ID
  async function loadTask(id: string): Promise<CoworksTask | null> {
    try {
      isLoading.value = true
      const task = await invoke<RustCoworksTask>('coworks_load_task', { id })
      const converted: CoworksTask = {
        ...task,
        appId: task.appId || undefined,
        status: task.status as 'active' | 'completed' | 'archived',
        skillInfo: task.skillInfo || undefined,
        sdkSessionId: task.sdkSessionId || undefined,
        model: task.model || undefined,
      }
      currentTaskId.value = id
      currentTask.value = converted
      return converted
    } catch (e) {
      console.error('[useTasks] Failed to load task:', e)
      return null
    } finally {
      isLoading.value = false
    }
  }

  // Save current task
  async function saveCurrentTask() {
    if (!currentTask.value) return

    const task = currentTask.value
    task.updatedAt = Date.now()

    // Convert to Rust format
    const rustTask: RustCoworksTask = {
      ...task,
      appId: task.appId || null,
      skillInfo: task.skillInfo || null,
      sdkSessionId: task.sdkSessionId || null,
      model: task.model || null,
    }

    // Save task (this also updates the index)
    await invoke('coworks_save_task', { task: rustTask })

    // Update local index
    const indexItem = tasks.value.find(t => t.id === task.id)
    if (indexItem) {
      indexItem.name = task.name
      indexItem.status = task.status
      indexItem.updatedAt = task.updatedAt
      indexItem.messageCount = task.messages.length
    }
  }

  // Save a specific task by reference (for background session persistence)
  async function saveTaskRef(task: CoworksTask) {
    task.updatedAt = Date.now()
    const rustTask: RustCoworksTask = {
      ...task,
      appId: task.appId || null,
      skillInfo: task.skillInfo || null,
      sdkSessionId: task.sdkSessionId || null,
      model: task.model || null,
    }
    await invoke('coworks_save_task', { task: rustTask })
    const indexItem = tasks.value.find(t => t.id === task.id)
    if (indexItem) {
      indexItem.name = task.name
      indexItem.status = task.status
      indexItem.updatedAt = task.updatedAt
      indexItem.messageCount = task.messages.length
    }
  }

  // Add message to current task
  function addMessage(message: ChatMessage) {
    if (!currentTask.value) return
    currentTask.value.messages.push(message)
  }

  // Update last message (for streaming)
  function updateLastMessage(update: Partial<ChatMessage>) {
    if (!currentTask.value || currentTask.value.messages.length === 0) return
    const lastIndex = currentTask.value.messages.length - 1
    currentTask.value.messages[lastIndex] = {
      ...currentTask.value.messages[lastIndex],
      ...update,
    }
  }

  // Get messages of current task
  const messages = computed(() => currentTask.value?.messages || [])

  // Add related file
  function addRelatedFile(file: RelatedFile) {
    if (!currentTask.value) return
    // Check if already exists with same path and operation
    const exists = currentTask.value.relatedFiles.some(
      f => f.path === file.path && f.operation === file.operation
    )
    if (!exists) {
      currentTask.value.relatedFiles.push(file)
    }
  }

  // Add work log
  function addWorkLog(action: string, details?: string) {
    if (!currentTask.value) return
    currentTask.value.workLogs.push({
      id: generateId(),
      action,
      timestamp: Date.now(),
      details,
    })
  }

  // Get related files of current task
  const relatedFiles = computed(() => currentTask.value?.relatedFiles || [])

  // Get work logs of current task
  const workLogs = computed(() => currentTask.value?.workLogs || [])

  // Complete a task
  async function completeTask(id: string) {
    const task = currentTask.value?.id === id ? currentTask.value : null
    if (task) {
      task.status = 'completed'
      await saveCurrentTask()
    } else {
      // Load, update, save
      const loaded = await loadTask(id)
      if (loaded) {
        loaded.status = 'completed'
        await saveCurrentTask()
      }
    }
  }

  // Archive a task
  async function archiveTask(id: string) {
    const task = currentTask.value?.id === id ? currentTask.value : null
    if (task) {
      task.status = 'archived'
      await saveCurrentTask()
    } else {
      // Load, update, save
      const loaded = await loadTask(id)
      if (loaded) {
        loaded.status = 'archived'
        await saveCurrentTask()
      }
    }
  }

  // Delete a task
  async function deleteTask(id: string) {
    // Delete from backend (this also updates the index)
    await invoke('coworks_delete_task_item', { id })

    // Update local index
    const index = tasks.value.findIndex(t => t.id === id)
    if (index !== -1) {
      tasks.value.splice(index, 1)
    }

    // Clear current if it was deleted
    if (currentTaskId.value === id) {
      currentTaskId.value = null
      currentTask.value = null
    }
  }

  // Clear current task (without deleting from history)
  function clearCurrentTask() {
    currentTaskId.value = null
    currentTask.value = null
  }

  // Get display name for task (includes directory name)
  function getTaskDisplayName(task: TaskIndexItem): string {
    const dirName = getDirName(task.workDir)
    return `${task.name} (${dirName})`
  }

  // Set SDK session ID for current task (for conversation continuity)
  function setSdkSessionId(sdkSessionId: string) {
    if (currentTask.value) {
      currentTask.value.sdkSessionId = sdkSessionId
    }
  }

  return {
    tasks: sortedTasks,
    activeTasks,
    currentTaskId,
    currentTask,
    messages,
    relatedFiles,
    workLogs,
    isLoading,
    initialize,
    createTask,
    loadTask,
    saveCurrentTask,
    saveTaskRef,
    addMessage,
    updateLastMessage,
    addRelatedFile,
    addWorkLog,
    completeTask,
    archiveTask,
    deleteTask,
    clearCurrentTask,
    getTaskDisplayName,
    setSdkSessionId,
  }
}
