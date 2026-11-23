<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebLinksAddon } from '@xterm/addon-web-links'
import { CanvasAddon } from '@xterm/addon-canvas'
import { SearchAddon } from '@xterm/addon-search'
import '@xterm/xterm/css/xterm.css'
import { useTerminalStore } from '@/stores/terminalStore'
import { useSettingsStore } from '@/stores/settingsStore'
import { useWindowStore } from '@/stores/windowStore'
import { useTerminalPty } from '@/composables/useTerminalPty'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import TerminalFilePanel from './TerminalFilePanel.vue'
import TerminalSkillPanel from './TerminalSkillPanel.vue'

interface SkillInfo {
  skillName: string
  skillDescription: string
  pluginName: string
  pluginDescription: string
  pluginVersion: string
  marketplaceName: string
  agents: string[]
  mcpServers: Record<string, unknown>
}

const props = defineProps<{
  windowId: string
  windowData?: {
    cwd?: string
    initialCommand?: string
    type?: 'project' | 'skill'
    skillInfo?: SkillInfo
    iconEnvVars?: Record<string, string>
  }
}>()

const emit = defineEmits<{
  (e: 'cwd-changed', path: string): void
}>()

const terminalRef = ref<HTMLElement>()
const searchInputRef = ref<HTMLInputElement>()
const terminalStore = useTerminalStore()
const settingsStore = useSettingsStore()
const windowStore = useWindowStore()
const pty = useTerminalPty()

const PANEL_EXPANDED_WIDTH = 200
const PANEL_COLLAPSED_WIDTH = 24
const PANEL_WIDTH_DELTA = PANEL_EXPANDED_WIDTH - PANEL_COLLAPSED_WIDTH
const BASE_MIN_WIDTH = 600

const currentWindow = computed(() =>
  windowStore.windows.find(w => w.id === props.windowId)
)

let terminal: Terminal | null = null
let canvasAddon: CanvasAddon | null = null
let searchAddon: SearchAddon | null = null
let unlisten: UnlistenFn | null = null
let unlistenDragDrop: UnlistenFn | null = null
let isInitialized = false
let resizeObserver: ResizeObserver | null = null
const currentCwd = ref<string>('')
const isDragOver = ref(false)

// Context menu state
const contextMenu = ref<{ show: boolean; x: number; y: number }>({
  show: false, x: 0, y: 0
})
const hasSelection = ref(false)

// Search state
const isSearchOpen = ref(false)
const searchQuery = ref('')

// Search methods
const findNext = () => {
  if (searchAddon && searchQuery.value) {
    searchAddon.findNext(searchQuery.value, { caseSensitive: false })
  }
}

const findPrevious = () => {
  if (searchAddon && searchQuery.value) {
    searchAddon.findPrevious(searchQuery.value, { caseSensitive: false })
  }
}

const closeSearch = () => {
  isSearchOpen.value = false
  searchQuery.value = ''
  terminal?.focus()
}

// Context menu handlers
const showContextMenu = (event: MouseEvent) => {
  event.preventDefault()
  hasSelection.value = terminal?.hasSelection() ?? false
  contextMenu.value = { show: true, x: event.clientX, y: event.clientY }
}

const hideContextMenu = () => {
  contextMenu.value = { ...contextMenu.value, show: false }
}

const handleCopy = async () => {
  hideContextMenu()
  const selection = terminal?.getSelection()
  if (selection) {
    await navigator.clipboard.writeText(selection)
  }
}

const handlePaste = async () => {
  hideContextMenu()
  try {
    const text = await navigator.clipboard.readText()
    if (text && terminal) {
      terminal.paste(text)
    }
  } catch (e) {
    console.error('Paste failed:', e)
  }
}

// Send Shift+Tab to terminal (for Claude Code mode switching)
const sendShiftTab = () => {
  pty.write('\x1b[Z')
}

