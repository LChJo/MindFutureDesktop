import { ref, reactive, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useSettingsStore } from '@/stores/settingsStore'
import { ENV_DEFAULTS } from '@/config/defaults'

// Shared interfaces
export interface DownloadProgress {
  component: string
  downloaded: number
  total: number
  percent: number
  stage: string // "downloading", "extracting", "done", "error"
  message: string
}

export interface NodejsStatus {
  installed: boolean
  version: string | null
  path: string | null
}

export interface GitStatus {
  installed: boolean
  version: string | null
  path: string | null
  source: string | null
  error: string | null
}

export interface UvStatus {
  installed: boolean
  version: string | null
  path: string | null
  source: string | null
  error: string | null
}

export interface ClaudeCodeStatus {
  installed: boolean
  version: string | null
  error: string | null
}

export function useSetupGuide() {
  const settingsStore = useSettingsStore()

  // Download progress state
  const gitDownloadProgress = ref<DownloadProgress | null>(null)
  const uvDownloadProgress = ref<DownloadProgress | null>(null)
  let unlistenProgress: UnlistenFn | null = null

  // Dependency status
  const isChecking = ref(true)
  const setupStep = ref('')
  const installError = ref('')

  // Node.js state
  const nodejsReady = ref(false)
  const nodejsVersion = ref<string | null>(null)
  const nodejsInstalling = ref(false)

  // Git state
  const gitReady = ref(false)
  const gitVersion = ref<string | null>(null)
  const gitDownloading = ref(false)

  // UV state
  const uvReady = ref(false)
  const uvVersion = ref<string | null>(null)
  const uvInstalling = ref(false)

  // Claude Code state
  const claudeCodeReady = ref(false)
  const claudeCodeInstalling = ref(false)
  const claudeCodeError = ref<string | null>(null)

  // Environment variables state
  const envVars = reactive({ ...ENV_DEFAULTS })
  const envVarsReady = ref(false)
  const envVarsSaved = ref(false)

  // Computed: all basic dependencies ready
  // UV is now optional, so it doesn't block initialization
  const allDepsReady = computed(() => {
    return nodejsReady.value && gitReady.value && claudeCodeReady.value
  })

  // Computed: all environment variables filled
  const allEnvVarsReady = computed(() => {
    return envVars.ANTHROPIC_AUTH_TOKEN.trim() !== '' &&
           envVars.ANTHROPIC_MODEL.trim() !== '' &&
           envVars.ANTHROPIC_BASE_URL.trim() !== ''
  })

  // Setup download progress listener
  const setupProgressListener = async () => {
    unlistenProgress = await listen<DownloadProgress>('download-progress', (event) => {
      const progress = event.payload
      if (progress.component === 'git') {
        gitDownloadProgress.value = progress
        if (progress.stage === 'done' || progress.stage === 'error') {
          setTimeout(() => {
            gitDownloadProgress.value = null
          }, 2000)
        }
      } else if (progress.component === 'uv') {
        uvDownloadProgress.value = progress
        if (progress.stage === 'done' || progress.stage === 'error') {
          setTimeout(() => {
            uvDownloadProgress.value = null
          }, 2000)
        }
      }
    })
  }

  // Cleanup progress listener
  const cleanupProgressListener = () => {
    if (unlistenProgress) {
      unlistenProgress()
      unlistenProgress = null
    }
  }

  // Check all dependencies status
  const checkStatus = async () => {
    isChecking.value = true
    installError.value = ''
    claudeCodeError.value = null
    setupStep.value = '正在初始化...'

    try {
      // Step 1: Check Node.js status
      setupStep.value = '正在检测 Node.js...'
      const nodejsStatus = await invoke<NodejsStatus>('check_nodejs_status')
      nodejsReady.value = nodejsStatus.installed
      nodejsVersion.value = nodejsStatus.version

      // Auto-install Node.js if not installed (it's bundled)
      if (!nodejsReady.value) {
        setupStep.value = '正在安装 Node.js...'
        nodejsInstalling.value = true
        try {
          await invoke('extract_nodejs_cmd')
          const recheckNodejs = await invoke<NodejsStatus>('check_nodejs_status')
          nodejsReady.value = recheckNodejs.installed
          nodejsVersion.value = recheckNodejs.version
        } catch (e) {
          installError.value = `Node.js 释放失败: ${e}`
          nodejsInstalling.value = false
          isChecking.value = false
          return
        }
        nodejsInstalling.value = false
      }

      // Step 2: Check Git status
      setupStep.value = '正在检测 Git...'
      const gitStatus = await invoke<GitStatus>('check_git_status')
      gitReady.value = gitStatus.installed
      gitVersion.value = gitStatus.version

      // Auto-install Git if not installed
      if (!gitReady.value) {
        isChecking.value = false
        gitDownloading.value = true
        gitDownloadProgress.value = null
        try {
          await invoke('install_git_cmd')
          const recheckGit = await invoke<GitStatus>('check_git_status')
          gitReady.value = recheckGit.installed
          gitVersion.value = recheckGit.version
        } catch (e) {
          installError.value = `Git 安装失败: ${e}`
          gitDownloading.value = false
          return
        }
        gitDownloading.value = false
        isChecking.value = true
      }

      // Step 3: Check UV status (UV is optional, no auto-install)
      setupStep.value = '正在检测 UV...'
      try {
        const uvStatus = await invoke<UvStatus>('check_uv_status')
        uvReady.value = uvStatus.installed
        uvVersion.value = uvStatus.version
        // UV is optional - just check status, don't auto-install
      } catch (e) {
        // UV check failed, but don't block - it's optional
        console.warn('UV check failed (optional):', e)
        uvReady.value = false
        uvVersion.value = null
      }

      // Step 4: Check Claude Code status
      setupStep.value = '正在检测 Claude Code...'
      const claudeStatus = await invoke<ClaudeCodeStatus>('check_claude_code_status')
      claudeCodeReady.value = claudeStatus.installed
      claudeCodeError.value = claudeStatus.error

      // Auto-install Claude Code if not installed and Node.js is ready
      if (!claudeCodeReady.value && nodejsReady.value) {
        setupStep.value = '正在安装 Claude Code...'
        claudeCodeInstalling.value = true
        try {
          await invoke<string>('install_claude_code_cmd')
          const recheckClaude = await invoke<ClaudeCodeStatus>('check_claude_code_status')
          claudeCodeReady.value = recheckClaude.installed
          claudeCodeError.value = recheckClaude.error
        } finally {
          claudeCodeInstalling.value = false
        }
      }

      // Load environment variables if Claude Code is installed
      if (claudeCodeReady.value) {
        checkEnvVars()
      }

      setupStep.value = '检测完成'
    } catch (e) {
      installError.value = `状态检测失败: ${e}`
      setupStep.value = '检测失败'
    } finally {
      isChecking.value = false
    }
  }

  // Install Git manually
  const installGit = async () => {
    gitDownloading.value = true
    gitDownloadProgress.value = null
    installError.value = ''

    try {
      await invoke('install_git_cmd')
      const gitStatus = await invoke<GitStatus>('check_git_status')
      gitReady.value = gitStatus.installed
      gitVersion.value = gitStatus.version
    } catch (e) {
      installError.value = `Git 安装失败: ${e}`
    } finally {
      gitDownloading.value = false
    }
  }

  // Install UV manually
  const installUv = async () => {
    uvInstalling.value = true
    uvDownloadProgress.value = null
    installError.value = ''

    try {
      await invoke('install_uv_cmd')
      const uvStatus = await invoke<UvStatus>('check_uv_status')
      uvReady.value = uvStatus.installed
      uvVersion.value = uvStatus.version
    } catch (e) {
      installError.value = `UV 安装失败: ${e}`
    } finally {
      uvInstalling.value = false
    }
  }

  // Install Claude Code manually
  const installClaudeCode = async () => {
    if (!nodejsReady.value) {
      installError.value = 'Node.js 尚未就绪'
      return
    }

    claudeCodeInstalling.value = true
    installError.value = ''

    try {
      await invoke<string>('install_claude_code_cmd')
      const claudeStatus = await invoke<ClaudeCodeStatus>('check_claude_code_status')
      claudeCodeReady.value = claudeStatus.installed
      claudeCodeError.value = claudeStatus.error

      if (!claudeStatus.installed) {
        installError.value = claudeStatus.error || '安装完成但验证失败'
      } else {
        checkEnvVars()
      }
    } catch (e) {
      installError.value = `安装失败: ${e}`
    } finally {
      claudeCodeInstalling.value = false
    }
  }

  // Check and load existing environment variables
  const checkEnvVars = () => {
    const saved = settingsStore.environmentVariables

    const hasToken = !!saved.ANTHROPIC_AUTH_TOKEN?.trim()
    const hasModel = !!saved.ANTHROPIC_MODEL?.trim()
    const hasBaseUrl = !!saved.ANTHROPIC_BASE_URL?.trim()

    envVarsReady.value = hasToken && hasModel && hasBaseUrl

    if (saved.ANTHROPIC_AUTH_TOKEN) envVars.ANTHROPIC_AUTH_TOKEN = saved.ANTHROPIC_AUTH_TOKEN
    if (saved.ANTHROPIC_MODEL) envVars.ANTHROPIC_MODEL = saved.ANTHROPIC_MODEL
    if (saved.ANTHROPIC_BASE_URL) envVars.ANTHROPIC_BASE_URL = saved.ANTHROPIC_BASE_URL
  }

  // Save environment variables
  const saveEnvVars = () => {
    settingsStore.setEnvironmentVariable('ANTHROPIC_AUTH_TOKEN', envVars.ANTHROPIC_AUTH_TOKEN.trim())
    settingsStore.setEnvironmentVariable('ANTHROPIC_MODEL', envVars.ANTHROPIC_MODEL.trim())
    settingsStore.setEnvironmentVariable('ANTHROPIC_BASE_URL', envVars.ANTHROPIC_BASE_URL.trim())

    settingsStore.customAnthropicModel = envVars.ANTHROPIC_MODEL.trim()
    settingsStore.save()

    envVarsReady.value = true
    envVarsSaved.value = true
  }

  // Load environment variables from store, falling back to system env vars
  const loadEnvVars = async () => {
    const saved = settingsStore.environmentVariables
    if (saved.ANTHROPIC_AUTH_TOKEN) envVars.ANTHROPIC_AUTH_TOKEN = saved.ANTHROPIC_AUTH_TOKEN
    if (saved.ANTHROPIC_MODEL) envVars.ANTHROPIC_MODEL = saved.ANTHROPIC_MODEL
    if (saved.ANTHROPIC_BASE_URL) envVars.ANTHROPIC_BASE_URL = saved.ANTHROPIC_BASE_URL

    // For empty fields, try reading from system environment variables
    const missingKeys: string[] = []
    if (!envVars.ANTHROPIC_AUTH_TOKEN.trim()) missingKeys.push('ANTHROPIC_AUTH_TOKEN')
    if (!envVars.ANTHROPIC_BASE_URL.trim()) missingKeys.push('ANTHROPIC_BASE_URL')

    if (missingKeys.length > 0) {
      try {
        const systemVars = await invoke<Record<string, string | null>>('get_system_env_vars', { keys: missingKeys })
        if (systemVars.ANTHROPIC_AUTH_TOKEN && !envVars.ANTHROPIC_AUTH_TOKEN.trim()) {
          envVars.ANTHROPIC_AUTH_TOKEN = systemVars.ANTHROPIC_AUTH_TOKEN
        }
        if (systemVars.ANTHROPIC_BASE_URL && !envVars.ANTHROPIC_BASE_URL.trim()) {
          envVars.ANTHROPIC_BASE_URL = systemVars.ANTHROPIC_BASE_URL
        }
      } catch (e) {
        console.error('Failed to read system environment variables:', e)
      }
    }
  }

  return {
    // Download progress
    gitDownloadProgress,
    uvDownloadProgress,

    // Status flags
    isChecking,
    setupStep,
    installError,

    // Node.js state
    nodejsReady,
    nodejsVersion,
    nodejsInstalling,

    // Git state
    gitReady,
    gitVersion,
    gitDownloading,

    // UV state
    uvReady,
    uvVersion,
    uvInstalling,

    // Claude Code state
    claudeCodeReady,
    claudeCodeInstalling,
    claudeCodeError,

    // Environment variables state
    envVars,
    envVarsReady,
    envVarsSaved,

    // Computed
    allDepsReady,
    allEnvVarsReady,

    // Methods
    setupProgressListener,
    cleanupProgressListener,
    checkStatus,
    installGit,
    installUv,
    installClaudeCode,
    checkEnvVars,
    saveEnvVars,
    loadEnvVars,
  }
}
