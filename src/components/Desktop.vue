<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useDesktopStore } from '@/stores/desktopStore'
import { useWindowStore } from '@/stores/windowStore'
import DesktopIcon from './DesktopIcon.vue'
import type { DesktopIcon as DesktopIconType } from '@/types/models'
import { open } from '@tauri-apps/plugin-dialog'
import { mkdir, exists } from '@tauri-apps/plugin-fs'
import { homeDir, join } from '@tauri-apps/api/path'
import { invoke } from '@tauri-apps/api/core'

interface PluginInfo {
  name: string
  description: string
  version: string
  agents: string[]
  mcpServers?: Record<string, unknown>
}

const desktopStore = useDesktopStore()
const windowStore = useWindowStore()

// Track icons being processed to prevent double-click opening multiple windows
const processingIcons = ref(new Set<string>())

// Get or create plugin-specific directory in user home (~/.MoontonClaudeDesktop/plugins/pluginName)
const getPluginDirectory = async (pluginName: string): Promise<string> => {
  const home = await homeDir()
  const pluginDir = await join(home, '.MoontonClaudeDesktop', 'plugins', pluginName)

  // Create directory if it doesn't exist
  const dirExists = await exists(pluginDir)
  if (!dirExists) {
    await mkdir(pluginDir, { recursive: true })
  }

  return pluginDir
}

// Get full plugin info including agents and mcpServers
const getPluginFullInfo = async (marketplaceName: string, pluginName: string): Promise<PluginInfo | null> => {
  try {
    const plugins = await invoke<PluginInfo[]>('get_marketplace_plugins', { name: marketplaceName })
    return plugins.find(p => p.name === pluginName) || null
  } catch (e) {
    console.error('Failed to get plugin info:', e)
    return null
  }
}

const contextMenuVisible = ref(false)
const contextMenuPosition = ref({ x: 0, y: 0 })

const handleIconClick = async (icon: DesktopIconType) => {
  // Prevent double-click from opening multiple windows
  if (processingIcons.value.has(icon.id)) {
    return
  }
  processingIcons.value.add(icon.id)

  // Auto-release after 500ms to handle edge cases
  setTimeout(() => {
    processingIcons.value.delete(icon.id)
  }, 500)

  if (icon.type === 'add-project-link') {
    // Open folder selection dialog
    const selected = await open({
      directory: true,
      multiple: false,
      title: '选择项目目录'
    })

    if (selected && typeof selected === 'string') {
      desktopStore.addProjectLink(selected)
    }
    return
  }

  if (icon.type === 'project-link' && icon.command) {
    // Open terminal with project path as cwd, auto-run claude
    // Append custom args if set
    const claudeCmd = icon.customArgs || 'claude'
    windowStore.openTerminalWithCwd(icon.command, icon.name, claudeCmd, icon.id, {
      iconEnvVars: icon.environmentVariables
    })
    return
  }

  if (icon.type === 'dynamic-function' && icon.dynamicFunctionInfo) {
    // Open the dynamic function in an iframe window
    windowStore.openWindow(icon, {
      functionName: icon.dynamicFunctionInfo.functionName,
      functionPath: '',
      backendPort: null
    })
    return
  }

  if (icon.type === 'plugin-shortcut' && icon.pluginInfo) {
    // Open terminal with claude command for the plugin skill
    const { pluginName, pluginDescription, marketplaceName, skillName, skillDescription } = icon.pluginInfo

    // Get full plugin info (agents, mcpServers, version)
    const pluginFullInfo = await getPluginFullInfo(marketplaceName, pluginName)

    // Get or create plugin-specific directory
    const pluginDir = await getPluginDirectory(pluginName)

    // Extract just the skill name from path (e.g., "./skills/image-generator" -> "image-generator")
    const cleanSkillName = skillName.split('/').pop() || skillName

    // Use pluginName:skillName format for Claude skill invocation
    const skillIdentifier = `${pluginName}:${cleanSkillName}`
    const prompt = `使用 ${skillIdentifier} 完成接下来的任务。Plugin from ${marketplaceName} marketplace. Description: ${pluginDescription}`
    const claudeCommand = `claude "${prompt.replace(/"/g, '\\"')}"`

    // Pass skillInfo for skill panel display
    windowStore.openTerminalWithCwd(pluginDir, `${cleanSkillName} - Claude`, claudeCommand, icon.id, {
      type: 'skill',
      skillInfo: {
        skillName: cleanSkillName,
        skillDescription: skillDescription || '',
        pluginName,
        pluginDescription: pluginDescription || '',
        pluginVersion: pluginFullInfo?.version || '',
        marketplaceName,
        agents: pluginFullInfo?.agents || [],
        mcpServers: pluginFullInfo?.mcpServers || {}
      },
      iconEnvVars: icon.environmentVariables
    })
    return
  }

  windowStore.openWindow(icon)
}

