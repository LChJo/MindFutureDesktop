import { defineStore } from 'pinia'
import { markRaw, type Component } from 'vue'
import type { WindowInstance, WindowPosition, WindowState, DesktopIcon } from '@/types/models'
import { useSettingsStore } from './settingsStore'

// Component mapping
const componentMap: Record<string, () => Promise<{ default: Component }>> = {
  terminal: () => import('@/components/Terminal.vue'),
  'file-manager': () => import('@/components/FileManager.vue'),
  settings: () => import('@/components/Settings.vue'),
  'plugin-market': () => import('@/components/PluginMarket.vue'),
  'plugin-market-full': () => import('@/components/PluginMarketFull.vue'),
  'mcp-manager': () => import('@/components/McpManager.vue'),
  'setup-guide': () => import('@/components/SetupGuideWindow.vue'),
  'env-vars': () => import('@/components/Settings.vue'),  // Reuse Settings with env tab
  'coworks': () => import('@/components/Coworks.vue'),
  'llm-router': () => import('@/components/LlmRouter.vue'),
  'dynamic-function': () => import('@/components/DynamicFunctionHost.vue'),
  'dynamic-function-creator': () => import('@/components/claudeApps/ClaudeAppsView.vue'),
  'dev-tools': () => import('@/components/DevTools.vue'),
  'claude-aliases': () => import('@/components/ClaudeAliases.vue'),
  'claude-bot': () => import('@/components/claudeBot/ClaudeBot.vue')
}

export interface WindowStoreState {
  windows: WindowInstance[]
  activeWindowId: string | null
  maxZIndex: number
}

