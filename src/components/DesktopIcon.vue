<script setup lang="ts">
import { ref, computed, onUnmounted, nextTick } from 'vue'
import type { DesktopIcon } from '@/types/models'
import { useDesktopStore } from '@/stores/desktopStore'
import { useWindowStore } from '@/stores/windowStore'
import { invoke } from '@tauri-apps/api/core'
import { exists, mkdir } from '@tauri-apps/plugin-fs'
import { homeDir, join } from '@tauri-apps/api/path'

const props = defineProps<{
  icon: DesktopIcon
}>()

const emit = defineEmits<{
  click: [icon: DesktopIcon]
  drag: [iconId: string, position: { x: number; y: number }]
}>()

const desktopStore = useDesktopStore()
const windowStore = useWindowStore()

const isDragging = ref(false)
const dragOffset = ref({ x: 0, y: 0 })
const currentDragPosition = ref({ x: 0, y: 0 })
const contextMenuVisible = ref(false)
const contextMenuPosition = ref({ x: 0, y: 0 })
const contextMenuRef = ref<HTMLElement | null>(null)
const clickStartTime = ref(0)
const clickStartPosition = ref({ x: 0, y: 0 })
const isExecutingCommand = ref(false) // Prevent double-click on context menu items

// Check if this icon can be deleted
const isDeletable = computed(() => desktopStore.isIconDeletable(props.icon.id))

// Check if this is a project link
const isProjectLink = computed(() => props.icon.type === 'project-link')

// Check if this is a plugin shortcut
const isPluginShortcut = computed(() => props.icon.type === 'plugin-shortcut')

// Check if this is a dynamic function
const isDynamicFunction = computed(() => props.icon.type === 'dynamic-function')

// Environment variables dialog state
const envDialogVisible = ref(false)
const editingEnvVars = ref<Record<string, string>>({})
const newEnvKey = ref('')
const newEnvValue = ref('')

// Custom args dialog state
const argsDialogVisible = ref(false)
const customArgsInput = ref('')

// Rename dialog state
const renameDialogVisible = ref(false)
const renameInput = ref('')

// Check if the focused window belongs to this icon
const isFocused = computed(() => {
  // Find if any visible window with this icon's id is currently active
  const activeId = windowStore.activeWindowId
  if (!activeId) return false

  const activeWindow = windowStore.windows.find(w => w.id === activeId)
  return activeWindow && activeWindow.iconId === props.icon.id
})

// Claude commands for project links
const claudeCommands = [
  { command: '', label: '启动终端', icon: 'fa-solid fa-terminal', description: '只打开终端，不自动执行命令' },
  { command: 'mcd', label: '通过路由启动', icon: 'fa-solid fa-route', description: '通过 LLM 路由启动 Claude Code' },
  { command: 'claude -c', label: '继续对话', icon: 'fa-solid fa-comments', description: '继续最近的对话' },
  { command: 'claude --dangerously-skip-permissions', label: '完全信任', icon: 'fa-solid fa-shield-halved', description: '跳过所有权限确认' },
  { command: 'claude /init', label: '初始化项目', icon: 'fa-solid fa-wand-magic-sparkles', description: '创建 CLAUDE.md' },
  { command: 'claude /review', label: '代码审查', icon: 'fa-solid fa-magnifying-glass-chart', description: '请求代码审查' }
]

// Confirmation dialog state
const confirmDialogVisible = ref(false)

// Constants for drag detection
const DRAG_THRESHOLD = 5 // pixels
const CLICK_TIMEOUT = 200 // ms

// Computed snapped position for visual feedback
const snappedPosition = computed(() => {
  if (!isDragging.value) return props.icon.position
  const gridSize = desktopStore.gridSize
  return {
    x: Math.round(currentDragPosition.value.x / gridSize) * gridSize,
    y: Math.round(currentDragPosition.value.y / gridSize) * gridSize
  }
})

const handleClick = () => {
  if (!isDragging.value) {
    emit('click', props.icon)
  }
}

// Pointer-based drag handling for smoother experience
const handlePointerDown = (event: PointerEvent) => {
  if (event.button !== 0) return // Only left click

  clickStartTime.value = Date.now()
  clickStartPosition.value = { x: event.clientX, y: event.clientY }
  dragOffset.value = {
    x: event.clientX - props.icon.position.x,
    y: event.clientY - props.icon.position.y
  }

  // Capture pointer for smooth tracking
  ;(event.target as HTMLElement).setPointerCapture(event.pointerId)

  // Add move and up listeners
  document.addEventListener('pointermove', handlePointerMove)
  document.addEventListener('pointerup', handlePointerUp)
}