const handlePanelToggled = (expanded: boolean) => {
  const win = currentWindow.value
  if (!win) return

  const panelWidth = expanded ? PANEL_EXPANDED_WIDTH : PANEL_COLLAPSED_WIDTH
  windowStore.updateWindowData(props.windowId, {
    minWidth: BASE_MIN_WIDTH + panelWidth
  })

  // normal mode: adjust window width so terminal content width stays the same
  if (win.state === 'normal') {
    const delta = expanded ? PANEL_WIDTH_DELTA : -PANEL_WIDTH_DELTA
    windowStore.updateWindowPosition(props.windowId, {
      width: Math.max(BASE_MIN_WIDTH + panelWidth, win.position.width + delta)
    })
  }
  // maximized mode: don't touch window, flex layout handles it
}

interface AppPaths {
  app_data: string
  config: string
  logs: string
  cache: string
  workspace: string
}

const initTerminal = async () => {
  if (!terminalRef.value || isInitialized) return
  isInitialized = true

  // One Dark theme - Atom editor inspired color scheme
  terminal = new Terminal({
    cursorBlink: true,
    fontSize: settingsStore.terminalFontSize || 14,
    fontFamily: settingsStore.terminalFontFamily || 'Consolas, "Courier New", monospace',
    theme: {
      background: '#282c34',
      foreground: '#abb2bf',
      cursor: '#528bff',
      cursorAccent: '#282c34',
      selectionBackground: 'rgba(82, 139, 255, 0.3)',
      black: '#282c34',
      red: '#e06c75',
      green: '#98c379',
      yellow: '#e5c07b',
      blue: '#61afef',
      magenta: '#c678dd',
      cyan: '#56b6c2',
      white: '#abb2bf',
      brightBlack: '#5c6370',
      brightRed: '#e06c75',
      brightGreen: '#98c379',
      brightYellow: '#e5c07b',
      brightBlue: '#61afef',
      brightMagenta: '#c678dd',
      brightCyan: '#56b6c2',
      brightWhite: '#ffffff'
    },
    allowProposedApi: true,
  })

  // Custom keyboard handler
  terminal.attachCustomKeyEventHandler((event: KeyboardEvent) => {
    // Ctrl/Cmd+F: Open search
    if ((event.ctrlKey || event.metaKey) && event.key === 'f' && event.type === 'keydown') {
      isSearchOpen.value = true
      nextTick(() => searchInputRef.value?.focus())
      return false
    }

    // Escape: Close search if open
    if (event.key === 'Escape' && event.type === 'keydown' && isSearchOpen.value) {
      closeSearch()
      return false
    }

    // Ctrl/Cmd+C: Copy selected text if there's a selection, otherwise send to terminal
    if ((event.ctrlKey || event.metaKey) && event.key === 'c' && event.type === 'keydown') {
      if (terminal?.hasSelection()) {
        const selection = terminal.getSelection()
        navigator.clipboard.writeText(selection)
        return false
      }
      return true
    }

    // Ctrl/Cmd+V: Block and let paste event handle it
    if ((event.ctrlKey || event.metaKey) && event.key === 'v') {
      return false
    }

    // Ctrl/Cmd+Shift+C: Always copy
    if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key === 'C' && event.type === 'keydown') {
      if (terminal?.hasSelection()) {
        navigator.clipboard.writeText(terminal.getSelection())
      }
      return false
    }

    // Ctrl/Cmd+Shift+V: Block and let paste event handle it
    if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key === 'V') {
      return false
    }

    // Shift+Tab: Trigger Claude Code mode switching
    if (event.key === 'Tab' && event.shiftKey && event.type === 'keydown') {
      sendShiftTab()
      return false
    }

    return true
  })

  const fitAddon = new FitAddon()
  terminal.loadAddon(fitAddon)
  terminal.loadAddon(new WebLinksAddon())

  terminal.open(terminalRef.value)

  // Load Canvas addon for hardware-accelerated rendering
  try {
    canvasAddon = new CanvasAddon()
    terminal.loadAddon(canvasAddon)
  } catch {
    canvasAddon = null
  }

  // Load Search addon
  searchAddon = new SearchAddon()
  terminal.loadAddon(searchAddon)

  // Bind terminal and fitAddon to composable
  pty.bind(terminal, fitAddon)

  fitAddon.fit()
  terminal.focus()

  // --- Terminal title processing ---
  // Claude Code CLI title patterns:
  // "⠐ Task" / "⠂ Task" (braille spinners U+2800-U+28FF) → working
  // "✳ Claude Code" / "✳ Task" (✳ U+2733) → idle/completed
  // Shell path / "claude" / "npm" etc. → transient
  let lastTitleProcessed = ''
  let lastTitleTime = 0

  function processTerminalTitle(title: string) {
    // Deduplicate: OSC 0/2 handlers + onTitleChange may both fire for the same sequence
    const now = Date.now()
    if (title === lastTitleProcessed && now - lastTitleTime < 100) return
    lastTitleProcessed = title
    lastTitleTime = now

    // Strip potential leading invisible characters (BOM, zero-width) that
    // ConPTY may prepend, which would break the ^ regex anchor
    const stripped = title.replace(/^[\uFEFF\u200B-\u200D\u2060]*/g, '')

    console.debug('[Terminal] title:', JSON.stringify(stripped),
      'codes:', [...stripped].slice(0, 5).map(c => 'U+' + (c.codePointAt(0) ?? 0).toString(16).padStart(4, '0')))

    const isClaudeSpinner = /^[\u2800-\u28FF]\s/.test(stripped)
    const isClaudeIdle = /^\u2733\s/.test(stripped)
    const wasWorking = currentWindow.value?.data?.terminalWorking === true

    // Spinner → show task name; idle (✳) → restore original project title
    // Fallback: skip generic "Claude Code" — only update with specific task descriptions
    if (isClaudeSpinner) {
      const cleanTitle = stripped.replace(/^[\u2800-\u28FF]\s+/, '')
      if (cleanTitle && !/^claude\s*code$/i.test(cleanTitle)) {
        windowStore.updateWindowTitle(props.windowId, cleanTitle)
      }
    } else if (isClaudeIdle) {
      windowStore.restoreWindowTitle(props.windowId)
    }

    windowStore.updateWindowData(props.windowId, {
      terminalTitle: title,
      terminalWorking: isClaudeSpinner,
    })

    // spinner → ✳ transition = task completed
    if (wasWorking && isClaudeIdle) {
      windowStore.updateWindowData(props.windowId, {
        terminalStatus: 'completed',
        terminalStatusTime: Date.now(),
      })
    }
  }

  // Register OSC 0/2 handlers for direct title capture — acts as a redundant
  // mechanism alongside onTitleChange. On some Windows ConPTY configurations
  // xterm.js onTitleChange may not fire, but custom OSC handlers still capture
  // the raw sequence data.
  terminal.parser.registerOscHandler(0, (data) => {
    processTerminalTitle(data)
    return false // Don't suppress default xterm.js handling
  })
  terminal.parser.registerOscHandler(2, (data) => {
    processTerminalTitle(data)
    return false
  })

  // Register OSC 7 handler for directory tracking
  terminal.parser.registerOscHandler(7, (data) => {
    const match = data.match(/file:\/\/localhost\/(.+)/)
    if (match) {
      const path = decodeURIComponent(match[1]).replace(/\//g, '\\')
      currentCwd.value = path
      emit('cwd-changed', path)
    }
    // Shell returned (Claude Code exited) — clear status & restore title
    windowStore.updateWindowData(props.windowId, {
      terminalStatus: null,
      terminalWorking: false,
      terminalTitle: null,
    })
    windowStore.restoreWindowTitle(props.windowId)
    return true
  })

  // Fallback: onTitleChange catches titles set by xterm.js internal OSC handling
  terminal.onTitleChange((title) => {
    processTerminalTitle(title)
  })

  // Bell signal — auxiliary completion signal
  terminal.onBell(() => {
    const title = currentWindow.value?.data?.terminalTitle as string | undefined
    if (title) {
      windowStore.updateWindowData(props.windowId, {
        terminalStatus: 'completed',
        terminalStatusTime: Date.now(),
      })
    }
  })

  // Set initial cwd
  currentCwd.value = props.windowData?.cwd || ''

  // Intercept native paste event in capture phase to prevent xterm's own paste
  // handler (on .xterm-helper-textarea) from also firing — without capture:true
  // the event reaches xterm first (bubble from textarea), then bubbles here,
  // causing every paste to be sent twice.  Capture fires on the way DOWN, before
  // the textarea's bubble listener, so stopPropagation() prevents the duplicate.
  // Use terminal.paste() instead of invoke() directly so xterm can apply
  // bracketed-paste mode wrapping (\x1b[200~...\x1b[201~) that PSReadLine and
  // other readline-aware shells rely on for correct multi-line paste behaviour.
  terminalRef.value.addEventListener('paste', (e: ClipboardEvent) => {
    e.preventDefault()
    e.stopPropagation()
    const text = e.clipboardData?.getData('text')
    if (terminal && text) {
      terminal.paste(text)
    }
  }, { capture: true })

  // Intercept Tab/Shift+Tab at container level to prevent browser focus switching
  terminalRef.value.addEventListener('keydown', (e: KeyboardEvent) => {
    if (e.key === 'Tab') {
      e.preventDefault()
    }
  }, { capture: true })

  // Track IME composition state (both local for scroll prevention and composable for resize)
  // Use CSS class toggle instead of JS scroll manipulation to avoid DOM reflow
  // that causes Windows IME candidate window to dismiss
  terminalRef.value.addEventListener('compositionstart', () => {
    pty.setComposing(true)
    terminalRef.value?.classList.add('ime-composing')
  })
  terminalRef.value.addEventListener('compositionend', () => {
    pty.setComposing(false)
    terminalRef.value?.classList.remove('ime-composing')
  })

  // Context menu (right-click Copy/Paste)
  terminalRef.value.addEventListener('contextmenu', showContextMenu)
  document.addEventListener('click', hideContextMenu)

  // Drag and drop file path insertion using Tauri native API
  const appWindow = getCurrentWebviewWindow()
  unlistenDragDrop = await appWindow.onDragDropEvent((event) => {
    if (event.payload.type === 'over') {
      isDragOver.value = true
    } else if (event.payload.type === 'leave') {
      isDragOver.value = false
    } else if (event.payload.type === 'drop') {
      isDragOver.value = false

      const filePaths = event.payload.paths as string[]
      if (!filePaths || filePaths.length === 0 || !pty.getPtyId()) return

      const paths: string[] = filePaths.map(p => p.includes(' ') ? `"${p}"` : p)
      const pathString = paths.join(' ')
      pty.write(pathString)
    }
  })

  const cols = terminal.cols
  const rows = terminal.rows

  try {
    // Get app paths for default working directory
    const appPaths = await invoke<AppPaths>('get_app_paths_cmd')
    const defaultCwd = props.windowData?.cwd || settingsStore.defaultWorkingDirectory || appPaths.workspace

    // Get environment variables: merge global settings with icon-specific vars
    const globalEnvVars = settingsStore.environmentVariables || {}
    const iconEnvVars = (props.windowData?.iconEnvVars as Record<string, string>) || {}
    const mergedEnvVars = { ...globalEnvVars, ...iconEnvVars }
    const envVars = Object.keys(mergedEnvVars).length > 0 ? mergedEnvVars : null

    // Build alias definitions to inject into terminal
    const aliasConfigs = settingsStore.claudeAliases
      .filter(a => a.name.trim())
      .map(a => ({
        name: a.name.trim(),
        anthropic_model: a.anthropicModel || null,
        anthropic_auth_token: a.anthropicAuthToken || null,
        anthropic_base_url: a.anthropicBaseUrl || null,
        custom_env_vars: a.customEnvVars && Object.keys(a.customEnvVars).length > 0 ? a.customEnvVars : null,
      }))
    const aliasesParam = aliasConfigs.length > 0 ? aliasConfigs : null

    // Spawn PTY
    const id = await invoke<string>('spawn_terminal', {
      cols,
      rows,
      cwd: defaultCwd,
      env: envVars,
      promptConfig: settingsStore.promptConfig,
      aliases: aliasesParam
    })
    pty.setPtyId(id)

    // Create terminal instance in store
    terminalStore.createTerminal(props.windowId, id)

    // Listen for PTY output with batching
    let initialCommandSent = false
    unlisten = await listen(`terminal-output-${id}`, (event) => {
      if (!terminal || !event.payload) return

      // Convert payload to Uint8Array
      const payload = event.payload as number[] | Uint8Array | string
      let chunk: Uint8Array

      if (typeof payload === 'string') {
        chunk = new TextEncoder().encode(payload)
      } else if (Array.isArray(payload)) {
        chunk = new Uint8Array(payload)
      } else if (payload instanceof Uint8Array) {
        chunk = payload
      } else {
        chunk = new Uint8Array(Object.values(payload))
      }

      pty.onPtyData(chunk)

      // Shell ready: first PTY output means prompt is displayed
      if (!initialCommandSent && props.windowData?.initialCommand && pty.getPtyId()) {
        initialCommandSent = true
        pty.write(props.windowData!.initialCommand + '\r')
      }
    })

    // Handle user input - send to PTY
    terminal.onData((data) => {
      pty.write(data)
      // Clear completed status on user input
      if (currentWindow.value?.data?.terminalStatus === 'completed') {
        windowStore.updateWindowData(props.windowId, { terminalStatus: null })
      }
    })

  } catch (error) {
    console.error('Failed to spawn terminal:', error)
    terminal.writeln(`\x1b[31mFailed to start PowerShell: ${error}\x1b[0m`)
  }

  // Handle container resize with simple debounce
  resizeObserver = new ResizeObserver(() => {
    pty.scheduleResize()
  })
  resizeObserver.observe(terminalRef.value)

  // Set initial minWidth (panel starts expanded)
  windowStore.updateWindowData(props.windowId, {
    minWidth: BASE_MIN_WIDTH + PANEL_EXPANDED_WIDTH
  })
}

