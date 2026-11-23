import { defineStore } from 'pinia'
import { readTextFile, writeTextFile, exists, mkdir, BaseDirectory } from '@tauri-apps/plugin-fs'
import { invoke } from '@tauri-apps/api/core'

export interface Shortcut {
  id: string
  type: 'terminal' | 'file-manager' | 'custom'
  command?: string
  args?: string
}

export interface PromptColors {
  path: string             // 路径颜色 (ANSI 256)
  gitClean: string         // Git 干净状态颜色
  gitDirty: string         // Git 脏状态颜色
  env: string              // 环境颜色
  separator: string        // 分隔符颜色
}

export interface LlmRouterConfig {
  selectedProviderId: string  // 'claude' | 'openai' | 'gemini'
  modelProviders: Record<string, string>  // { claude: 'model-name', openai: 'model-name', gemini: 'model-name' }
  thinkingLevel: 'none' | 'low' | 'medium' | 'high'  // 推理深度 (Reasoning Level for GPT-5.2)
}

export interface ClaudeAlias {
  id: string                              // 'alias-{timestamp}'
  name: string                            // 别名命令名 (如 'claude-opus')
  description: string                     // 可选描述
  anthropicModel: string                  // ANTHROPIC_MODEL (空=不设置)
  anthropicAuthToken: string              // ANTHROPIC_AUTH_TOKEN (空=不设置)
  anthropicBaseUrl: string                // ANTHROPIC_BASE_URL (空=不设置)
  customEnvVars: Record<string, string>   // 自定义环境变量
  createdAt: number
  updatedAt: number
}

export interface CoworksModelProvider {
  id: string          // 'custom-{timestamp}'
  name: string        // 显示名称
  model: string       // 模型 ID
  baseUrl: string     // API Base URL（空 = 使用默认 Anthropic）
  apiKey: string      // API Key（空 = 使用全局 ANTHROPIC_AUTH_TOKEN）
}

export interface CoworksModelConfig {
  defaultModel: string                // 'auto' 或 provider.id
  providers: CoworksModelProvider[]   // 自定义模型列表
  maxTurns: number                    // 最大对话轮次 (Agent 循环次数)
  streamingEnabled: boolean           // 是否启用流式输出
}

export interface PromptConfig {
  enabled: boolean                              // 是否启用自定义提示符
  style: 'minimal' | 'powerline' | 'classic'    // 预设样式
  showGitBranch: boolean                        // 显示 Git 分支
  showGitStatus: boolean                        // 显示 Git 状态 (dirty/clean)
  showPythonEnv: boolean                        // 显示 Python 虚拟环境
  showNodeEnv: boolean                          // 显示 Node.js 版本
  showPath: 'full' | 'short' | 'basename'       // 路径显示模式
  colors: PromptColors
}

export interface SettingsState {
  environmentVariables: Record<string, string>
  defaultCli: string
  claudeCodePath: string
  defaultWorkingDirectory: string
  terminalFontSize: number
  terminalFontFamily: string
  defaultWindowWidth: number
  defaultWindowHeight: number
  textEditorPath: string
  shortcuts: Shortcut[]
  keybindings: Record<string, string>
  customAnthropicModel: string  // Persisted custom model value for ANTHROPIC_MODEL selector
  customGeminiModel: string     // Persisted custom model value for GEMINI_MODEL selector
  customOpenAIModel: string     // Persisted custom model value for OPENAI_MODEL selector
  promptConfig: PromptConfig    // 终端提示符配置
  llmRouterConfig: LlmRouterConfig  // LLM 路由配置（独立于环境变量）
  claudeAliases: ClaudeAlias[]       // Claude 别名配置
  updateUrl: string                   // 自定义更新源 URL
  coworksModelConfig: CoworksModelConfig  // Coworks 聊天模型配置
}

// Relative paths from Home directory
const CONFIG_DIR = '.MoontonClaudeDesktop/config'
const CONFIG_RELATIVE_PATH = '.MoontonClaudeDesktop/config/settings.json'

