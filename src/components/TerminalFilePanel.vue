<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ask } from '@tauri-apps/plugin-dialog'

interface FileEntry {
  name: string
  path: string
  isDir: boolean
  size: number
  modified: string
}

interface GitStatus {
  is_git_repo: boolean
  branch: string | null
  has_changes: boolean
  ahead: number
  behind: number
}

const props = defineProps<{
  cwd: string
}>()

const emit = defineEmits<{
  (e: 'open-file', path: string): void
  (e: 'send-shift-tab'): void
  (e: 'panel-toggled', expanded: boolean): void
}>()

// Send Shift+Tab to terminal (for Claude Code mode switching)
const sendShiftTab = () => {
  emit('send-shift-tab')
}

const isCollapsed = ref(false)
const files = ref<FileEntry[]>([])
const loading = ref(false)
const error = ref('')
const currentPath = ref('')
const isProcessing = ref(false) // Prevent double-click issues

let refreshTimer: ReturnType<typeof setTimeout> | null = null

const scheduleRefresh = () => {
  if (isCollapsed.value || !currentPath.value || loading.value) return
  if (refreshTimer) clearTimeout(refreshTimer)
  refreshTimer = setTimeout(() => {
    loadDirectory(currentPath.value)
  }, 500)
}
const gitStatus = ref<GitStatus>({
  is_git_repo: false,
  branch: null,
  has_changes: false,
  ahead: 0,
  behind: 0
})

// File statistics
const fileStats = computed(() => {
  const dirs = files.value.filter(f => f.isDir).length
  const fileCount = files.value.filter(f => !f.isDir).length
  return { dirs, files: fileCount, total: files.value.length }
})

// Get directory name from path
const dirName = computed(() => {
  if (!currentPath.value) return ''
  const parts = currentPath.value.split(/[/\\]/)
  return parts[parts.length - 1] || parts[parts.length - 2] || ''
})

// Context menu
const contextMenu = ref<{ show: boolean; x: number; y: number; item: FileEntry | null }>({
  show: false,
  x: 0,
  y: 0,
  item: null
})

const togglePanel = () => {
  const newCollapsed = !isCollapsed.value
  isCollapsed.value = newCollapsed
  emit('panel-toggled', !newCollapsed)
}

const loadDirectory = async (path: string) => {
  if (!path) return

  loading.value = true
  error.value = ''

  try {
    files.value = await invoke<FileEntry[]>('list_directory', { path })
    currentPath.value = path
    // Load git status
    loadGitStatus(path)
  } catch (e) {
    error.value = String(e)
    files.value = []
  } finally {
    loading.value = false
  }
}

const loadGitStatus = async (path: string) => {
  try {
    gitStatus.value = await invoke<GitStatus>('get_git_status', { path })
  } catch (e) {
    gitStatus.value = {
      is_git_repo: false,
      branch: null,
      has_changes: false,
      ahead: 0,
      behind: 0
    }
  }
}

const handleDoubleClick = async (item: FileEntry) => {
  // Prevent double-click from triggering multiple times
  if (isProcessing.value) return
  isProcessing.value = true
  setTimeout(() => { isProcessing.value = false }, 300)

  if (item.isDir) {
    // Navigate into directory (panel only, no terminal command)
    await loadDirectory(item.path)
  } else {
    // Open file with default application
    try {
      await invoke('open_file_default', { path: item.path })
    } catch (e) {
      console.error('Failed to open file:', e)
    }
  }
}

const goUp = async () => {
  if (!currentPath.value) return
  const parent = currentPath.value.replace(/[\\/][^\\/]+$/, '')
  if (parent && parent !== currentPath.value) {
    await loadDirectory(parent)
  }
}

const refresh = () => {
  loadDirectory(currentPath.value)
}

const showContextMenu = (event: MouseEvent, item: FileEntry) => {
  contextMenu.value = {
    show: true,
    x: event.clientX,
    y: event.clientY,
    item
  }
}

const hideContextMenu = () => {
  contextMenu.value.show = false
}

const handleContextAction = async (action: string) => {
  const item = contextMenu.value.item
  if (!item) return

  hideContextMenu()

  switch (action) {
    case 'open':
      await invoke('open_file_default', { path: item.path })
      break
    case 'reveal':
      await invoke('reveal_in_explorer', { path: item.path })
      break
    case 'copy-path':
      navigator.clipboard.writeText(item.path)
      break
    case 'delete': {
      const confirmed = await ask(`Delete "${item.name}"?`, {
        title: '删除确认',
        kind: 'warning',
      })
      if (confirmed) {
        try {
          await invoke('delete_file', { path: item.path })
          refresh()
        } catch (e) {
          alert(`Failed to delete: ${e}`)
        }
      }
      break
    }
  }
}

const openInExplorer = async () => {
  if (currentPath.value) {
    try {
      await invoke('reveal_in_explorer', { path: currentPath.value })
    } catch (e) {
      console.error('Failed to open in explorer:', e)
    }
  }
}