const handlePointerMove = (event: PointerEvent) => {
  const dx = event.clientX - clickStartPosition.value.x
  const dy = event.clientY - clickStartPosition.value.y
  const distance = Math.sqrt(dx * dx + dy * dy)

  // Only start dragging if moved past threshold
  if (!isDragging.value && distance > DRAG_THRESHOLD) {
    isDragging.value = true
  }

  if (isDragging.value) {
    // Calculate new position with bounds checking
    const desktopHeight = window.innerHeight - 48 // Subtract taskbar height
    const desktopWidth = window.innerWidth

    currentDragPosition.value = {
      x: Math.max(0, Math.min(desktopWidth - 80, event.clientX - dragOffset.value.x)),
      y: Math.max(0, Math.min(desktopHeight - 100, event.clientY - dragOffset.value.y))
    }
  }
}

const handlePointerUp = () => {
  document.removeEventListener('pointermove', handlePointerMove)
  document.removeEventListener('pointerup', handlePointerUp)

  const timeDiff = Date.now() - clickStartTime.value

  if (isDragging.value) {
    // Emit final snapped position
    emit('drag', props.icon.id, snappedPosition.value)
    isDragging.value = false
  } else if (timeDiff < CLICK_TIMEOUT) {
    // It was a click, not a drag
    handleClick()
  }
}

// Cleanup on unmount
onUnmounted(() => {
  document.removeEventListener('pointermove', handlePointerMove)
  document.removeEventListener('pointerup', handlePointerUp)
})

const handleContextMenu = (event: MouseEvent) => {
  event.preventDefault()
  event.stopPropagation()

  // Show context menu for deletable icons, project links, plugin shortcuts, or dynamic functions
  if (isDeletable.value || isProjectLink.value || isPluginShortcut.value || isDynamicFunction.value) {
    // Set initial position
    contextMenuPosition.value = { x: event.clientX, y: event.clientY }
    contextMenuVisible.value = true

    // Adjust position after menu renders to prevent overflow
    nextTick(() => {
      if (contextMenuRef.value) {
        const menu = contextMenuRef.value
        const menuRect = menu.getBoundingClientRect()
        const viewportHeight = window.innerHeight
        const viewportWidth = window.innerWidth

        let { x, y } = contextMenuPosition.value

        // Adjust if menu overflows bottom
        if (y + menuRect.height > viewportHeight - 10) {
          y = viewportHeight - menuRect.height - 10
        }

        // Adjust if menu overflows right
        if (x + menuRect.width > viewportWidth - 10) {
          x = viewportWidth - menuRect.width - 10
        }

        // Ensure menu doesn't go above or left of viewport
        x = Math.max(10, x)
        y = Math.max(10, y)

        contextMenuPosition.value = { x, y }
      }
    })
  }
}

// Execute Claude command in terminal
const executeClaudeCommand = (command: string) => {
  // Prevent double-click from opening multiple terminals
  if (isExecutingCommand.value) return
  isExecutingCommand.value = true
  setTimeout(() => { isExecutingCommand.value = false }, 500)

  if (isProjectLink.value && props.icon.command) {
    const finalCommand = command || undefined
    windowStore.openTerminalWithCwd(props.icon.command, props.icon.name, finalCommand, props.icon.id)
  }
  closeContextMenu()
}

const closeContextMenu = () => {
  contextMenuVisible.value = false
}

const handleDelete = () => {
  if (isDeletable.value) {
    desktopStore.removeProjectLink(props.icon.id)
  }
  closeContextMenu()
}

// Close context menu when clicking outside
const handleGlobalClick = () => {
  closeContextMenu()
}

// Show confirmation dialog for fresh start
const showFreshStartDialog = () => {
  closeContextMenu()
  confirmDialogVisible.value = true
}

// Cancel fresh start
const cancelFreshStart = () => {
  confirmDialogVisible.value = false
}

// Open environment variables dialog
const openEnvDialog = () => {
  closeContextMenu()
  // Copy current env vars for editing
  editingEnvVars.value = { ...(props.icon.environmentVariables || {}) }
  newEnvKey.value = ''
  newEnvValue.value = ''
  envDialogVisible.value = true
}

// Close environment variables dialog
const closeEnvDialog = () => {
  envDialogVisible.value = false
}

// Add new environment variable
const addEnvVar = () => {
  const key = newEnvKey.value.trim()
  const value = newEnvValue.value.trim()
  if (key && value) {
    editingEnvVars.value[key] = value
    newEnvKey.value = ''
    newEnvValue.value = ''
  }
}