export const useSettingsStore = defineStore('settings', {
  state: (): SettingsState => ({
    environmentVariables: {},
    defaultCli: 'powershell',
    claudeCodePath: '',
    defaultWorkingDirectory: '',
    terminalFontSize: 14,
    terminalFontFamily: 'Consolas, "Courier New", monospace',
    defaultWindowWidth: 1000,
    defaultWindowHeight: 600,
    textEditorPath: 'code',
    shortcuts: [],
    keybindings: {
      'ctrl+alt+t': 'openTerminal',
      'ctrl+alt+f': 'openFileManager',
      'ctrl+w': 'closeWindow',
      'alt+tab': 'switchWindow'
    },
    customAnthropicModel: '',
    customGeminiModel: '',
    customOpenAIModel: '',
    llmRouterConfig: {
      selectedProviderId: 'claude',
      modelProviders: {
        claude: 'Sonnet',
        openai: 'gpt-5.2',
        gemini: 'gemini-3-pro-preview'
      },
      thinkingLevel: 'medium'  // 默认中等推理
    },
    claudeAliases: [],
    updateUrl: '',
    coworksModelConfig: {
      defaultModel: 'auto',
      providers: [],
      maxTurns: 30,
      streamingEnabled: true,
    },
    promptConfig: {
      enabled: true,
      style: 'minimal',
      showGitBranch: true,
      showGitStatus: true,
      showPythonEnv: true,
      showNodeEnv: false,
      showPath: 'short',
      colors: {
        path: '39',        // 青色
        gitClean: '34',    // 蓝色
        gitDirty: '33',    // 黄色
        env: '35',         // 洋红
        separator: '90'    // 灰色
      }
    }
  }),

  actions: {
    async ensureConfigDir() {
      try {
        const dirExists = await exists(CONFIG_DIR, {
          baseDir: BaseDirectory.Home
        })
        if (!dirExists) {
          await mkdir(CONFIG_DIR, {
            baseDir: BaseDirectory.Home,
            recursive: true
          })
        }
      } catch (e) {
        console.error('Failed to create config directory:', e)
        throw e
      }
    },

    async save() {
      try {
        await this.ensureConfigDir()
        await writeTextFile(CONFIG_RELATIVE_PATH, JSON.stringify(this.$state, null, 2), {
          baseDir: BaseDirectory.Home
        })
      } catch (e) {
        console.error('Failed to save settings:', e)
        // Fallback to localStorage
        localStorage.setItem('mcd-settings', JSON.stringify(this.$state))
      }
    },

    async load() {
      try {
        const fileExists = await exists(CONFIG_RELATIVE_PATH, {
          baseDir: BaseDirectory.Home
        })
        if (!fileExists) {
          // Try to load from localStorage for migration
          this.loadFromLocalStorage()
          // Save to new config file
          await this.save()
          return
        }

        const content = await readTextFile(CONFIG_RELATIVE_PATH, {
          baseDir: BaseDirectory.Home
        })
        const data = JSON.parse(content)
        Object.assign(this.$state, data)
      } catch (e) {
        console.error('Failed to load settings from file:', e)
        this.loadFromLocalStorage()
      }
    },

    loadFromLocalStorage() {
      const saved = localStorage.getItem('mcd-settings')
      if (saved) {
        try {
          const data = JSON.parse(saved)
          Object.assign(this.$state, data)
        } catch (e) {
          console.error('Failed to load settings from localStorage:', e)
        }
      }
    },

    async setEnvironmentVariable(key: string, value: string) {
      this.environmentVariables[key] = value
      await this.save()
      // On macOS, also update ~/.zshrc to persist env vars across sessions
      try {
        await invoke('update_shell_env', { key, value })
      } catch (e) {
        console.error('Failed to update shell environment:', e)
      }
    },

    async removeEnvironmentVariable(key: string) {
      delete this.environmentVariables[key]
      await this.save()
      // On macOS, also remove from ~/.zshrc
      try {
        await invoke('update_shell_env', { key, value: null })
      } catch (e) {
        console.error('Failed to update shell environment:', e)
      }
    },

    setKeybinding(key: string, action: string) {
      this.keybindings[key] = action
      this.save()
    },

    async setLlmRouterConfig(config: Partial<LlmRouterConfig>) {
      if (config.selectedProviderId !== undefined) {
        this.llmRouterConfig.selectedProviderId = config.selectedProviderId
      }
      if (config.modelProviders !== undefined) {
        this.llmRouterConfig.modelProviders = {
          ...this.llmRouterConfig.modelProviders,
          ...config.modelProviders
        }
      }
      if (config.thinkingLevel !== undefined) {
        this.llmRouterConfig.thinkingLevel = config.thinkingLevel
      }
      await this.save()
    },

    async setLlmRouterProvider(providerId: string, model?: string) {
      this.llmRouterConfig.selectedProviderId = providerId
      if (model !== undefined) {
        this.llmRouterConfig.modelProviders[providerId] = model
      }
      await this.save()
    },

    async setLlmRouterModel(providerId: string, model: string) {
      this.llmRouterConfig.modelProviders[providerId] = model
      await this.save()
    },

    async addClaudeAlias(alias: Omit<ClaudeAlias, 'id' | 'createdAt' | 'updatedAt'>): Promise<ClaudeAlias> {
      const now = Date.now()
      const newAlias: ClaudeAlias = {
        ...alias,
        id: `alias-${now}`,
        createdAt: now,
        updatedAt: now
      }
      this.claudeAliases = [...this.claudeAliases, newAlias]
      await this.save()
      return newAlias
    },

    async updateClaudeAlias(id: string, updates: Partial<Omit<ClaudeAlias, 'id' | 'createdAt'>>): Promise<void> {
      this.claudeAliases = this.claudeAliases.map(a =>
        a.id === id ? { ...a, ...updates, updatedAt: Date.now() } : a
      )
      await this.save()
    },

    async removeClaudeAlias(id: string): Promise<void> {
      this.claudeAliases = this.claudeAliases.filter(a => a.id !== id)
      await this.save()
    },

    async setCoworksDefaultModel(model: string): Promise<void> {
      this.coworksModelConfig = {
        ...this.coworksModelConfig,
        defaultModel: model,
      }
      await this.save()
    },

    async setCoworksConfig(updates: Partial<Pick<CoworksModelConfig, 'maxTurns' | 'streamingEnabled'>>): Promise<void> {
      this.coworksModelConfig = {
        ...this.coworksModelConfig,
        ...updates,
      }
      await this.save()
    },

    async addCoworksProvider(provider: Omit<CoworksModelProvider, 'id'>): Promise<CoworksModelProvider> {
      const newProvider: CoworksModelProvider = {
        ...provider,
        id: `custom-${Date.now()}`,
      }
      this.coworksModelConfig = {
        ...this.coworksModelConfig,
        providers: [...this.coworksModelConfig.providers, newProvider],
      }
      await this.save()
      return newProvider
    },

    async updateCoworksProvider(id: string, updates: Partial<Omit<CoworksModelProvider, 'id'>>): Promise<void> {
      this.coworksModelConfig = {
        ...this.coworksModelConfig,
        providers: this.coworksModelConfig.providers.map(p =>
          p.id === id ? { ...p, ...updates } : p
        ),
      }
      await this.save()
    },

    async removeCoworksProvider(id: string): Promise<void> {
      this.coworksModelConfig = {
        ...this.coworksModelConfig,
        providers: this.coworksModelConfig.providers.filter(p => p.id !== id),
      }
      // If removed provider was the default, reset to auto
      if (this.coworksModelConfig.defaultModel === id) {
        this.coworksModelConfig = {
          ...this.coworksModelConfig,
          defaultModel: 'auto',
        }
      }
      await this.save()
    },
  }
})
