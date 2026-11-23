import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { PluginInfoExtended, PluginMeta } from '@/types/pluginMarket'

export interface MarketplaceSource {
  source: string // "github" or "git"
  repo?: string // for github: "owner/repo"
  url?: string // for git: full URL
}

export interface MarketplaceInfo {
  name: string
  source: MarketplaceSource
  installLocation: string
  lastUpdated: string
  description?: string
  version?: string
  owner?: string
  pluginCount: number
}

export interface PluginAuthor {
  name: string
}

export interface PluginInfo {
  name: string
  source: string
  description: string
  version: string
  author?: PluginAuthor
  skills: string[]
  agents: string[]
  tags: string[]
  mcpServers?: Record<string, unknown>
}

export interface InstalledPluginInfo {
  name: string
  marketplace: string
  version: string
  installedAt: string
  installPath: string
}

export interface PluginRepairAction {
  pluginKey: string
  action: string
  details: string
}

export interface RepairResult {
  actions: PluginRepairAction[]
  installedPluginsFixed: number
  settingsFixed: number
  totalInstalled: number
}

export interface MarketplaceStoreState {
  marketplaces: MarketplaceInfo[]
  installedPlugins: InstalledPluginInfo[]
  enabledPlugins: Record<string, boolean> // Plugin enabled status from settings.json
  allPluginsExtended: PluginInfoExtended[] // All plugins with extended info
  pluginIcons: Record<string, string> // Cache for plugin icons: key -> data URL
  loading: boolean
  loadingExtended: boolean // Loading extended plugin list
  updating: string | null // Name of marketplace being updated
  repairing: boolean // Whether repair is in progress
  error: string | null
}

