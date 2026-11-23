import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useSettingsStore } from '@/stores/settingsStore'

export interface UpdateCheckResult {
  update_available: boolean
  current_version: string
  remote_version: string | null
  release_notes: string | null
  download_url: string | null
  download_size: number | null
}

export interface DownloadProgressPayload {
  downloaded: number
  total: number
  percent: number
}

export type ModalPhase = 'confirm' | 'downloading' | 'downloaded'

// Singleton state shared across all callers
const checking = ref(false)
const downloading = ref(false)
const downloadProgress = ref<DownloadProgressPayload>({ downloaded: 0, total: 0, percent: 0 })
const lastCheckResult = ref<UpdateCheckResult | null>(null)
const error = ref('')
const showModal = ref(false)
const modalPhase = ref<ModalPhase>('confirm')
const installerPath = ref('')

export function useAutoUpdate() {
  const checkForUpdate = async (options?: { silent?: boolean }): Promise<UpdateCheckResult | null> => {
    const silent = options?.silent ?? false
    checking.value = true
    error.value = ''

    try {
      const settingsStore = useSettingsStore()
      const updateUrl = settingsStore.updateUrl || undefined

      const result = await invoke<UpdateCheckResult>('check_for_update', { updateUrl })
      lastCheckResult.value = result
      return result
    } catch (e) {
      if (!silent) {
        error.value = String(e)
      }
      return null
    } finally {
      checking.value = false
    }
  }

  const confirmUpdate = async (): Promise<void> => {
    const result = lastCheckResult.value
    if (!result?.update_available || !result.download_url) return

    modalPhase.value = 'downloading'
    downloading.value = true
    downloadProgress.value = { downloaded: 0, total: 0, percent: 0 }
    error.value = ''

    const unlisten = await listen<DownloadProgressPayload>('update-download-progress', (event) => {
      downloadProgress.value = event.payload
    })

    try {
      const path = await invoke<string>('download_update', {
        downloadUrl: result.download_url,
        totalSize: result.download_size ?? null,
      })

      unlisten()
      downloading.value = false
      installerPath.value = path
      modalPhase.value = 'downloaded'
    } catch (e) {
      unlisten()
      downloading.value = false
      error.value = String(e)
      modalPhase.value = 'confirm'
    }
  }

  const confirmInstall = async (): Promise<void> => {
    if (!installerPath.value) return

    try {
      await invoke('install_update', { installerPath: installerPath.value })
    } catch (e) {
      error.value = String(e)
    }
  }

  const cancelUpdate = (): void => {
    showModal.value = false
    modalPhase.value = 'confirm'
    error.value = ''
  }

  const checkAndPrompt = async (options?: { silent?: boolean }): Promise<void> => {
    const result = await checkForUpdate(options)
    if (result?.update_available && result.download_url) {
      modalPhase.value = 'confirm'
      showModal.value = true
    }
  }

  return {
    checking,
    downloading,
    downloadProgress,
    lastCheckResult,
    error,
    showModal,
    modalPhase,
    installerPath,
    checkForUpdate,
    checkAndPrompt,
    confirmUpdate,
    confirmInstall,
    cancelUpdate,
  }
}
