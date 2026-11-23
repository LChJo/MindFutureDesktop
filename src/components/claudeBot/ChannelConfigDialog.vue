<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useBotStore } from '@/stores/botStore'
import { useBotChannels } from '@/composables/claudeBot/useBotChannels'
import type { BotChannelsConfig, MessageFilterConfig } from '@/types/claude-bot'

const props = defineProps<{
  botId: string
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const botStore = useBotStore()
const channels = useBotChannels()
const activeTab = ref<'telegram' | 'discord' | 'feishu' | 'filter'>('telegram')
const saving = ref(false)
const testing = ref<string | null>(null)
const testResult = ref<{ platform: string; success: boolean; message: string } | null>(null)

const showHelp = ref(false)

// 提示消息
const hintMessage = ref('')
let hintTimer: ReturnType<typeof setTimeout> | null = null

function showHint(msg: string) {
  hintMessage.value = msg
  if (hintTimer) clearTimeout(hintTimer)
  hintTimer = setTimeout(() => { hintMessage.value = '' }, 3000)
}

const config = ref<BotChannelsConfig>({
  telegram: { enabled: false, botToken: '', chatId: '' },
  discord: { enabled: false, botToken: '', channelId: '' },
  feishu: { enabled: false, appId: '', appSecret: '', chatId: '', replyAllGroupMessages: false },
})

const filterConfig = ref<MessageFilterConfig>({
  enabled: false,
  prompt: '',
  model: 'Haiku',
})
const filterSaving = ref(false)

// 连接状态轮询
let statusPollTimer: ReturnType<typeof setInterval> | null = null

function startStatusPolling() {
  stopStatusPolling()
  botStore.refreshChannelStatus(props.botId)
  statusPollTimer = setInterval(() => {
    botStore.refreshChannelStatus(props.botId)
  }, 3000)
}

function stopStatusPolling() {
  if (statusPollTimer) {
    clearInterval(statusPollTimer)
    statusPollTimer = null
  }
}

/** Get the real-time connection state for a platform: "disconnected" | "connecting" | "connected" */
function getPlatformState(platform: string): string {
  const statuses = botStore.channelStatuses[props.botId]
  const found = statuses?.find(s => s.platform === platform)
  return found?.state ?? 'disconnected'
}

const feishuConnState = computed(() => getPlatformState('feishu'))

async function loadConfig() {
  const loaded = await botStore.loadChannelConfig(props.botId)
  config.value = {
    telegram: loaded.telegram ?? { enabled: false, botToken: '', chatId: '' },
    discord: loaded.discord ?? { enabled: false, botToken: '', channelId: '' },
    feishu: loaded.feishu ?? { enabled: false, appId: '', appSecret: '', chatId: '', replyAllGroupMessages: false },
  }
  // Load filter config from bot definition
  const bot = botStore.bots.find(b => b.id === props.botId)
  if (bot?.messageFilter) {
    filterConfig.value = { ...filterConfig.value, ...bot.messageFilter }
  }
}

async function handleSaveFilter() {
  filterSaving.value = true
  try {
    await botStore.updateBot(props.botId, { messageFilter: { ...filterConfig.value } })
    showHint('过滤配置已保存')
  } catch (e) {
    console.error('[ChannelConfig] Save filter failed:', e)
    showHint('保存失败')
  } finally {
    filterSaving.value = false
  }
}

// === 验证 ===
function validatePlatform(platform: string): string | null {
  if (platform === 'telegram') {
    const t = config.value.telegram
    if (!t?.botToken?.trim()) return '请填写 Bot Token'
    if (!t?.chatId?.trim()) return '请填写 Chat ID'
  } else if (platform === 'discord') {
    const d = config.value.discord
    if (!d?.botToken?.trim()) return '请填写 Bot Token'
    if (!d?.channelId?.trim()) return '请填写 Channel ID'
  } else if (platform === 'feishu') {
    const f = config.value.feishu
    if (!f?.appId?.trim()) return '请填写 App ID'
    if (!f?.appSecret?.trim()) return '请填写 App Secret'
    // chatId is optional — replies auto-target the source group
  }
  return null
}

// === 设置 enabled 状态 ===
function setPlatformEnabled(platform: string, enabled: boolean) {
  if (platform === 'telegram' && config.value.telegram) {
    config.value = { ...config.value, telegram: { ...config.value.telegram, enabled } }
  } else if (platform === 'discord' && config.value.discord) {
    config.value = { ...config.value, discord: { ...config.value.discord, enabled } }
  } else if (platform === 'feishu' && config.value.feishu) {
    config.value = { ...config.value, feishu: { ...config.value.feishu, enabled } }
  }
}

function isPlatformEnabled(platform: string): boolean {
  if (platform === 'telegram') return !!config.value.telegram?.enabled
  if (platform === 'discord') return !!config.value.discord?.enabled
  if (platform === 'feishu') return !!config.value.feishu?.enabled
  return false
}

function hasAnyEnabled(): boolean {
  return !!(
    config.value.telegram?.enabled ||
    config.value.discord?.enabled ||
    config.value.feishu?.enabled
  )
}

// === 连接/断开 ===
async function handleConnect(platform: string) {
  const error = validatePlatform(platform)
  if (error) {
    showHint(error)
    return
  }

  saving.value = true
  try {
    setPlatformEnabled(platform, true)
    await botStore.saveChannelConfig(props.botId, config.value)

    const bot = botStore.bots.find(b => b.id === props.botId)
    if (bot) await channels.restartListening(bot)
    await botStore.refreshChannelStatus(props.botId)
  } catch (e) {
    console.error('[ChannelConfig] Connect failed:', e)
    showHint('连接失败，请检查配置')
    setPlatformEnabled(platform, false)
  } finally {
    saving.value = false
  }
}

async function handleDisconnect(platform: string) {
  saving.value = true
  try {
    setPlatformEnabled(platform, false)
    await botStore.saveChannelConfig(props.botId, config.value)

    const bot = botStore.bots.find(b => b.id === props.botId)
    if (bot) {
      if (hasAnyEnabled()) {
        await channels.restartListening(bot)
      } else {
        await channels.stopListening(bot.id)
      }
    }
    await botStore.refreshChannelStatus(props.botId)
  } finally {
    saving.value = false
  }
}

// === 重启（保存并应用最新配置）===
async function handleRestart(platform: string) {
  const error = validatePlatform(platform)
  if (error) {
    showHint(error)
    return
  }
  saving.value = true
  try {
    await botStore.saveChannelConfig(props.botId, config.value)
    const bot = botStore.bots.find(b => b.id === props.botId)
    if (bot) await channels.restartListening(bot)
    await botStore.refreshChannelStatus(props.botId)
    showHint('配置已应用')
  } catch (e) {
    console.error('[ChannelConfig] Restart failed:', e)
    showHint('重启失败，请检查配置')
  } finally {
    saving.value = false
  }
}

// === 测试连接 ===
async function handleTest(platform: string) {
  const error = validatePlatform(platform)
  if (error) {
    showHint(error)
    return
  }

  testing.value = platform
  testResult.value = null

  // 先保存当前配置再测试
  await botStore.saveChannelConfig(props.botId, config.value)
  const success = await botStore.testChannelConnection(props.botId, platform)
  testResult.value = {
    platform,
    success,
    message: success ? '连接成功' : '连接失败，请检查配置',
  }
  testing.value = null
}

function handleOverlayClick(e: MouseEvent) {
  if ((e.target as HTMLElement).classList.contains('channel-dialog-overlay')) {
    emit('close')
  }
}

watch(() => props.visible, (v) => {
  if (v) {
    loadConfig()
    testResult.value = null
    hintMessage.value = ''
    startStatusPolling()
  } else {
    stopStatusPolling()
  }
})

onMounted(() => {
  if (props.visible) {
    loadConfig()
    startStatusPolling()
  }
})

onUnmounted(() => {
  stopStatusPolling()
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="channel-dialog-overlay"
      @click="handleOverlayClick"
    >
      <div class="channel-dialog">
        <!-- Header -->
        <div class="dialog-header">
          <div class="header-title">
            <i class="fa-solid fa-tower-broadcast" />
            频道配置
          </div>
          <button
            class="close-btn"
            @click="emit('close')"
          >
            <i class="fa-solid fa-xmark" />
          </button>
        </div>

        <!-- Tabs -->
        <div class="dialog-tabs">
          <button
            class="tab-btn"
            :class="{ active: activeTab === 'telegram' }"
            @click="activeTab = 'telegram'"
          >
            <i class="fa-brands fa-telegram" />
            Telegram
            <span
              v-if="isPlatformEnabled('telegram')"
              class="tab-dot"
            />
          </button>
          <button
            class="tab-btn"
            :class="{ active: activeTab === 'discord' }"
            @click="activeTab = 'discord'"
          >
            <i class="fa-brands fa-discord" />
            Discord
            <span
              v-if="isPlatformEnabled('discord')"
              class="tab-dot"
            />
          </button>
          <button
            class="tab-btn"
            :class="{ active: activeTab === 'feishu' }"
            @click="activeTab = 'feishu'"
          >
            <i class="fa-solid fa-feather" />
            Feishu
            <span
              v-if="isPlatformEnabled('feishu')"
              class="tab-dot"
            />
          </button>
          <button
            class="tab-btn"
            :class="{ active: activeTab === 'filter' }"
            @click="activeTab = 'filter'"
          >
            <i class="fa-solid fa-filter" />
            过滤
            <span
              v-if="filterConfig.enabled"
              class="tab-dot tab-dot-filter"
            />
          </button>
        </div>

        <!-- Tab content -->
        <div class="dialog-body">
          <!-- Telegram -->
          <div
            v-if="activeTab === 'telegram' && config.telegram"
            class="tab-content"
          >
            <div class="platform-status-row">
              <div class="platform-status">
                <span
                  v-if="config.telegram.enabled"
                  class="status-active"
                >
                  <i class="fa-solid fa-circle" /> 已连接
                </span>
                <span
                  v-else
                  class="status-inactive"
                >
                  <i class="fa-regular fa-circle" /> 未连接
                </span>
              </div>
              <div class="platform-action-buttons">
                <button
                  v-if="config.telegram.enabled"
                  class="restart-btn"
                  :disabled="saving"
                  @click="handleRestart('telegram')"
                >
                  <i class="fa-solid fa-rotate" /> 重启
                </button>
                <button
                  v-if="config.telegram.enabled"
                  class="disconnect-btn"
                  :disabled="saving"
                  @click="handleDisconnect('telegram')"
                >
                  断开
                </button>
                <button
                  v-else
                  class="connect-btn"
                  :disabled="saving"
                  @click="handleConnect('telegram')"
                >
                  连接
                </button>
              </div>
            </div>
            <label class="field">
              <span class="field-label">Bot Token</span>
              <input
                type="password"
                :value="config.telegram.botToken"
                placeholder="123456:ABC-DEF..."
                @input="config = { ...config, telegram: { ...config.telegram!, botToken: ($event.target as HTMLInputElement).value } }"
              >
            </label>
            <label class="field">
              <span class="field-label">Chat ID</span>
              <input
                :value="config.telegram.chatId"
                placeholder="-1001234567890"
                @input="config = { ...config, telegram: { ...config.telegram!, chatId: ($event.target as HTMLInputElement).value } }"
              >
            </label>
            <!-- Help -->
            <button
              class="help-toggle"
              @click="showHelp = !showHelp"
            >
              <i class="fa-solid fa-circle-info" />
              {{ showHelp ? '隐藏配置指南' : '配置指南' }}
              <i
                :class="showHelp ? 'fa-solid fa-chevron-up' : 'fa-solid fa-chevron-down'"
                style="margin-left: auto; font-size: 10px;"
              />
            </button>
            <div
              v-if="showHelp"
              class="help-box"
            >
              <ol>
                <li>在 Telegram 搜索 <strong>@BotFather</strong>，发送 <code>/newbot</code></li>
                <li>按提示设置名称，获得 Bot Token（格式：<code>123456:ABC-DEF...</code>）</li>
                <li>将 Bot 添加到目标群组</li>
                <li>访问 <code>https://api.telegram.org/bot&#123;TOKEN&#125;/getUpdates</code>，从 JSON 中找到 <code>chat.id</code>（群组 ID 通常为负数，如 <code>-1001234567890</code>）</li>
                <li>如果在群组中使用，请在 @BotFather 中发送 <code>/setprivacy</code> → <strong>Disable</strong>，关闭隐私模式后 Bot 才能接收所有群消息（否则只能收到 /命令 和 @提及）</li>
              </ol>
            </div>
          </div>

          <!-- Discord -->
          <div
            v-if="activeTab === 'discord' && config.discord"
            class="tab-content"
          >
            <div class="platform-status-row">
              <div class="platform-status">
                <span
                  v-if="config.discord.enabled"
                  class="status-active"
                >
                  <i class="fa-solid fa-circle" /> 已连接
                </span>
                <span
                  v-else
                  class="status-inactive"
                >
                  <i class="fa-regular fa-circle" /> 未连接
                </span>
              </div>
              <div class="platform-action-buttons">
                <button
                  v-if="config.discord.enabled"
                  class="restart-btn"
                  :disabled="saving"
                  @click="handleRestart('discord')"
                >
                  <i class="fa-solid fa-rotate" /> 重启
                </button>
                <button
                  v-if="config.discord.enabled"
                  class="disconnect-btn"
                  :disabled="saving"
                  @click="handleDisconnect('discord')"
                >
                  断开
                </button>
                <button
                  v-else
                  class="connect-btn"
                  :disabled="saving"
                  @click="handleConnect('discord')"
                >
                  连接
                </button>
              </div>
            </div>
            <label class="field">
              <span class="field-label">Bot Token</span>
              <input
                type="password"
                :value="config.discord.botToken"
                placeholder="MTk4Njc..."
                @input="config = { ...config, discord: { ...config.discord!, botToken: ($event.target as HTMLInputElement).value } }"
              >
            </label>
            <label class="field">
              <span class="field-label">Channel ID</span>
              <input
                :value="config.discord.channelId"
                placeholder="123456789012345678"
                @input="config = { ...config, discord: { ...config.discord!, channelId: ($event.target as HTMLInputElement).value } }"
              >
            </label>
            <button
              class="help-toggle"
              @click="showHelp = !showHelp"
            >
              <i class="fa-solid fa-circle-info" />
              {{ showHelp ? '隐藏配置指南' : '配置指南' }}
              <i
                :class="showHelp ? 'fa-solid fa-chevron-up' : 'fa-solid fa-chevron-down'"
                style="margin-left: auto; font-size: 10px;"
              />
            </button>
            <div
              v-if="showHelp"
              class="help-box"
            >
              <ol>
                <li>访问 <code>discord.com/developers/applications</code>，创建 Application</li>
                <li>进入 Bot 页签，点击 Reset Token 获取 Bot Token</li>
                <li>开启 <strong>MESSAGE CONTENT INTENT</strong>（Privileged Gateway Intents）</li>
                <li>用 OAuth2 URL Generator 生成邀请链接（勾选 bot + Send Messages），邀请到服务器</li>
                <li>在 Discord 开启开发者模式（设置 → 高级），右键频道 → 复制频道 ID</li>
              </ol>
            </div>
          </div>

          <!-- Feishu -->
          <div
            v-if="activeTab === 'feishu' && config.feishu"
            class="tab-content"
          >
            <div class="platform-status-row">
              <div class="platform-status">
                <span
                  v-if="feishuConnState === 'connected'"
                  class="status-active"
                >
                  <i class="fa-solid fa-circle" /> 已连接
                </span>
                <span
                  v-else-if="feishuConnState === 'connecting'"
                  class="status-connecting"
                >
                  <i class="fa-solid fa-spinner fa-spin" /> 连接中...
                </span>
                <span
                  v-else
                  class="status-inactive"
                >
                  <i class="fa-regular fa-circle" /> 未连接
                </span>
              </div>
              <div class="platform-action-buttons">
                <button
                  v-if="config.feishu.enabled"
                  class="restart-btn"
                  :disabled="saving"
                  @click="handleRestart('feishu')"
                >
                  <i class="fa-solid fa-rotate" /> 重启
                </button>
                <button
                  v-if="config.feishu.enabled"
                  class="disconnect-btn"
                  :disabled="saving"
                  @click="handleDisconnect('feishu')"
                >
                  断开
                </button>
                <button
                  v-else
                  class="connect-btn"
                  :disabled="saving"
                  @click="handleConnect('feishu')"
                >
                  连接
                </button>
              </div>
            </div>
            <!-- 长连接说明 -->
            <div class="feishu-ws-info">
              <div class="ws-badge">
                <i class="fa-solid fa-link" /> WebSocket 长连接
              </div>
              <div class="ws-desc">
                通过飞书 SDK 建立持久连接，实时接收消息。无需公网 IP，无需轮询。
                <template v-if="feishuConnState === 'connected'">
                  <br><span class="ws-hint-active">Bot 正在监听私聊消息和群内 @bot 消息，断线后自动重连。</span>
                </template>
                <template v-else-if="feishuConnState === 'connecting'">
                  <br><span class="ws-hint-connecting">正在建立 WebSocket 连接，请稍候...</span>
                </template>
                <template v-else>
                  <br><span class="ws-hint-inactive">点击「连接」后，将建立 WebSocket 长连接开始监听消息。</span>
                </template>
              </div>
            </div>
            <label class="field">
              <span class="field-label">App ID</span>
              <input
                :value="config.feishu.appId"
                placeholder="cli_a1b2c3..."
                @input="config = { ...config, feishu: { ...config.feishu!, appId: ($event.target as HTMLInputElement).value } }"
              >
            </label>
            <label class="field">
              <span class="field-label">App Secret</span>
              <input
                type="password"
                :value="config.feishu.appSecret"
                placeholder="..."
                @input="config = { ...config, feishu: { ...config.feishu!, appSecret: ($event.target as HTMLInputElement).value } }"
              >
            </label>
            <label class="field">
              <span class="field-label">Chat ID <span class="field-hint">（可选 — 仅用于主动推送，回复消息自动使用来源群）</span></span>
              <input
                :value="config.feishu.chatId"
                placeholder="oc_a1b2c3..."
                @input="config = { ...config, feishu: { ...config.feishu!, chatId: ($event.target as HTMLInputElement).value } }"
              >
            </label>
            <label class="field toggle-field">
              <div class="toggle-row">
                <input
                  type="checkbox"
                  :checked="config.feishu.replyAllGroupMessages ?? false"
                  @change="config = { ...config, feishu: { ...config.feishu!, replyAllGroupMessages: ($event.target as HTMLInputElement).checked } }"
                >
                <span class="field-label">响应所有群消息</span>
              </div>
              <span class="field-hint toggle-hint">关闭时仅响应群内 @机器人 的消息，开启时响应群内所有消息</span>
            </label>
            <button
              class="help-toggle"
              @click="showHelp = !showHelp"
            >
              <i class="fa-solid fa-circle-info" />
              {{ showHelp ? '隐藏配置指南' : '配置指南' }}
              <i
                :class="showHelp ? 'fa-solid fa-chevron-up' : 'fa-solid fa-chevron-down'"
                style="margin-left: auto; font-size: 10px;"
              />
            </button>
            <div
              v-if="showHelp"
              class="help-box"
            >
              <div class="help-section">
                <div class="help-section-title">
                  1. 创建飞书应用
                </div>
                <ul>
                  <li>访问 <code>open.feishu.cn/app</code>，点击「创建自建应用」</li>
                  <li>在「凭证与基础信息」页面获取 <strong>App ID</strong> 和 <strong>App Secret</strong></li>
                </ul>
              </div>
              <div class="help-section">
                <div class="help-section-title">
                  2. 添加权限
                </div>
                <ul>
                  <li>进入「权限管理」，搜索并添加以下权限：</li>
                  <li><code>im:message.p2p_msg:readonly</code> — 接收私聊消息</li>
                  <li><code>im:message.group_at_msg:readonly</code> — 接收群内 @bot 消息</li>
                  <li><code>im:message:send_as_bot</code> — 以机器人身份发送消息</li>
                  <li><code>im:message.reactions:write_only</code> — 消息表情回复（收到消息后自动标记 👍）</li>
                </ul>
              </div>
              <div class="help-section">
                <div class="help-section-title">
                  3. 配置长连接事件订阅
                </div>
                <ul>
                  <li>进入「事件与回调」页面</li>
                  <li>在「订阅方式」中选择 <strong>「使用长连接接收事件」</strong></li>
                  <li class="help-note">
                    注意：首次启用需先点击本页面的「连接」建立 WebSocket 连接，飞书后台才能检测到并允许保存
                  </li>
                  <li>在「事件列表」中添加 <strong>「接收消息 v2.0」</strong>（im.message.receive_v1）</li>
                </ul>
              </div>
              <div class="help-section">
                <div class="help-section-title">
                  4. 启用机器人能力
                </div>
                <ul>
                  <li>进入「机器人」页面，启用机器人功能</li>
                </ul>
              </div>
              <div class="help-section">
                <div class="help-section-title">
                  5. Chat ID（可选）
                </div>
                <ul>
                  <li>回复消息时会自动发送到来源群组，无需配置 Chat ID</li>
                  <li>如需主动推送消息（如定时任务），可在群设置 → 群管理中找到「群号」，格式为 <code>oc_xxx</code></li>
                </ul>
              </div>
              <div class="help-section">
                <div class="help-section-title">
                  6. 发布应用
                </div>
                <ul>
                  <li>在「版本管理与发布」中创建版本并发布</li>
                  <li>管理员审批通过后，权限生效</li>
                </ul>
              </div>
            </div>
          </div>

          <!-- Filter -->
          <div
            v-if="activeTab === 'filter'"
            class="tab-content"
          >
            <div class="filter-desc">
              <i class="fa-solid fa-circle-info" />
              收到飞书消息后，先用小模型（Haiku）快速判断是否交给 Bot 处理。通过才点表情确认、启动 Agent；被拦截时静默撤回表情。
            </div>
            <label class="field toggle-field">
              <div class="toggle-row">
                <input
                  type="checkbox"
                  :checked="filterConfig.enabled"
                  @change="filterConfig = { ...filterConfig, enabled: ($event.target as HTMLInputElement).checked }"
                >
                <span class="field-label">启用消息过滤</span>
              </div>
            </label>
            <label class="field">
              <span class="field-label">判断模型</span>
              <select
                :value="filterConfig.model"
                class="filter-model-select"
                @change="filterConfig = { ...filterConfig, model: ($event.target as HTMLSelectElement).value }"
              >
                <option value="Haiku">Haiku（推荐，速度快）</option>
                <option value="Sonnet">Sonnet</option>
                <option value="Opus">Opus</option>
              </select>
            </label>
            <label class="field">
              <span class="field-label">过滤 Prompt</span>
              <textarea
                :value="filterConfig.prompt"
                class="filter-prompt"
                placeholder="你是消息过滤器。判断用户消息是否应由 AI 助手处理。应处理则回复 YES，否则回复 NO。"
                rows="5"
                @input="filterConfig = { ...filterConfig, prompt: ($event.target as HTMLTextAreaElement).value }"
              />
              <span class="field-hint">模型回复以 YES 开头视为通过，否则拦截。</span>
              <span
                v-if="filterConfig.enabled && !filterConfig.prompt.trim()"
                class="field-hint field-hint-warn"
              >
                启用过滤时必须填写 Prompt
              </span>
            </label>
            <div class="filter-actions">
              <button
                class="connect-btn"
                :disabled="filterSaving || (filterConfig.enabled && !filterConfig.prompt.trim())"
                @click="handleSaveFilter"
              >
                <i
                  v-if="filterSaving"
                  class="fa-solid fa-spinner fa-spin"
                />
                <i
                  v-else
                  class="fa-solid fa-floppy-disk"
                />
                保存过滤配置
              </button>
            </div>
          </div>

          <!-- Test result -->
          <div
            v-if="testResult"
            class="test-result"
            :class="{ success: testResult.success, error: !testResult.success }"
          >
            <i :class="testResult.success ? 'fa-solid fa-check-circle' : 'fa-solid fa-exclamation-circle'" />
            {{ testResult.message }}
          </div>

          <!-- Hint message -->
          <Transition name="hint-fade">
            <div
              v-if="hintMessage"
              class="hint-bar"
            >
              <i class="fa-solid fa-circle-exclamation" />
              {{ hintMessage }}
            </div>
          </Transition>
        </div>

        <!-- Footer -->
        <div class="dialog-footer">
          <button
            v-if="activeTab !== 'filter'"
            class="test-btn"
            :disabled="testing !== null"
            @click="handleTest(activeTab)"
          >
            <i :class="testing ? 'fa-solid fa-spinner fa-spin' : 'fa-solid fa-plug'" />
            {{ testing ? '测试中...' : '测试连接' }}
          </button>
          <button
            class="close-dialog-btn"
            @click="emit('close')"
          >
            关闭
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped lang="less">
.channel-dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10001;
}

.channel-dialog {
  width: 480px;
  max-height: 80vh;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  display: flex;
  flex-direction: column;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid #30363d;
}

.header-title {
  font-size: 14px;
  font-weight: 600;
  color: #e6edf3;
  display: flex;
  align-items: center;
  gap: 8px;
}

.close-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  color: #8b949e;
  cursor: pointer;
  border-radius: 4px;

  &:hover {
    background: #21262d;
    color: #e6edf3;
  }
}

.dialog-tabs {
  display: flex;
  padding: 0 20px;
  border-bottom: 1px solid #30363d;
}

.tab-btn {
  padding: 10px 16px;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  color: #8b949e;
  cursor: pointer;
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 6px;
  transition: color 0.15s, border-color 0.15s;
  position: relative;

  &:hover {
    color: #e6edf3;
  }

  &.active {
    color: #58a6ff;
    border-bottom-color: #58a6ff;
  }
}

.tab-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #3fb950;
  display: inline-block;
}