// Remove environment variable
const removeEnvVar = (key: string) => {
  delete editingEnvVars.value[key]
}

// Save environment variables
const saveEnvVars = () => {
  desktopStore.updateIconEnvironmentVariables(props.icon.id, editingEnvVars.value)
  envDialogVisible.value = false
}

// Open custom args dialog
const openArgsDialog = () => {
  closeContextMenu()
  customArgsInput.value = props.icon.customArgs || 'claude'
  argsDialogVisible.value = true
}

// Close custom args dialog
const closeArgsDialog = () => {
  argsDialogVisible.value = false
}

// Open rename dialog
const openRenameDialog = () => {
  closeContextMenu()
  renameInput.value = props.icon.name
  renameDialogVisible.value = true
}

// Close rename dialog
const closeRenameDialog = () => {
  renameDialogVisible.value = false
}

// Save rename
const saveRename = () => {
  const trimmed = renameInput.value.trim()
  if (trimmed) {
    desktopStore.updateIconName(props.icon.id, trimmed)
  }
  renameDialogVisible.value = false
}

// Save custom args
const saveCustomArgs = () => {
  const trimmed = customArgsInput.value.trim()
  const valueToSave = (trimmed === 'claude' || trimmed === '') ? '' : trimmed
  desktopStore.updateIconCustomArgs(props.icon.id, valueToSave)
  argsDialogVisible.value = false
}

// Reveal folder in explorer
const revealInFolder = async () => {
  closeContextMenu()

  try {
    if (isProjectLink.value && props.icon.command) {
      // For project links, directly reveal the project folder
      await invoke('reveal_in_explorer', { path: props.icon.command })
    } else if (isPluginShortcut.value && props.icon.pluginInfo) {
      // For plugin shortcuts, get or create the plugin directory first (~/.MoontonClaudeDesktop/plugins/pluginName)
      const pluginName = props.icon.pluginInfo.pluginName
      const home = await homeDir()
      const pluginDir = await join(home, '.MoontonClaudeDesktop', 'plugins', pluginName)

      // Create directory if it doesn't exist
      const dirExists = await exists(pluginDir)
      if (!dirExists) {
        await mkdir(pluginDir, { recursive: true })
      }

      // Reveal in explorer
      await invoke('reveal_in_explorer', { path: pluginDir })
    } else if (isDynamicFunction.value && props.icon.dynamicFunctionInfo) {
      const funcName = props.icon.dynamicFunctionInfo.functionName
      const home = await homeDir()
      const funcDir = await join(home, '.MoontonClaudeDesktop', 'dynamic-functions', funcName)
      await invoke('reveal_in_explorer', { path: funcDir })
    }
  } catch (e) {
    console.error('Failed to reveal in folder:', e)
  }
}

// Confirm and execute fresh start - delete .claude folder and start fresh
const confirmFreshStart = () => {
  // Prevent double-click
  if (isExecutingCommand.value) return
  isExecutingCommand.value = true
  setTimeout(() => { isExecutingCommand.value = false }, 500)

  confirmDialogVisible.value = false
  if (isProjectLink.value && props.icon.command) {
    // Convert project path to Claude's project folder name format
    // e.g., "I:\AI\Banana" -> "I--AI-Banana"
    const projectPath = props.icon.command
    const projectFolderName = projectPath.replace(':', '').replace(/\\/g, '-')

    // PowerShell command to delete project-specific Claude files and folders
    // The script:
    // 1. Deletes local .claude/ folder
    // 2. Deletes CLAUDE.md and CLAUDE.local.md
    // 3. Finds all session UUIDs in the project folder
    // 4. Deletes corresponding file-history folders for each session
    // 5. Deletes the project folder in ~/.claude/projects/
    const resetCommand = `
$projectFolder = "$env:USERPROFILE\\.claude\\projects\\${projectFolderName}"
$fileHistoryRoot = "$env:USERPROFILE\\.claude\\file-history"

Write-Host "Cleaning Claude context for this project..." -ForegroundColor Yellow
Write-Host ""

# Delete local .claude folder
if (Test-Path .claude) {
  Remove-Item -Recurse -Force .claude
  Write-Host "  Deleted: .claude/" -ForegroundColor Gray
}

# Delete CLAUDE.md files
if (Test-Path CLAUDE.md) {
  Remove-Item -Force CLAUDE.md
  Write-Host "  Deleted: CLAUDE.md" -ForegroundColor Gray
}
if (Test-Path CLAUDE.local.md) {
  Remove-Item -Force CLAUDE.local.md
  Write-Host "  Deleted: CLAUDE.local.md" -ForegroundColor Gray
}

# Find and delete session-specific file-history folders
if (Test-Path $projectFolder) {
  $sessionFiles = Get-ChildItem -Path $projectFolder -Filter "*.jsonl" -File -ErrorAction SilentlyContinue
  foreach ($sessionFile in $sessionFiles) {
    $sessionId = $sessionFile.BaseName
    # Skip agent files (they start with "agent-")
    if (-not $sessionId.StartsWith("agent-")) {
      $sessionHistoryPath = Join-Path $fileHistoryRoot $sessionId
      if (Test-Path $sessionHistoryPath) {
        Remove-Item -Recurse -Force $sessionHistoryPath
        Write-Host "  Deleted: file-history/$sessionId/" -ForegroundColor Gray
      }
    }
  }
  # Delete the project folder
  Remove-Item -Recurse -Force $projectFolder
  Write-Host "  Deleted: projects/${projectFolderName}/" -ForegroundColor Gray
}

Write-Host ""
Write-Host "Claude context cleared successfully!" -ForegroundColor Green
Write-Host "Starting fresh session..." -ForegroundColor Cyan
Write-Host ""

claude`
    windowStore.openTerminalWithCwd(props.icon.command, props.icon.name, resetCommand, props.icon.id)
  }
}
</script>