export const useWindowStore = defineStore('window', {
  state: (): WindowStoreState => ({
    windows: [],
    activeWindowId: null,
    maxZIndex: 1000
  }),

  getters: {
    activeWindow: (state): WindowInstance | undefined => {
      return state.windows.find(w => w.id === state.activeWindowId)
    },

    visibleWindows: (state): WindowInstance[] => {
      return state.windows.filter(w => w.state !== 'minimized')
    },

    minimizedWindows: (state): WindowInstance[] => {
      return state.windows.filter(w => w.state === 'minimized')
    }
  },

  actions: {
    async openWindow(icon: DesktopIcon, windowData?: Record<string, unknown>) {
      // Check if window for this icon already exists
      const existingWindow = this.windows.find(w => w.iconId === icon.id)
      if (existingWindow) {
        this.focusWindow(existingWindow.id)
        if (existingWindow.state === 'minimized') {
          this.restoreWindow(existingWindow.id)
        }
        return existingWindow
      }

      // Load component
      let component: Component | string = icon.type
      if (componentMap[icon.type]) {
        try {
          const module = await componentMap[icon.type]()
          component = markRaw(module.default)
        } catch (e) {
          console.error(`Failed to load component for ${icon.type}:`, e)
        }
      }

      // Calculate default position using settings
      const settingsStore = useSettingsStore()
      const offset = this.windows.length * 30
      const defaultPosition: WindowPosition = {
        x: 100 + offset,
        y: 80 + offset,
        width: settingsStore.defaultWindowWidth || 1000,
        height: settingsStore.defaultWindowHeight || 600
      }

      const windowId = `window-${Date.now()}`
      const newWindow: WindowInstance = {
        id: windowId,
        iconId: icon.id,
        title: icon.name,
        component,
        state: 'normal',
        position: defaultPosition,
        zIndex: ++this.maxZIndex,
        data: windowData  // Pass window data to component
      }

      this.windows.push(newWindow)
      this.activeWindowId = windowId
      this.save()

      return newWindow
    },

    closeWindow(windowId: string) {
      const index = this.windows.findIndex(w => w.id === windowId)
      if (index !== -1) {
        this.windows.splice(index, 1)
        if (this.activeWindowId === windowId) {
          this.activeWindowId = this.windows.length > 0
            ? this.windows[this.windows.length - 1].id
            : null
        }
        this.save()
      }
    },

    minimizeWindow(windowId: string) {
      const window = this.windows.find(w => w.id === windowId)
      if (window) {
        window.state = 'minimized'
        if (this.activeWindowId === windowId) {
          // Focus next visible window
          const visibleWindows = this.windows.filter(w => w.state !== 'minimized')
          this.activeWindowId = visibleWindows.length > 0
            ? visibleWindows[visibleWindows.length - 1].id
            : null
        }
        this.save()
      }
    },

    maximizeWindow(windowId: string) {
      const window = this.windows.find(w => w.id === windowId)
      if (window) {
        window.state = window.state === 'maximized' ? 'normal' : 'maximized'
        this.save()
      }
    },

    restoreWindow(windowId: string) {
      const window = this.windows.find(w => w.id === windowId)
      if (window) {
        window.state = 'normal'
        this.focusWindow(windowId)
        this.save()
      }
    },

    focusWindow(windowId: string) {
      const window = this.windows.find(w => w.id === windowId)
      if (window) {
        window.zIndex = ++this.maxZIndex
        this.activeWindowId = windowId
      }
    },

    updateWindowPosition(windowId: string, position: Partial<WindowPosition>) {
      const window = this.windows.find(w => w.id === windowId)
      if (window) {
        Object.assign(window.position, position)
        this.save()
      }
    },

    updateWindowData(windowId: string, data: Record<string, unknown>) {
      const window = this.windows.find(w => w.id === windowId)
      if (window) {
        window.data = { ...window.data, ...data }
      }
    },

    updateWindowTitle(windowId: string, title: string) {
      const window = this.windows.find(w => w.id === windowId)
      if (window) {
        if (!window.data?.originalTitle) {
          this.updateWindowData(windowId, { originalTitle: window.title })
        }
        window.title = title
      }
    },

    restoreWindowTitle(windowId: string) {
      const window = this.windows.find(w => w.id === windowId)
      if (window && window.data?.originalTitle) {
        window.title = window.data.originalTitle as string
      }
    },

    setWindowState(windowId: string, state: WindowState) {
      const window = this.windows.find(w => w.id === windowId)
      if (window) {
        window.state = state
        this.save()
      }
    },

    switchToNextWindow() {
      const visibleWindows = this.windows.filter(w => w.state !== 'minimized')
      if (visibleWindows.length <= 1) return

      const currentIndex = visibleWindows.findIndex(w => w.id === this.activeWindowId)
      const nextIndex = (currentIndex + 1) % visibleWindows.length
      this.focusWindow(visibleWindows[nextIndex].id)
    },

    async openTerminalWithCwd(cwd: string, projectName: string, initialCommand?: string, sourceIconId?: string, extraData?: Record<string, unknown>) {
      // Load terminal component
      let component: Component | string = 'terminal'
      if (componentMap['terminal']) {
        try {
          const module = await componentMap['terminal']()
          component = markRaw(module.default)
        } catch (e) {
          console.error('Failed to load terminal component:', e)
        }
      }

      // Calculate default position using settings
      const settingsStore = useSettingsStore()
      const offset = this.windows.length * 30
      const defaultPosition: WindowPosition = {
        x: 100 + offset,
        y: 80 + offset,
        width: settingsStore.defaultWindowWidth || 1000,
        height: settingsStore.defaultWindowHeight || 600
      }

      const windowId = `window-${Date.now()}`
      const newWindow: WindowInstance = {
        id: windowId,
        iconId: sourceIconId || `project-terminal-${Date.now()}`,
        title: `${projectName} - 终端`,
        component,
        state: 'normal',
        position: defaultPosition,
        zIndex: ++this.maxZIndex,
        data: { cwd, initialCommand, ...extraData } // Pass cwd, initialCommand and extra data
      }

      this.windows.push(newWindow)
      this.activeWindowId = windowId
      this.save()

      return newWindow
    },

    save() {
      // Save window state (without component references)
      const saveData = this.windows.map(w => ({
        id: w.id,
        iconId: w.iconId,
        title: w.title,
        state: w.state,
        position: w.position,
        zIndex: w.zIndex
      }))
      localStorage.setItem('mcd-windows', JSON.stringify({
        windows: saveData,
        activeWindowId: this.activeWindowId
      }))
    },

    load() {
      const saved = localStorage.getItem('mcd-windows')
      if (saved) {
        try {
          const data = JSON.parse(saved)
          this.activeWindowId = data.activeWindowId
          this.maxZIndex = Math.max(...data.windows.map((w: WindowInstance) => w.zIndex), 1000)
        } catch (e) {
          console.error('Failed to load windows:', e)
        }
      }
    }
  }
})