const getFileIcon = (item: FileEntry) => {
  if (item.isDir) return 'fa fa-folder'
  const ext = item.name.split('.').pop()?.toLowerCase()
  switch (ext) {
    case 'js':
    case 'ts':
    case 'jsx':
    case 'tsx':
      return 'fa fa-file-code'
    case 'json':
    case 'xml':
    case 'yaml':
    case 'yml':
      return 'fa fa-file-alt'
    case 'md':
    case 'txt':
      return 'fa fa-file-lines'
    case 'png':
    case 'jpg':
    case 'jpeg':
    case 'gif':
    case 'svg':
      return 'fa fa-file-image'
    default:
      return 'fa fa-file'
  }
}

// Watch for cwd changes from parent
watch(() => props.cwd, (newCwd) => {
  if (newCwd && newCwd !== currentPath.value) {
    loadDirectory(newCwd)
  }
}, { immediate: true })

watch(isCollapsed, (collapsed) => {
  if (!collapsed && currentPath.value) {
    loadDirectory(currentPath.value)
  }
})

onMounted(() => {
  document.addEventListener('click', hideContextMenu)
})

onUnmounted(() => {
  if (refreshTimer) clearTimeout(refreshTimer)
  document.removeEventListener('click', hideContextMenu)
})

defineExpose({ scheduleRefresh })
</script>

<template>
  <div
    class="terminal-file-panel"
    :class="{ collapsed: isCollapsed }"
  >
    <!-- Toggle button -->
    <div
      class="panel-toggle"
      :title="isCollapsed ? 'Show files' : 'Hide files'"
      @click="togglePanel"
    >
      <i :class="isCollapsed ? 'fa fa-chevron-right' : 'fa fa-chevron-left'" />
    </div>

    <!-- Panel content -->
    <div
      v-show="!isCollapsed"
      class="panel-content"
    >
      <!-- Status bar with directory info -->
      <div class="status-bar">
        <div class="dir-info">
          <i class="fa fa-folder-open dir-icon" />
          <span class="dir-name">{{ dirName || 'No directory' }}</span>
        </div>
        <div
          v-if="gitStatus.is_git_repo"
          class="git-info"
        >
          <i class="fa fa-code-branch git-icon" />
          <span class="branch-name">{{ gitStatus.branch || 'unknown' }}</span>
          <span
            v-if="gitStatus.has_changes"
            class="git-dirty"
            title="Uncommitted changes"
          >*</span>
          <span
            v-if="gitStatus.ahead > 0"
            class="git-ahead"
            :title="`${gitStatus.ahead} commit(s) ahead`"
          >
            <i class="fa fa-arrow-up" />{{ gitStatus.ahead }}
          </span>
          <span
            v-if="gitStatus.behind > 0"
            class="git-behind"
            :title="`${gitStatus.behind} commit(s) behind`"
          >
            <i class="fa fa-arrow-down" />{{ gitStatus.behind }}
          </span>
        </div>
      </div>

      <!-- Header with actions -->
      <div class="panel-header">
        <button
          class="icon-btn"
          title="Shift+Tab (Mode Switch)"
          @click="sendShiftTab"
        >
          <i class="fa fa-exchange-alt" />
        </button>
        <button
          class="icon-btn"
          title="Go up"
          @click="goUp"
        >
          <i class="fa fa-arrow-up" />
        </button>
        <button
          class="icon-btn"
          title="Refresh"
          @click="refresh"
        >
          <i class="fa fa-refresh" />
        </button>
        <button
          class="icon-btn"
          title="Open in Explorer"
          @click="openInExplorer"
        >
          <i class="fa fa-folder-open" />
        </button>
        <div
          class="stats-badge"
          :title="`${fileStats.dirs} folders, ${fileStats.files} files`"
        >
          {{ fileStats.total }}
        </div>
      </div>

      <!-- Current path display -->
      <div
        class="current-path"
        :title="currentPath"
      >
        {{ currentPath || 'No directory' }}
      </div>

      <!-- File list -->
      <div class="file-list">
        <div
          v-if="loading"
          class="loading"
        >
          <i class="fa fa-spinner fa-spin" /> Loading...
        </div>
        <div
          v-else-if="error"
          class="error"
        >
          {{ error }}
        </div>
        <div
          v-else-if="files.length === 0"
          class="empty"
        >
          Empty directory
        </div>
        <div
          v-for="item in files"
          v-else
          :key="item.path"
          class="file-item"
          :class="{ 'is-dir': item.isDir }"
          @dblclick="handleDoubleClick(item)"
          @contextmenu.prevent="showContextMenu($event, item)"
        >
          <i :class="getFileIcon(item)" />
          <span class="file-name">{{ item.name }}</span>
        </div>
      </div>
    </div>

    <!-- Context menu -->
    <div
      v-if="contextMenu.show"
      class="context-menu"
      :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
    >
      <div
        class="menu-item"
        @click="handleContextAction('open')"
      >
        <i class="fa fa-external-link" /> Open
      </div>
      <div
        class="menu-item"
        @click="handleContextAction('reveal')"
      >
        <i class="fa fa-folder-open" /> Reveal in Explorer
      </div>
      <div
        class="menu-item"
        @click="handleContextAction('copy-path')"
      >
        <i class="fa fa-copy" /> Copy Path
      </div>
      <div class="menu-separator" />
      <div
        class="menu-item danger"
        @click="handleContextAction('delete')"
      >
        <i class="fa fa-trash" /> Delete
      </div>
    </div>
  </div>
