import { defineStore } from 'pinia'

export interface DesktopIcon {
  id: string
  name: string
  icon: string // Font Awesome class
  position: { x: number; y: number }
  type: 'terminal' | 'file-manager' | 'settings' | 'plugin-market' | 'plugin-market-full' | 'mcp-manager' | 'custom' | 'add-project-link' | 'project-link' | 'plugin-shortcut' | 'setup-guide' | 'coworks' | 'llm-router' | 'dynamic-function' | 'dynamic-function-creator' | 'dev-tools' | 'claude-aliases' | 'claude-bot'
  command?: string // For project-link, this stores the project path
  pluginInfo?: {
    pluginName: string
    pluginDescription: string
    marketplaceName: string
    skillName: string
  }
  dynamicFunctionInfo?: {
    functionName: string
    displayName: string
    backendEnabled: boolean
  }
  environmentVariables?: Record<string, string> // 图标专属环境变量
  customArgs?: string // 用户自定义启动命令（完整命令，如 "claude -r"）
}

// Grid constants
const GRID_SIZE = 100

// Default icons configuration - generated fresh on each startup
// Positions are assigned dynamically based on available vertical space
// Order: terminal, add-project-link, settings, coworks, llm-router, plugin-market, plugin-market-full
const DEFAULT_ICONS_CONFIG: Omit<DesktopIcon, 'position'>[] = [
  {
    id: 'terminal',
    name: '终端',
    icon: 'fa-solid fa-terminal',
    type: 'terminal'
  },
  {
    id: 'add-project-link',
    name: '工作目录',
    icon: 'fa-solid fa-folder-plus',
    type: 'add-project-link'
  },
  {
    id: 'settings',
    name: '设置',
    icon: 'fa-solid fa-gear',
    type: 'settings'
  },
  {
    id: 'coworks',
    name: 'Coworks',
    icon: 'fa-solid fa-users',
    type: 'coworks'
  },
  {
    id: 'llm-router',
    name: 'LLM 路由',
    icon: 'fa-solid fa-route',
    type: 'llm-router'
  },
  {
    id: 'plugin-market',
    name: 'Claude管理',
    icon: 'fa-solid fa-toolbox',
    type: 'plugin-market'
  },
  {
    id: 'plugin-market-full',
    name: '插件商店',
    icon: 'fa-solid fa-store',
    type: 'plugin-market-full'
  },
  {
    id: 'dynamic-function-creator',
    name: 'Claude 轻应用',
    icon: 'fa-solid fa-cubes',
    type: 'dynamic-function-creator'
  },
  {
    id: 'dev-tools',
    name: 'DevTools',
    icon: 'fa-solid fa-chart-line',
    type: 'dev-tools'
  },
  {
    id: 'claude-aliases',
    name: 'Claude 别名',
    icon: 'fa-solid fa-masks-theater',
    type: 'claude-aliases'
  },
  {
    id: 'claude-bot',
    name: 'Claude Bot',
    icon: 'fa-solid fa-frog',
    type: 'claude-bot'
  }
]