onMounted(() => {
  initTerminal()
})

onUnmounted(async () => {
  if (unlisten) unlisten()
  if (unlistenDragDrop) unlistenDragDrop()
  if (resizeObserver) resizeObserver.disconnect()
  document.removeEventListener('click', hideContextMenu)
  canvasAddon?.dispose()
  searchAddon?.dispose()

  if (pty.getPtyId()) {
    await pty.kill()
    terminalStore.destroyTerminal(props.windowId)
  }
  terminal?.dispose()
  pty.destroy()
})

// Watch for window state changes (minimize/restore)
watch(
  () => windowStore.windows.find(w => w.id === props.windowId)?.state,
  (newState, oldState) => {
    if (oldState === 'minimized' && newState !== 'minimized') {
      nextTick(() => {
        pty.doResize()
        terminal?.focus()
      })
    }
  }
)
</script>

<template>
  <div class="terminal-wrapper">
    <!-- Skill info panel for skill terminals -->
    <TerminalSkillPanel
      v-if="windowData?.type === 'skill' && windowData.skillInfo"
      :skill-info="windowData.skillInfo"
    />
    <!-- File panel for project terminals -->
    <TerminalFilePanel
      v-else
      :cwd="currentCwd"
      @send-shift-tab="sendShiftTab"
      @panel-toggled="handlePanelToggled"
    />
    <div class="terminal-container">
      <!-- Search bar -->
      <div
        v-if="isSearchOpen"
        class="terminal-search"
      >
        <input
          ref="searchInputRef"
          v-model="searchQuery"
          type="text"
          placeholder="Search..."
          @keydown.enter.exact="findNext"
          @keydown.shift.enter="findPrevious"
          @keydown.escape="closeSearch"
        >
        <button
          title="Previous (Shift+Enter)"
          @click="findPrevious"
        >
          <i class="fa fa-chevron-up" />
        </button>
        <button
          title="Next (Enter)"
          @click="findNext"
        >
          <i class="fa fa-chevron-down" />
        </button>
        <button
          title="Close (Escape)"
          @click="closeSearch"
        >
          <i class="fa fa-times" />
        </button>
      </div>
      <div
        ref="terminalRef"
        class="terminal"
        :class="{ 'drag-over': isDragOver }"
      />
      <!-- Right-click context menu -->
      <div
        v-if="contextMenu.show"
        class="context-menu"
        :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
      >
        <div
          class="menu-item"
          :class="{ disabled: !hasSelection }"
          @click="hasSelection && handleCopy()"
        >
          <i class="fa fa-copy" /> Copy
          <span class="shortcut">Ctrl+C</span>
        </div>
        <div
          class="menu-item"
          @click="handlePaste()"
        >
          <i class="fa fa-paste" /> Paste
          <span class="shortcut">Ctrl+V</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="less" scoped>
