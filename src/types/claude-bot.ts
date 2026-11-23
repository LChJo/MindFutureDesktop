export type BotStatus = 'idle' | 'working' | 'error' | 'stopped' | 'paused'

// === Bot Definition ===
export interface MessageFilterConfig {
  enabled: boolean
  prompt: string
  model: string   // e.g. "Haiku", "Sonnet", "Opus"
}

export interface BotDefinition {
  id: string
  name: string
  description: string
  status: BotStatus
  model: string
  createdAt: number
  updatedAt: number
  systemPrompt: string
  spriteVariant: number
  color: string
  workDir: string
  sidecarSessionId?: string
  sdkSessionId?: string
  totalInteractions: number
  lastActiveAt?: number
  errorMessage?: string
  scheduleConfig?: BotScheduleConfig
  messageFilter?: MessageFilterConfig
}

// === Bot Index (持久化索引) ===
export interface BotIndexItem {
  id: string
  name: string
  status: BotStatus
  createdAt: number
  updatedAt: number
  lastActiveAt?: number
}

// === Bot Conversation ===
export interface BotConversation {
  botId: string
  messages: BotChatMessage[]
  sdkSessionId?: string
  updatedAt: number
}

export interface BotChatMessage {
  id: string
  role: 'user' | 'assistant' | 'system'
  content: string
  timestamp: number
  toolUsages?: BotToolUsage[]
  isSystem?: boolean
}

export interface BotToolUsage {
  id: string
  toolName: string
  toolId: string
  input?: Record<string, unknown>
  summary?: string
  status: 'pending' | 'running' | 'completed' | 'error'
}

// === Channel Types ===
export interface TelegramChannelConfig {
  enabled: boolean
  botToken: string
  chatId: string
}

export interface DiscordChannelConfig {
  enabled: boolean
  botToken: string
  channelId: string
}

export interface FeishuChannelConfig {
  enabled: boolean
  appId: string
  appSecret: string
  chatId?: string
  replyAllGroupMessages?: boolean
}

export interface BotChannelsConfig {
  telegram?: TelegramChannelConfig
  discord?: DiscordChannelConfig
  feishu?: FeishuChannelConfig
}

export interface ChannelStatus {
  platform: string
  connected: boolean
  /** Real-time connection state: "disconnected" | "connecting" | "connected" */
  state: string
  error?: string
}

export interface ChannelAttachment {
  path: string
  name: string
}

export interface ChannelIncomingMessage {
  platform: string
  channelId: string
  sender: string
  content: string
  timestamp: number
  attachments?: ChannelAttachment[]
  /** Platform-specific message ID (e.g. Feishu message_id) for reaction management. */
  messageId?: string
}

// === Schedule Types ===
export interface IntervalEntry {
  intervalMinutes: number   // 间隔分钟数
  message?: string          // 独立消息内容（留空则回退到 BotScheduleConfig.message）
  chatId?: string           // 独立目标 Chat ID（留空则依赖来源频道）
}

export interface FixedTimeEntry {
  time: string          // "HH:mm"
  repeatDays?: number[] // 0=周日..6=周六；undefined = 每天
  message?: string      // 独立消息内容（留空则回退到 BotScheduleConfig.message）
  chatId?: string       // 独立目标 Chat ID（留空则依赖来源频道）
}

export interface BotScheduleConfig {
  intervalEnabled: boolean
  intervals: IntervalEntry[]        // 多条间隔配置
  fixedTimeEnabled: boolean
  fixedTimes: FixedTimeEntry[]
  message: string                   // 全局默认消息（未单独配置时使用）
}

/** 将旧格式 BotScheduleConfig 迁移为新格式 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function migrateScheduleConfig(raw: any): BotScheduleConfig {
  if (!raw) {
    return {
      intervalEnabled: false,
      intervals: [],
      fixedTimeEnabled: false,
      fixedTimes: [],
      message: '',
    }
  }
  // 最新格式（intervals 数组）
  if (Array.isArray(raw.intervals)) {
    return {
      intervalEnabled: raw.intervalEnabled ?? false,
      intervals: raw.intervals,
      fixedTimeEnabled: raw.fixedTimeEnabled ?? false,
      fixedTimes: raw.fixedTimes ?? [],
      message: raw.message ?? '',
    }
  }
  // 中间格式（intervalMinutes 单条 + 可选 intervalMessage/intervalChatId）
  if ('intervalEnabled' in raw) {
    const minutes = raw.intervalMinutes ?? 60
    const entry: IntervalEntry = {
      intervalMinutes: minutes,
      ...(raw.intervalMessage ? { message: raw.intervalMessage } : {}),
      ...(raw.intervalChatId ? { chatId: raw.intervalChatId } : {}),
    }
    return {
      intervalEnabled: raw.intervalEnabled ?? false,
      intervals: minutes ? [entry] : [],
      fixedTimeEnabled: raw.fixedTimeEnabled ?? false,
      fixedTimes: raw.fixedTimes ?? [],
      message: raw.message ?? '',
    }
  }
  // 最旧格式迁移
  const isInterval = raw.type === 'interval'
  const oldTimes: string[] = raw.fixedTimes ?? []
  const oldMinutes = raw.intervalMinutes ?? 60
  return {
    intervalEnabled: !!raw.enabled && isInterval,
    intervals: (!!raw.enabled && isInterval && oldMinutes)
      ? [{ intervalMinutes: oldMinutes }]
      : [],
    fixedTimeEnabled: !!raw.enabled && !isInterval,
    fixedTimes: oldTimes.map((time: string) => ({
      time,
      repeatDays: raw.repeatDays?.length ? [...raw.repeatDays] : undefined,
    })),
    message: raw.message ?? '',
  }
}

// === Web Server API Types ===
export interface BotStatusApiResponse {
  bots: BotDefinition[]
  appVersion: string
  timestamp: number
}
