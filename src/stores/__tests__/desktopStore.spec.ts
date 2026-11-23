/**
 * Unit tests for desktopStore.
 * Tests icon CRUD, position management, wallpaper, and isIconDeletable getter.
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

describe('desktopStore', () => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let useDesktopStore: any

  beforeEach(async () => {
    setActivePinia(createPinia())
    localStorageMock.clear()
    vi.clearAllMocks()

    // Dynamic import after pinia is set up
    const mod = await import('@/stores/desktopStore')
    useDesktopStore = mod.useDesktopStore
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  // ─── Default state ───────────────────────────────────────────────

  describe('default state', () => {
    it('should start with empty icons and userIcons arrays', () => {
      const store = useDesktopStore()
      expect(store.icons).toEqual([])
      expect(store.userIcons).toEqual([])
    })

    it('should have default grid size of 100', () => {
      const store = useDesktopStore()
      expect(store.gridSize).toBe(100)
    })

    it('should have default wallpaper', () => {
      const store = useDesktopStore()
      expect(store.wallpaper).toBe('url(/wallpaper.png)')
    })
  })

  // ─── isIconDeletable getter ─────────────────────────────────────

  describe('isIconDeletable', () => {
    it('should return true for project- prefixed icons', () => {
      const store = useDesktopStore()
      expect(store.isIconDeletable('project-12345')).toBe(true)
    })

    it('should return true for icon- prefixed icons', () => {
      const store = useDesktopStore()
      expect(store.isIconDeletable('icon-12345')).toBe(true)
    })

    it('should return true for plugin- prefixed icons', () => {
      const store = useDesktopStore()
      expect(store.isIconDeletable('plugin-12345')).toBe(true)
    })

    it('should return true for dynfunc- prefixed icons', () => {
      const store = useDesktopStore()
      expect(store.isIconDeletable('dynfunc-12345')).toBe(true)
    })

    it('should return false for default system icons', () => {
      const store = useDesktopStore()
      expect(store.isIconDeletable('terminal')).toBe(false)
      expect(store.isIconDeletable('settings')).toBe(false)
      expect(store.isIconDeletable('coworks')).toBe(false)
    })
  })

  // ─── initializeIcons ────────────────────────────────────────────

  describe('initializeIcons', () => {
    it('should populate icons with default icons', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      // Default icons: terminal, add-project-link, settings, coworks, llm-router,
      // plugin-market, plugin-market-full, dynamic-function-creator, dev-tools,
      // claude-aliases, claude-bot = 11
      expect(store.icons.length).toBe(11)
    })

    it('should include user icons alongside defaults after initialization', () => {
      const store = useDesktopStore()
      store.userIcons = [
        {
          id: 'project-100',
          name: 'TestProject',
          icon: 'fa-solid fa-folder-open',
          position: { x: 200, y: 200 },
          type: 'project-link' as const,
          command: '/path/to/project'
        }
      ]
      store.initializeIcons()
      // 11 defaults + 1 user icon
      expect(store.icons.length).toBe(12)
      expect(store.icons.some((i: { id: string }) => i.id === 'project-100')).toBe(true)
    })

    it('should avoid positions occupied by user icons for default icons', () => {
      const store = useDesktopStore()
      // Place a user icon at position (0, 0) which is where the first default icon would go
      store.userIcons = [
        {
          id: 'project-100',
          name: 'Blocker',
          icon: 'fa-solid fa-folder-open',
          position: { x: 0, y: 0 },
          type: 'project-link' as const,
          command: '/path/to/project'
        }
      ]
      store.initializeIcons()
      // The first default icon (terminal) should NOT be at (0,0)
      const terminalIcon = store.icons.find((i: { id: string }) => i.id === 'terminal')
      expect(terminalIcon).toBeDefined()
      const atZeroZero = terminalIcon!.position.x === 0 && terminalIcon!.position.y === 0
      expect(atZeroZero).toBe(false)
    })
  })

  // ─── addIcon ────────────────────────────────────────────────────

  describe('addIcon', () => {
    it('should add a new icon with auto-generated id', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const result = store.addIcon({
        name: 'Custom',
        icon: 'fa-solid fa-star',
        position: { x: 300, y: 300 },
        type: 'custom' as const
      })
      expect(result.id).toMatch(/^icon-\d+$/)
      expect(result.name).toBe('Custom')
    })

    it('should add icon to both icons and userIcons arrays', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const before = store.icons.length
      store.addIcon({
        name: 'New',
        icon: 'fa-solid fa-star',
        position: { x: 300, y: 300 },
        type: 'custom' as const
      })
      expect(store.icons.length).toBe(before + 1)
      expect(store.userIcons.length).toBe(1)
    })

    it('should persist to localStorage after adding', () => {
      const store = useDesktopStore()
      store.addIcon({
        name: 'Saved',
        icon: 'fa-solid fa-star',
        position: { x: 0, y: 0 },
        type: 'custom' as const
      })
      expect(localStorageMock.setItem).toHaveBeenCalledWith('mcd-desktop', expect.any(String))
    })
  })

  // ─── removeIcon ─────────────────────────────────────────────────

  describe('removeIcon', () => {
    it('should remove a user-added icon', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const icon = store.addIcon({
        name: 'Removable',
        icon: 'fa-solid fa-star',
        position: { x: 300, y: 300 },
        type: 'custom' as const
      })
      const result = store.removeIcon(icon.id)
      expect(result).toBe(true)
      expect(store.userIcons.length).toBe(0)
    })

    it('should not remove a default system icon', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const result = store.removeIcon('terminal')
      expect(result).toBe(false)
    })

    it('should return false when icon id not found', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      // Force isIconDeletable to pass by using a deletable prefix
      const result = store.removeIcon('icon-nonexistent')
      expect(result).toBe(false)
    })

    it('should persist after removal', () => {
      const store = useDesktopStore()
      const icon = store.addIcon({
        name: 'WillRemove',
        icon: 'fa-solid fa-star',
        position: { x: 0, y: 0 },
        type: 'custom' as const
      })
      vi.clearAllMocks()
      store.removeIcon(icon.id)
      expect(localStorageMock.setItem).toHaveBeenCalled()
    })
  })

  // ─── addProjectLink ─────────────────────────────────────────────

  describe('addProjectLink', () => {
    it('should create a project link icon with correct type', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const icon = store.addProjectLink('/home/user/my-project')
      expect(icon.type).toBe('project-link')
      expect(icon.command).toBe('/home/user/my-project')
    })

    it('should extract project name from path (unix)', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const icon = store.addProjectLink('/home/user/my-project')
      expect(icon.name).toBe('my-project')
    })

    it('should extract project name from path (windows)', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const icon = store.addProjectLink('C:\\Users\\me\\MyProject')
      expect(icon.name).toBe('MyProject')
    })

    it('should generate id with project- prefix', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const icon = store.addProjectLink('/path')
      expect(icon.id).toMatch(/^project-\d+$/)
    })

    it('should find a non-conflicting position', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const icon = store.addProjectLink('/path/to/project')
      // Position should be on the grid
      expect(icon.position.x % 100).toBe(0)
      expect(icon.position.y % 100).toBe(0)
    })
  })

  // ─── addPluginShortcut ──────────────────────────────────────────

  describe('addPluginShortcut', () => {
    it('should create a plugin shortcut icon', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const result = store.addPluginShortcut('my-plugin', 'A plugin', 'marketplace', './skills/image-gen')
      expect(result.success).toBe(true)
      expect(result.icon.type).toBe('plugin-shortcut')
      expect(result.icon.id).toMatch(/^plugin-\d+$/)
    })

    it('should extract clean skill name for display', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const result = store.addPluginShortcut('my-plugin', 'desc', 'mp', './skills/image-gen')
      expect(result.icon.name).toBe('image-gen')
    })

    it('should store pluginInfo on the icon', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const result = store.addPluginShortcut('my-plugin', 'desc', 'mp', './skills/img')
      expect(result.icon.pluginInfo).toEqual({
        pluginName: 'my-plugin',
        pluginDescription: 'desc',
        marketplaceName: 'mp',
        skillName: './skills/img'
      })
    })

    it('should prevent duplicate plugin+skill combination', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      store.addPluginShortcut('plug', 'desc', 'mp', 'skill-a')
      const dup = store.addPluginShortcut('plug', 'desc', 'mp', 'skill-a')
      expect(dup.success).toBe(false)
      expect(dup.message).toContain('已存在')
    })

    it('should allow different skills from same plugin', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const r1 = store.addPluginShortcut('plug', 'desc', 'mp', 'skill-a')
      const r2 = store.addPluginShortcut('plug', 'desc', 'mp', 'skill-b')
      expect(r1.success).toBe(true)
      expect(r2.success).toBe(true)
    })
  })

  // ─── addDynamicFunction ─────────────────────────────────────────

  describe('addDynamicFunction', () => {
    it('should create a dynamic function icon', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const result = store.addDynamicFunction('my-func', 'My Function', 'fa-solid fa-cube', false)
      expect(result.success).toBe(true)
      expect(result.icon.type).toBe('dynamic-function')
      expect(result.icon.id).toMatch(/^dynfunc-\d+$/)
    })

    it('should store dynamicFunctionInfo', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const result = store.addDynamicFunction('my-func', 'My Function', '', true)
      expect(result.icon.dynamicFunctionInfo).toEqual({
        functionName: 'my-func',
        displayName: 'My Function',
        backendEnabled: true
      })
    })

    it('should use default icon when empty string provided', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const result = store.addDynamicFunction('fn', 'Fn', '', false)
      expect(result.icon.icon).toBe('fa-solid fa-cube')
    })

    it('should prevent duplicate function names', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      store.addDynamicFunction('fn', 'Fn', '', false)
      const dup = store.addDynamicFunction('fn', 'Fn2', '', false)
      expect(dup.success).toBe(false)
      expect(dup.message).toContain('已存在')
    })
  })

  // ─── updateIconPosition ─────────────────────────────────────────

  describe('updateIconPosition', () => {
    it('should update position of an existing icon', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const icon = store.addIcon({
        name: 'Move Me',
        icon: 'fa-solid fa-star',
        position: { x: 0, y: 0 },
        type: 'custom' as const
      })
      store.updateIconPosition(icon.id, { x: 500, y: 400 })
      const updated = store.icons.find((i: { id: string }) => i.id === icon.id)
      expect(updated!.position).toEqual({ x: 500, y: 400 })
    })

    it('should also update position in userIcons', () => {
      const store = useDesktopStore()
      const icon = store.addIcon({
        name: 'Sync',
        icon: 'fa-solid fa-star',
        position: { x: 0, y: 0 },
        type: 'custom' as const
      })
      store.updateIconPosition(icon.id, { x: 200, y: 300 })
      const userIcon = store.userIcons.find((i: { id: string }) => i.id === icon.id)
      expect(userIcon!.position).toEqual({ x: 200, y: 300 })
    })

    it('should do nothing for non-existent icon id', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const before = JSON.stringify(store.icons)
      store.updateIconPosition('nonexistent', { x: 999, y: 999 })
      expect(JSON.stringify(store.icons)).toBe(before)
    })

    it('should persist after position update', () => {
      const store = useDesktopStore()
      const icon = store.addIcon({
        name: 'Persist',
        icon: 'fa-solid fa-star',
        position: { x: 0, y: 0 },
        type: 'custom' as const
      })
      vi.clearAllMocks()
      store.updateIconPosition(icon.id, { x: 100, y: 100 })
      expect(localStorageMock.setItem).toHaveBeenCalled()
    })
  })

  // ─── findNextAvailablePosition ──────────────────────────────────

  describe('findNextAvailablePosition', () => {
    it('should return grid-aligned position', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const pos = store.findNextAvailablePosition(1, 1)
      expect(pos.x % 100).toBe(0)
      expect(pos.y % 100).toBe(0)
    })

    it('should skip occupied positions', () => {
      const store = useDesktopStore()
      // Place icons at (1,1) and (2,1) by adding user icons there
      store.initializeIcons()
      store.addIcon({
        name: 'A',
        icon: '',
        position: { x: 100, y: 100 },
        type: 'custom' as const
      })
      const pos = store.findNextAvailablePosition(1, 1)
      // Should not be (100, 100) because that is now occupied
      const isOccupied = pos.x === 100 && pos.y === 100
      expect(isOccupied).toBe(false)
    })
  })

  // ─── setWallpaper ───────────────────────────────────────────────

  describe('setWallpaper', () => {
    it('should update wallpaper value', () => {
      const store = useDesktopStore()
      store.setWallpaper('url(/new-bg.jpg)')
      expect(store.wallpaper).toBe('url(/new-bg.jpg)')
    })

    it('should persist wallpaper change', () => {
      const store = useDesktopStore()
      vi.clearAllMocks()
      store.setWallpaper('url(/bg.png)')
      expect(localStorageMock.setItem).toHaveBeenCalled()
    })
  })

  // ─── save / load round-trip ─────────────────────────────────────

  describe('save and load', () => {
    it('should save user icons to localStorage', () => {
      const store = useDesktopStore()
      store.addIcon({
        name: 'SaveTest',
        icon: 'fa-solid fa-star',
        position: { x: 200, y: 200 },
        type: 'custom' as const
      })
      const savedStr = localStorageMock.setItem.mock.calls[0][1] as string
      const saved = JSON.parse(savedStr)
      expect(saved.userIcons).toHaveLength(1)
      expect(saved.userIcons[0].name).toBe('SaveTest')
    })

    it('should not save default icons, only user icons', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      store.save()
      const savedStr = localStorageMock.setItem.mock.calls[0][1] as string
      const saved = JSON.parse(savedStr)
      expect(saved.userIcons).toHaveLength(0)
    })

    it('should load user icons and reinitialize defaults', () => {
      const store = useDesktopStore()
      // Prepare saved data in localStorage
      const savedData = {
        userIcons: [
          {
            id: 'project-999',
            name: 'Loaded',
            icon: 'fa-solid fa-folder-open',
            position: { x: 200, y: 200 },
            type: 'project-link',
            command: '/loaded/path'
          }
        ],
        gridSize: 100,
        wallpaper: 'url(/custom.png)'
      }
      localStorageMock.getItem.mockReturnValue(JSON.stringify(savedData))

      store.load()
      expect(store.userIcons.length).toBe(1)
      expect(store.userIcons[0].name).toBe('Loaded')
      expect(store.wallpaper).toBe('url(/custom.png)')
      // Default icons should also be present
      expect(store.icons.length).toBeGreaterThan(1)
    })

    it('should snap loaded icon positions to grid', () => {
      const store = useDesktopStore()
      const savedData = {
        userIcons: [
          {
            id: 'project-888',
            name: 'Unsnapped',
            icon: 'fa-solid fa-folder-open',
            position: { x: 157, y: 243 },
            type: 'project-link',
            command: '/path'
          }
        ],
        gridSize: 100,
        wallpaper: 'url(/wallpaper.png)'
      }
      localStorageMock.getItem.mockReturnValue(JSON.stringify(savedData))
      store.load()
      // 157 rounds to 200 (Math.round(157/100)*100)
      expect(store.userIcons[0].position.x).toBe(200)
      // 243 rounds to 200
      expect(store.userIcons[0].position.y).toBe(200)
    })

    it('should handle corrupted localStorage gracefully', () => {
      const store = useDesktopStore()
      localStorageMock.getItem.mockReturnValue('NOT_VALID_JSON')
      expect(() => store.load()).not.toThrow()
      // Icons should still be initialized with defaults
      expect(store.icons.length).toBeGreaterThan(0)
    })

    it('should handle missing localStorage data', () => {
      const store = useDesktopStore()
      localStorageMock.getItem.mockReturnValue(null as unknown as string)
      expect(() => store.load()).not.toThrow()
      expect(store.icons.length).toBeGreaterThan(0)
    })
  })

  // ─── updateDesktopHeight ────────────────────────────────────────

  describe('updateDesktopHeight', () => {
    it('should update height and re-layout icons', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const prevPositions = store.icons.map((i: { id: string; position: { x: number; y: number } }) => ({
        id: i.id,
        ...i.position
      }))

      // Significantly change height
      store.updateDesktopHeight(400)
      expect(store.desktopHeight).toBe(400)

      // After re-layout, positions may have changed
      const newPositions = store.icons.map((i: { id: string; position: { x: number; y: number } }) => ({
        id: i.id,
        ...i.position
      }))
      // At minimum, verify it did not crash and icons exist
      expect(store.icons.length).toBeGreaterThan(0)
      // With a height of 400, fewer icons fit per column, so layout should differ
      // (comparing JSON is a simple check)
      expect(JSON.stringify(newPositions)).not.toBe(JSON.stringify(prevPositions))
    })

    it('should not re-layout when height has not changed', () => {
      const store = useDesktopStore()
      store.desktopHeight = 800
      store.initializeIcons()
      const spy = vi.spyOn(store, 'initializeIcons')
      store.updateDesktopHeight(800)
      // initializeIcons should NOT be called again if height unchanged
      expect(spy).not.toHaveBeenCalled()
    })
  })

  // ─── removeProjectLink / removePluginShortcut / removeDynamicFunction ─

  describe('removal helpers', () => {
    it('removeProjectLink should delegate to removeIcon', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const icon = store.addProjectLink('/path/to/proj')
      expect(store.removeProjectLink(icon.id)).toBe(true)
      expect(store.userIcons.length).toBe(0)
    })

    it('removePluginShortcut should find and remove by pluginName', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      store.addPluginShortcut('my-plug', 'desc', 'mp', 'skill')
      expect(store.removePluginShortcut('my-plug')).toBe(true)
    })

    it('removePluginShortcut should return false when not found', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      expect(store.removePluginShortcut('nonexistent')).toBe(false)
    })

    it('removeDynamicFunction should find and remove by functionName', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      store.addDynamicFunction('fn', 'Fn', '', false)
      expect(store.removeDynamicFunction('fn')).toBe(true)
    })

    it('removeDynamicFunction should return false when not found', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      expect(store.removeDynamicFunction('missing')).toBe(false)
    })

    it('hasPluginShortcut should return true when exists', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      store.addPluginShortcut('my-plug', 'desc', 'mp', 'skill')
      expect(store.hasPluginShortcut('my-plug')).toBe(true)
    })

    it('hasPluginShortcut should return false when not exists', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      expect(store.hasPluginShortcut('no-plug')).toBe(false)
    })
  })

  // ─── getOccupiedPositions ───────────────────────────────────────

  describe('getOccupiedPositions', () => {
    it('should return set of occupied grid positions', () => {
      const store = useDesktopStore()
      store.initializeIcons()
      const occupied = store.getOccupiedPositions
      expect(occupied).toBeInstanceOf(Set)
      expect(occupied.size).toBeGreaterThan(0)
    })

    it('should include user icon positions', () => {
      const store = useDesktopStore()
      store.addIcon({
        name: 'A',
        icon: '',
        position: { x: 500, y: 500 },
        type: 'custom' as const
      })
      const occupied = store.getOccupiedPositions
      expect(occupied.has('5,5')).toBe(true)
    })
  })

  // ─── updateIconName ─────────────────────────────────────────────

  describe('updateIconName', () => {
    it('should update icon name in both arrays', () => {
      const store = useDesktopStore()
      const icon = store.addIcon({
        name: 'OldName',
        icon: '',
        position: { x: 0, y: 0 },
        type: 'custom' as const
      })
      store.updateIconName(icon.id, 'NewName')
      const updated = store.icons.find((i: { id: string }) => i.id === icon.id)
      expect(updated!.name).toBe('NewName')
      const userUpdated = store.userIcons.find((i: { id: string }) => i.id === icon.id)
      expect(userUpdated!.name).toBe('NewName')
    })

    it('should trim whitespace from name', () => {
      const store = useDesktopStore()
      const icon = store.addIcon({
        name: 'Old',
        icon: '',
        position: { x: 0, y: 0 },
        type: 'custom' as const
      })
      store.updateIconName(icon.id, '  Trimmed  ')
      const updated = store.icons.find((i: { id: string }) => i.id === icon.id)
      expect(updated!.name).toBe('Trimmed')
    })

    it('should not update if name is empty or whitespace only', () => {
      const store = useDesktopStore()
      const icon = store.addIcon({
        name: 'Keep',
        icon: '',
        position: { x: 0, y: 0 },
        type: 'custom' as const
      })
      store.updateIconName(icon.id, '   ')
      const current = store.icons.find((i: { id: string }) => i.id === icon.id)
      expect(current!.name).toBe('Keep')
    })
  })

  // ─── updateIconEnvironmentVariables ─────────────────────────────

  describe('updateIconEnvironmentVariables', () => {
    it('should set environment variables on an icon', () => {
      const store = useDesktopStore()
      const icon = store.addIcon({
        name: 'EnvIcon',
        icon: '',
        position: { x: 0, y: 0 },
        type: 'custom' as const
      })
      store.updateIconEnvironmentVariables(icon.id, { API_KEY: 'abc123' })
      const updated = store.icons.find((i: { id: string }) => i.id === icon.id)
      expect(updated!.environmentVariables).toEqual({ API_KEY: 'abc123' })
    })

    it('should clear environment variables when empty object provided', () => {
      const store = useDesktopStore()
      const icon = store.addIcon({
        name: 'ClearEnv',
        icon: '',
        position: { x: 0, y: 0 },
        type: 'custom' as const
      })
      store.updateIconEnvironmentVariables(icon.id, { KEY: 'val' })
      store.updateIconEnvironmentVariables(icon.id, {})
      const updated = store.icons.find((i: { id: string }) => i.id === icon.id)
      expect(updated!.environmentVariables).toBeUndefined()
    })
  })
})