.terminal-wrapper {
  display: flex;
  width: 100%;
  height: 100%;
  background: #282c34;
  overflow: hidden;
}

.terminal-container {
  flex: 1;
  height: 100%;
  background: #282c34;
  overflow: hidden;
  position: relative;
  min-width: 0;
}

// Search bar (One Dark style)
.terminal-search {
  position: absolute;
  top: 8px;
  right: 16px;
  z-index: 10;
  display: flex;
  align-items: center;
  gap: 4px;
  background: #3c4049;
  border: 1px solid #4b5263;
  border-radius: 4px;
  padding: 4px 8px;

  input {
    background: #282c34;
    border: 1px solid #4b5263;
    border-radius: 3px;
    color: #abb2bf;
    padding: 4px 8px;
    font-size: 12px;
    width: 200px;

    &:focus {
      outline: none;
      border-color: #528bff;
    }

    &::placeholder {
      color: #5c6370;
    }
  }

  button {
    background: transparent;
    border: none;
    color: #abb2bf;
    padding: 4px 6px;
    cursor: pointer;
    border-radius: 3px;
    display: flex;
    align-items: center;
    justify-content: center;

    &:hover {
      background: #4b5263;
    }
  }
}

.terminal {
  position: absolute;
  top: 8px;
  left: 8px;
  right: 0;
  bottom: 8px;

  :deep(.xterm) {
    height: 100%;
    position: relative;
    overflow: hidden !important;
  }

  // Ensure xterm screen doesn't cause horizontal scroll
  :deep(.xterm-screen) {
    position: relative;
    overflow: hidden !important;
  }

  // Fix xterm rows container to prevent width changes during IME
  :deep(.xterm-rows) {
    overflow: hidden !important;
  }

  // Give helper textarea enough width for IME candidate window anchoring
  :deep(.xterm-helper-textarea) {
    min-width: 1em !important;
    opacity: 0 !important;
    overflow: hidden !important;
  }

  :deep(.xterm-viewport) {
    overflow-x: hidden !important;
    overflow-y: auto !important;

    &::-webkit-scrollbar {
      width: 8px;
    }

    &::-webkit-scrollbar-track {
      background: #2d2d2d;
    }

    &::-webkit-scrollbar-thumb {
      background: #555;
      border-radius: 4px;

      &:hover {
        background: #666;
      }
    }
  }

  // During IME composition, lock horizontal scroll via CSS to avoid DOM reflow
  &.ime-composing {
    :deep(.xterm-viewport),
    :deep(.xterm-screen) {
      overflow-x: hidden !important;
    }
  }

  // Drag-over visual feedback
  &.drag-over {
    outline: 2px dashed #569cd6;
    outline-offset: -2px;
  }
}

// Right-click context menu (One Dark palette)
.context-menu {
  position: fixed;
  background: #282c34;
  border: 1px solid #3e4451;
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  z-index: 9999;
  min-width: 160px;
  padding: 4px 0;
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  cursor: pointer;
  color: #abb2bf;
  font-size: 13px;

  &:hover:not(.disabled) {
    background: #61afef;
    color: #fff;
  }

  &.disabled {
    opacity: 0.4;
    cursor: default;
  }

  .shortcut {
    margin-left: auto;
    font-size: 11px;
    opacity: 0.5;
  }
}
</style>