.dialog-body {
  padding: 20px;
  overflow-y: auto;
  flex: 1;
}

.tab-content {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

// === Platform 状态行 ===
.platform-status-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 8px;
}

.platform-status {
  display: flex;
  align-items: center;
}

.status-active {
  font-size: 12px;
  color: #3fb950;
  display: flex;
  align-items: center;
  gap: 6px;
  i { font-size: 6px; }
}

.status-connecting {
  font-size: 12px;
  color: #d29922;
  display: flex;
  align-items: center;
  gap: 6px;
  i { font-size: 10px; }
}

.status-inactive {
  font-size: 12px;
  color: #8b949e;
  display: flex;
  align-items: center;
  gap: 6px;
  i { font-size: 6px; }
}

.platform-action-buttons {
  display: flex;
  gap: 6px;
  align-items: center;
}

.restart-btn {
  padding: 5px 12px;
  background: rgba(88, 166, 255, 0.08);
  border: 1px solid rgba(88, 166, 255, 0.2);
  border-radius: 4px;
  color: #58a6ff;
  cursor: pointer;
  font-size: 12px;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 4px;

  &:hover:not(:disabled) { background: rgba(88, 166, 255, 0.18); }
  &:disabled { opacity: 0.5; cursor: not-allowed; }
}

.connect-btn {
  padding: 5px 14px;
  background: #238636;
  border: 1px solid rgba(240, 246, 252, 0.1);
  border-radius: 4px;
  color: #fff;
  cursor: pointer;
  font-size: 12px;
  font-weight: 600;

  &:hover:not(:disabled) { background: #2ea043; }
  &:disabled { opacity: 0.5; cursor: not-allowed; }
}

.disconnect-btn {
  padding: 5px 14px;
  background: #da3633;
  border: 1px solid rgba(240, 246, 252, 0.1);
  border-radius: 4px;
  color: #fff;
  cursor: pointer;
  font-size: 12px;
  font-weight: 600;

  &:hover:not(:disabled) { background: #f85149; }
  &:disabled { opacity: 0.5; cursor: not-allowed; }
}

// === 通用 Field ===
.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-label {
  font-size: 12px;
  color: #8b949e;
}

.field input {
  padding: 8px 12px;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  color: #e6edf3;
  font-size: 13px;
  font-family: 'Courier New', monospace;

  &:focus {
    outline: none;
    border-color: #58a6ff;
  }

  &::placeholder {
    color: #484f58;
  }
}

// === Feishu WebSocket Info ===
.feishu-ws-info {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px 14px;
  background: rgba(56, 132, 255, 0.05);
  border: 1px solid rgba(56, 132, 255, 0.12);
  border-radius: 8px;
}

.ws-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  font-weight: 600;
  color: #58a6ff;
  letter-spacing: 0.3px;

  i { font-size: 10px; }
}

.ws-desc {
  font-size: 11px;
  color: #8b949e;
  line-height: 1.5;
}

.ws-hint-active {
  color: #3fb950;
}

.ws-hint-connecting {
  color: #d29922;
}

.ws-hint-inactive {
  color: #8b949e;
}

.field-hint {
  font-weight: 400;
  color: #6e7681;
  font-size: 11px;
}

.toggle-field {
  .toggle-row {
    display: flex;
    align-items: center;
    gap: 8px;

    input[type="checkbox"] {
      width: 14px;
      height: 14px;
      margin: 0;
      cursor: pointer;
    }

    .field-label {
      margin: 0;
    }
  }

  .toggle-hint {
    display: block;
    margin-top: 2px;
    padding-left: 22px;
  }
}

// === Help ===
.help-toggle {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 12px;
  background: rgba(88, 166, 255, 0.06);
  border: 1px solid rgba(88, 166, 255, 0.15);
  border-radius: 6px;
  color: #58a6ff;
  cursor: pointer;
  font-size: 12px;
  transition: background 0.15s;

  &:hover {
    background: rgba(88, 166, 255, 0.1);
  }
}

.help-box {
  background: rgba(88, 166, 255, 0.06);
  border: 1px solid rgba(88, 166, 255, 0.15);
  border-radius: 6px;
  padding: 12px 16px;
  color: #8b949e;
  font-size: 12px;
  line-height: 1.6;

  ol {
    margin: 0;
    padding-left: 20px;
  }

  li {
    margin-bottom: 4px;
  }

  code {
    background: rgba(110, 118, 129, 0.15);
    padding: 1px 5px;
    border-radius: 3px;
    font-family: 'Courier New', monospace;
    font-size: 11px;
    color: #e6edf3;
  }

  strong {
    color: #e6edf3;
  }

  ul {
    margin: 0;
    padding-left: 18px;
    list-style: disc;
  }
}

.help-section {
  margin-bottom: 10px;

  &:last-child { margin-bottom: 0; }
}

.help-section-title {
  font-size: 12px;
  font-weight: 600;
  color: #e6edf3;
  margin-bottom: 4px;
}

.help-note {
  color: #d29922;
  font-style: italic;
}

// === Test result ===
.test-result {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  border-radius: 6px;
  font-size: 12px;
  margin-top: 8px;

  &.success {
    background: rgba(63, 185, 80, 0.1);
    border: 1px solid rgba(63, 185, 80, 0.2);
    color: #3fb950;
  }

  &.error {
    background: rgba(248, 81, 73, 0.1);
    border: 1px solid rgba(248, 81, 73, 0.2);
    color: #f85149;
  }
}

// === Hint ===
.hint-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: rgba(210, 153, 34, 0.1);
  border: 1px solid rgba(210, 153, 34, 0.3);
  border-radius: 6px;
  color: #d29922;
  font-size: 12px;
  margin-top: 8px;
}