// Calculate icon positions based on available desktop height
// Icons flow vertically first, then wrap to next column when exceeding available height
// Avoids positions already occupied by user icons
function calculateDefaultIconPositions(desktopHeight: number, userIcons: DesktopIcon[]): DesktopIcon[] {
  // Desktop has 48px taskbar at bottom and 10px padding, plus account for icon height (~90px)
  const availableHeight = desktopHeight - 48 - 20 - 90
  const maxIconsPerColumn = Math.max(1, Math.floor(availableHeight / GRID_SIZE) + 1)
  const maxColumns = 20 // Maximum columns to search

  // Collect positions occupied by user icons
  const occupiedPositions = new Set<string>()
  for (const icon of userIcons) {
    const gridX = Math.round(icon.position.x / GRID_SIZE)
    const gridY = Math.round(icon.position.y / GRID_SIZE)
    occupiedPositions.add(`${gridX},${gridY}`)
  }

  // Find next available position, avoiding occupied ones
  const findNextPosition = (startIndex: number): { column: number; row: number } => {
    let searchIndex = startIndex
    while (searchIndex < maxColumns * maxIconsPerColumn) {
      const column = Math.floor(searchIndex / maxIconsPerColumn)
      const row = searchIndex % maxIconsPerColumn
      const posKey = `${column},${row}`
      if (!occupiedPositions.has(posKey)) {
        // Mark this position as occupied for subsequent icons
        occupiedPositions.add(posKey)
        return { column, row }
      }
      searchIndex++
    }
    // Fallback if all positions searched
    return { column: Math.floor(startIndex / maxIconsPerColumn), row: startIndex % maxIconsPerColumn }
  }

  return DEFAULT_ICONS_CONFIG.map((iconConfig, index) => {
    const { column, row } = findNextPosition(index)
    return {
      ...iconConfig,
      position: { x: column * GRID_SIZE, y: row * GRID_SIZE }
    }
  })
}

export interface DesktopState {
  icons: DesktopIcon[]
  userIcons: DesktopIcon[] // User-added icons (project links, custom)
  gridSize: number
  wallpaper: string
  desktopHeight: number // Track desktop height for auto-layout
}

