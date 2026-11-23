import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { BotDefinition, BotIndexItem, BotChannelsConfig, BotScheduleConfig, ChannelStatus } from '@/types/claude-bot'

export interface BotStoreState {
  bots: BotDefinition[]
  failedBots: { id: string; name: string }[]
  selectedBotId: string | null
  interactionBotId: string | null
  loading: boolean
  webServerPort: number | null
  webServerRunning: boolean
  channelStatuses: Record<string, ChannelStatus[]>
}

export const useBotStore = defineStore('bot', {
  state: (): BotStoreState => ({
    bots: [],
    failedBots: [],
    selectedBotId: null,
    interactionBotId: null,
    loading: false,
    webServerPort: null,
    webServerRunning: false,
    channelStatuses: {},
  }),

  getters: {
    selectedBot: (state): BotDefinition | undefined => {
      return state.bots.find(b => b.id === state.selectedBotId)
    },

    interactionBot: (state): BotDefinition | undefined => {
      return state.bots.find(b => b.id === state.interactionBotId)
    },

    activeBots: (state): BotDefinition[] => {
      return state.bots.filter(b => b.status !== 'stopped')
    },
  },

  actions: {
    async loadBots() {
      this.loading = true
      try {
        const items: BotIndexItem[] = await invoke('bot_list_bots')
        const bots: BotDefinition[] = []
        const failed: { id: string; name: string }[] = []
        for (const item of items) {
          try {
            const bot: BotDefinition = await invoke('bot_get_bot', { botId: item.id })
            bots.push(bot)
          } catch (e) {
            console.error(`Failed to load bot ${item.id}:`, e)
            failed.push({ id: item.id, name: item.name })
          }
        }
        this.bots = bots
        this.failedBots = failed
      } catch (e) {
        console.error('Failed to load bots:', e)
      } finally {
        this.loading = false
      }
    },

    async createBot(params: {
      name: string
      description: string
      model: string
    }): Promise<BotDefinition | null> {
      try {
        const systemPrompt = buildSystemPrompt(params.name, params.description)
        const bot: BotDefinition = await invoke('bot_create_bot', {
          name: params.name,
          description: params.description,
          triggerType: 'obedient',
          model: params.model,
          systemPrompt,
          scheduleConfig: null,
          proactiveConfig: null,
        })
        this.bots = [...this.bots, bot]
        return bot
      } catch (e) {
        console.error('Failed to create bot:', e)
        return null
      }
    },

    async deleteBot(botId: string) {
      try {
        await invoke('bot_delete_bot', { botId })
        this.bots = this.bots.filter(b => b.id !== botId)
        if (this.selectedBotId === botId) {
          this.selectedBotId = null
        }
        if (this.interactionBotId === botId) {
          this.interactionBotId = null
        }
      } catch (e) {
        console.error('Failed to delete bot:', e)
      }
    },

    async updateBot(botId: string, updates: Partial<BotDefinition>) {
      try {
        const updatedBot: BotDefinition = await invoke('bot_update_bot', {
          botId,
          updates: JSON.stringify(updates),
        })
        this.bots = this.bots.map(b => b.id === botId ? updatedBot : b)
      } catch (e) {
        console.error('Failed to update bot:', e)
      }
    },

    async updateBotStatus(botId: string, status: BotDefinition['status'], errorMessage?: string) {
      const updates: Partial<BotDefinition> = { status }
      if (errorMessage !== undefined) {
        updates.errorMessage = errorMessage
      }
      if (status === 'working') {
        updates.lastActiveAt = Date.now()
      }
      await this.updateBot(botId, updates)
    },

    selectBotForInteraction(botId: string | null) {
      this.interactionBotId = botId
    },

    async startWebServer() {
      try {
        const port: number = await invoke('bot_start_web_server')
        this.webServerPort = port
        this.webServerRunning = true
      } catch (e) {
        console.error('Failed to start web server:', e)
      }
    },

    async stopWebServer() {
      try {
        await invoke('bot_stop_web_server')
        this.webServerPort = null
        this.webServerRunning = false
      } catch (e) {
        console.error('Failed to stop web server:', e)
      }
    },

    async checkWebServerStatus() {
      try {
        const port: number | null = await invoke('bot_get_web_server_status')
        this.webServerPort = port
        this.webServerRunning = port !== null
      } catch (e) {
        console.error('Failed to check web server status:', e)
      }
    },

    // --- Channel management ---

    async loadChannelConfig(botId: string): Promise<BotChannelsConfig> {
      try {
        return await invoke('channel_get_config', { botId })
      } catch (e) {
        console.error('Failed to load channel config:', e)
        return {}
      }
    },

    async saveChannelConfig(botId: string, config: BotChannelsConfig): Promise<void> {
      try {
        await invoke('channel_save_config', { botId, config })
      } catch (e) {
        console.error('Failed to save channel config:', e)
      }
    },

    async testChannelConnection(botId: string, platform: string): Promise<boolean> {
      try {
        await invoke('channel_test_connection', { botId, platform })
        return true
      } catch (e) {
        console.error(`Channel ${platform} test failed:`, e)
        return false
      }
    },

    async startChannels(botId: string): Promise<void> {
      try {
        await invoke('channel_start', { botId })
        await this.refreshChannelStatus(botId)
      } catch (e) {
        console.error('Failed to start channels:', e)
      }
    },

    async stopChannels(botId: string): Promise<void> {
      try {
        await invoke('channel_stop', { botId })
        await this.refreshChannelStatus(botId)
      } catch (e) {
        console.error('Failed to stop channels:', e)
      }
    },

    async refreshChannelStatus(botId: string): Promise<void> {
      try {
        const statuses: ChannelStatus[] = await invoke('channel_get_status', { botId })
        this.channelStatuses = { ...this.channelStatuses, [botId]: statuses }
      } catch (e) {
        console.error('Failed to get channel status:', e)
      }
    },

    // --- Schedule management ---

    async loadScheduleConfig(botId: string): Promise<BotScheduleConfig | null> {
      try {
        const bot: BotDefinition = await invoke('bot_get_bot', { botId })
        return bot.scheduleConfig ?? null
      } catch (e) {
        console.error('Failed to load schedule config:', e)
        return null
      }
    },

    async saveScheduleConfig(botId: string, config: BotScheduleConfig): Promise<void> {
      try {
        await invoke('bot_update_bot', {
          botId,
          updates: JSON.stringify({ scheduleConfig: config }),
        })
        this.bots = this.bots.map(b =>
          b.id === botId ? { ...b, scheduleConfig: config } : b
        )
      } catch (e) {
        console.error('Failed to save schedule config:', e)
      }
    },
  },
})

