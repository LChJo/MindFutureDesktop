<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { message } from '@tauri-apps/plugin-dialog'
import { useSettingsStore } from '@/stores/settingsStore'
import { useDesktopStore } from '@/stores/desktopStore'
import { useFileSystemStore } from '@/stores/fileSystemStore'
import { usePluginStore } from '@/stores/pluginStore'
import { useKeyboard } from '@/composables/useKeyboard'
import { useAutoUpdate } from '@/composables/useAutoUpdate'
import { useBotStore } from '@/stores/botStore'
import { useBotChannels } from '@/composables/claudeBot/useBotChannels'
import { useBotScheduler } from '@/composables/claudeBot/useBotScheduler'
import Desktop from '@/components/Desktop.vue'
import WindowManager from '@/components/WindowManager.vue'
import Taskbar from '@/components/Taskbar.vue'
import SetupGuide from '@/components/SetupGuide.vue'
import WidgetPanel from '@/components/WidgetPanel.vue'
import UpdateModal from '@/components/UpdateModal.vue'

const settingsStore = useSettingsStore()
const desktopStore = useDesktopStore()
const fileSystemStore = useFileSystemStore()
const pluginStore = usePluginStore()

// Setup guide visibility - default true until we check
const showSetupGuide = ref(true)
const isCheckingSetup = ref(true)
// Flag to skip directly to environment variables page
const skipToEnvPage = ref(false)

// Initialize keyboard shortcuts
useKeyboard()

// Auto-update (silent startup check)
const { checkAndPrompt } = useAutoUpdate()

// Check if all required dependencies are installed (Claude Code, Git)
// UV is optional and not checked here
const checkAllDependencies = async (): Promise<boolean> => {
  try {
    // Check Claude Code
    const claudeInstalled = await invoke<boolean>('check_claude_code_installed_quick')
    if (!claudeInstalled) return false

    // Check Git
    const gitStatus = await invoke<{ installed: boolean }>('check_git_status')
    if (!gitStatus.installed) return false

    // UV is optional - not required for initialization

    return true
  } catch (e) {
    console.error('Failed to check dependencies:', e)
    return false
  }
}

// Check if required environment variables are configured
const checkRequiredEnvVars = (): boolean => {
  const envVars = settingsStore.environmentVariables
  const hasAuthToken = !!envVars['ANTHROPIC_AUTH_TOKEN']?.trim()
  const hasBaseUrl = !!envVars['ANTHROPIC_BASE_URL']?.trim()
  return hasAuthToken && hasBaseUrl
}

// Handle setup complete
const onSetupComplete = () => {
  showSetupGuide.value = false
}

onMounted(async () => {
  // Check if running from DMG mount (macOS) - warn before anything else
  try {
    const fromDmg = await invoke<boolean>('check_running_from_dmg')
    if (fromDmg) {
      await message(
        '应用正在从磁盘映像中运行，部分功能将无法正常使用。\n\n请将 Moonton Claude Desktop 拖拽到"应用程序"文件夹后再打开。',
        { title: '请先安装应用', kind: 'warning' }
      )
    }
  } catch { /* ignore on non-macOS */ }

  // Load saved state (desktop.load() will automatically initialize icons)
  await settingsStore.load()
  desktopStore.load()
  fileSystemStore.load()
  pluginStore.load()

  // Fetch available plugins
  pluginStore.fetchAvailablePlugins()

  // Check all required dependencies (Claude Code, Git, UV)
  const allInstalled = await checkAllDependencies()
  const envVarsConfigured = checkRequiredEnvVars()

  if (allInstalled && envVarsConfigured) {
    // All dependencies installed and env vars configured - show desktop
    showSetupGuide.value = false
  } else if (allInstalled && !envVarsConfigured) {
    // Dependencies installed but env vars not configured - skip to env page
    showSetupGuide.value = true
    skipToEnvPage.value = true
  }
  // else: show full setup guide (dependencies not installed)

  isCheckingSetup.value = false

  // Fire-and-forget silent update check after startup
  checkAndPrompt({ silent: true })

  // Boot bot channel listeners at app level so they run even before
  // the ClaudeBot panel is opened (fire-and-forget).
  const botStore = useBotStore()
  const botChannels = useBotChannels()
  botStore.loadBots().then(async () => {
    for (const bot of botStore.bots) {
      // Reset stale status from previous session (no sidecar survives restart)
      if (bot.status === 'working' || bot.status === 'error') {
        await botStore.updateBot(bot.id, { status: 'idle', errorMessage: undefined })
      }
      const hasEnabled = await botChannels.hasEnabledChannels(bot.id)
      if (hasEnabled) {
        botChannels.startListening(bot).catch(e =>
          console.error('[App] Failed to start channel listener:', e)
        )
      }
    }

    // Restore saved schedules so they survive app restart
    const { initSchedules } = useBotScheduler()
    await initSchedules()
  }).catch(() => {})
})
</script>

<template>
  <div id="desktop-app">
    <Desktop />
    <WindowManager />
    <Taskbar />
    <!-- Widget Panel - Top Right -->
    <WidgetPanel />
    <!-- Update Modal -->
    <UpdateModal />
    <!-- Setup Guide Overlay -->
    <SetupGuide
      v-if="showSetupGuide && !isCheckingSetup"
      :skip-to-env-page="skipToEnvPage"
      @complete="onSetupComplete"
    />
  </div>
</template>

<style lang="less">
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html,
body {
  width: 100%;
  height: 100%;
  overflow: hidden;
}

#desktop-app {
  font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  position: relative;
}
</style>
