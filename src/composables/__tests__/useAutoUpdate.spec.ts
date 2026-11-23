/**
 * Unit tests for useAutoUpdate composable
 * Tests: checkForUpdate, confirmUpdate, confirmInstall, cancelUpdate, checkAndPrompt
 *
 * Uses static import + manual singleton state reset (no vi.resetModules)
 * to ensure v8 coverage provider can track this file.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

// --- Tauri mocks ---
const mockInvoke = vi.fn()
const mockListen = vi.fn()

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}))

vi.mock('@tauri-apps/plugin-fs', () => ({
  readTextFile: vi.fn(),
  writeTextFile: vi.fn(),
  exists: vi.fn(),
  mkdir: vi.fn(),
  BaseDirectory: { Home: 'Home' },
}))

// Static import — v8 coverage tracks this correctly
import { useAutoUpdate } from '@/composables/useAutoUpdate'

/**
 * Reset singleton refs to their initial values between tests.
 * The composable exposes all singletons, so we mutate them directly.
 */
function resetState() {
  const c = useAutoUpdate()
  c.checking.value = false
  c.downloading.value = false
  c.downloadProgress.value = { downloaded: 0, total: 0, percent: 0 }
  c.lastCheckResult.value = null
  c.error.value = ''
  c.showModal.value = false
  c.modalPhase.value = 'confirm'
  c.installerPath.value = ''
}