export const useMarketplaceStore = defineStore('marketplace', {
  state: (): MarketplaceStoreState => ({
    marketplaces: [],
    installedPlugins: [],
    enabledPlugins: {},
    allPluginsExtended: [],
    pluginIcons: {},
    loading: false,
    loadingExtended: false,
    updating: null,
    repairing: false,
    error: null
  }),

  getters: {
    totalPluginCount: (state): number => {
      return state.marketplaces.reduce((sum, m) => sum + m.pluginCount, 0)
    },

    getMarketplace: (state) => (name: string): MarketplaceInfo | undefined => {
      return state.marketplaces.find(m => m.name === name)
    },

    isPluginInstalled: (state) => (pluginName: string): boolean => {
      return state.installedPlugins.some(p => p.name === pluginName)
    },

    getInstalledPlugin: (state) => (pluginName: string): InstalledPluginInfo | undefined => {
      return state.installedPlugins.find(p => p.name === pluginName)
    },

    isPluginEnabled: (state) => (pluginName: string, marketplaceName: string): boolean => {
      const key = `${pluginName}@${marketplaceName}`
      return state.enabledPlugins[key] === true
    },

    // Get all unique categories from extended plugins
    allCategories: (state): string[] => {
      const categories = new Set<string>()
      state.allPluginsExtended.forEach(p => {
        if (p.meta?.category) {
          categories.add(p.meta.category)
        }
      })
      return Array.from(categories).sort()
    },

    // Get all unique tags from extended plugins
    allTags: (state): string[] => {
      const tags = new Set<string>()
      state.allPluginsExtended.forEach(p => {
        p.tags.forEach(tag => tags.add(tag))
      })
      return Array.from(tags).sort()
    },

    // Get installed plugins count
    installedCount: (state): number => {
      return state.allPluginsExtended.filter(p => p.isInstalled).length
    },

    // Get plugins with updates available
    updatableCount: (state): number => {
      return state.allPluginsExtended.filter(p => p.hasUpdate).length
    }
  },

  actions: {
    async loadMarketplaces() {
      this.loading = true
      this.error = null

      try {
        const marketplaces = await invoke<MarketplaceInfo[]>('get_marketplaces')
        this.marketplaces = marketplaces
      } catch (e) {
        this.error = `Failed to load marketplaces: ${e}`
        console.error('Failed to load marketplaces:', e)
      } finally {
        this.loading = false
      }
    },

    async addMarketplace(sourceType: 'github' | 'url', sourceValue: string) {
      this.loading = true
      this.error = null

      try {
        const marketplace = await invoke<MarketplaceInfo>('add_marketplace', {
          sourceType,
          sourceValue
        })
        this.marketplaces.push(marketplace)
        // Sort by name
        this.marketplaces.sort((a, b) => a.name.localeCompare(b.name))
        return marketplace
      } catch (e) {
        this.error = `Failed to add marketplace: ${e}`
        throw e
      } finally {
        this.loading = false
      }
    },

    async updateMarketplace(name: string) {
      this.updating = name
      this.error = null

      try {
        const updated = await invoke<MarketplaceInfo>('update_marketplace', { name })
        const index = this.marketplaces.findIndex(m => m.name === name)
        if (index !== -1) {
          this.marketplaces[index] = updated
        }
        return updated
      } catch (e) {
        this.error = `Failed to update marketplace: ${e}`
        throw e
      } finally {
        this.updating = null
      }
    },

    async updateAllMarketplaces() {
      const results: { name: string; success: boolean; error?: string }[] = []

      for (const marketplace of this.marketplaces) {
        try {
          await this.updateMarketplace(marketplace.name)
          results.push({ name: marketplace.name, success: true })
        } catch (e) {
          results.push({ name: marketplace.name, success: false, error: String(e) })
        }
      }

      return results
    },

    async removeMarketplace(name: string) {
      this.loading = true
      this.error = null

      try {
        await invoke('remove_marketplace', { name })
        const index = this.marketplaces.findIndex(m => m.name === name)
        if (index !== -1) {
          this.marketplaces.splice(index, 1)
        }
      } catch (e) {
        this.error = `Failed to remove marketplace: ${e}`
        throw e
      } finally {
        this.loading = false
      }
    },

    async getMarketplacePlugins(name: string): Promise<PluginInfo[]> {
      try {
        return await invoke<PluginInfo[]>('get_marketplace_plugins', { name })
      } catch (e) {
        console.error('Failed to get marketplace plugins:', e)
        throw e
      }
    },

    async getAllPlugins(): Promise<[string, PluginInfo][]> {
      try {
        return await invoke<[string, PluginInfo][]>('get_all_plugins')
      } catch (e) {
        console.error('Failed to get all plugins:', e)
        throw e
      }
    },

    /**
     * Load all plugins with extended info (metadata, status, etc.)
     * Used by the full plugin market view
     */
    async loadAllPluginsExtended() {
      this.loadingExtended = true
      this.error = null

      try {
        const plugins = await invoke<PluginInfoExtended[]>('get_all_plugins_extended')
        this.allPluginsExtended = plugins
        return plugins
      } catch (e) {
        this.error = `Failed to load plugins: ${e}`
        console.error('Failed to load extended plugins:', e)
        throw e
      } finally {
        this.loadingExtended = false
      }
    },

    /**
     * Get plugin metadata
     */
    async getPluginMeta(marketplaceName: string, pluginName: string): Promise<PluginMeta | null> {
      try {
        const meta = await invoke<PluginMeta | null>('get_plugin_meta', {
          marketplaceName,
          pluginName
        })
        return meta
      } catch (e) {
        console.error('Failed to get plugin meta:', e)
        return null
      }
    },

    /**
     * Get plugin icon (cached)
     */
    async getPluginIcon(marketplaceName: string, pluginName: string): Promise<string | null> {
      const key = `${pluginName}@${marketplaceName}`

      // Return cached icon if available
      if (this.pluginIcons[key]) {
        return this.pluginIcons[key]
      }

      try {
        const icon = await invoke<string | null>('get_plugin_icon', {
          marketplaceName,
          pluginName
        })

        if (icon) {
          this.pluginIcons[key] = icon
        }

        return icon
      } catch (e) {
        console.error('Failed to get plugin icon:', e)
        return null
      }
    },

    /**
     * Update plugin status in the extended list after install/uninstall/toggle
     */
    updatePluginStatus(pluginName: string, marketplaceName: string, updates: Partial<PluginInfoExtended>) {
      const index = this.allPluginsExtended.findIndex(
        p => p.name === pluginName && p.marketplaceName === marketplaceName
      )
      if (index !== -1) {
        this.allPluginsExtended[index] = {
          ...this.allPluginsExtended[index],
          ...updates
        }
      }
    },

    async loadInstalledPlugins() {
      try {
        const installed = await invoke<InstalledPluginInfo[]>('get_installed_plugins')
        this.installedPlugins = installed
      } catch (e) {
        console.error('Failed to load installed plugins:', e)
        this.installedPlugins = []
      }
    },

    /**
     * Initialize plugin data and auto-repair if needed.
     * Called on app startup to ensure consistency.
     */
    async initializeAndRepairIfNeeded() {
      try {
        // Load marketplaces first
        await this.loadMarketplaces()

        // Load installed plugins
        await this.loadInstalledPlugins()

        // Load enabled plugins status
        await this.loadEnabledPlugins()

        // Note: Auto-repair removed to avoid conflicts with external Claude CLI
        // User can manually trigger repair from the UI if needed
        if (this.marketplaces.length > 0 && this.installedPlugins.length === 0) {
          // No installed plugins; user can manually repair via UI
        }
      } catch (e) {
        console.error('Failed to initialize plugin data:', e)
      }
    },

    async installPlugin(marketplaceName: string, pluginName: string) {
      try {
        const installed = await invoke<InstalledPluginInfo>('install_plugin', {
          marketplaceName,
          pluginName
        })
        this.installedPlugins.push(installed)
        // Also update enabledPlugins state (backend sets it to true on install)
        const key = `${pluginName}@${marketplaceName}`
        this.enabledPlugins[key] = true
        return installed
      } catch (e) {
        console.error('Failed to install plugin:', e)
        throw e
      }
    },

    async uninstallPlugin(pluginName: string, marketplaceName?: string) {
      try {
        await invoke('uninstall_plugin', { pluginName, marketplaceName })
        const index = this.installedPlugins.findIndex(p => p.name === pluginName)
        if (index !== -1) {
          this.installedPlugins.splice(index, 1)
        }
        // Also remove from enabledPlugins state
        if (marketplaceName) {
          const key = `${pluginName}@${marketplaceName}`
          delete this.enabledPlugins[key]
        }
      } catch (e) {
        console.error('Failed to uninstall plugin:', e)
        throw e
      }
    },

    async loadEnabledPlugins() {
      try {
        const enabled = await invoke<Record<string, boolean>>('get_all_enabled_plugins')
        this.enabledPlugins = enabled
      } catch (e) {
        console.error('Failed to load enabled plugins:', e)
        this.enabledPlugins = {}
      }
    },

    async enablePlugin(pluginName: string, marketplaceName: string) {
      try {
        await invoke('enable_plugin', { pluginName, marketplaceName })
        const key = `${pluginName}@${marketplaceName}`
        this.enabledPlugins[key] = true
      } catch (e) {
        console.error('Failed to enable plugin:', e)
        throw e
      }
    },

    async disablePlugin(pluginName: string, marketplaceName: string) {
      try {
        await invoke('disable_plugin', { pluginName, marketplaceName })
        const key = `${pluginName}@${marketplaceName}`
        this.enabledPlugins[key] = false
      } catch (e) {
        console.error('Failed to disable plugin:', e)
        throw e
      }
    },

    async togglePluginEnabled(pluginName: string, marketplaceName: string) {
      const key = `${pluginName}@${marketplaceName}`
      const isEnabled = this.enabledPlugins[key] === true
      if (isEnabled) {
        await this.disablePlugin(pluginName, marketplaceName)
      } else {
        await this.enablePlugin(pluginName, marketplaceName)
      }
    },

    async repairPluginRegistry(): Promise<RepairResult> {
      this.repairing = true
      this.error = null

      try {
        const result = await invoke<RepairResult>('repair_plugin_registry')

        // Reload all data after repair
        await this.loadInstalledPlugins()
        await this.loadEnabledPlugins()

        return result
      } catch (e) {
        this.error = `修复失败: ${e}`
        console.error('Failed to repair plugin registry:', e)
        throw e
      } finally {
        this.repairing = false
      }
    },

    formatLastUpdated(isoString: string): string {
      try {
        const date = new Date(isoString)
        return date.toLocaleDateString('zh-CN', {
          year: 'numeric',
          month: 'short',
          day: 'numeric',
          hour: '2-digit',
          minute: '2-digit'
        })
      } catch {
        return isoString
      }
    },

    getSourceDisplay(source: MarketplaceSource): string {
      if (source.source === 'github' && source.repo) {
        return `GitHub: ${source.repo}`
      } else if (source.url) {
        // Truncate long URLs
        const url = source.url
        if (url.length > 50) {
          return url.substring(0, 47) + '...'
        }
        return url
      }
      return 'Unknown source'
    }
  }
})