<template>
  <!-- Ghost preview showing snapped position -->
  <div
    v-if="isDragging"
    class="desktop-icon ghost"
    :style="{
      left: snappedPosition.x + 'px',
      top: snappedPosition.y + 'px'
    }"
  >
    <div class="icon-wrapper">
      <i :class="icon.icon" />
    </div>
    <span class="icon-label">{{ icon.name }}</span>
  </div>

  <!-- Main icon -->
  <div
    class="desktop-icon"
    :class="{ dragging: isDragging, focused: isFocused }"
    :style="{
      left: (isDragging ? currentDragPosition.x : icon.position.x) + 'px',
      top: (isDragging ? currentDragPosition.y : icon.position.y) + 'px'
    }"
    @pointerdown="handlePointerDown"
    @dblclick.stop="handleClick"
    @contextmenu="handleContextMenu"
  >
    <div class="icon-wrapper">
      <i :class="icon.icon" />
    </div>
    <span class="icon-label">{{ icon.name }}</span>

    <!-- Context Menu for project links and deletable icons -->
    <Teleport to="body">
      <div
        v-if="contextMenuVisible"
        ref="contextMenuRef"
        class="icon-context-menu"
        :style="{ left: contextMenuPosition.x + 'px', top: contextMenuPosition.y + 'px' }"
        @click.stop
      >
        <!-- Claude commands for project links -->
        <template v-if="isProjectLink">
          <div class="context-menu-header">
            Claude Code
          </div>
          <div
            v-for="cmd in claudeCommands"
            :key="cmd.command"
            class="context-menu-item claude-cmd"
            :title="cmd.description"
            @click="executeClaudeCommand(cmd.command)"
          >
            <i :class="cmd.icon" />
            <span>{{ cmd.label }}</span>
          </div>
          <div class="context-menu-separator" />
          <div
            class="context-menu-item danger"
            title="删除所有 Claude 记忆和上下文，重新开始"
            @click="showFreshStartDialog"
          >
            <i class="fa-solid fa-rotate-right" />
            <span>重新开始</span>
          </div>
        </template>

        <!-- Folder option for dynamic functions -->
        <template v-if="isDynamicFunction">
          <div class="context-menu-separator" />
          <div
            class="context-menu-item folder"
            title="在资源管理器中打开文件夹"
            @click="revealInFolder"
          >
            <i class="fa-solid fa-folder-open" />
            <span>打开目录</span>
          </div>
        </template>

        <!-- Environment variables and folder options for project links and plugin shortcuts -->
        <template v-if="isProjectLink || isPluginShortcut">
          <div class="context-menu-separator" />
          <div
            class="context-menu-item folder"
            title="在资源管理器中打开文件夹"
            @click="revealInFolder"
          >
            <i class="fa-solid fa-folder-open" />
            <span>转到文件夹</span>
          </div>
          <div
            class="context-menu-item env-var"
            title="设置此项目/技能的专属环境变量"
            @click="openEnvDialog"
          >
            <i class="fa-solid fa-key" />
            <span>环境变量</span>
          </div>
        </template>

        <!-- Custom args option for deletable icons -->
        <template v-if="isDeletable">
          <div class="context-menu-separator" />
          <div
            class="context-menu-item custom-args"
            title="设置双击启动时执行的完整命令"
            @click="openArgsDialog"
          >
            <i class="fa-solid fa-terminal" />
            <span>自定义启动命令</span>
          </div>
        </template>

        <!-- Rename option for deletable icons -->
        <div
          v-if="isDeletable"
          class="context-menu-separator"
        />
        <div
          v-if="isDeletable"
          class="context-menu-item rename"
          title="修改图标显示名称"
          @click="openRenameDialog"
        >
          <i class="fa-solid fa-pen" />
          <span>改名</span>
        </div>

        <!-- Delete option for deletable icons -->
        <div
          v-if="isDeletable"
          class="context-menu-separator"
        />
        <div
          v-if="isDeletable"
          class="context-menu-item delete"
          @click="handleDelete"
        >
          <i class="fa-solid fa-trash" />
          <span>删除链接</span>
        </div>
      </div>
      <div
        v-if="contextMenuVisible"
        class="context-menu-backdrop"
        @click="handleGlobalClick"
      />

      <!-- Confirmation Dialog for Fresh Start -->
      <div
        v-if="confirmDialogVisible"
        class="confirm-dialog-overlay"
        @click="cancelFreshStart"
      >
        <div
          class="confirm-dialog"
          @click.stop
        >
          <div class="confirm-dialog-icon">
            <i class="fa-solid fa-triangle-exclamation" />
          </div>
          <div class="confirm-dialog-title">
            确认重新开始
          </div>
          <div class="confirm-dialog-message">
            此操作将删除以下文件和目录：
            <ul>
              <li><code>.claude/</code> 目录（设置、规则、命令）</li>
              <li><code>CLAUDE.md</code> 文件（项目记忆）</li>
              <li><code>CLAUDE.local.md</code> 文件（本地配置）</li>
              <li><code>~/.claude/projects/</code> 下对应项目文件夹</li>
              <li><code>~/.claude/file-history/</code> 下关联的会话历史</li>
            </ul>
            <strong>此操作不可撤销！</strong>
          </div>
          <div class="confirm-dialog-actions">
            <button
              class="btn btn-cancel"
              @click="cancelFreshStart"
            >
              取消
            </button>
            <button
              class="btn btn-danger"
              @click="confirmFreshStart"
            >
              确认重置
            </button>
          </div>
        </div>
      </div>

      <!-- Environment Variables Dialog -->
      <div
        v-if="envDialogVisible"
        class="env-dialog-overlay"
        @click="closeEnvDialog"
      >
        <div
          class="env-dialog"
          @click.stop
        >
          <div class="env-dialog-header">
            <i class="fa-solid fa-key" />
            <span>{{ icon.name }} - 环境变量</span>
          </div>
          <div class="env-dialog-content">
            <div class="env-dialog-hint">
              这些环境变量仅在打开此{{ isProjectLink ? '项目' : '技能' }}的终端时生效，会覆盖全局设置中的同名变量。
            </div>
            <!-- Existing variables list -->
            <div class="env-list">
              <div
                v-for="(value, key) in editingEnvVars"
                :key="key"
                class="env-item"
              >
                <span class="env-key">{{ key }}</span>
                <span class="env-value">{{ value }}</span>
                <button
                  class="env-remove-btn"
                  title="删除"
                  @click="removeEnvVar(String(key))"
                >
                  <i class="fa-solid fa-times" />
                </button>
              </div>
              <div
                v-if="Object.keys(editingEnvVars).length === 0"
                class="env-empty"
              >
                暂无环境变量
              </div>
            </div>
            <!-- Add new variable -->
            <div class="env-add-row">
              <input
                v-model="newEnvKey"
                class="env-input key-input"
                placeholder="变量名"
                @keydown.enter="addEnvVar"
              >
              <input
                v-model="newEnvValue"
                class="env-input value-input"
                placeholder="值"
                @keydown.enter="addEnvVar"
              >
              <button
                class="env-add-btn"
                :disabled="!newEnvKey.trim() || !newEnvValue.trim()"
                @click="addEnvVar"
              >
                <i class="fa-solid fa-plus" />
              </button>
            </div>
          </div>
          <div class="env-dialog-actions">
            <button
              class="btn btn-cancel"
              @click="closeEnvDialog"
            >
              取消
            </button>
            <button
              class="btn btn-primary"
              @click="saveEnvVars"
            >
              保存
            </button>
          </div>
        </div>
      </div>

      <!-- Custom Args Dialog -->
      <div
        v-if="argsDialogVisible"
        class="env-dialog-overlay"
        @click="closeArgsDialog"
      >
        <div
          class="args-dialog"
          @click.stop
        >
          <div class="args-dialog-header">
            <i class="fa-solid fa-terminal" />
            <span>{{ icon.name }} - 自定义启动命令</span>
          </div>
          <div class="args-dialog-content">
            <div class="args-dialog-hint">
              设置双击启动时执行的完整命令，例如 <code>claude -r</code> 或 <code>mcd</code>
            </div>
            <input
              v-model="customArgsInput"
              class="args-input"
              placeholder="claude"
              @keydown.enter="saveCustomArgs"
            >
          </div>
          <div class="args-dialog-actions">
            <button
              class="btn btn-cancel"
              @click="closeArgsDialog"
            >
              取消
            </button>
            <button
              class="btn btn-primary"
              @click="saveCustomArgs"
            >
              保存
            </button>
          </div>
        </div>
      </div>
      <!-- Rename Dialog -->
      <div
        v-if="renameDialogVisible"
        class="env-dialog-overlay"
        @click="closeRenameDialog"
      >
        <div
          class="args-dialog"
          @click.stop
        >
          <div class="args-dialog-header">
            <i class="fa-solid fa-pen" />
            <span>改名</span>
          </div>
          <div class="args-dialog-content">
            <div class="args-dialog-hint">
              修改图标的显示名称（不影响实际路径）
            </div>
            <input
              v-model="renameInput"
              class="args-input"
              placeholder="输入新名称"
              @keydown.enter="saveRename"
              @keydown.esc="closeRenameDialog"
            >
          </div>
          <div class="args-dialog-actions">
            <button
              class="btn btn-cancel"
              @click="closeRenameDialog"
            >
              取消
            </button>
            <button
              class="btn btn-primary rename-btn"
              :disabled="!renameInput.trim()"
              @click="saveRename"
            >
              保存
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style lang="less" scoped>
.desktop-icon {
  position: absolute;
  width: 80px;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 8px;
  cursor: pointer;
  user-select: none;
  border-radius: 8px;
  transition: background 0.2s;
  touch-action: none; // Prevent touch scrolling during drag

  &:hover {
    background: rgba(255, 255, 255, 0.15);
  }

  &:active:not(.dragging) {
    transform: scale(0.95);
  }

  &.dragging {
    opacity: 0.9;
    transform: scale(1.05);
    z-index: 1000;
    cursor: grabbing;
    background: rgba(255, 255, 255, 0.15);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
  }

  &.ghost {
    opacity: 0.4;
    pointer-events: none;
    border: 2px dashed rgba(255, 255, 255, 0.5);
    background: rgba(255, 255, 255, 0.1);
    transform: scale(1);
    z-index: 999;
  }

  &.focused {
    background: rgba(124, 110, 184, 0.4);
    box-shadow:
      0 0 30px rgba(124, 110, 184, 0.6),
      0 0 0 3px rgba(124, 110, 184, 0.7),
      inset 0 0 20px rgba(124, 110, 184, 0.3);
    z-index: 100;
    animation: focusPulse 2s ease-in-out infinite;

    .icon-wrapper i {
      filter: drop-shadow(0 0 12px rgba(124, 110, 184, 0.8));
      color: #c4b8ff;
    }

    .icon-label {
      color: #e8e0ff;
      text-shadow:
        0 0 15px rgba(124, 110, 184, 1),
        0 1px 3px rgba(0, 0, 0, 0.6);
    }
  }

  @keyframes focusPulse {
    0%, 100% {
      box-shadow:
        0 0 30px rgba(124, 110, 184, 0.6),
        0 0 0 3px rgba(124, 110, 184, 0.7),
        inset 0 0 20px rgba(124, 110, 184, 0.3);
    }
    50% {
      box-shadow:
        0 0 40px rgba(124, 110, 184, 0.8),
        0 0 0 3px rgba(124, 110, 184, 0.9),
        inset 0 0 25px rgba(124, 110, 184, 0.4);
    }
  }

  .icon-wrapper {
    width: 48px;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 6px;

    i {
      font-size: 40px;
      color: white;
      text-shadow: 0 2px 6px rgba(30, 58, 95, 0.4);
    }
  }

  .icon-label {
    color: white;
    font-size: 12px;
    text-align: center;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
    word-break: break-word;
    max-width: 100%;
    line-height: 1.3;
  }
}
</style>