const handleIconDrag = (iconId: string, newPosition: { x: number; y: number }) => {
  // Snap to grid
  const gridSize = desktopStore.gridSize
  const snappedPosition = {
    x: Math.round(newPosition.x / gridSize) * gridSize,
    y: Math.round(newPosition.y / gridSize) * gridSize
  }
  desktopStore.updateIconPosition(iconId, snappedPosition)
}

const handleContextMenu = (event: MouseEvent) => {
  event.preventDefault()
  contextMenuPosition.value = { x: event.clientX, y: event.clientY }
  contextMenuVisible.value = true
}

const closeContextMenu = () => {
  contextMenuVisible.value = false
}

const handleDesktopClick = () => {
  closeContextMenu()
}

const refreshDesktop = () => {
  desktopStore.load()
  closeContextMenu()
}

// Handle window resize to re-layout icons when vertical space changes
const handleResize = () => {
  desktopStore.updateDesktopHeight(window.innerHeight)
}

// Debounced resize handler to avoid too frequent updates
let resizeTimeout: ReturnType<typeof setTimeout> | null = null
const debouncedResize = () => {
  if (resizeTimeout) {
    clearTimeout(resizeTimeout)
  }
  resizeTimeout = setTimeout(handleResize, 150)
}

onMounted(() => {
  // Set initial desktop height
  desktopStore.updateDesktopHeight(window.innerHeight)
  // Listen for window resize
  window.addEventListener('resize', debouncedResize)
})

onUnmounted(() => {
  window.removeEventListener('resize', debouncedResize)
  if (resizeTimeout) {
    clearTimeout(resizeTimeout)
  }
})
</script>

<template>
  <div
    class="desktop"
    :style="{
      background: desktopStore.wallpaper,
      backgroundSize: '100% 100%'
    }"
    @contextmenu="handleContextMenu"
    @click="handleDesktopClick"
  >
    <DesktopIcon
      v-for="icon in desktopStore.icons"
      :key="icon.id"
      :icon="icon"
      @click="handleIconClick(icon)"
      @drag="handleIconDrag"
    />

    <!-- Context Menu -->
    <Teleport to="body">
      <div
        v-if="contextMenuVisible"
        class="context-menu"
        :style="{ left: contextMenuPosition.x + 'px', top: contextMenuPosition.y + 'px' }"
        @click.stop
      >
        <div
          class="context-menu-item"
          @click="refreshDesktop"
        >
          <i class="fa-solid fa-rotate" />
          <span>刷新</span>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style lang="less" scoped>
.desktop {
  width: 100%;
  height: calc(100vh - 48px);
  position: relative;
  overflow: hidden;
  user-select: none;
  padding: 10px; // Padding from edges for icons
  box-sizing: border-box;
}

.context-menu {
  position: fixed;
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(12px);
  border-radius: 8px;
  box-shadow: 0 4px 20px rgba(45, 74, 106, 0.15);
  padding: 8px 0;
  min-width: 180px;
  z-index: 10001;

  .context-menu-item {
    padding: 10px 16px;
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    transition: background 0.2s;

    i {
      width: 16px;
      text-align: center;
      color: #7c6eb8;
    }

    span {
      font-size: 14px;
    }

    &:hover {
      background: #f0ecf8;
    }
  }

  .context-menu-separator {
    height: 1px;
    background: #e8e0f0;
    margin: 6px 0;
  }
}
</style>