</template>

<style lang="less" scoped>
// One Dark color palette
@bg-dark: #21252b;
@bg-main: #282c34;
@bg-highlight: #2c313a;
@bg-selection: #3e4451;
@text-primary: #abb2bf;
@text-secondary: #5c6370;
@accent-blue: #61afef;
@accent-green: #98c379;
@accent-yellow: #e5c07b;
@accent-red: #e06c75;
@accent-magenta: #c678dd;
@accent-cyan: #56b6c2;
@border-color: #3e4451;

.terminal-file-panel {
  display: flex;
  flex-shrink: 0;
  background: @bg-dark;
  border-right: 1px solid @border-color;
  position: relative;
  width: 200px;

  &.collapsed {
    width: 24px;

    .panel-content {
      display: none;
    }
  }
}

.panel-toggle {
  width: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  color: @text-secondary;
  flex-shrink: 0;

  &:hover {
    color: @text-primary;
    background: @bg-highlight;
  }
}

.panel-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}

// Enhanced status bar
.status-bar {
  padding: 8px;
  background: linear-gradient(180deg, @bg-highlight 0%, @bg-dark 100%);
  border-bottom: 1px solid @border-color;
}

.dir-info {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;

  .dir-icon {
    color: @accent-yellow;
    font-size: 14px;
  }

  .dir-name {
    font-size: 13px;
    font-weight: 600;
    color: @text-primary;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.git-info {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;

  .git-icon {
    color: @accent-magenta;
  }

  .branch-name {
    color: @accent-magenta;
    font-weight: 500;
    max-width: 100px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-dirty {
    color: @accent-yellow;
    font-weight: bold;
  }

  .git-ahead {
    color: @accent-green;
    display: flex;
    align-items: center;
    gap: 2px;
    font-size: 10px;

    i {
      font-size: 8px;
    }
  }

  .git-behind {
    color: @accent-red;
    display: flex;
    align-items: center;
    gap: 2px;
    font-size: 10px;

    i {
      font-size: 8px;
    }
  }
}

.panel-header {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px;
  border-bottom: 1px solid @border-color;
  background: @bg-dark;
}

.icon-btn {
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  color: @text-secondary;
  cursor: pointer;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;

  &:hover {
    background: @bg-selection;
    color: @accent-blue;
  }
}

.stats-badge {
  margin-left: auto;
  padding: 2px 6px;
  font-size: 10px;
  background: @bg-selection;
  color: @text-secondary;
  border-radius: 10px;
}

.current-path {
  padding: 4px 8px;
  font-size: 10px;
  color: @text-secondary;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  border-bottom: 1px solid @border-color;
  background: @bg-dark;
}

.file-list {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  background: @bg-dark;

  &::-webkit-scrollbar {
    width: 6px;
  }

  &::-webkit-scrollbar-track {
    background: @bg-dark;
  }

  &::-webkit-scrollbar-thumb {
    background: @bg-selection;
    border-radius: 3px;

    &:hover {
      background: @text-secondary;
    }
  }
}

.file-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  cursor: pointer;
  color: @text-primary;
  font-size: 13px;

  &:hover {
    background: @bg-highlight;
  }

  &.is-dir {
    color: @accent-yellow;

    i {
      color: @accent-yellow;
    }
  }

  i {
    width: 16px;
    text-align: center;
    flex-shrink: 0;
    color: @accent-blue;
  }

  .file-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.loading, .error, .empty {
  padding: 16px;
  text-align: center;
  color: @text-secondary;
  font-size: 12px;
}

.error {
  color: @accent-red;
}

.context-menu {
  position: fixed;
  background: @bg-main;
  border: 1px solid @border-color;
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  z-index: 9999;
  min-width: 150px;
  padding: 4px 0;
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  cursor: pointer;
  color: @text-primary;
  font-size: 13px;

  &:hover {
    background: @accent-blue;
    color: #fff;
  }

  &.danger {
    color: @accent-red;

    &:hover {
      background: @accent-red;
      color: #fff;
    }
  }

  i {
    width: 16px;
    text-align: center;
  }
}

.menu-separator {
  height: 1px;
  background: @border-color;
  margin: 4px 0;
}
</style>
