/**
 * Sample test to verify Vitest + Pinia testing infrastructure.
 * This validates:
 * - Vitest runs correctly
 * - @/ path alias resolves
 * - Pinia store testing works
 * - happy-dom environment is functional
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

// Mock Tauri APIs before importing stores
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

describe('Test Infrastructure', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('should create a Pinia store instance', async () => {
    // Dynamic import after mocks are set up
    const { useSettingsStore } = await import('@/stores/settingsStore')
    const store = useSettingsStore()
    expect(store).toBeDefined()
    expect(store.terminalFontSize).toBe(14)
  })

  it('should resolve @/ path alias', async () => {
    const { useSettingsStore } = await import('@/stores/settingsStore')
    const store = useSettingsStore()
    expect(store.defaultCli).toBe('powershell')
  })

  it('should support happy-dom environment', () => {
    expect(typeof document).toBe('object')
    expect(typeof window).toBe('object')
  })
})