export function buildSystemPrompt(name: string, description: string): string {
  return [
    `你是 "${name}"，一个 AI 机器人。`,
    '',
    `## 你的职责`,
    description,
    '',
    `## 文件结构`,
    `- CLAUDE.md: 持久记忆，每次会话自动加载（由 SDK settingSources 注入）`,
    `- output/: 生成的文件默认存放在此目录，用户可直接查看`,
    `- conversation.json: 可选的对话记录文件，全部的交互历史（仅在查询历史时使用）`,
    `- workspace/: 工作文件存储区`,
    '',
    `## 行为规则`,
    `- 精确执行用户指令，完成后报告结果，必要时可以使用 markdown 格式来组织内容。`,
    `- 将生成的文件（代码、文档、数据等）保存到 output/ 目录`,
    '',
    `## CLAUDE.md 记忆维护`,
    `- CLAUDE.md 是你的持久记忆，每次会话自动加载`,
    `- 仅当有长期价值的信息时才更新，日常问答无需记录`,
    `- 记忆分为以下章节，更新时写入对应章节：`,
    `  - 「用户画像」：沟通风格、语言偏好、专业领域、需求模式`,
    `  - 「工具与工作流偏好」：偏好的工具链、代码风格、框架选择`,
    `  - 「关键知识」：项目架构、重要约定、领域知识`,
    `  - 「错误规避」：纠正过的错误、需避免的陷阱`,
    `  - 「近期摘要」：最近 8 条重要记录（日期 + 一句话要点）`,
    `- 每条记忆保留足够上下文（1-2 句），避免过度精简导致遗忘原因`,
    `- 全文控制在 200 行以内，超出时精简最旧或最低价值的条目`,
    '',
    `## Skills 使用`,
    `- 使用 Skill 工具调用已安装的 skills`,
    `- 用户输入 "/skills" 时列出可用技能`,
    `- 用户输入 "/技能名称" 或 "/skill 技能名称" 时，调用对应技能`,
    `- skills 路径: ~/.claude/skills 和 ~/.claude/plugins/cache/ (插件安装)`,
  ].join('\n')
}
