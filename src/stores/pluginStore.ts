import { defineStore } from 'pinia'
import type { Plugin } from '@/types/models'

export interface PluginStoreState {
  availablePlugins: Plugin[]
  installedPlugins: Plugin[]
  loading: boolean
  error: string | null
}

// Mock plugin data - in real app, this would come from an API
const mockPlugins: Plugin[] = [
  {
    id: 'claude-code-helper',
    name: 'Claude Code Helper',
    description: '增强Claude Code CLI的使用体验，提供智能命令补全和历史记录',
    version: '1.2.0',
    author: 'Moonton',
    downloads: 15234,
    rating: 4.8
  },
  {
    id: 'git-integration',
    name: 'Git Integration',
    description: '深度Git集成，可视化分支管理和提交历史',
    version: '2.0.1',
    author: 'DevTools',
    downloads: 28901,
    rating: 4.6
  },
  {
    id: 'markdown-preview',
    name: 'Markdown Preview',
    description: '实时Markdown预览，支持Mermaid图表和代码高亮',
    version: '1.5.3',
    author: 'DocTools',
    downloads: 12456,
    rating: 4.7
  },
  {
    id: 'theme-dark-plus',
    name: 'Dark Plus Theme',
    description: '优雅的深色主题，减少眼睛疲劳',
    version: '1.0.0',
    author: 'Themes',
    downloads: 45678,
    rating: 4.9
  },
  {
    id: 'file-sync',
    name: 'File Sync',
    description: '与云存储同步文件，支持Dropbox、Google Drive',
    version: '1.1.0',
    author: 'CloudTools',
    downloads: 8765,
    rating: 4.3
  },
  {
    id: 'code-snippets',
    name: 'Code Snippets',
    description: '代码片段管理器，快速插入常用代码',
    version: '2.1.0',
    author: 'DevTools',
    downloads: 21345,
    rating: 4.5
  }
]

export const usePluginStore = defineStore('plugin', {
  state: (): PluginStoreState => ({
    availablePlugins: [],
    installedPlugins: [],
    loading: false,
    error: null
  }),

  getters: {
    isInstalled: (state) => (pluginId: string): boolean => {
      return state.installedPlugins.some(p => p.id === pluginId)
    },

    enabledPlugins: (state): Plugin[] => {
      return state.installedPlugins.filter(p => p.enabled)
    },

    getInstalledPlugin: (state) => (pluginId: string): Plugin | undefined => {
      return state.installedPlugins.find(p => p.id === pluginId)
    }
  },

  actions: {
    async fetchAvailablePlugins() {
      this.loading = true
      this.error = null

      try {
        // Simulate API call
        await new Promise(resolve => setTimeout(resolve, 500))
        this.availablePlugins = mockPlugins
      } catch (e) {
        this.error = 'Failed to fetch plugins'
        console.error('Failed to fetch plugins:', e)
      } finally {
        this.loading = false
      }
    },

    async installPlugin(plugin: Plugin) {
      if (this.isInstalled(plugin.id)) {
        throw new Error('Plugin already installed')
      }

      // Simulate installation
      await new Promise(resolve => setTimeout(resolve, 300))

      this.installedPlugins.push({
        ...plugin,
        enabled: true
      })
      this.save()
    },

    async uninstallPlugin(pluginId: string) {
      const index = this.installedPlugins.findIndex(p => p.id === pluginId)
      if (index !== -1) {
        this.installedPlugins.splice(index, 1)
        this.save()
      }
    },

    togglePlugin(pluginId: string) {
      const plugin = this.installedPlugins.find(p => p.id === pluginId)
      if (plugin) {
        plugin.enabled = !plugin.enabled
        this.save()
      }
    },

    updatePluginConfig(pluginId: string, config: Record<string, unknown>) {
      const plugin = this.installedPlugins.find(p => p.id === pluginId)
      if (plugin) {
        plugin.config = { ...plugin.config, ...config }
        this.save()
      }
    },

    save() {
      localStorage.setItem('mcd-plugins', JSON.stringify(this.installedPlugins))
    },

    load() {
      const saved = localStorage.getItem('mcd-plugins')
      if (saved) {
        try {
          this.installedPlugins = JSON.parse(saved)
        } catch (e) {
          console.error('Failed to load plugins:', e)
        }
      }
    }
  }
})