<style lang="less">
// Global styles for context menu (not scoped)
.icon-context-menu {
  position: fixed;
  background: white;
  border-radius: 8px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
  padding: 8px 0;
  min-width: 180px;
  z-index: 10002;

  .context-menu-header {
    padding: 6px 16px;
    font-size: 12px;
    font-weight: 600;
    color: #999;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .context-menu-separator {
    height: 1px;
    background: #e0e0e0;
    margin: 6px 0;
  }

  .context-menu-item {
    padding: 10px 16px;
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    transition: background 0.2s;

    i {
      width: 16px;
      text-align: center;
      color: #666;
    }

    span {
      font-size: 14px;
    }

    &:hover {
      background: #f0f0f0;
    }

    &.claude-cmd {
      i {
        color: #8b7eb8;
      }

      &:hover {
        background: #f5f3ff;
      }
    }

    &.danger {
      i {
        color: #e8a090;
      }

      &:hover {
        background: #fdf5f3;
      }
    }

    &.delete {
      i {
        color: #ff5f57;
      }

      &:hover {
        background: #fff0f0;
      }
    }
  }
}

// Confirmation dialog styles
.confirm-dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10003;
}

.confirm-dialog {
  background: white;
  border-radius: 12px;
  padding: 24px;
  max-width: 400px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  animation: dialogFadeIn 0.2s ease-out;

  @keyframes dialogFadeIn {
    from {
      opacity: 0;
      transform: scale(0.95);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .confirm-dialog-icon {
    text-align: center;
    margin-bottom: 16px;

    i {
      font-size: 48px;
      color: #f59e0b;
    }
  }

  .confirm-dialog-title {
    font-size: 18px;
    font-weight: 600;
    color: #333;
    text-align: center;
    margin-bottom: 12px;
  }

  .confirm-dialog-message {
    font-size: 14px;
    color: #666;
    line-height: 1.6;
    margin-bottom: 20px;

    ul {
      margin: 12px 0;
      padding-left: 20px;

      li {
        margin: 6px 0;
      }
    }

    code {
      background: #f3f4f6;
      padding: 2px 6px;
      border-radius: 4px;
      font-family: monospace;
      font-size: 13px;
      color: #ef4444;
    }

    strong {
      color: #ef4444;
      display: block;
      text-align: center;
      margin-top: 12px;
    }
  }

  .confirm-dialog-actions {
    display: flex;
    gap: 12px;
    justify-content: flex-end;

    .btn {
      padding: 10px 20px;
      border-radius: 8px;
      font-size: 14px;
      font-weight: 500;
      cursor: pointer;
      border: none;
      transition: all 0.2s;

      &.btn-cancel {
        background: #f3f4f6;
        color: #666;

        &:hover {
          background: #e5e7eb;
        }
      }

      &.btn-danger {
        background: #ef4444;
        color: white;

        &:hover {
          background: #dc2626;
        }
      }
    }
  }
}

.context-menu-backdrop {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  z-index: 10001;
}

// Folder menu item style
.icon-context-menu .context-menu-item.folder {
  i {
    color: #f59e0b;
  }

  &:hover {
    background: #fffbeb;
  }
}

// Environment variables menu item style
.icon-context-menu .context-menu-item.env-var {
  i {
    color: #10b981;
  }

  &:hover {
    background: #ecfdf5;
  }
}

// Custom args menu item style
.icon-context-menu .context-menu-item.custom-args {
  i {
    color: #3b82f6;
  }

  &:hover {
    background: #eff6ff;
  }
}

// Rename menu item style
.icon-context-menu .context-menu-item.rename {
  i {
    color: #8b5cf6;
  }

  &:hover {
    background: #f5f3ff;
  }
}

// Environment variables dialog styles
.env-dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10003;
}

.env-dialog {
  background: white;
  border-radius: 12px;
  width: 480px;
  max-width: 90vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  animation: dialogFadeIn 0.2s ease-out;

  .env-dialog-header {
    padding: 16px 20px;
    border-bottom: 1px solid #e5e7eb;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 16px;
    font-weight: 600;
    color: #333;

    i {
      color: #10b981;
    }
  }

  .env-dialog-content {
    padding: 16px 20px;
    flex: 1;
    overflow-y: auto;
  }

  .env-dialog-hint {
    font-size: 13px;
    color: #666;
    margin-bottom: 16px;
    padding: 10px 12px;
    background: #f9fafb;
    border-radius: 6px;
    border-left: 3px solid #10b981;
  }

  .env-list {
    margin-bottom: 16px;
    max-height: 200px;
    overflow-y: auto;
  }

  .env-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    background: #f3f4f6;
    border-radius: 6px;
    margin-bottom: 8px;

    .env-key {
      font-family: monospace;
      font-size: 13px;
      font-weight: 600;
      color: #374151;
      min-width: 120px;
    }

    .env-value {
      flex: 1;
      font-family: monospace;
      font-size: 13px;
      color: #6b7280;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .env-remove-btn {
      background: none;
      border: none;
      color: #9ca3af;
      cursor: pointer;
      padding: 4px 6px;
      border-radius: 4px;
      transition: all 0.2s;

      &:hover {
        color: #ef4444;
        background: #fee2e2;
      }
    }
  }

  .env-empty {
    text-align: center;
    color: #9ca3af;
    font-size: 14px;
    padding: 20px;
  }

  .env-add-row {
    display: flex;
    gap: 8px;

    .env-input {
      padding: 10px 12px;
      border: 1px solid #d1d5db;
      border-radius: 6px;
      font-size: 14px;
      transition: border-color 0.2s;

      &:focus {
        outline: none;
        border-color: #10b981;
      }

      &.key-input {
        width: 140px;
        font-family: monospace;
      }

      &.value-input {
        flex: 1;
      }
    }

    .env-add-btn {
      padding: 10px 14px;
      background: #10b981;
      color: white;
      border: none;
      border-radius: 6px;
      cursor: pointer;
      transition: background 0.2s;

      &:hover:not(:disabled) {
        background: #059669;
      }

      &:disabled {
        background: #d1d5db;
        cursor: not-allowed;
      }
    }
  }

  .env-dialog-actions {
    padding: 16px 20px;
    border-top: 1px solid #e5e7eb;
    display: flex;
    gap: 12px;
    justify-content: flex-end;

    .btn {
      padding: 10px 20px;
      border-radius: 8px;
      font-size: 14px;
      font-weight: 500;
      cursor: pointer;
      border: none;
      transition: all 0.2s;

      &.btn-cancel {
        background: #f3f4f6;
        color: #666;

        &:hover {
          background: #e5e7eb;
        }
      }

      &.btn-primary {
        background: #10b981;
        color: white;

        &:hover {
          background: #059669;
        }
      }
    }
  }
}