export const useDesktopStore = defineStore('desktop', {
  state: (): DesktopState => ({
    icons: [],
    userIcons: [],
    gridSize: 100,
    wallpaper: 'url(/wallpaper.png)',
    desktopHeight: window.innerHeight || 800
  }),

  getters: {
    // Check if an icon is deletable (only user-added icons can be deleted)
    isIconDeletable: () => (iconId: string) => {
      return iconId.startsWith('project-') || iconId.startsWith('icon-') || iconId.startsWith('plugin-') || iconId.startsWith('dynfunc-')
    },

    // Get all occupied grid positions
    getOccupiedPositions(): Set<string> {
      const occupied = new Set<string>()
      for (const icon of this.icons) {
        const gridX = Math.round(icon.position.x / GRID_SIZE)
        const gridY = Math.round(icon.position.y / GRID_SIZE)
        occupied.add(`${gridX},${gridY}`)
      }
      return occupied
    }
  },

  actions: {
    // Initialize icons - merge default icons with user icons
    initializeIcons() {
      // Calculate default icons with positions based on current desktop height
      // Pass userIcons to avoid position conflicts
      const defaultIcons = calculateDefaultIconPositions(this.desktopHeight, this.userIcons)

      // Merge with user icons
      this.icons = [...defaultIcons, ...this.userIcons]
    },

    // Update desktop height and re-layout icons if needed
    updateDesktopHeight(height: number) {
      if (this.desktopHeight !== height) {
        this.desktopHeight = height
        // Re-initialize to recalculate default icon positions
        this.initializeIcons()
      }
    },

    // Find next available grid position, avoiding all occupied positions
    findNextAvailablePosition(startCol: number, startRow: number): { x: number; y: number } {
      const occupied = this.getOccupiedPositions
      const maxCols = 12  // Maximum columns
      const maxRows = 20  // Maximum rows

      // Search from starting position
      for (let row = startRow; row < maxRows; row++) {
        // First row starts from startCol, subsequent rows start from column 1 (avoid system icon column 0)
        const colStart = row === startRow ? startCol : 1
        for (let col = colStart; col < maxCols; col++) {
          if (!occupied.has(`${col},${row}`)) {
            return { x: col * GRID_SIZE, y: row * GRID_SIZE }
          }
        }
      }

      // If all positions are filled, return end position
      return { x: startCol * GRID_SIZE, y: maxRows * GRID_SIZE }
    },

    addIcon(icon: Omit<DesktopIcon, 'id'>) {
      const newIcon: DesktopIcon = {
        ...icon,
        id: `icon-${Date.now()}`
      }
      this.userIcons.push(newIcon)
      this.icons.push(newIcon)
      this.save()
      return newIcon
    },

    removeIcon(iconId: string) {
      // Only allow removing user-added icons
      if (!this.isIconDeletable(iconId)) {
        console.warn('Cannot remove default icon:', iconId)
        return false
      }

      // Remove from userIcons
      const userIndex = this.userIcons.findIndex(i => i.id === iconId)
      if (userIndex !== -1) {
        this.userIcons.splice(userIndex, 1)
      }

      // Remove from icons
      const index = this.icons.findIndex(i => i.id === iconId)
      if (index !== -1) {
        this.icons.splice(index, 1)
        this.save()
        return true
      }
      return false
    },

    addProjectLink(projectPath: string) {
      // Get project name from path
      const projectName = projectPath.split(/[\\/]/).pop() || projectPath

      // Find next available position starting from (1, 1) - avoids system icon column and first row
      const position = this.findNextAvailablePosition(1, 1)

      const newIcon: DesktopIcon = {
        id: `project-${Date.now()}`,
        name: projectName,
        icon: 'fa-solid fa-folder-open',
        position,
        type: 'project-link',
        command: projectPath
      }

      this.userIcons.push(newIcon)
      this.icons.push(newIcon)
      this.save()
      return newIcon
    },

    removeProjectLink(iconId: string) {
      return this.removeIcon(iconId)
    },

    addPluginShortcut(pluginName: string, pluginDescription: string, marketplaceName: string, skillName: string) {
      // Check if shortcut already exists for this plugin + skill combination
      const existing = this.icons.find(
        i => i.type === 'plugin-shortcut' &&
             i.pluginInfo?.pluginName === pluginName &&
             i.pluginInfo?.skillName === skillName
      )
      if (existing) {
        return { success: false, message: '该技能快捷方式已存在', icon: existing }
      }

      // Find next available position starting from (2, 0) - third column, first row
      const position = this.findNextAvailablePosition(2, 0)

      // Extract clean skill name from path (e.g., "./skills/image-generator" -> "image-generator")
      const cleanSkillName = skillName.split('/').pop() || skillName
      // Display name: clean skill name
      const displayName = cleanSkillName

      const newIcon: DesktopIcon = {
        id: `plugin-${Date.now()}`,
        name: displayName,
        icon: 'fa-solid fa-wand-magic-sparkles', // Plugin magic wand icon
        position,
        type: 'plugin-shortcut',
        pluginInfo: {
          pluginName,
          pluginDescription,
          marketplaceName,
          skillName
        }
      }

      this.userIcons.push(newIcon)
      this.icons.push(newIcon)
      this.save()
      return { success: true, message: '技能快捷方式已创建', icon: newIcon }
    },

    removePluginShortcut(pluginName: string) {
      const icon = this.icons.find(
        i => i.type === 'plugin-shortcut' && i.pluginInfo?.pluginName === pluginName
      )
      if (icon) {
        return this.removeIcon(icon.id)
      }
      return false
    },

    hasPluginShortcut(pluginName: string): boolean {
      return this.icons.some(
        i => i.type === 'plugin-shortcut' && i.pluginInfo?.pluginName === pluginName
      )
    },

    addDynamicFunction(functionName: string, displayName: string, icon: string, backendEnabled: boolean) {
      // Check if shortcut already exists
      const existing = this.icons.find(
        i => i.type === 'dynamic-function' && i.dynamicFunctionInfo?.functionName === functionName
      )
      if (existing) {
        return { success: false, message: '该动态功能已存在', icon: existing }
      }

      const position = this.findNextAvailablePosition(2, 0)

      const newIcon: DesktopIcon = {
        id: `dynfunc-${Date.now()}`,
        name: displayName,
        icon: icon || 'fa-solid fa-cube',
        position,
        type: 'dynamic-function',
        dynamicFunctionInfo: {
          functionName,
          displayName,
          backendEnabled
        }
      }

      this.userIcons.push(newIcon)
      this.icons.push(newIcon)
      this.save()
      return { success: true, message: '动态功能已创建', icon: newIcon }
    },

    removeDynamicFunction(functionName: string) {
      const icon = this.icons.find(
        i => i.type === 'dynamic-function' && i.dynamicFunctionInfo?.functionName === functionName
      )
      if (icon) {
        return this.removeIcon(icon.id)
      }
      return false
    },

    updateIconPosition(iconId: string, position: { x: number; y: number }) {
      const icon = this.icons.find(i => i.id === iconId)
      if (icon) {
        icon.position = position

        // Also update in userIcons if it's a user icon
        const userIcon = this.userIcons.find(i => i.id === iconId)
        if (userIcon) {
          userIcon.position = position
        }

        this.save()
      }
    },

    updateIconEnvironmentVariables(iconId: string, envVars: Record<string, string>) {
      const icon = this.icons.find(i => i.id === iconId)
      if (icon) {
        icon.environmentVariables = Object.keys(envVars).length > 0 ? envVars : undefined

        // Also update in userIcons if it's a user icon
        const userIcon = this.userIcons.find(i => i.id === iconId)
        if (userIcon) {
          userIcon.environmentVariables = icon.environmentVariables
        }

        this.save()
      }
    },

    updateIconName(iconId: string, name: string) {
      const trimmed = name.trim()
      if (!trimmed) return

      const iconExists = this.icons.some(i => i.id === iconId)
      if (iconExists) {
        this.icons = this.icons.map(i => i.id === iconId ? { ...i, name: trimmed } : i)
        this.userIcons = this.userIcons.map(i => i.id === iconId ? { ...i, name: trimmed } : i)
        this.save()
      }
    },

    updateIconCustomArgs(iconId: string, args: string) {
      const icon = this.icons.find(i => i.id === iconId)
      if (icon) {
        icon.customArgs = args.trim() || undefined

        // Also update in userIcons if it's a user icon
        const userIcon = this.userIcons.find(i => i.id === iconId)
        if (userIcon) {
          userIcon.customArgs = icon.customArgs
        }

        this.save()
      }
    },

    setWallpaper(wallpaper: string) {
      this.wallpaper = wallpaper
      this.save()
    },

    save() {
      // Only save user-added icons and settings, not default icons
      const saveData = {
        userIcons: this.userIcons,
        gridSize: this.gridSize,
        wallpaper: this.wallpaper
      }
      localStorage.setItem('mcd-desktop', JSON.stringify(saveData))
    },

    load() {
      const saved = localStorage.getItem('mcd-desktop')
      if (saved) {
        try {
          const data = JSON.parse(saved)
          // Load user icons and snap them to grid (migration for old positions)
          if (data.userIcons) {
            this.userIcons = data.userIcons.map((icon: DesktopIcon) => ({
              ...icon,
              position: {
                x: Math.round(icon.position.x / GRID_SIZE) * GRID_SIZE,
                y: Math.round(icon.position.y / GRID_SIZE) * GRID_SIZE
              }
            }))
          // Migration: convert old args-only customArgs to full command format
          this.userIcons = this.userIcons.map((icon: DesktopIcon) => {
            if (icon.customArgs) {
              const firstToken = icon.customArgs.trim().split(/\s+/)[0]
              if (firstToken.startsWith('-') || firstToken.startsWith('/')) {
                return { ...icon, customArgs: `claude ${icon.customArgs.trim()}` }
              }
            }
            return icon
          })
          }
          // Load settings
          if (data.gridSize) {
            this.gridSize = data.gridSize
          }
          if (data.wallpaper) {
            this.wallpaper = data.wallpaper
          }
        } catch (e) {
          console.error('Failed to load desktop:', e)
        }
      }

      // Always initialize icons (merge defaults with user icons)
      this.initializeIcons()
    }
  }
})