describe('useAutoUpdate', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    setActivePinia(createPinia())
    resetState()
  })

  function getComposable() {
    return useAutoUpdate()
  }

  // -- checkForUpdate --

  describe('checkForUpdate', () => {
    it('should return update result on success', async () => {
      const updateResult = {
        update_available: true,
        current_version: '1.0.0',
        remote_version: '1.1.0',
        release_notes: 'Bug fixes',
        download_url: 'https://example.com/update.exe',
        download_size: 1024,
      }
      mockInvoke.mockResolvedValueOnce(updateResult)

      const composable = getComposable()

      expect(composable.checking.value).toBe(false)
      const result = await composable.checkForUpdate()
      expect(composable.checking.value).toBe(false)
      expect(result).toEqual(updateResult)
      expect(composable.lastCheckResult.value).toEqual(updateResult)
      expect(composable.error.value).toBe('')
    })

    it('should set error on failure (non-silent)', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Network error'))

      const composable = getComposable()
      const result = await composable.checkForUpdate()

      expect(result).toBeNull()
      expect(composable.error.value).toBe('Error: Network error')
      expect(composable.checking.value).toBe(false)
    })

    it('should suppress error in silent mode', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Network error'))

      const composable = getComposable()
      const result = await composable.checkForUpdate({ silent: true })

      expect(result).toBeNull()
      expect(composable.error.value).toBe('')
    })

    it('should pass updateUrl from settings store', async () => {
      const { useSettingsStore } = await import('@/stores/settingsStore')
      const store = useSettingsStore()
      store.updateUrl = 'https://custom.cdn/updates'

      mockInvoke.mockResolvedValueOnce({
        update_available: false,
        current_version: '1.0.0',
        remote_version: null,
        release_notes: null,
        download_url: null,
        download_size: null,
      })

      const composable = getComposable()
      await composable.checkForUpdate()

      expect(mockInvoke).toHaveBeenCalledWith('check_for_update', {
        updateUrl: 'https://custom.cdn/updates',
      })
    })

    it('should pass undefined updateUrl when store value is empty', async () => {
      mockInvoke.mockResolvedValueOnce({
        update_available: false,
        current_version: '1.0.0',
        remote_version: null,
        release_notes: null,
        download_url: null,
        download_size: null,
      })

      const composable = getComposable()
      await composable.checkForUpdate()

      expect(mockInvoke).toHaveBeenCalledWith('check_for_update', {
        updateUrl: undefined,
      })
    })

    it('should set checking=true while in progress', async () => {
      let resolveInvoke: (v: unknown) => void
      mockInvoke.mockReturnValueOnce(
        new Promise((resolve) => { resolveInvoke = resolve })
      )

      const composable = getComposable()
      const promise = composable.checkForUpdate()

      expect(composable.checking.value).toBe(true)

      resolveInvoke!({
        update_available: false,
        current_version: '1.0.0',
        remote_version: null,
        release_notes: null,
        download_url: null,
        download_size: null,
      })
      await promise

      expect(composable.checking.value).toBe(false)
    })
  })

  // -- confirmUpdate --

  describe('confirmUpdate', () => {
    it('should download update and set installerPath on success', async () => {
      const mockUnlisten = vi.fn()
      mockListen.mockResolvedValueOnce(mockUnlisten)
      mockInvoke.mockResolvedValueOnce('/path/to/installer.exe')

      const composable = getComposable()

      // Pre-set lastCheckResult as if checkForUpdate was called
      composable.lastCheckResult.value = {
        update_available: true,
        current_version: '1.0.0',
        remote_version: '1.1.0',
        release_notes: null,
        download_url: 'https://example.com/update.exe',
        download_size: 2048,
      }

      await composable.confirmUpdate()

      expect(composable.modalPhase.value).toBe('downloaded')
      expect(composable.installerPath.value).toBe('/path/to/installer.exe')
      expect(composable.downloading.value).toBe(false)
      expect(mockUnlisten).toHaveBeenCalled()

      expect(mockInvoke).toHaveBeenCalledWith('download_update', {
        downloadUrl: 'https://example.com/update.exe',
        totalSize: 2048,
      })
    })

    it('should handle download error and reset to confirm phase', async () => {
      const mockUnlisten = vi.fn()
      mockListen.mockResolvedValueOnce(mockUnlisten)
      mockInvoke.mockRejectedValueOnce(new Error('Download failed'))

      const composable = getComposable()
      composable.lastCheckResult.value = {
        update_available: true,
        current_version: '1.0.0',
        remote_version: '1.1.0',
        release_notes: null,
        download_url: 'https://example.com/update.exe',
        download_size: null,
      }

      await composable.confirmUpdate()

      expect(composable.error.value).toBe('Error: Download failed')
      expect(composable.modalPhase.value).toBe('confirm')
      expect(composable.downloading.value).toBe(false)
      expect(mockUnlisten).toHaveBeenCalled()
    })

    it('should do nothing when no update result available', async () => {
      const composable = getComposable()
      composable.lastCheckResult.value = null

      await composable.confirmUpdate()

      expect(mockListen).not.toHaveBeenCalled()
      expect(mockInvoke).not.toHaveBeenCalled()
    })

    it('should do nothing when update not available', async () => {
      const composable = getComposable()
      composable.lastCheckResult.value = {
        update_available: false,
        current_version: '1.0.0',
        remote_version: null,
        release_notes: null,
        download_url: null,
        download_size: null,
      }

      await composable.confirmUpdate()

      expect(mockListen).not.toHaveBeenCalled()
    })

    it('should forward download progress events', async () => {
      let progressCallback: ((event: { payload: unknown }) => void) | null = null
      const mockUnlisten = vi.fn()
      mockListen.mockImplementationOnce((_event: string, cb: (event: { payload: unknown }) => void) => {
        progressCallback = cb
        return Promise.resolve(mockUnlisten)
      })

      let resolveDownload: (v: string) => void
      mockInvoke.mockReturnValueOnce(
        new Promise<string>((resolve) => { resolveDownload = resolve })
      )

      const composable = getComposable()
      composable.lastCheckResult.value = {
        update_available: true,
        current_version: '1.0.0',
        remote_version: '1.1.0',
        release_notes: null,
        download_url: 'https://example.com/update.exe',
        download_size: 1000,
      }

      const promise = composable.confirmUpdate()

      // Simulate progress events
      progressCallback!({ payload: { downloaded: 500, total: 1000, percent: 50 } })
      expect(composable.downloadProgress.value).toEqual({ downloaded: 500, total: 1000, percent: 50 })

      progressCallback!({ payload: { downloaded: 1000, total: 1000, percent: 100 } })
      expect(composable.downloadProgress.value).toEqual({ downloaded: 1000, total: 1000, percent: 100 })

      resolveDownload!('/installer.exe')
      await promise
    })

    it('should pass null totalSize when download_size is null', async () => {
      const mockUnlisten = vi.fn()
      mockListen.mockResolvedValueOnce(mockUnlisten)
      mockInvoke.mockResolvedValueOnce('/installer.exe')

      const composable = getComposable()
      composable.lastCheckResult.value = {
        update_available: true,
        current_version: '1.0.0',
        remote_version: '1.1.0',
        release_notes: null,
        download_url: 'https://example.com/update.exe',
        download_size: null,
      }

      await composable.confirmUpdate()

      expect(mockInvoke).toHaveBeenCalledWith('download_update', {
        downloadUrl: 'https://example.com/update.exe',
        totalSize: null,
      })
    })
  })

  // -- confirmInstall --

  describe('confirmInstall', () => {
    it('should invoke install_update with installer path', async () => {
      mockInvoke.mockResolvedValueOnce(undefined)

      const composable = getComposable()
      composable.installerPath.value = '/path/to/installer.exe'

      await composable.confirmInstall()

      expect(mockInvoke).toHaveBeenCalledWith('install_update', {
        installerPath: '/path/to/installer.exe',
      })
    })

    it('should set error on install failure', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Install failed'))

      const composable = getComposable()
      composable.installerPath.value = '/path/to/installer.exe'

      await composable.confirmInstall()

      expect(composable.error.value).toBe('Error: Install failed')
    })

    it('should do nothing when installerPath is empty', async () => {
      const composable = getComposable()
      composable.installerPath.value = ''

      await composable.confirmInstall()

      expect(mockInvoke).not.toHaveBeenCalled()
    })
  })

  // -- cancelUpdate --

  describe('cancelUpdate', () => {
    it('should reset modal state', () => {
      const composable = getComposable()
      composable.showModal.value = true
      composable.modalPhase.value = 'downloading'
      composable.error.value = 'some error'

      composable.cancelUpdate()

      expect(composable.showModal.value).toBe(false)
      expect(composable.modalPhase.value).toBe('confirm')
      expect(composable.error.value).toBe('')
    })
  })

  // -- checkAndPrompt --

  describe('checkAndPrompt', () => {
    it('should show modal when update is available', async () => {
      const updateResult = {
        update_available: true,
        current_version: '1.0.0',
        remote_version: '1.1.0',
        release_notes: null,
        download_url: 'https://example.com/update.exe',
        download_size: null,
      }
      mockInvoke.mockResolvedValueOnce(updateResult)

      const composable = getComposable()
      await composable.checkAndPrompt()

      expect(composable.showModal.value).toBe(true)
      expect(composable.modalPhase.value).toBe('confirm')
    })

    it('should not show modal when no update available', async () => {
      mockInvoke.mockResolvedValueOnce({
        update_available: false,
        current_version: '1.0.0',
        remote_version: null,
        release_notes: null,
        download_url: null,
        download_size: null,
      })

      const composable = getComposable()
      await composable.checkAndPrompt()

      expect(composable.showModal.value).toBe(false)
    })

    it('should not show modal when update available but no download_url', async () => {
      mockInvoke.mockResolvedValueOnce({
        update_available: true,
        current_version: '1.0.0',
        remote_version: '1.1.0',
        release_notes: null,
        download_url: null,
        download_size: null,
      })

      const composable = getComposable()
      await composable.checkAndPrompt()

      expect(composable.showModal.value).toBe(false)
    })

    it('should pass silent option through to checkForUpdate', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('fail'))

      const composable = getComposable()
      await composable.checkAndPrompt({ silent: true })

      expect(composable.error.value).toBe('')
      expect(composable.showModal.value).toBe(false)
    })
  })
})