// Custom args dialog styles
.args-dialog {
  background: white;
  border-radius: 12px;
  width: 400px;
  max-width: 90vw;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  animation: dialogFadeIn 0.2s ease-out;

  .args-dialog-header {
    padding: 16px 20px;
    border-bottom: 1px solid #e5e7eb;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 16px;
    font-weight: 600;
    color: #333;

    i {
      color: #3b82f6;
    }
  }

  .args-dialog-content {
    padding: 20px;
  }

  .args-dialog-hint {
    font-size: 13px;
    color: #666;
    margin-bottom: 16px;
    padding: 10px 12px;
    background: #f9fafb;
    border-radius: 6px;
    border-left: 3px solid #3b82f6;

    code {
      background: #e0e7ff;
      padding: 2px 6px;
      border-radius: 4px;
      font-family: monospace;
      font-size: 12px;
      color: #3b82f6;
    }
  }

  .args-input {
    width: 100%;
    padding: 12px 14px;
    border: 1px solid #d1d5db;
    border-radius: 8px;
    font-size: 14px;
    font-family: monospace;
    transition: border-color 0.2s, box-shadow 0.2s;

    &:focus {
      outline: none;
      border-color: #3b82f6;
      box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
    }

    &::placeholder {
      color: #9ca3af;
    }
  }

  .args-dialog-actions {
    padding: 16px 20px;
    border-top: 1px solid #e5e7eb;
    display: flex;
    gap: 12px;
    justify-content: flex-end;

    .btn {
      padding: 10px 20px;
      border-radius: 8px;
      font-size: 14px;
      font-weight: 500;
      cursor: pointer;
      border: none;
      transition: all 0.2s;

      &.btn-cancel {
        background: #f3f4f6;
        color: #666;

        &:hover {
          background: #e5e7eb;
        }
      }

      &.btn-primary {
        background: #3b82f6;
        color: white;

        &:hover {
          background: #2563eb;
        }
      }

      &.rename-btn {
        background: #8b5cf6;

        &:hover:not(:disabled) {
          background: #7c3aed;
        }

        &:disabled {
          background: #d1d5db;
          cursor: not-allowed;
        }
      }
    }
  }
}
</style>
