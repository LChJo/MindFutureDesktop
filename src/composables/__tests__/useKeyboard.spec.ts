/**
 * Unit tests for useKeyboard composable
 * Tests: parseKeyCombo, key handlers, lifecycle (onMounted/onUnmounted)
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, type VueWrapper } from '@vue/test-utils'
import { defineComponent } from 'vue'

// --- Tauri mocks ---
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-fs', () => ({
  readTextFile: vi.fn(),
  writeTextFile: vi.fn(),
  exists: vi.fn(),
  mkdir: vi.fn(),
  BaseDirectory: { Home: 'Home' },
}))

// --- Store mocks ---
// We mock the stores at the module level because useKeyboard imports them directly.
const mockOpenWindow = vi.fn()
const mockCloseWindow = vi.fn()
const mockSwitchToNextWindow = vi.fn()

const mockWindowStore = {
  activeWindowId: null as string | null,
  windows: [] as Array<{ id: string }>,
  openWindow: mockOpenWindow,
  closeWindow: mockCloseWindow,
  switchToNextWindow: mockSwitchToNextWindow,
}

const mockDesktopStore = {
  icons: [] as Array<{ id: string; type: string; name: string }>,
}

vi.mock('@/stores/windowStore', () => ({
  useWindowStore: () => mockWindowStore,
}))

vi.mock('@/stores/desktopStore', () => ({
  useDesktopStore: () => mockDesktopStore,
}))

import { useKeyboard } from '@/composables/useKeyboard'

// Store the keyboard composable instance so tests can call methods directly.
// This avoids TypeScript issues with wrapper.vm property access.
let kbInstance: ReturnType<typeof useKeyboard> | null = null

// Helper: create a test component that uses useKeyboard in its setup
function createTestComponent() {
  return defineComponent({
    setup() {
      const kb = useKeyboard()
      kbInstance = kb
      return { kb }
    },
    template: '<div></div>',
  })
}

// Helper: create a KeyboardEvent
function createKeyEvent(key: string, modifiers: { ctrl?: boolean; alt?: boolean; shift?: boolean; meta?: boolean } = {}): KeyboardEvent {
  return new KeyboardEvent('keydown', {
    key,
    ctrlKey: modifiers.ctrl ?? false,
    altKey: modifiers.alt ?? false,
    shiftKey: modifiers.shift ?? false,
    metaKey: modifiers.meta ?? false,
    bubbles: true,
    cancelable: true,
  })
}

describe('useKeyboard', () => {
  let wrapper: VueWrapper | null = null

  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()

    // Reset mock store state
    mockWindowStore.activeWindowId = null
    mockWindowStore.windows = []
    mockDesktopStore.icons = []
    kbInstance = null
  })

  afterEach(() => {
    if (wrapper) {
      wrapper.unmount()
      wrapper = null
    }
  })

  // -- parseKeyCombo (tested indirectly via keydown events) --

  describe('parseKeyCombo', () => {
    it('should handle ctrl+alt+t correctly', () => {
      const terminalIcon = { id: 'terminal', type: 'terminal', name: 'Terminal' }
      mockDesktopStore.icons = [terminalIcon]

      wrapper = mount(createTestComponent())

      const event = createKeyEvent('t', { ctrl: true, alt: true })
      window.dispatchEvent(event)

      expect(mockOpenWindow).toHaveBeenCalledWith(terminalIcon)
    })

    it('should handle ctrl+w correctly', () => {
      mockWindowStore.activeWindowId = 'win-1'

      wrapper = mount(createTestComponent())

      const event = createKeyEvent('w', { ctrl: true })
      window.dispatchEvent(event)

      expect(mockCloseWindow).toHaveBeenCalledWith('win-1')
    })

    it('should handle alt+tab correctly', () => {
      wrapper = mount(createTestComponent())

      const event = createKeyEvent('Tab', { alt: true })
      window.dispatchEvent(event)

      expect(mockSwitchToNextWindow).toHaveBeenCalled()
    })

    it('should handle ctrl+alt+f correctly', () => {
      const fileManagerIcon = { id: 'file-manager', type: 'file-manager', name: 'Files' }
      mockDesktopStore.icons = [fileManagerIcon]

      wrapper = mount(createTestComponent())

      const event = createKeyEvent('f', { ctrl: true, alt: true })
      window.dispatchEvent(event)

      expect(mockOpenWindow).toHaveBeenCalledWith(fileManagerIcon)
    })

    it('should ignore unmapped key combos', () => {
      wrapper = mount(createTestComponent())

      const event = createKeyEvent('x', { ctrl: true })
      window.dispatchEvent(event)

      expect(mockOpenWindow).not.toHaveBeenCalled()
      expect(mockCloseWindow).not.toHaveBeenCalled()
      expect(mockSwitchToNextWindow).not.toHaveBeenCalled()
    })

    it('should handle modifier-only keys without action', () => {
      wrapper = mount(createTestComponent())

      // Pressing just 'Control' key alone
      const event = createKeyEvent('Control', { ctrl: true })
      window.dispatchEvent(event)

      expect(mockOpenWindow).not.toHaveBeenCalled()
      expect(mockCloseWindow).not.toHaveBeenCalled()
    })

    it('should handle shift modifier in combo', () => {
      wrapper = mount(createTestComponent())

      // ctrl+shift+t is not mapped, should be ignored
      const event = createKeyEvent('t', { ctrl: true, shift: true })
      window.dispatchEvent(event)

      expect(mockOpenWindow).not.toHaveBeenCalled()
    })

    it('should handle space key normalization', () => {
      wrapper = mount(createTestComponent())

      // ctrl+space is not mapped, but space should be normalized
      const event = createKeyEvent(' ', { ctrl: true })
      window.dispatchEvent(event)

      // No mapped handler for ctrl+space, so no action
      expect(mockOpenWindow).not.toHaveBeenCalled()
    })
  })

  // -- Key handlers --

  describe('openTerminal', () => {
    it('should find terminal icon and open window', () => {
      const terminalIcon = { id: 'terminal', type: 'terminal', name: 'Terminal' }
      mockDesktopStore.icons = [terminalIcon, { id: 'other', type: 'settings', name: 'Settings' }]

      wrapper = mount(createTestComponent())
      kbInstance!.openTerminal()

      expect(mockOpenWindow).toHaveBeenCalledWith(terminalIcon)
    })

    it('should do nothing when no terminal icon exists', () => {
      mockDesktopStore.icons = [{ id: 'settings', type: 'settings', name: 'Settings' }]

      wrapper = mount(createTestComponent())
      kbInstance!.openTerminal()

      expect(mockOpenWindow).not.toHaveBeenCalled()
    })
  })

  describe('openFileManager', () => {
    it('should find file-manager icon and open window', () => {
      const fmIcon = { id: 'fm', type: 'file-manager', name: 'Files' }
      mockDesktopStore.icons = [fmIcon]

      wrapper = mount(createTestComponent())
      kbInstance!.openFileManager()

      expect(mockOpenWindow).toHaveBeenCalledWith(fmIcon)
    })

    it('should do nothing when no file-manager icon exists', () => {
      mockDesktopStore.icons = []

      wrapper = mount(createTestComponent())
      kbInstance!.openFileManager()

      expect(mockOpenWindow).not.toHaveBeenCalled()
    })
  })

  describe('closeActiveWindow', () => {
    it('should close the active window', () => {
      mockWindowStore.activeWindowId = 'active-win'

      wrapper = mount(createTestComponent())
      kbInstance!.closeActiveWindow()

      expect(mockCloseWindow).toHaveBeenCalledWith('active-win')
    })

    it('should do nothing when no active window', () => {
      mockWindowStore.activeWindowId = null

      wrapper = mount(createTestComponent())
      kbInstance!.closeActiveWindow()

      expect(mockCloseWindow).not.toHaveBeenCalled()
    })
  })

  describe('switchWindow', () => {
    it('should call switchToNextWindow', () => {
      wrapper = mount(createTestComponent())
      kbInstance!.switchWindow()

      expect(mockSwitchToNextWindow).toHaveBeenCalled()
    })
  })

  // -- Lifecycle --

  describe('lifecycle', () => {
    it('should add keydown listener on mount', () => {
      const addSpy = vi.spyOn(window, 'addEventListener')

      wrapper = mount(createTestComponent())

      expect(addSpy).toHaveBeenCalledWith('keydown', expect.any(Function))
      addSpy.mockRestore()
    })

    it('should remove keydown listener on unmount', () => {
      const removeSpy = vi.spyOn(window, 'removeEventListener')

      wrapper = mount(createTestComponent())
      wrapper.unmount()
      wrapper = null

      expect(removeSpy).toHaveBeenCalledWith('keydown', expect.any(Function))
      removeSpy.mockRestore()
    })

    it('should not respond to keys after unmount', () => {
      const terminalIcon = { id: 'terminal', type: 'terminal', name: 'Terminal' }
      mockDesktopStore.icons = [terminalIcon]

      wrapper = mount(createTestComponent())
      wrapper.unmount()
      wrapper = null

      // Dispatch after unmount
      const event = createKeyEvent('t', { ctrl: true, alt: true })
      window.dispatchEvent(event)

      expect(mockOpenWindow).not.toHaveBeenCalled()
    })
  })
})
