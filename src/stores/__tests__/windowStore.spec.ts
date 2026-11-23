/**
 * Unit tests for windowStore.
 * Tests window lifecycle: open, close, focus, minimize, maximize, restore,
 * switchToNextWindow, updateWindowPosition, updateWindowTitle/restoreWindowTitle.
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

// Mock Tauri APIs (settingsStore dependency)
vi.mock('@tauri-apps/plugin-fs', () => ({
  readTextFile: vi.fn(),
  writeTextFile: vi.fn(),
  exists: vi.fn(),
  mkdir: vi.fn(),
  BaseDirectory: { Home: 'Home' },
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

// Mock Vue dynamic imports used in componentMap
vi.mock('@/components/Terminal.vue', () => ({ default: { name: 'Terminal' } }))
vi.mock('@/components/FileManager.vue', () => ({ default: { name: 'FileManager' } }))
vi.mock('@/components/Settings.vue', () => ({ default: { name: 'Settings' } }))
vi.mock('@/components/PluginMarket.vue', () => ({ default: { name: 'PluginMarket' } }))
vi.mock('@/components/PluginMarketFull.vue', () => ({ default: { name: 'PluginMarketFull' } }))
vi.mock('@/components/McpManager.vue', () => ({ default: { name: 'McpManager' } }))
vi.mock('@/components/SetupGuideWindow.vue', () => ({ default: { name: 'SetupGuide' } }))
vi.mock('@/components/Coworks.vue', () => ({ default: { name: 'Coworks' } }))
vi.mock('@/components/LlmRouter.vue', () => ({ default: { name: 'LlmRouter' } }))
vi.mock('@/components/DynamicFunctionHost.vue', () => ({ default: { name: 'DynamicFunctionHost' } }))
vi.mock('@/components/claudeApps/ClaudeAppsView.vue', () => ({ default: { name: 'ClaudeAppsView' } }))
vi.mock('@/components/DevTools.vue', () => ({ default: { name: 'DevTools' } }))
vi.mock('@/components/ClaudeAliases.vue', () => ({ default: { name: 'ClaudeAliases' } }))
vi.mock('@/components/claudeBot/ClaudeBot.vue', () => ({ default: { name: 'ClaudeBot' } }))

// Helper to create a minimal DesktopIcon for testing
function makeIcon(overrides: Record<string, unknown> = {}) {
  return {
    id: `test-icon-${Date.now()}-${Math.random()}`,
    name: 'Test',
    icon: 'fa-solid fa-star',
    position: { x: 0, y: 0 },
    type: 'terminal' as const,
    ...overrides,
  }
}

describe('windowStore', () => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let useWindowStore: any

  beforeEach(async () => {
    setActivePinia(createPinia())
    localStorageMock.clear()
    vi.clearAllMocks()

    const mod = await import('@/stores/windowStore')
    useWindowStore = mod.useWindowStore
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  // ─── Default state ───────────────────────────────────────────────

  describe('default state', () => {
    it('should start with empty windows', () => {
      const store = useWindowStore()
      expect(store.windows).toEqual([])
    })

    it('should have null activeWindowId', () => {
      const store = useWindowStore()
      expect(store.activeWindowId).toBeNull()
    })

    it('should have maxZIndex of 1000', () => {
      const store = useWindowStore()
      expect(store.maxZIndex).toBe(1000)
    })
  })

  // ─── openWindow ─────────────────────────────────────────────────

  describe('openWindow', () => {
    it('should create a new window with correct properties', async () => {
      const store = useWindowStore()
      const icon = makeIcon({ id: 'terminal' })
      const win = await store.openWindow(icon)
      expect(win).toBeDefined()
      expect(win.id).toMatch(/^window-\d+$/)
      expect(win.iconId).toBe('terminal')
      expect(win.title).toBe('Test')
      expect(win.state).toBe('normal')
    })

    it('should set the new window as active', async () => {
      const store = useWindowStore()
      const icon = makeIcon({ id: 'settings', type: 'settings' })
      const win = await store.openWindow(icon)
      expect(store.activeWindowId).toBe(win.id)
    })

    it('should increment maxZIndex for each new window', async () => {
      const store = useWindowStore()
      const icon1 = makeIcon({ id: 'icon-1' })
      const icon2 = makeIcon({ id: 'icon-2' })
      const w1 = await store.openWindow(icon1)
      const w2 = await store.openWindow(icon2)
      expect(w2.zIndex).toBeGreaterThan(w1.zIndex)
    })

    it('should focus existing window instead of creating duplicate', async () => {
      const store = useWindowStore()
      const icon = makeIcon({ id: 'my-unique-icon' })
      const win1 = await store.openWindow(icon)
      const win2 = await store.openWindow(icon)
      expect(win1.id).toBe(win2.id)
      expect(store.windows.length).toBe(1)
    })

    it('should restore minimized window when re-opening same icon', async () => {
      const store = useWindowStore()
      const icon = makeIcon({ id: 'restore-test' })
      const win = await store.openWindow(icon)
      store.minimizeWindow(win.id)
      expect(win.state).toBe('minimized')

      await store.openWindow(icon)
      expect(win.state).toBe('normal')
    })

    it('should load component via dynamic import for known types', async () => {
      const store = useWindowStore()
      const icon = makeIcon({ id: 'term-icon', type: 'terminal' })
      const win = await store.openWindow(icon)
      // Component should be loaded (markRaw applied), not a string
      expect(typeof win.component).not.toBe('string')
    })

    it('should pass windowData to the window', async () => {
      const store = useWindowStore()
      const icon = makeIcon({ id: 'data-icon' })
      const win = await store.openWindow(icon, { key: 'value' })
      expect(win.data).toEqual({ key: 'value' })
    })

    it('should offset position based on number of existing windows', async () => {
      const store = useWindowStore()
      const icon1 = makeIcon({ id: 'off-1' })
      const icon2 = makeIcon({ id: 'off-2' })
      const w1 = await store.openWindow(icon1)
      const w2 = await store.openWindow(icon2)
      expect(w2.position.x).toBeGreaterThan(w1.position.x)
      expect(w2.position.y).toBeGreaterThan(w1.position.y)
    })
  })

  // ─── closeWindow ────────────────────────────────────────────────

  describe('closeWindow', () => {
    it('should remove the window from the list', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'close-me' }))
      store.closeWindow(win.id)
      expect(store.windows.length).toBe(0)
    })

    it('should set activeWindowId to last remaining window', async () => {
      const store = useWindowStore()
      const win1 = await store.openWindow(makeIcon({ id: 'keep' }))
      const win2 = await store.openWindow(makeIcon({ id: 'remove' }))
      store.closeWindow(win2.id)
      expect(store.activeWindowId).toBe(win1.id)
    })

    it('should set activeWindowId to null when last window closed', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'only-one' }))
      store.closeWindow(win.id)
      expect(store.activeWindowId).toBeNull()
    })

    it('should do nothing for nonexistent window id', async () => {
      const store = useWindowStore()
      await store.openWindow(makeIcon({ id: 'stable' }))
      store.closeWindow('window-nonexistent')
      expect(store.windows.length).toBe(1)
    })

    it('should persist after close', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'persist-close' }))
      vi.clearAllMocks()
      store.closeWindow(win.id)
      expect(localStorageMock.setItem).toHaveBeenCalledWith('mcd-windows', expect.any(String))
    })
  })

  // ─── focusWindow ────────────────────────────────────────────────

  describe('focusWindow', () => {
    it('should set the window as active', async () => {
      const store = useWindowStore()
      const win1 = await store.openWindow(makeIcon({ id: 'f1' }))
      await store.openWindow(makeIcon({ id: 'f2' }))
      store.focusWindow(win1.id)
      expect(store.activeWindowId).toBe(win1.id)
    })

    it('should increment zIndex on focus', async () => {
      const store = useWindowStore()
      const win1 = await store.openWindow(makeIcon({ id: 'z1' }))
      const win2 = await store.openWindow(makeIcon({ id: 'z2' }))
      const z2Before = win2.zIndex
      store.focusWindow(win1.id)
      expect(win1.zIndex).toBeGreaterThan(z2Before)
    })

    it('should increment maxZIndex counter', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'mx' }))
      const before = store.maxZIndex
      store.focusWindow(win.id)
      expect(store.maxZIndex).toBe(before + 1)
    })

    it('should do nothing for nonexistent window', async () => {
      const store = useWindowStore()
      await store.openWindow(makeIcon({ id: 'existing' }))
      const activeBefore = store.activeWindowId
      store.focusWindow('nonexistent')
      expect(store.activeWindowId).toBe(activeBefore)
    })
  })

  // ─── minimizeWindow ─────────────────────────────────────────────

  describe('minimizeWindow', () => {
    it('should set window state to minimized', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'min-1' }))
      store.minimizeWindow(win.id)
      expect(win.state).toBe('minimized')
    })

    it('should focus next visible window after minimize', async () => {
      const store = useWindowStore()
      const win1 = await store.openWindow(makeIcon({ id: 'v1' }))
      const win2 = await store.openWindow(makeIcon({ id: 'v2' }))
      store.minimizeWindow(win2.id)
      expect(store.activeWindowId).toBe(win1.id)
    })

    it('should set activeWindowId to null when all minimized', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'all-min' }))
      store.minimizeWindow(win.id)
      expect(store.activeWindowId).toBeNull()
    })

    it('should do nothing for nonexistent window', async () => {
      const store = useWindowStore()
      await store.openWindow(makeIcon({ id: 'safe' }))
      store.minimizeWindow('nonexistent')
      expect(store.windows[0].state).toBe('normal')
    })
  })

  // ─── maximizeWindow ─────────────────────────────────────────────

  describe('maximizeWindow', () => {
    it('should toggle from normal to maximized', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'max-1' }))
      store.maximizeWindow(win.id)
      expect(win.state).toBe('maximized')
    })

    it('should toggle from maximized back to normal', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'max-toggle' }))
      store.maximizeWindow(win.id)
      store.maximizeWindow(win.id)
      expect(win.state).toBe('normal')
    })

    it('should do nothing for nonexistent window', () => {
      const store = useWindowStore()
      expect(() => store.maximizeWindow('nonexistent')).not.toThrow()
    })
  })

  // ─── restoreWindow ──────────────────────────────────────────────

  describe('restoreWindow', () => {
    it('should restore minimized window to normal state', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'restore-1' }))
      store.minimizeWindow(win.id)
      store.restoreWindow(win.id)
      expect(win.state).toBe('normal')
    })

    it('should focus the restored window', async () => {
      const store = useWindowStore()
      const win1 = await store.openWindow(makeIcon({ id: 'r1' }))
      await store.openWindow(makeIcon({ id: 'r2' }))
      store.minimizeWindow(win1.id)
      store.restoreWindow(win1.id)
      expect(store.activeWindowId).toBe(win1.id)
    })

    it('should do nothing for nonexistent window', () => {
      const store = useWindowStore()
      expect(() => store.restoreWindow('nonexistent')).not.toThrow()
    })
  })

  // ─── switchToNextWindow ─────────────────────────────────────────

  describe('switchToNextWindow', () => {
    it('should cycle focus to the next visible window', async () => {
      const store = useWindowStore()
      const win1 = await store.openWindow(makeIcon({ id: 'sw1' }))
      const win2 = await store.openWindow(makeIcon({ id: 'sw2' }))
      // Currently active is win2
      store.focusWindow(win1.id)
      store.switchToNextWindow()
      expect(store.activeWindowId).toBe(win2.id)
    })

    it('should wrap around when at the end', async () => {
      const store = useWindowStore()
      const win1 = await store.openWindow(makeIcon({ id: 'wrap1' }))
      const win2 = await store.openWindow(makeIcon({ id: 'wrap2' }))
      // Focus win2, then switch -> should go to win1
      store.focusWindow(win2.id)
      store.switchToNextWindow()
      expect(store.activeWindowId).toBe(win1.id)
    })

    it('should skip minimized windows', async () => {
      const store = useWindowStore()
      const win1 = await store.openWindow(makeIcon({ id: 'skip1' }))
      const win2 = await store.openWindow(makeIcon({ id: 'skip2' }))
      const win3 = await store.openWindow(makeIcon({ id: 'skip3' }))
      store.minimizeWindow(win2.id)
      store.focusWindow(win1.id)
      store.switchToNextWindow()
      // Should skip minimized win2 and go to win3
      expect(store.activeWindowId).toBe(win3.id)
    })

    it('should do nothing when only one visible window', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'alone' }))
      store.switchToNextWindow()
      expect(store.activeWindowId).toBe(win.id)
    })

    it('should do nothing when no windows', () => {
      const store = useWindowStore()
      expect(() => store.switchToNextWindow()).not.toThrow()
    })
  })

  // ─── updateWindowPosition ───────────────────────────────────────

  describe('updateWindowPosition', () => {
    it('should update position properties', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'pos-1' }))
      store.updateWindowPosition(win.id, { x: 200, y: 150 })
      expect(win.position.x).toBe(200)
      expect(win.position.y).toBe(150)
    })

    it('should partially update position (only provided fields)', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'partial-pos' }))
      const originalHeight = win.position.height
      store.updateWindowPosition(win.id, { width: 800 })
      expect(win.position.width).toBe(800)
      expect(win.position.height).toBe(originalHeight)
    })

    it('should do nothing for nonexistent window', () => {
      const store = useWindowStore()
      expect(() => store.updateWindowPosition('nope', { x: 0 })).not.toThrow()
    })
  })

  // ─── updateWindowTitle / restoreWindowTitle ─────────────────────

  describe('updateWindowTitle and restoreWindowTitle', () => {
    it('should update the window title', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'title-1', name: 'Original' }))
      store.updateWindowTitle(win.id, 'New Title')
      expect(win.title).toBe('New Title')
    })

    it('should save original title before first update', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'title-save', name: 'First' }))
      store.updateWindowTitle(win.id, 'Second')
      expect(win.data?.originalTitle).toBe('First')
    })

    it('should not overwrite originalTitle on subsequent updates', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'title-keep', name: 'Original' }))
      store.updateWindowTitle(win.id, 'Changed Once')
      store.updateWindowTitle(win.id, 'Changed Twice')
      expect(win.data?.originalTitle).toBe('Original')
    })

    it('should restore original title', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'title-restore', name: 'Restore Me' }))
      store.updateWindowTitle(win.id, 'Temporary')
      store.restoreWindowTitle(win.id)
      expect(win.title).toBe('Restore Me')
    })

    it('should do nothing if no originalTitle to restore', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'no-restore', name: 'Keep' }))
      store.restoreWindowTitle(win.id)
      expect(win.title).toBe('Keep')
    })

    it('should do nothing for nonexistent window id', () => {
      const store = useWindowStore()
      expect(() => store.updateWindowTitle('nope', 'X')).not.toThrow()
      expect(() => store.restoreWindowTitle('nope')).not.toThrow()
    })
  })

  // ─── Getters ────────────────────────────────────────────────────

  describe('getters', () => {
    it('activeWindow should return the active window instance', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'active-g' }))
      expect(store.activeWindow?.id).toBe(win.id)
    })

    it('activeWindow should return undefined when no active window', () => {
      const store = useWindowStore()
      expect(store.activeWindow).toBeUndefined()
    })

    it('visibleWindows should exclude minimized windows', async () => {
      const store = useWindowStore()
      const win1 = await store.openWindow(makeIcon({ id: 'vis1' }))
      await store.openWindow(makeIcon({ id: 'vis2' }))
      store.minimizeWindow(win1.id)
      expect(store.visibleWindows.length).toBe(1)
    })

    it('minimizedWindows should only include minimized windows', async () => {
      const store = useWindowStore()
      const win1 = await store.openWindow(makeIcon({ id: 'mw1' }))
      await store.openWindow(makeIcon({ id: 'mw2' }))
      store.minimizeWindow(win1.id)
      expect(store.minimizedWindows.length).toBe(1)
      expect(store.minimizedWindows[0].id).toBe(win1.id)
    })
  })

  // ─── save/load ──────────────────────────────────────────────────

  describe('save and load', () => {
    it('should save window state to localStorage', async () => {
      const store = useWindowStore()
      await store.openWindow(makeIcon({ id: 'save-w' }))
      expect(localStorageMock.setItem).toHaveBeenCalledWith('mcd-windows', expect.any(String))
    })

    it('should load maxZIndex from saved data', () => {
      const store = useWindowStore()
      const savedData = {
        windows: [
          { id: 'w1', iconId: 'i1', title: 'T', state: 'normal', position: { x: 0, y: 0, width: 800, height: 600 }, zIndex: 1050 }
        ],
        activeWindowId: 'w1'
      }
      localStorageMock.getItem.mockReturnValue(JSON.stringify(savedData))
      store.load()
      expect(store.maxZIndex).toBe(1050)
      expect(store.activeWindowId).toBe('w1')
    })

    it('should handle corrupted localStorage gracefully', () => {
      const store = useWindowStore()
      localStorageMock.getItem.mockReturnValue('INVALID')
      expect(() => store.load()).not.toThrow()
    })

    it('should handle missing localStorage data', () => {
      const store = useWindowStore()
      localStorageMock.getItem.mockReturnValue(null as unknown as string)
      expect(() => store.load()).not.toThrow()
    })
  })

  // ─── updateWindowData ───────────────────────────────────────────

  describe('updateWindowData', () => {
    it('should merge new data with existing window data', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'data-merge' }), { key1: 'val1' })
      store.updateWindowData(win.id, { key2: 'val2' })
      expect(win.data).toEqual({ key1: 'val1', key2: 'val2' })
    })

    it('should do nothing for nonexistent window', () => {
      const store = useWindowStore()
      expect(() => store.updateWindowData('nope', { a: 1 })).not.toThrow()
    })
  })

  // ─── openTerminalWithCwd ─────────────────────────────────────────

  describe('openTerminalWithCwd', () => {
    it('should create a terminal window with cwd in data', async () => {
      const store = useWindowStore()
      const win = await store.openTerminalWithCwd('/home/user/project', 'MyProject')
      expect(win).toBeDefined()
      expect(win.data?.cwd).toBe('/home/user/project')
      expect(win.title).toBe('MyProject - 终端')
    })

    it('should set the window as active', async () => {
      const store = useWindowStore()
      const win = await store.openTerminalWithCwd('/path', 'Proj')
      expect(store.activeWindowId).toBe(win.id)
    })

    it('should pass initialCommand in data', async () => {
      const store = useWindowStore()
      const win = await store.openTerminalWithCwd('/path', 'Proj', 'npm install')
      expect(win.data?.initialCommand).toBe('npm install')
    })

    it('should use sourceIconId when provided', async () => {
      const store = useWindowStore()
      const win = await store.openTerminalWithCwd('/path', 'Proj', undefined, 'project-123')
      expect(win.iconId).toBe('project-123')
    })

    it('should generate iconId when sourceIconId not provided', async () => {
      const store = useWindowStore()
      const win = await store.openTerminalWithCwd('/path', 'Proj')
      expect(win.iconId).toMatch(/^project-terminal-\d+$/)
    })

    it('should merge extra data into window data', async () => {
      const store = useWindowStore()
      const win = await store.openTerminalWithCwd(
        '/path', 'Proj', undefined, undefined,
        { customKey: 'customValue' }
      )
      expect(win.data?.customKey).toBe('customValue')
      expect(win.data?.cwd).toBe('/path')
    })

    it('should load terminal component', async () => {
      const store = useWindowStore()
      const win = await store.openTerminalWithCwd('/path', 'Proj')
      // Component should be loaded via dynamic import (not a string)
      expect(typeof win.component).not.toBe('string')
    })

    it('should offset position based on existing windows', async () => {
      const store = useWindowStore()
      const win1 = await store.openTerminalWithCwd('/path1', 'P1')
      const win2 = await store.openTerminalWithCwd('/path2', 'P2')
      expect(win2.position.x).toBeGreaterThan(win1.position.x)
    })

    it('should persist after opening terminal', async () => {
      const store = useWindowStore()
      vi.clearAllMocks()
      await store.openTerminalWithCwd('/path', 'Proj')
      expect(localStorageMock.setItem).toHaveBeenCalledWith('mcd-windows', expect.any(String))
    })
  })

  // ─── setWindowState ─────────────────────────────────────────────

  describe('setWindowState', () => {
    it('should directly set window state', async () => {
      const store = useWindowStore()
      const win = await store.openWindow(makeIcon({ id: 'state-set' }))
      store.setWindowState(win.id, 'maximized')
      expect(win.state).toBe('maximized')
    })

    it('should do nothing for nonexistent window', () => {
      const store = useWindowStore()
      expect(() => store.setWindowState('nope', 'normal')).not.toThrow()
    })
  })
})