.hint-fade-enter-active,
.hint-fade-leave-active {
  transition: opacity 0.25s;
}
.hint-fade-enter-from,
.hint-fade-leave-to {
  opacity: 0;
}

// === Footer ===
.dialog-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-top: 1px solid #30363d;
}

.test-btn {
  padding: 8px 16px;
  background: rgba(88, 166, 255, 0.1);
  border: 1px solid rgba(88, 166, 255, 0.2);
  border-radius: 6px;
  color: #58a6ff;
  cursor: pointer;
  font-size: 12px;
  display: flex;
  align-items: center;
  gap: 6px;

  &:hover:not(:disabled) {
    background: rgba(88, 166, 255, 0.15);
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.close-dialog-btn {
  padding: 8px 20px;
  background: #21262d;
  border: 1px solid #30363d;
  border-radius: 6px;
  color: #e6edf3;
  cursor: pointer;
  font-size: 12px;

  &:hover {
    background: #30363d;
  }
}

.tab-dot-filter {
  background: #58a6ff !important;
}

.filter-desc {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 8px 10px;
  margin-bottom: 12px;
  border-radius: 6px;
  background: rgba(88, 166, 255, 0.08);
  border: 1px solid rgba(88, 166, 255, 0.2);
  font-size: 11px;
  color: #8b949e;
  line-height: 1.6;

  i {
    color: #58a6ff;
    flex-shrink: 0;
    margin-top: 2px;
  }
}

.filter-model-select {
  width: 100%;
  padding: 6px 10px;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  color: #e6edf3;
  font-size: 12px;
  cursor: pointer;

  &:focus {
    outline: none;
    border-color: #58a6ff;
  }
}

.filter-prompt {
  width: 100%;
  padding: 8px 10px;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  color: #e6edf3;
  font-size: 12px;
  resize: vertical;
  min-height: 80px;
  font-family: inherit;
  line-height: 1.5;

  &:focus {
    outline: none;
    border-color: #58a6ff;
  }

  &::placeholder {
    color: #484f58;
  }
}

.filter-actions {
  margin-top: 8px;
  display: flex;
  justify-content: flex-end;
}

.field-hint-warn {
  color: #d29922;
  display: block;
  margin-top: 4px;
}
</style>
