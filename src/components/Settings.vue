<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import CoworksModelSettings from '@/components/CoworksModelSettings.vue'
import { useSettingsStore } from '@/stores/settingsStore'
import { useDesktopStore } from '@/stores/desktopStore'
import { useAutoUpdate } from '@/composables/useAutoUpdate'
import { open, ask } from '@tauri-apps/plugin-dialog'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { readTextFile, writeTextFile, exists, mkdir } from '@tauri-apps/plugin-fs'
import { homeDir, join } from '@tauri-apps/api/path'

const props = defineProps<{
  windowId?: string
  windowData?: {
    initialTab?: string
  }
}>()

// Node.js status types
interface NodejsStatus {
  installed: boolean
  version: string | null
  path: string | null
  source: string | null // "system" or "bundled"
}

interface ClaudeCodeStatus {
  installed: boolean
  version: string | null
}

const settingsStore = useSettingsStore()
const desktopStore = useDesktopStore()

// Auto-update
const {
  checking: updateChecking,
  downloading: updateDownloading,
  downloadProgress: updateProgress,
  lastCheckResult: updateResult,
  error: updateError,
  checkAndPrompt,
  checkForUpdate: checkForUpdateFn,
} = useAutoUpdate()

// Resolved default update URL (from config file or hardcoded fallback)
const defaultUpdateUrl = ref('')
const loadDefaultUpdateUrl = async () => {
  try {
    defaultUpdateUrl.value = await invoke<string>('get_default_update_url')
  } catch {
    defaultUpdateUrl.value = ''
  }
}

// Use initialTab from windowData if provided, otherwise default to 'appearance'
const activeTab = ref(props.windowData?.initialTab || 'appearance')

// Font detection types
interface FontInfo {
  name: string
  is_monospace: boolean
}

interface FontOption {
  value: string
  label: string
  category: 'recommended-en' | 'recommended-cn' | 'system' | 'default'
}

// Recommended fonts configuration
const recommendedFonts = {
  english: [
    { name: 'Cascadia Code', fallback: 'Consolas, monospace' },
    { name: 'Cascadia Mono', fallback: 'Consolas, monospace' },
    { name: 'Consolas', fallback: '"Courier New", monospace' },
    { name: 'JetBrains Mono', fallback: 'Consolas, monospace' },
    { name: 'SF Mono', fallback: 'Menlo, monospace' },
    { name: 'Menlo', fallback: 'Monaco, monospace' },
    { name: 'Monaco', fallback: 'Menlo, monospace' },
    { name: 'Fira Code', fallback: 'Consolas, monospace' },
  ],
  chinese: [
    { name: 'Sarasa Mono SC', label: '更纱黑体' },
    { name: 'Sarasa Term SC', label: '更纱终端' },
    { name: 'Source Han Mono SC', label: '思源等宽' },
    { name: 'Noto Sans Mono CJK SC', label: 'Noto Sans Mono CJK' },
    { name: 'Microsoft YaHei Mono', label: '微软雅黑等宽' },
    { name: 'PingFang SC', label: '苹方' },
  ]
}

// Font detection state
const systemFonts = ref<FontInfo[]>([])
const availableFonts = ref<FontOption[]>([])
const fontsLoading = ref(false)
const fontsLoaded = ref(false)

// Load system fonts from Tauri backend
const loadSystemFonts = async () => {
  fontsLoading.value = true
  try {
    const fonts = await invoke<FontInfo[]>('get_system_fonts')
    systemFonts.value = fonts
    buildFontOptions()
    fontsLoaded.value = true
  } catch (e) {
    console.error('Failed to load system fonts:', e)
    buildFallbackOptions()
    fontsLoaded.value = true
  } finally {
    fontsLoading.value = false
  }
}

// Build font options with categories
const buildFontOptions = () => {
  const options: FontOption[] = []
  const addedNames = new Set<string>()
  const systemFontNames = systemFonts.value.map(f => f.name.toLowerCase())

  // 1. Add available recommended English fonts
  for (const font of recommendedFonts.english) {
    if (systemFontNames.some(f => f === font.name.toLowerCase())) {
      options.push({
        value: `"${font.name}", ${font.fallback}`,
        label: font.name,
        category: 'recommended-en'
      })
      addedNames.add(font.name.toLowerCase())
    }
  }

  // 2. Add available recommended Chinese fonts
  for (const font of recommendedFonts.chinese) {
    if (systemFontNames.some(f => f === font.name.toLowerCase())) {
      options.push({
        value: `"${font.name}", Consolas, monospace`,
        label: font.label || font.name,
        category: 'recommended-cn'
      })
      addedNames.add(font.name.toLowerCase())
    }
  }

  // 3. Add other detected monospace fonts
  for (const font of systemFonts.value) {
    if (font.is_monospace && !addedNames.has(font.name.toLowerCase())) {
      options.push({
        value: `"${font.name}", monospace`,
        label: font.name,
        category: 'system'
      })
    }
  }

  // 4. Add default option
  options.push({
    value: 'monospace',
    label: '系统等宽字体',
    category: 'default'
  })

  availableFonts.value = options
}

// Fallback options when font detection fails
const buildFallbackOptions = () => {
  availableFonts.value = [
    { value: 'Consolas, "Courier New", monospace', label: 'Consolas', category: 'recommended-en' },
    { value: '"Cascadia Code", Consolas, monospace', label: 'Cascadia Code', category: 'recommended-en' },
    { value: '"Cascadia Mono", Consolas, monospace', label: 'Cascadia Mono', category: 'recommended-en' },
    { value: '"Courier New", Consolas, monospace', label: 'Courier New', category: 'recommended-en' },
    { value: 'monospace', label: '系统等宽字体', category: 'default' }
  ]
}

// Helper: check if category has fonts
const hasCategory = (category: FontOption['category']) => {
  return availableFonts.value.some(f => f.category === category)
}

// Helper: get fonts by category
const fontsByCategory = (category: FontOption['category']) => {
  return availableFonts.value.filter(f => f.category === category)
}

const tabs = [
  { id: 'appearance', label: '外观设置', icon: 'fa-solid fa-palette' },
  { id: 'claudecode', label: 'Claude Code', icon: 'fa-solid fa-robot' },
  { id: 'env', label: '环境变量', icon: 'fa-solid fa-code' },
  { id: 'coworks', label: '对话模型', icon: 'fa-solid fa-comments' },
  { id: 'nodejs', label: 'Node.js', icon: 'fa-brands fa-node-js' },
  { id: 'keybindings', label: '快捷键', icon: 'fa-solid fa-keyboard' },
  { id: 'about', label: '关于', icon: 'fa-solid fa-circle-info' }
]

// Node.js management state
const nodejsStatus = ref<NodejsStatus>({ installed: false, version: null, path: null, source: null })
const claudeCodeStatus = ref<ClaudeCodeStatus>({ installed: false, version: null })
const nodejsLoading = ref(false)
const nodejsStatusLoaded = ref(false)
const claudeCodeLoading = ref(false)
const claudeCodeStatusLoaded = ref(false)
const claudeCodeInstalling = ref(false)
const nodejsError = ref('')
const claudeCodeError = ref('')

// Load Node.js status only
const loadNodejsStatus = async () => {
  nodejsLoading.value = true
  nodejsError.value = ''
  try {
    nodejsStatus.value = await invoke<NodejsStatus>('check_nodejs_status')
    nodejsStatusLoaded.value = true
  } catch (e) {
    nodejsError.value = String(e)
  } finally {
    nodejsLoading.value = false
  }
}

// Load Claude Code status
const loadClaudeCodeStatus = async () => {
  claudeCodeLoading.value = true
  claudeCodeError.value = ''
  try {
    // Always check Node.js status (in case it changed)
    nodejsStatus.value = await invoke<NodejsStatus>('check_nodejs_status')
    nodejsStatusLoaded.value = true

    if (nodejsStatus.value.installed) {
      claudeCodeStatus.value = await invoke<ClaudeCodeStatus>('check_claude_code_status')
    }
    claudeCodeStatusLoaded.value = true
  } catch (e) {
    claudeCodeError.value = String(e)
    console.error('Failed to load Claude Code status:', e)
  } finally {
    claudeCodeLoading.value = false
  }
}

// Extract Node.js
const extractNodejs = async () => {
  nodejsLoading.value = true
  nodejsError.value = ''
  try {
    await invoke('extract_nodejs_cmd')
    await loadNodejsStatus()
  } catch (e) {
    nodejsError.value = String(e)
  } finally {
    nodejsLoading.value = false
  }
}

// Install Claude Code
const installClaudeCode = async () => {
  claudeCodeInstalling.value = true
  claudeCodeError.value = ''
  try {
    await invoke<string>('install_claude_code_cmd')
    // Force reload status after installation/update
    claudeCodeStatusLoaded.value = false
    await loadClaudeCodeStatus()
  } catch (e) {
    claudeCodeError.value = String(e)
  } finally {
    claudeCodeInstalling.value = false
  }
}

// Claude Code settings (~/.claude/settings.json)
const claudeSettings = ref<Record<string, unknown>>({})
const claudeSettingsLoaded = ref(false)
const claudeSettingsSaving = ref(false)

const claudeModel = computed({
  get: () => (claudeSettings.value.model as string) || '',
  set: (v: string) => { claudeSettings.value = { ...claudeSettings.value, model: v }; saveClaudeSettings() }
})
const claudeLanguage = computed({
  get: () => (claudeSettings.value.language as string) || '',
  set: (v: string) => { claudeSettings.value = { ...claudeSettings.value, language: v }; saveClaudeSettings() }
})
const alwaysThinking = computed({
  get: () => !!claudeSettings.value.alwaysThinkingEnabled,
  set: (v: boolean) => { claudeSettings.value = { ...claudeSettings.value, alwaysThinkingEnabled: v }; saveClaudeSettings() }
})
const skipDangerousPrompt = computed({
  get: () => !!claudeSettings.value.skipDangerousModePermissionPrompt,
  set: (v: boolean) => { claudeSettings.value = { ...claudeSettings.value, skipDangerousModePermissionPrompt: v }; saveClaudeSettings() }
})
const promptSuggestion = computed({
  get: () => !!claudeSettings.value.promptSuggestionEnabled,
  set: (v: boolean) => { claudeSettings.value = { ...claudeSettings.value, promptSuggestionEnabled: v }; saveClaudeSettings() }
})
const permissionsDefaultMode = computed({
  get: () => {
    const perms = claudeSettings.value.permissions as Record<string, unknown> | undefined
    return (perms?.defaultMode as string) || 'default'
  },
  set: (v: string) => {
    const perms = (claudeSettings.value.permissions as Record<string, unknown>) || {}
    claudeSettings.value = { ...claudeSettings.value, permissions: { ...perms, defaultMode: v } }
    saveClaudeSettings()
  }
})
const disableExperimentalBetas = computed({
  get: () => !!claudeSettings.value.CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS,
  set: (v: boolean) => {
    const { CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS: _, ...rest } = claudeSettings.value
    claudeSettings.value = v ? { ...rest, CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS: true } : rest
    saveClaudeSettings()
  }
})
const maxAutocompactFailures = computed({
  get: () => {
    const v = claudeSettings.value.MAX_CONSECUTIVE_AUTOCOMPACT_FAILURES
    return v !== undefined ? Number(v) : ''
  },
  set: (v: number | string) => {
    const num = v === '' ? undefined : Number(v)
    if (num !== undefined && isNaN(num)) return
    claudeSettings.value = { ...claudeSettings.value, MAX_CONSECUTIVE_AUTOCOMPACT_FAILURES: num }
    saveClaudeSettings()
  }
})
const apiTimeoutMs = computed({
  get: () => {
    const v = claudeSettings.value.API_TIMEOUT_MS
    return v !== undefined ? Number(v) : ''
  },
  set: (v: number | string) => {
    const num = v === '' ? undefined : Number(v)
    if (num !== undefined && isNaN(num)) return
    claudeSettings.value = { ...claudeSettings.value, API_TIMEOUT_MS: num }
    saveClaudeSettings()
  }
})
const effortLevel = computed({
  get: () => (claudeSettings.value.effortLevel as string) || '',
  set: (v: string) => {
    const { effortLevel: _, ...rest } = claudeSettings.value
    claudeSettings.value = v ? { ...rest, effortLevel: v } : rest
    saveClaudeSettings()
  }
})

const loadClaudeSettings = async () => {
  try {
    const home = await homeDir()
    const filePath = await join(home, '.claude', 'settings.json')
    const fileExists = await exists(filePath)
    if (fileExists) {
      const content = await readTextFile(filePath)
      claudeSettings.value = JSON.parse(content)
    }
    claudeSettingsLoaded.value = true
  } catch (e) {
    console.error('Failed to load Claude settings:', e)
  }
}

const saveClaudeSettings = async () => {
  if (claudeSettingsSaving.value) return
  claudeSettingsSaving.value = true
  try {
    const home = await homeDir()
    const filePath = await join(home, '.claude', 'settings.json')
    const claudeDir = await join(home, '.claude')
    if (!(await exists(claudeDir))) {
      await mkdir(claudeDir, { recursive: true })
    }
    await writeTextFile(filePath, JSON.stringify(claudeSettings.value, null, 2))
  } catch (e) {
    console.error('Failed to save Claude settings:', e)
  } finally {
    claudeSettingsSaving.value = false
  }
}

// Install mcd CLI
const mcdInstalling = ref(false)
const mcdError = ref('')
const mcdSuccess = ref(false)

const installMcdCli = async () => {
  mcdInstalling.value = true
  mcdError.value = ''
  mcdSuccess.value = false
  try {
    await invoke<string>('install_mcd_cli_cmd')
    mcdSuccess.value = true
  } catch (e) {
    mcdError.value = String(e)
  } finally {
    mcdInstalling.value = false
  }
}

// Wallpaper selection
const selectWallpaper = async () => {
  const selected = await open({
    multiple: false,
    filters: [{
      name: '图片',
      extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp']
    }],
    title: '选择桌面壁纸'
  })

  if (selected && typeof selected === 'string') {
    // Convert to asset URL for Tauri
    const assetUrl = convertFileSrc(selected)
    desktopStore.setWallpaper(`url('${assetUrl}')`)
  }
}

// Reset to default wallpaper
const resetWallpaper = () => {
  desktopStore.setWallpaper('url(/wallpaper.png)')
}

// Save appearance settings (terminal font size, window dimensions)
const saveAppearanceSettings = () => {
  settingsStore.save()
}

// Prompt style presets
const promptPresets = [
  {
    id: 'minimal' as const,
    name: '简约',
    preview: '~/project main ❯'
  },
  {
    id: 'powerline' as const,
    name: 'Powerline',
    preview: ' ~/project  main ❯'
  },
  {
    id: 'classic' as const,
    name: '经典',
    preview: 'PS ~/project (main) >'
  }
]

// Select prompt preset
const selectPromptPreset = (presetId: 'minimal' | 'powerline' | 'classic') => {
  settingsStore.promptConfig.style = presetId
  saveAppearanceSettings()
}

// Generate prompt preview HTML with ANSI colors
const promptPreviewHtml = computed(() => {
  const config = settingsStore.promptConfig
  if (!config.enabled) {
    return '<span style="color: #abb2bf;">PS C:\\Users\\User&gt; </span>'
  }

  const colorMap: Record<string, string> = {
    '33': '#e5c07b',  // 黄色 (git dirty)
    '34': '#61afef',  // 蓝色 (git clean)
    '35': '#c678dd',  // 洋红 (env)
    '39': '#56b6c2',  // 青色 (path)
    '90': '#5c6370'   // 灰色 (separator)
  }

  const pathColor = colorMap[config.colors.path] || '#56b6c2'
  const gitColor = colorMap[config.colors.gitClean] || '#61afef'
  const envColor = colorMap[config.colors.env] || '#c678dd'

  let html = ''
  const pathDisplay = '~/project'
  const gitBranch = 'main'
  const pyEnv = 'venv'

  if (config.style === 'powerline') {
    if (config.showPythonEnv) {
      html += `<span style="color: ${envColor};">  ${pyEnv} </span>`
    }
    html += `<span style="color: ${pathColor};">  ${pathDisplay} </span>`
    if (config.showGitBranch) {
      html += `<span style="color: ${gitColor};">  ${gitBranch} </span>`
    }
    html += '<span style="color: #abb2bf;"> ❯ </span>'
  } else if (config.style === 'classic') {
    html += '<span style="color: #abb2bf;">PS </span>'
    html += `<span style="color: ${pathColor};">${pathDisplay}</span>`
    if (config.showGitBranch) {
      html += `<span style="color: ${gitColor};"> (${gitBranch})</span>`
    }
    if (config.showPythonEnv) {
      html += `<span style="color: ${envColor};"> [${pyEnv}]</span>`
    }
    html += '<span style="color: #abb2bf;">&gt; </span>'
  } else {
    // minimal style
    if (config.showPythonEnv) {
      html += `<span style="color: ${envColor};">(${pyEnv})</span> `
    }
    html += `<span style="color: ${pathColor};">${pathDisplay}</span>`
    if (config.showGitBranch) {
      html += ` <span style="color: ${gitColor};">${gitBranch}</span>`
    }
    html += '<span style="color: #abb2bf;"> ❯ </span>'
  }

  return html
})

// Environment variables management
const newEnvKey = ref('')
const newEnvValue = ref('')

// Windows: sync env vars to system user environment variables
const isWindows = navigator.platform.startsWith('Win')
const syncingToSystem = ref(false)
const syncSuccess = ref(false)
const syncError = ref('')

const syncEnvVarsToSystem = async () => {
  syncingToSystem.value = true
  syncSuccess.value = false
  syncError.value = ''
  try {
    const vars: Record<string, string> = {}
    for (const [key, value] of Object.entries(settingsStore.environmentVariables)) {
      if (key && value) {
        vars[key] = value
      }
    }
    await invoke('sync_env_vars_to_windows_user', { vars })
    syncSuccess.value = true
    setTimeout(() => { syncSuccess.value = false }, 3000)
  } catch (e) {
    syncError.value = String(e)
  } finally {
    syncingToSystem.value = false
  }
}

// ANTHROPIC_MODEL preset options (editable)
const anthropicModelPresets = ref([
  { id: 'sonnet', value: 'Sonnet', label: 'Sonnet', desc: '默认推荐' },
  { id: 'opus', value: 'Opus', label: 'Opus', desc: '最强能力' },
])

// GEMINI_MODEL preset options
const geminiModelPresets = ref([
  { id: 'gemini-3-pro', value: 'gemini-3-pro-preview', label: 'Gemini 3 Pro Preview', desc: '最新模型' },
])

// OPENAI_MODEL preset options
const openaiModelPresets = ref([
  { id: 'gpt-5-2', value: 'gpt-5.2', label: 'GPT 5.2', desc: '最新模型' },
])

// Anthropic model selection state - default to 'custom'
const selectedModelId = ref<string>('custom')
// Load custom model value from persisted store
const customModelValue = ref(settingsStore.customAnthropicModel || '')

// Gemini model selection state
const selectedGeminiModelId = ref<string>('custom')
const customGeminiModelValue = ref(settingsStore.customGeminiModel || '')

// OpenAI model selection state
const selectedOpenAIModelId = ref<string>('custom')
const customOpenAIModelValue = ref(settingsStore.customOpenAIModel || '')

// Initialize model selection for all providers
const initModelSelection = () => {
  // Initialize Anthropic model selection
  const currentAnthropicModel = settingsStore.environmentVariables['ANTHROPIC_MODEL'] || ''
  if (settingsStore.customAnthropicModel) {
    customModelValue.value = settingsStore.customAnthropicModel
  }
  const matchedAnthropicPreset = anthropicModelPresets.value.find(m => m.value === currentAnthropicModel)
  if (matchedAnthropicPreset) {
    selectedModelId.value = matchedAnthropicPreset.id
  } else {
    selectedModelId.value = 'custom'
    if (!customModelValue.value && currentAnthropicModel) {
      customModelValue.value = currentAnthropicModel
      settingsStore.customAnthropicModel = currentAnthropicModel
      settingsStore.save()
    }
  }

  // Initialize Gemini model selection
  const currentGeminiModel = settingsStore.environmentVariables['GEMINI_MODEL'] || ''
  if (settingsStore.customGeminiModel) {
    customGeminiModelValue.value = settingsStore.customGeminiModel
  }
  const matchedGeminiPreset = geminiModelPresets.value.find(m => m.value === currentGeminiModel)
  if (matchedGeminiPreset) {
    selectedGeminiModelId.value = matchedGeminiPreset.id
  } else {
    selectedGeminiModelId.value = 'custom'
    if (!customGeminiModelValue.value && currentGeminiModel) {
      customGeminiModelValue.value = currentGeminiModel
      settingsStore.customGeminiModel = currentGeminiModel
      settingsStore.save()
    }
  }

  // Initialize OpenAI model selection
  const currentOpenAIModel = settingsStore.environmentVariables['OPENAI_MODEL'] || ''
  if (settingsStore.customOpenAIModel) {
    customOpenAIModelValue.value = settingsStore.customOpenAIModel
  }
  const matchedOpenAIPreset = openaiModelPresets.value.find(m => m.value === currentOpenAIModel)
  if (matchedOpenAIPreset) {
    selectedOpenAIModelId.value = matchedOpenAIPreset.id
  } else {
    selectedOpenAIModelId.value = 'custom'
    if (!customOpenAIModelValue.value && currentOpenAIModel) {
      customOpenAIModelValue.value = currentOpenAIModel
      settingsStore.customOpenAIModel = currentOpenAIModel
      settingsStore.save()
    }
  }
}

// Select a preset model
const selectPresetModel = (presetId: string) => {
  selectedModelId.value = presetId
  const preset = anthropicModelPresets.value.find(p => p.id === presetId)
  if (preset) {
    // Don't overwrite customModelValue - keep user's last custom input
    setEnvValue('ANTHROPIC_MODEL', preset.value)
  }
}

// Update preset model value (editable)
const updatePresetValue = (presetId: string, newValue: string) => {
  const preset = anthropicModelPresets.value.find(p => p.id === presetId)
  if (preset) {
    preset.value = newValue
    // If this preset is selected, update the env var
    if (selectedModelId.value === presetId) {
      // Don't overwrite customModelValue - keep user's last custom input
      setEnvValue('ANTHROPIC_MODEL', newValue)
    }
  }
}

// Switch to custom model mode
const switchToCustomModel = () => {
  selectedModelId.value = 'custom'
  if (customModelValue.value) {
    setEnvValue('ANTHROPIC_MODEL', customModelValue.value)
  }
}

// Update custom model value
const updateCustomModel = (value: string) => {
  customModelValue.value = value
  // Persist custom value to store
  settingsStore.customAnthropicModel = value
  settingsStore.save()
  if (selectedModelId.value === 'custom') {
    setEnvValue('ANTHROPIC_MODEL', value)
  }
}

// Gemini model selection functions
const selectGeminiPresetModel = (presetId: string) => {
  selectedGeminiModelId.value = presetId
  const preset = geminiModelPresets.value.find(p => p.id === presetId)
  if (preset) {
    setEnvValue('GEMINI_MODEL', preset.value)
  }
}

const updateGeminiPresetValue = (presetId: string, newValue: string) => {
  const preset = geminiModelPresets.value.find(p => p.id === presetId)
  if (preset) {
    preset.value = newValue
    if (selectedGeminiModelId.value === presetId) {
      setEnvValue('GEMINI_MODEL', newValue)
    }
  }
}

const switchToCustomGeminiModel = () => {
  selectedGeminiModelId.value = 'custom'
  if (customGeminiModelValue.value) {
    setEnvValue('GEMINI_MODEL', customGeminiModelValue.value)
  }
}

const updateCustomGeminiModel = (value: string) => {
  customGeminiModelValue.value = value
  settingsStore.customGeminiModel = value
  settingsStore.save()
  if (selectedGeminiModelId.value === 'custom') {
    setEnvValue('GEMINI_MODEL', value)
  }
}

// OpenAI model selection functions
const selectOpenAIPresetModel = (presetId: string) => {
  selectedOpenAIModelId.value = presetId
  const preset = openaiModelPresets.value.find(p => p.id === presetId)
  if (preset) {
    setEnvValue('OPENAI_MODEL', preset.value)
  }
}

const updateOpenAIPresetValue = (presetId: string, newValue: string) => {
  const preset = openaiModelPresets.value.find(p => p.id === presetId)
  if (preset) {
    preset.value = newValue
    if (selectedOpenAIModelId.value === presetId) {
      setEnvValue('OPENAI_MODEL', newValue)
    }
  }
}

const switchToCustomOpenAIModel = () => {
  selectedOpenAIModelId.value = 'custom'
  if (customOpenAIModelValue.value) {
    setEnvValue('OPENAI_MODEL', customOpenAIModelValue.value)
  }
}

const updateCustomOpenAIModel = (value: string) => {
  customOpenAIModelValue.value = value
  settingsStore.customOpenAIModel = value
  settingsStore.save()
  if (selectedOpenAIModelId.value === 'custom') {
    setEnvValue('OPENAI_MODEL', value)
  }
}

// Predefined environment variable keys
const claudeEnvKeys = ['ANTHROPIC_AUTH_TOKEN', 'ANTHROPIC_MODEL', 'ANTHROPIC_BASE_URL']
const geminiEnvKeys = ['GOOGLE_GEMINI_BASE_URL', 'GEMINI_API_KEY', 'GEMINI_MODEL']
const openaiEnvKeys = ['OPENAI_API_KEY', 'OPENAI_MODEL']
const allPredefinedKeys = [...claudeEnvKeys, ...geminiEnvKeys, ...openaiEnvKeys]

// Get value for predefined env var
const getEnvValue = (key: string) => {
  return settingsStore.environmentVariables[key] || ''
}

// Set value for predefined env var
const setEnvValue = (key: string, value: string) => {
  if (value.trim()) {
    settingsStore.setEnvironmentVariable(key, value)
  } else {
    settingsStore.removeEnvironmentVariable(key)
  }
}

const addEnvironmentVariable = () => {
  if (newEnvKey.value && newEnvValue.value) {
    settingsStore.setEnvironmentVariable(newEnvKey.value, newEnvValue.value)
    newEnvKey.value = ''
    newEnvValue.value = ''
  }
}

const removeEnvironmentVariable = async (key: string) => {
  const confirmed = await ask(`确定删除环境变量 "${key}"？`, {
    title: '删除确认',
    kind: 'warning',
  })
  if (confirmed) {
    settingsStore.removeEnvironmentVariable(key)
  }
}

// Keybindings
const keybindingActions = [
  { key: 'Double Click', action: '打开图标/恢复窗口' },
  { key: 'Right Click', action: '打开右键菜单' },
  { key: 'Drag', action: '移动图标/窗口' },
  { key: 'Ctrl + C', action: '复制（终端选中文本）' },
  { key: 'Ctrl + V', action: '粘贴（终端）' },
  { key: 'Ctrl + Shift + C', action: '复制（备选）' },
  { key: 'Ctrl + Shift + V', action: '粘贴（备选）' }
]

// Custom env variables (excluding predefined ones)
const customEnvVariables = computed(() => {
  return Object.entries(settingsStore.environmentVariables)
    .filter(([key]) => !allPredefinedKeys.includes(key))
})

// Lazy load: only load status when switching to respective tabs
watch(activeTab, (newTab) => {
  if (newTab === 'appearance' && !fontsLoaded.value) {
    loadSystemFonts()
  }
  if (newTab === 'nodejs' && !nodejsStatusLoaded.value) {
    loadNodejsStatus()
  }
  if (newTab === 'claudecode' && !claudeCodeStatusLoaded.value) {
    loadClaudeCodeStatus()
  }
  if (newTab === 'claudecode' && !claudeSettingsLoaded.value) {
    loadClaudeSettings()
  }
  if (newTab === 'env') {
    initModelSelection()
  }
  if (newTab === 'about') {
    loadDefaultUpdateUrl()
    checkForUpdateFn({ silent: true })
  }
}, { immediate: true })

</script>

<template>
  <div class="settings">
    <!-- Sidebar -->
    <div class="settings-sidebar">
      <div
        v-for="tab in tabs"
        :key="tab.id"
        class="sidebar-item"
        :class="{ active: activeTab === tab.id }"
        @click="activeTab = tab.id"
      >
        <i :class="tab.icon" />
        <span>{{ tab.label }}</span>
      </div>
    </div>

    <!-- Content -->
    <div class="settings-content">
      <!-- Appearance Settings -->
      <div
        v-if="activeTab === 'appearance'"
        class="settings-panel"
      >
        <h3>外观设置</h3>

        <div class="setting-group">
          <label>桌面壁纸</label>
          <div class="wallpaper-preview-container">
            <div
              class="wallpaper-preview"
              :style="{
                background: desktopStore.wallpaper,
                backgroundSize: '100% 100%'
              }"
            />
          </div>
          <div class="wallpaper-actions">
            <button
              class="wallpaper-btn"
              @click="selectWallpaper"
            >
              <i class="fa-solid fa-image" />
              选择图片
            </button>
            <button
              class="wallpaper-btn secondary"
              @click="resetWallpaper"
            >
              <i class="fa-solid fa-rotate-left" />
              恢复默认
            </button>
          </div>
          <span class="setting-hint">选择图片文件作为桌面背景，图片将拉伸显示</span>
        </div>

        <div class="setting-group">
          <label>终端字体大小</label>
          <div class="input-with-unit">
            <input
              v-model.number="settingsStore.terminalFontSize"
              type="number"
              min="10"
              max="24"
              @change="saveAppearanceSettings"
            >
            <span class="unit">px</span>
          </div>
          <span class="setting-hint">设置终端窗口的字体大小（10-24px）</span>
        </div>

        <div class="setting-group">
          <label>终端字体</label>
          <div
            v-if="fontsLoading"
            class="font-loading"
          >
            <i class="fa-solid fa-spinner fa-spin" />
            <span>检测系统字体...</span>
          </div>
          <select
            v-else
            v-model="settingsStore.terminalFontFamily"
            class="font-selector"
            @change="saveAppearanceSettings"
          >
            <!-- 推荐英文字体 -->
            <optgroup
              v-if="hasCategory('recommended-en')"
              label="推荐英文字体"
            >
              <option
                v-for="font in fontsByCategory('recommended-en')"
                :key="font.value"
                :value="font.value"
              >
                {{ font.label }}
              </option>
            </optgroup>

            <!-- 推荐中文字体 -->
            <optgroup
              v-if="hasCategory('recommended-cn')"
              label="推荐中文字体"
            >
              <option
                v-for="font in fontsByCategory('recommended-cn')"
                :key="font.value"
                :value="font.value"
              >
                {{ font.label }}
              </option>
            </optgroup>

            <!-- 其他系统字体 -->
            <optgroup
              v-if="hasCategory('system')"
              label="其他等宽字体"
            >
              <option
                v-for="font in fontsByCategory('system')"
                :key="font.value"
                :value="font.value"
              >
                {{ font.label }}
              </option>
            </optgroup>

            <!-- 默认 -->
            <optgroup label="默认">
              <option
                v-for="font in fontsByCategory('default')"
                :key="font.value"
                :value="font.value"
              >
                {{ font.label }}
              </option>
            </optgroup>
          </select>
          <span class="setting-hint">选择终端显示字体，优先显示系统已安装的推荐字体</span>
        </div>

        <div class="setting-group">
          <label>窗口默认宽度</label>
          <div class="input-with-unit">
            <input
              v-model.number="settingsStore.defaultWindowWidth"
              type="number"
              min="600"
              max="1600"
              step="50"
              @change="saveAppearanceSettings"
            >
            <span class="unit">px</span>
          </div>
          <span class="setting-hint">新窗口的默认宽度（600-1600px）</span>
        </div>

        <div class="setting-group">
          <label>窗口默认高度</label>
          <div class="input-with-unit">
            <input
              v-model.number="settingsStore.defaultWindowHeight"
              type="number"
              min="400"
              max="1200"
              step="50"
              @change="saveAppearanceSettings"
            >
            <span class="unit">px</span>
          </div>
          <span class="setting-hint">新窗口的默认高度（400-1200px）</span>
        </div>

        <!-- 终端提示符设置 -->
        <div class="setting-group prompt-settings">
          <label>终端提示符样式</label>

          <!-- 启用开关 -->
          <div class="prompt-toggle">
            <label class="toggle-label">
              <input
                v-model="settingsStore.promptConfig.enabled"
                type="checkbox"
                @change="saveAppearanceSettings"
              >
              <span>启用自定义提示符</span>
            </label>
          </div>

          <!-- 样式预设 -->
          <div
            v-if="settingsStore.promptConfig.enabled"
            class="prompt-presets"
          >
            <div
              v-for="preset in promptPresets"
              :key="preset.id"
              class="preset-option"
              :class="{ active: settingsStore.promptConfig.style === preset.id }"
              @click="selectPromptPreset(preset.id)"
            >
              <div class="preset-preview">
                {{ preset.preview }}
              </div>
              <span class="preset-name">{{ preset.name }}</span>
            </div>
          </div>

          <!-- 显示选项 -->
          <div
            v-if="settingsStore.promptConfig.enabled"
            class="prompt-options"
          >
            <label class="option-label">
              <input
                v-model="settingsStore.promptConfig.showGitBranch"
                type="checkbox"
                @change="saveAppearanceSettings"
              >
              <span>显示 Git 分支</span>
            </label>
            <label class="option-label">
              <input
                v-model="settingsStore.promptConfig.showGitStatus"
                type="checkbox"
                @change="saveAppearanceSettings"
              >
              <span>显示 Git 状态</span>
            </label>
            <label class="option-label">
              <input
                v-model="settingsStore.promptConfig.showPythonEnv"
                type="checkbox"
                @change="saveAppearanceSettings"
              >
              <span>显示 Python 环境</span>
            </label>
            <label class="option-label">
              <input
                v-model="settingsStore.promptConfig.showNodeEnv"
                type="checkbox"
                @change="saveAppearanceSettings"
              >
              <span>显示 Node 版本</span>
            </label>
          </div>

          <!-- 路径显示模式 -->
          <div
            v-if="settingsStore.promptConfig.enabled"
            class="path-mode"
          >
            <label>路径显示</label>
            <select
              v-model="settingsStore.promptConfig.showPath"
              @change="saveAppearanceSettings"
            >
              <option value="full">
                完整路径
              </option>
              <option value="short">
                缩短路径 (~)
              </option>
              <option value="basename">
                仅文件夹名
              </option>
            </select>
          </div>

          <!-- 实时预览 -->
          <div
            v-if="settingsStore.promptConfig.enabled"
            class="prompt-preview-box"
          >
            <label>预览效果</label>
            <div
              class="preview-terminal"
              v-html="promptPreviewHtml"
            />
          </div>

          <span class="setting-hint">自定义终端提示符样式，类似 oh-my-posh 效果</span>
        </div>
      </div>

      <!-- Node.js Management -->
      <div
        v-if="activeTab === 'nodejs'"
        class="settings-panel nodejs-panel"
      >
        <h3>Node.js 环境</h3>

        <!-- Loading State -->
        <div
          v-if="nodejsLoading"
          class="nodejs-loading"
        >
          <i class="fa-solid fa-spinner fa-spin" />
          <span>加载中...</span>
        </div>

        <!-- Error Message -->
        <div
          v-if="nodejsError"
          class="nodejs-error"
        >
          <i class="fa-solid fa-exclamation-triangle" />
          <span>{{ nodejsError }}</span>
        </div>

        <!-- Node.js Section -->
        <div
          v-if="!nodejsLoading"
          class="nodejs-section"
        >
          <div class="nodejs-section-header">
            <i class="fa-brands fa-node-js" />
            <span>Node.js 运行环境</span>
          </div>
          <div class="nodejs-section-content">
            <div class="status-row">
              <span class="status-label">状态</span>
              <span
                v-if="nodejsStatus.installed"
                class="status-value installed"
              >
                <i class="fa-solid fa-check-circle" /> 已安装
                <span
                  v-if="nodejsStatus.source === 'system'"
                  class="source-badge system"
                >系统</span>
                <span
                  v-else-if="nodejsStatus.source === 'bundled'"
                  class="source-badge bundled"
                >内嵌</span>
              </span>
              <span
                v-else
                class="status-value not-installed"
              >
                <i class="fa-solid fa-times-circle" /> 未安装
              </span>
            </div>
            <div
              v-if="nodejsStatus.installed"
              class="status-row"
            >
              <span class="status-label">版本</span>
              <span class="status-value">v{{ nodejsStatus.version }}</span>
            </div>
            <div
              v-if="nodejsStatus.installed"
              class="status-row"
            >
              <span class="status-label">路径</span>
              <span class="status-value path">{{ nodejsStatus.path }}</span>
            </div>
            <div class="nodejs-actions">
              <!-- Show install button only when no Node.js found -->
              <button
                v-if="!nodejsStatus.installed"
                class="action-btn primary"
                :disabled="nodejsLoading"
                @click="extractNodejs"
              >
                <i class="fa-solid fa-download" />
                安装内嵌 Node.js
              </button>
              <!-- Show reinstall button only for bundled Node.js -->
              <button
                v-else-if="nodejsStatus.source === 'bundled'"
                class="action-btn secondary"
                :disabled="nodejsLoading"
                @click="extractNodejs"
              >
                <i class="fa-solid fa-rotate" />
                重新安装
              </button>
              <button
                class="action-btn secondary"
                :disabled="nodejsLoading"
                @click="loadNodejsStatus"
              >
                <i class="fa-solid fa-refresh" />
                刷新状态
              </button>
            </div>
          </div>
        </div>

        <!-- Info Note for system Node.js -->
        <p
          v-if="!nodejsLoading && nodejsStatus.installed && nodejsStatus.source === 'system'"
          class="settings-note success"
        >
          <i class="fa-solid fa-check-circle" />
          检测到系统已安装 Node.js，无需额外安装
        </p>

        <!-- Info Note for no Node.js -->
        <p
          v-if="!nodejsLoading && !nodejsStatus.installed"
          class="settings-note"
        >
          <i class="fa-solid fa-info-circle" />
          点击"安装内嵌 Node.js"释放轻量化 Node.js 运行环境
        </p>
      </div>

      <!-- Claude Code Management -->
      <div
        v-if="activeTab === 'claudecode'"
        class="settings-panel nodejs-panel"
      >
        <h3>Claude Code CLI</h3>

        <!-- Loading State -->
        <div
          v-if="claudeCodeLoading"
          class="nodejs-loading"
        >
          <i class="fa-solid fa-spinner fa-spin" />
          <span>加载中...</span>
        </div>

        <!-- Error Message -->
        <div
          v-if="claudeCodeError"
          class="nodejs-error"
        >
          <i class="fa-solid fa-exclamation-triangle" />
          <span>{{ claudeCodeError }}</span>
        </div>

        <!-- Node.js Not Installed Warning -->
        <div
          v-if="!claudeCodeLoading && !nodejsStatus.installed"
          class="nodejs-section"
        >
          <div class="nodejs-section-header">
            <i class="fa-solid fa-exclamation-triangle" />
            <span>需要先安装 Node.js</span>
          </div>
          <div class="nodejs-section-content">
            <p
              class="section-hint"
              style="margin-top: 0;"
            >
              <i class="fa-solid fa-info-circle" />
              Claude Code 依赖 Node.js 运行环境，请先前往 Node.js 标签页安装 Node.js
            </p>
            <div class="nodejs-actions">
              <button
                class="action-btn primary"
                @click="activeTab = 'nodejs'"
              >
                <i class="fa-brands fa-node-js" />
                前往安装 Node.js
              </button>
            </div>
          </div>
        </div>

        <!-- Claude Code Section -->
        <div
          v-if="!claudeCodeLoading && nodejsStatus.installed"
          class="nodejs-section"
        >
          <div class="nodejs-section-header">
            <i class="fa-solid fa-robot" />
            <span>安装状态</span>
          </div>
          <div class="nodejs-section-content">
            <div class="status-row">
              <span class="status-label">状态</span>
              <span
                v-if="claudeCodeStatus.installed"
                class="status-value installed"
              >
                <i class="fa-solid fa-check-circle" /> 已安装
              </span>
              <span
                v-else
                class="status-value not-installed"
              >
                <i class="fa-solid fa-times-circle" /> 未安装
              </span>
            </div>
            <div
              v-if="claudeCodeStatus.installed && claudeCodeStatus.version"
              class="status-row"
            >
              <span class="status-label">版本</span>
              <span class="status-value">{{ claudeCodeStatus.version }}</span>
            </div>
            <div class="nodejs-actions">
              <button
                v-if="!claudeCodeStatus.installed"
                class="action-btn primary"
                :disabled="claudeCodeInstalling"
                @click="installClaudeCode"
              >
                <i
                  v-if="claudeCodeInstalling"
                  class="fa-solid fa-spinner fa-spin"
                />
                <i
                  v-else
                  class="fa-solid fa-download"
                />
                {{ claudeCodeInstalling ? '安装中...' : '安装 Claude Code' }}
              </button>
              <button
                v-else
                class="action-btn secondary"
                :disabled="claudeCodeInstalling"
                @click="installClaudeCode"
              >
                <i
                  v-if="claudeCodeInstalling"
                  class="fa-solid fa-spinner fa-spin"
                />
                <i
                  v-else
                  class="fa-solid fa-rotate"
                />
                {{ claudeCodeInstalling ? '更新中...' : '更新 Claude Code' }}
              </button>
              <button
                class="action-btn secondary"
                :disabled="claudeCodeLoading"
                @click="loadClaudeCodeStatus"
              >
                <i class="fa-solid fa-refresh" />
                刷新状态
              </button>
            </div>
          </div>
        </div>

        <!-- Claude Code Settings -->
        <div
          v-if="!claudeCodeLoading && nodejsStatus.installed && claudeCodeStatus.installed && claudeSettingsLoaded"
          class="nodejs-section"
        >
          <div class="nodejs-section-header">
            <i class="fa-solid fa-sliders" />
            <span>Claude Code 设置</span>
          </div>
          <div class="nodejs-section-content">
            <p
              class="section-hint"
              style="margin-top: 0;"
            >
              <i class="fa-solid fa-info-circle" />
              修改 <code>~/.claude/settings.json</code>，更改后立即生效于下次 Claude Code 会话。
            </p>

            <!-- Model -->
            <div class="cc-setting-row">
              <div class="cc-setting-info">
                <span class="cc-setting-label">默认模型</span>
                <span class="cc-setting-desc">Claude Code 使用的默认模型</span>
              </div>
              <select
                v-model="claudeModel"
                class="cc-setting-select"
              >
                <option value="">
                  默认
                </option>
                <option value="opus">
                  Opus
                </option>
                <option value="sonnet">
                  Sonnet
                </option>
                <option value="haiku">
                  Haiku
                </option>
              </select>
            </div>

            <!-- Language -->
            <div class="cc-setting-row">
              <div class="cc-setting-info">
                <span class="cc-setting-label">回复语言</span>
                <span class="cc-setting-desc">Claude 回复使用的语言</span>
              </div>
              <input
                v-model.lazy="claudeLanguage"
                class="cc-setting-input"
                placeholder="例如：中文"
              >
            </div>

            <!-- Always Thinking -->
            <div class="cc-setting-row">
              <div class="cc-setting-info">
                <span class="cc-setting-label">扩展思考</span>
                <span class="cc-setting-desc">始终启用 Extended Thinking 模式</span>
              </div>
              <label class="cc-toggle-switch">
                <input
                  v-model="alwaysThinking"
                  type="checkbox"
                >
                <span class="cc-toggle-slider" />
              </label>
            </div>

            <!-- Skip Dangerous Prompt -->
            <div class="cc-setting-row">
              <div class="cc-setting-info">
                <span class="cc-setting-label">跳过危险确认</span>
                <span class="cc-setting-desc">跳过 dangerous 模式的权限确认弹窗</span>
              </div>
              <label class="cc-toggle-switch">
                <input
                  v-model="skipDangerousPrompt"
                  type="checkbox"
                >
                <span class="cc-toggle-slider" />
              </label>
            </div>

            <!-- Prompt Suggestion -->
            <div class="cc-setting-row">
              <div class="cc-setting-info">
                <span class="cc-setting-label">提示建议</span>
                <span class="cc-setting-desc">显示输入提示建议</span>
              </div>
              <label class="cc-toggle-switch">
                <input
                  v-model="promptSuggestion"
                  type="checkbox"
                >
                <span class="cc-toggle-slider" />
              </label>
            </div>

            <!-- Permissions Default Mode -->
            <div class="cc-setting-row">
              <div class="cc-setting-info">
                <span class="cc-setting-label">默认权限模式</span>
                <span class="cc-setting-desc">Claude Code 会话的默认权限模式</span>
              </div>
              <select
                v-model="permissionsDefaultMode"
                class="cc-setting-select"
              >
                <option value="default">
                  Default
                </option>
                <option value="acceptEdits">
                  Accept Edits
                </option>
                <option value="bypassPermissions">
                  Bypass Permissions
                </option>
                <option value="dontAsk">
                  Don't Ask
                </option>
                <option value="plan">
                  Plan
                </option>
              </select>
            </div>

            <!-- Disable Experimental Betas -->
            <div class="cc-setting-row">
              <div class="cc-setting-info">
                <span class="cc-setting-label">禁用实验性 Beta</span>
                <span class="cc-setting-desc">禁用 Claude Code 的实验性 Beta 功能</span>
              </div>
              <label class="cc-toggle-switch">
                <input
                  v-model="disableExperimentalBetas"
                  type="checkbox"
                >
                <span class="cc-toggle-slider" />
              </label>
            </div>

            <!-- Max Consecutive Autocompact Failures -->
            <div class="cc-setting-row">
              <div class="cc-setting-info">
                <span class="cc-setting-label">自动压缩最大失败次数</span>
                <span class="cc-setting-desc">连续自动压缩失败多少次后停止（留空使用默认值）</span>
              </div>
              <input
                v-model.lazy.number="maxAutocompactFailures"
                class="cc-setting-input"
                type="number"
                min="0"
                placeholder="默认"
                style="width: 80px"
              >
            </div>

            <!-- API Timeout MS -->
            <div class="cc-setting-row">
              <div class="cc-setting-info">
                <span class="cc-setting-label">API 超时时间 (ms)</span>
                <span class="cc-setting-desc">Claude Code API 请求超时时间，单位毫秒（留空使用默认值）</span>
              </div>
              <input
                v-model.lazy.number="apiTimeoutMs"
                class="cc-setting-input"
                type="number"
                min="0"
                placeholder="默认"
                style="width: 100px"
              >
            </div>

            <!-- Effort Level -->
            <div class="cc-setting-row">
              <div class="cc-setting-info">
                <span class="cc-setting-label">思考努力程度</span>
                <span class="cc-setting-desc">控制模型的思考深度和 token 消耗</span>
              </div>
              <select
                v-model="effortLevel"
                class="cc-setting-select"
              >
                <option value="">
                  默认
                </option>
                <option value="low">
                  Low（低）
                </option>
                <option value="medium">
                  Medium（中）
                </option>
                <option value="high">
                  High（高）
                </option>
              </select>
            </div>
          </div>
        </div>

        <!-- mcd CLI -->
        <div
          v-if="!claudeCodeLoading && nodejsStatus.installed && claudeCodeStatus.installed"
          class="nodejs-section"
        >
          <div class="nodejs-section-header">
            <i class="fa-solid fa-route" />
            <span>mcd CLI (LLM 路由)</span>
          </div>
          <div class="nodejs-section-content">
            <p
              class="section-hint"
              style="margin-top: 0;"
            >
              mcd 命令用于通过 LLM Router 调用 Claude，自动注入环境变量和启用路由功能。
            </p>
            <div class="nodejs-actions">
              <button
                class="action-btn primary"
                :disabled="mcdInstalling"
                @click="installMcdCli"
              >
                <i
                  v-if="mcdInstalling"
                  class="fa-solid fa-spinner fa-spin"
                />
                <i
                  v-else
                  class="fa-solid fa-download"
                />
                {{ mcdInstalling ? '安装中...' : '安装 mcd CLI' }}
              </button>
            </div>
            <div
              v-if="mcdError"
              class="error-message"
            >
              <i class="fa-solid fa-exclamation-circle" />
              {{ mcdError }}
            </div>
            <div
              v-if="mcdSuccess"
              class="success-message"
            >
              <i class="fa-solid fa-check-circle" />
              mcd CLI 安装成功！请打开新终端使用 <code>mcd</code> 命令。
            </div>
          </div>
        </div>

        <!-- Usage Info -->
        <div
          v-if="!claudeCodeLoading && nodejsStatus.installed && claudeCodeStatus.installed"
          class="nodejs-section"
        >
          <div class="nodejs-section-header">
            <i class="fa-solid fa-terminal" />
            <span>使用说明</span>
          </div>
          <div class="nodejs-section-content">
            <p
              class="section-hint"
              style="margin-top: 0;"
            >
              安装完成后，在终端中可以使用以下命令：
            </p>
            <div class="command-list">
              <div class="command-item">
                <code>claude</code>
                <span>启动 Claude Code</span>
              </div>
              <div class="command-item">
                <code>claude -c</code>
                <span>继续上次对话</span>
              </div>
              <div class="command-item">
                <code>claude /init</code>
                <span>初始化项目</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Coworks Model Settings -->
      <div
        v-if="activeTab === 'coworks'"
        class="settings-panel"
      >
        <CoworksModelSettings />
      </div>

      <!-- Environment Variables -->
      <div
        v-if="activeTab === 'env'"
        class="settings-panel env-panel"
      >
        <h3>环境变量</h3>

        <!-- Windows: 一键同步到系统用户环境变量 -->
        <div
          v-if="isWindows"
          class="env-sync-section"
        >
          <div class="env-sync-info">
            <i class="fa-brands fa-windows" />
            <span>将当前配置的环境变量写入 Windows 用户环境变量，使外部程序（如系统终端）也能读取。</span>
          </div>
          <div class="env-sync-actions">
            <button
              class="action-btn primary"
              :disabled="syncingToSystem"
              @click="syncEnvVarsToSystem"
            >
              <i
                v-if="syncingToSystem"
                class="fa-solid fa-spinner fa-spin"
              />
              <i
                v-else
                class="fa-solid fa-upload"
              />
              {{ syncingToSystem ? '同步中...' : '同步到系统环境变量' }}
            </button>
            <span
              v-if="syncSuccess"
              class="sync-ok"
            >
              <i class="fa-solid fa-check" /> 同步成功
            </span>
            <span
              v-if="syncError"
              class="sync-err"
            >
              <i class="fa-solid fa-triangle-exclamation" /> {{ syncError }}
            </span>
          </div>
        </div>

        <!-- Claude Environment Settings -->
        <div class="env-section">
          <div class="env-section-header">
            <i class="fa-solid fa-robot" />
            <span>Claude 环境设置</span>
          </div>
          <div class="env-section-content">
            <!-- ANTHROPIC_MODEL - Radio selector at the top -->
            <div class="env-field model-selector">
              <label>ANTHROPIC_MODEL</label>
              <div class="model-options">
                <!-- Custom option first -->
                <div
                  class="model-option custom"
                  :class="{ active: selectedModelId === 'custom' }"
                  @click="switchToCustomModel"
                >
                  <div class="radio-circle">
                    <div
                      v-if="selectedModelId === 'custom'"
                      class="radio-dot"
                    />
                  </div>
                  <span class="option-label">自定义</span>
                  <input
                    v-model="customModelValue"
                    type="text"
                    placeholder="输入模型名称"
                    @input="updateCustomModel(customModelValue)"
                    @focus="switchToCustomModel"
                  >
                </div>

                <!-- Preset options (editable) -->
                <div
                  v-for="preset in anthropicModelPresets"
                  :key="preset.id"
                  class="model-option preset"
                  :class="{ active: selectedModelId === preset.id }"
                  @click="selectPresetModel(preset.id)"
                >
                  <div class="radio-circle">
                    <div
                      v-if="selectedModelId === preset.id"
                      class="radio-dot"
                    />
                  </div>
                  <div class="option-content">
                    <span class="option-label">{{ preset.label }}</span>
                    <span class="option-desc">{{ preset.desc }}</span>
                  </div>
                  <input
                    type="text"
                    :value="preset.value"
                    @input="(e) => updatePresetValue(preset.id, (e.target as HTMLInputElement).value)"
                    @focus="selectPresetModel(preset.id)"
                    @click.stop
                  >
                </div>
              </div>
            </div>

            <div class="env-field">
              <label>ANTHROPIC_AUTH_TOKEN</label>
              <input
                type="password"
                :value="getEnvValue('ANTHROPIC_AUTH_TOKEN')"
                placeholder="sk-ant-xxxxx"
                @change="(e) => setEnvValue('ANTHROPIC_AUTH_TOKEN', (e.target as HTMLInputElement).value)"
              >
            </div>
            <div class="env-field">
              <label>ANTHROPIC_BASE_URL</label>
              <input
                type="text"
                :value="getEnvValue('ANTHROPIC_BASE_URL')"
                placeholder="https://api.anthropic.com"
                @change="(e) => setEnvValue('ANTHROPIC_BASE_URL', (e.target as HTMLInputElement).value)"
              >
            </div>
          </div>
        </div>

        <!-- Gemini Environment Settings -->
        <div class="env-section">
          <div class="env-section-header">
            <i class="fa-solid fa-gem" />
            <span>Gemini 环境设置</span>
          </div>
          <div class="env-section-content">
            <!-- GEMINI_MODEL selector -->
            <div class="env-field model-selector">
              <label>GEMINI_MODEL</label>
              <div class="model-options">
                <!-- Custom option first -->
                <div
                  class="model-option custom"
                  :class="{ active: selectedGeminiModelId === 'custom' }"
                  @click="switchToCustomGeminiModel"
                >
                  <div class="radio-circle">
                    <div
                      v-if="selectedGeminiModelId === 'custom'"
                      class="radio-dot"
                    />
                  </div>
                  <span class="option-label">自定义</span>
                  <input
                    v-model="customGeminiModelValue"
                    type="text"
                    placeholder="输入模型名称"
                    @input="updateCustomGeminiModel(customGeminiModelValue)"
                    @focus="switchToCustomGeminiModel"
                  >
                </div>

                <!-- Preset options (editable) -->
                <div
                  v-for="preset in geminiModelPresets"
                  :key="preset.id"
                  class="model-option preset"
                  :class="{ active: selectedGeminiModelId === preset.id }"
                  @click="selectGeminiPresetModel(preset.id)"
                >
                  <div class="radio-circle">
                    <div
                      v-if="selectedGeminiModelId === preset.id"
                      class="radio-dot"
                    />
                  </div>
                  <div class="option-content">
                    <span class="option-label">{{ preset.label }}</span>
                    <span class="option-desc">{{ preset.desc }}</span>
                  </div>
                  <input
                    type="text"
                    :value="preset.value"
                    @input="(e) => updateGeminiPresetValue(preset.id, (e.target as HTMLInputElement).value)"
                    @focus="selectGeminiPresetModel(preset.id)"
                    @click.stop
                  >
                </div>
              </div>
            </div>

            <div class="env-field">
              <label>GOOGLE_GEMINI_BASE_URL</label>
              <input
                type="text"
                :value="getEnvValue('GOOGLE_GEMINI_BASE_URL')"
                placeholder="https://generativelanguage.googleapis.com"
                @change="(e) => setEnvValue('GOOGLE_GEMINI_BASE_URL', (e.target as HTMLInputElement).value)"
              >
            </div>
            <div class="env-field">
              <label>GEMINI_API_KEY</label>
              <input
                type="password"
                :value="getEnvValue('GEMINI_API_KEY')"
                placeholder="AIzaSyxxxxx"
                @change="(e) => setEnvValue('GEMINI_API_KEY', (e.target as HTMLInputElement).value)"
              >
            </div>
          </div>
        </div>

        <!-- OpenAI Environment Settings -->
        <div class="env-section">
          <div class="env-section-header">
            <i class="fa-solid fa-brain" />
            <span>OpenAI 环境设置</span>
          </div>
          <div class="env-section-content">
            <!-- OPENAI_MODEL selector -->
            <div class="env-field model-selector">
              <label>OPENAI_MODEL</label>
              <div class="model-options">
                <!-- Custom option first -->
                <div
                  class="model-option custom"
                  :class="{ active: selectedOpenAIModelId === 'custom' }"
                  @click="switchToCustomOpenAIModel"
                >
                  <div class="radio-circle">
                    <div
                      v-if="selectedOpenAIModelId === 'custom'"
                      class="radio-dot"
                    />
                  </div>
                  <span class="option-label">自定义</span>
                  <input
                    v-model="customOpenAIModelValue"
                    type="text"
                    placeholder="输入模型名称"
                    @input="updateCustomOpenAIModel(customOpenAIModelValue)"
                    @focus="switchToCustomOpenAIModel"
                  >
                </div>

                <!-- Preset options (editable) -->
                <div
                  v-for="preset in openaiModelPresets"
                  :key="preset.id"
                  class="model-option preset"
                  :class="{ active: selectedOpenAIModelId === preset.id }"
                  @click="selectOpenAIPresetModel(preset.id)"
                >
                  <div class="radio-circle">
                    <div
                      v-if="selectedOpenAIModelId === preset.id"
                      class="radio-dot"
                    />
                  </div>
                  <div class="option-content">
                    <span class="option-label">{{ preset.label }}</span>
                    <span class="option-desc">{{ preset.desc }}</span>
                  </div>
                  <input
                    type="text"
                    :value="preset.value"
                    @input="(e) => updateOpenAIPresetValue(preset.id, (e.target as HTMLInputElement).value)"
                    @focus="selectOpenAIPresetModel(preset.id)"
                    @click.stop
                  >
                </div>
              </div>
            </div>

            <div class="env-field">
              <label>OPENAI_API_KEY</label>
              <input
                type="password"
                :value="getEnvValue('OPENAI_API_KEY')"
                placeholder="sk-xxxxx"
                @change="(e) => setEnvValue('OPENAI_API_KEY', (e.target as HTMLInputElement).value)"
              >
            </div>
          </div>
        </div>

        <!-- Custom Environment Variables -->
        <div class="env-section">
          <div class="env-section-header">
            <i class="fa-solid fa-sliders" />
            <span>自定义环境变量</span>
          </div>
          <div class="env-section-content">
            <div class="env-add-form">
              <input
                v-model="newEnvKey"
                type="text"
                placeholder="变量名"
                @keyup.enter="addEnvironmentVariable"
              >
              <input
                v-model="newEnvValue"
                type="text"
                placeholder="变量值"
                @keyup.enter="addEnvironmentVariable"
              >
              <button @click="addEnvironmentVariable">
                <i class="fa-solid fa-plus" />
                添加
              </button>
            </div>

            <div class="env-list">
              <div
                v-for="[key, value] in customEnvVariables"
                :key="key"
                class="env-item"
              >
                <span class="env-key">{{ key }}</span>
                <span class="env-value">{{ value }}</span>
                <button
                  class="delete-btn"
                  @click="removeEnvironmentVariable(key)"
                >
                  <i class="fa-solid fa-trash" />
                </button>
              </div>
              <div
                v-if="customEnvVariables.length === 0"
                class="env-empty"
              >
                暂无自定义环境变量
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Keybindings -->
      <div
        v-if="activeTab === 'keybindings'"
        class="settings-panel"
      >
        <h3>快捷键设置</h3>

        <div class="keybindings-list">
          <div
            v-for="binding in keybindingActions"
            :key="binding.key"
            class="keybinding-item"
          >
            <span class="keybinding-action">{{ binding.action }}</span>
            <span class="keybinding-key">{{ binding.key }}</span>
          </div>
        </div>

        <p class="settings-note">
          <i class="fa-solid fa-info-circle" />
          快捷键目前为只读，后续版本将支持自定义
        </p>
      </div>

      <!-- About -->
      <div
        v-if="activeTab === 'about'"
        class="settings-panel nodejs-panel"
      >
        <h3>关于</h3>

        <!-- App Info -->
        <div class="nodejs-section">
          <div class="nodejs-section-header">
            <i
              class="fa-solid fa-circle-info"
              style="color: #7c6eb8;"
            />
            <span>Moonton Claude Desktop</span>
          </div>
          <div class="nodejs-section-content">
            <div class="status-row">
              <span class="status-label">当前版本</span>
              <span class="status-value">
                {{ updateResult?.current_version || '加载中...' }}
              </span>
            </div>
            <div
              v-if="updateResult?.remote_version"
              class="status-row"
            >
              <span class="status-label">最新版本</span>
              <span class="status-value">
                {{ updateResult.remote_version }}
              </span>
            </div>

            <!-- Update actions -->
            <div class="nodejs-actions">
              <button
                class="action-btn primary"
                :disabled="updateChecking || updateDownloading"
                @click="checkAndPrompt()"
              >
                <i
                  v-if="updateChecking"
                  class="fa-solid fa-spinner fa-spin"
                />
                <i
                  v-else
                  class="fa-solid fa-rotate"
                />
                {{ updateChecking ? '检查中...' : '检查更新' }}
              </button>
            </div>

            <!-- Download progress -->
            <div
              v-if="updateDownloading"
              class="download-progress"
            >
              <div class="download-progress-bar">
                <div
                  class="download-progress-fill"
                  :style="{ width: updateProgress.percent + '%' }"
                />
              </div>
              <span class="download-progress-text">
                {{ updateProgress.percent.toFixed(1) }}%
              </span>
            </div>

            <!-- No update message -->
            <div
              v-if="updateResult && !updateResult.update_available && !updateChecking && !updateError"
              class="success-message"
            >
              <i class="fa-solid fa-check-circle" />
              当前已是最新版本
            </div>

            <!-- Error message -->
            <div
              v-if="updateError"
              class="error-message"
            >
              <i class="fa-solid fa-exclamation-circle" />
              {{ updateError }}
            </div>
          </div>
        </div>

        <!-- Update source -->
        <div class="nodejs-section">
          <div class="nodejs-section-header">
            <i
              class="fa-solid fa-server"
              style="color: #7c6eb8;"
            />
            <span>更新源</span>
          </div>
          <div class="nodejs-section-content">
            <div class="env-field">
              <label>更新源 URL</label>
              <input
                type="text"
                :value="settingsStore.updateUrl"
                :placeholder="defaultUpdateUrl || 'https://example.com/update-manifest.json'"
                @change="(e) => { settingsStore.updateUrl = (e.target as HTMLInputElement).value; settingsStore.save() }"
              >
            </div>
            <p
              class="section-hint"
              style="margin-top: 8px;"
            >
              <i class="fa-solid fa-info-circle" />
              留空时使用外部配置文件
              <code>~/.MoontonClaudeDesktop/config/update-config.json</code>
            </p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="less" scoped>
.settings {
  display: flex;
  height: 100%;
  background: #f5f5f5;
}

.settings-sidebar {
  width: 200px;
  background: white;
  border-right: 1px solid #e0e0e0;
  padding: 16px 0;

  .sidebar-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 20px;
    cursor: pointer;
    transition: all 0.2s;
    color: #555;

    i {
      width: 18px;
      text-align: center;
    }

    &:hover {
      background: #f5f5f5;
    }

    &.active {
      background: #f0ecf8;
      color: #7c6eb8;
      border-right: 3px solid #7c6eb8;
    }
  }
}

.settings-content {
  flex: 1;
  padding: 24px;
  overflow: auto;
  display: flex;
  flex-direction: column;
}

.settings-panel {
  flex: 1;

  h3 {
    margin: 0 0 24px 0;
    font-size: 18px;
    font-weight: 500;
    color: #333;
    padding-bottom: 12px;
    border-bottom: 1px solid #e0e0e0;
  }
}

.setting-group {
  margin-bottom: 20px;

  label {
    display: block;
    font-size: 14px;
    font-weight: 500;
    color: #333;
    margin-bottom: 8px;
  }

  input[type='text'],
  select {
    width: 100%;
    max-width: 400px;
    padding: 10px 12px;
    border: 1px solid #ddd;
    border-radius: 6px;
    font-size: 14px;
    outline: none;
    transition: border-color 0.2s;

    &:focus {
      border-color: #7c6eb8;
    }
  }

  .font-selector {
    max-width: 320px;
  }

  .font-loading {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    background: #fafafa;
    border: 1px solid #e0e0e0;
    border-radius: 6px;
    max-width: 320px;
    color: #666;
    font-size: 14px;

    i {
      color: #7c6eb8;
    }
  }

  .setting-hint {
    display: block;
    font-size: 12px;
    color: #888;
    margin-top: 6px;
  }

  .input-with-unit {
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 200px;

    input[type='number'] {
      flex: 1;
      padding: 10px 12px;
      border: 1px solid #ddd;
      border-radius: 6px;
      font-size: 14px;
      outline: none;
      transition: border-color 0.2s;

      &:focus {
        border-color: #7c6eb8;
      }
    }

    .unit {
      font-size: 14px;
      color: #666;
      min-width: 24px;
    }
  }

  .range-input {
    display: flex;
    align-items: center;
    gap: 12px;

    input[type='range'] {
      width: 200px;
    }

    span {
      font-size: 14px;
      color: #666;
      min-width: 50px;
    }
  }
}

// Prompt settings styles
.prompt-settings {
  .prompt-toggle {
    margin-bottom: 16px;

    .toggle-label {
      display: flex;
      align-items: center;
      gap: 8px;
      cursor: pointer;
      font-weight: normal;

      input[type='checkbox'] {
        width: 18px;
        height: 18px;
        cursor: pointer;
        accent-color: #7c6eb8;
      }

      span {
        font-size: 14px;
        color: #333;
      }
    }
  }

  .prompt-presets {
    display: flex;
    gap: 12px;
    margin-bottom: 16px;

    .preset-option {
      flex: 1;
      padding: 12px;
      background: #fafafa;
      border: 2px solid #e0e0e0;
      border-radius: 8px;
      cursor: pointer;
      transition: all 0.2s;
      text-align: center;

      &:hover {
        border-color: #7c6eb8;
        background: #f8f5ff;
      }

      &.active {
        border-color: #7c6eb8;
        background: #f0ecf8;
      }

      .preset-preview {
        font-family: 'Consolas', 'Monaco', monospace;
        font-size: 12px;
        color: #666;
        margin-bottom: 8px;
        padding: 8px;
        background: #282c34;
        border-radius: 4px;
        color: #abb2bf;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .preset-name {
        font-size: 13px;
        font-weight: 500;
        color: #333;
      }
    }
  }

  .prompt-options {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 8px;
    margin-bottom: 16px;

    .option-label {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 8px 12px;
      background: #fafafa;
      border-radius: 6px;
      cursor: pointer;
      font-weight: normal;

      &:hover {
        background: #f0f0f0;
      }

      input[type='checkbox'] {
        width: 16px;
        height: 16px;
        cursor: pointer;
        accent-color: #7c6eb8;
      }

      span {
        font-size: 13px;
        color: #555;
      }
    }
  }

  .path-mode {
    margin-bottom: 16px;

    label {
      display: block;
      font-size: 13px;
      color: #666;
      margin-bottom: 6px;
    }

    select {
      width: auto;
      min-width: 180px;
      padding: 8px 12px;
      border: 1px solid #ddd;
      border-radius: 6px;
      font-size: 14px;
      outline: none;
      cursor: pointer;

      &:focus {
        border-color: #7c6eb8;
      }
    }
  }

  .prompt-preview-box {
    margin-bottom: 12px;

    label {
      display: block;
      font-size: 13px;
      color: #666;
      margin-bottom: 6px;
    }

    .preview-terminal {
      font-family: 'Consolas', 'Monaco', monospace;
      font-size: 14px;
      padding: 12px 16px;
      background: #282c34;
      border-radius: 6px;
      color: #abb2bf;
      white-space: nowrap;
      overflow-x: auto;
    }
  }
}

.env-panel {
  overflow-y: auto;
}

.env-sync-section {
  background: rgba(0, 120, 212, 0.06);
  border: 1px solid rgba(0, 120, 212, 0.2);
  border-radius: 8px;
  padding: 12px 16px;
  margin-bottom: 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;

  .env-sync-info {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: #666;
    flex: 1;
  }

  .env-sync-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .sync-ok {
    color: #2e7d32;
    font-size: 13px;
  }

  .sync-err {
    color: #c62828;
    font-size: 13px;
    max-width: 300px;
    word-break: break-word;
  }
}

.env-section {
  background: white;
  border-radius: 8px;
  border: 1px solid #e0e0e0;
  margin-bottom: 16px;
  overflow: hidden;

  .env-section-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    background: #f8f8f8;
    border-bottom: 1px solid #e0e0e0;
    font-weight: 500;
    color: #333;

    i {
      color: #7c6eb8;
      width: 18px;
      text-align: center;
    }
  }

  .env-section-content {
    padding: 16px;
  }
}

.env-field {
  margin-bottom: 12px;

  &:last-child {
    margin-bottom: 0;
  }

  label {
    display: block;
    font-size: 12px;
    font-family: monospace;
    color: #666;
    margin-bottom: 6px;
  }

  input {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid #ddd;
    border-radius: 6px;
    font-size: 14px;
    outline: none;
    transition: border-color 0.2s;

    &:focus {
      border-color: #7c6eb8;
    }

    &::placeholder {
      color: #aaa;
    }
  }

  // Model selector styles
  &.model-selector {
    margin-bottom: 16px;

    .model-options {
      display: flex;
      flex-direction: column;
      gap: 6px;
      background: #fafafa;
      border: 1px solid #e0e0e0;
      border-radius: 8px;
      padding: 10px;
    }

    .model-option {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 10px 12px;
      border-radius: 6px;
      cursor: pointer;
      transition: all 0.2s;
      border: 1px solid transparent;

      &:hover {
        background: #f0ecf8;
      }

      &.active {
        background: #f0ecf8;
        border-color: #7c6eb8;
      }

      .radio-circle {
        width: 18px;
        height: 18px;
        border: 2px solid #ccc;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        flex-shrink: 0;

        .radio-dot {
          width: 10px;
          height: 10px;
          background: #7c6eb8;
          border-radius: 50%;
        }
      }

      &.active .radio-circle {
        border-color: #7c6eb8;
      }

      .option-content {
        display: flex;
        flex-direction: column;
        gap: 2px;
        flex: 1;
        min-width: 0;
      }

      .option-label {
        font-weight: 500;
        color: #333;
        font-size: 14px;
        font-family: inherit;
      }

      .option-value {
        font-family: monospace;
        font-size: 11px;
        color: #888;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .option-desc {
        font-size: 12px;
        color: #7c6eb8;
        margin-left: auto;
        flex-shrink: 0;
      }

      &.custom {
        .option-label {
          flex-shrink: 0;
          margin-right: 8px;
        }

        input {
          flex: 1;
          padding: 6px 10px;
          border: 1px solid #ddd;
          border-radius: 4px;
          font-size: 13px;
          outline: none;
          min-width: 0;

          &:focus {
            border-color: #7c6eb8;
          }
        }
      }

      &.preset {
        .option-content {
          flex-shrink: 0;
          min-width: 140px;
        }

        .option-desc {
          position: static;
          margin-left: 0;
        }

        input {
          flex: 1;
          padding: 6px 10px;
          border: 1px solid #ddd;
          border-radius: 4px;
          font-size: 12px;
          font-family: monospace;
          outline: none;
          min-width: 0;
          background: #fff;

          &:focus {
            border-color: #7c6eb8;
          }
        }
      }
    }
  }
}

.env-add-form {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;

  input {
    padding: 8px 12px;
    border: 1px solid #ddd;
    border-radius: 6px;
    font-size: 14px;
    outline: none;

    &:first-child {
      width: 150px;
    }

    &:nth-child(2) {
      flex: 1;
      max-width: 300px;
    }

    &:focus {
      border-color: #7c6eb8;
    }
  }

  button {
    padding: 8px 16px;
    background: #7c6eb8;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 14px;

    &:hover {
      background: #6a5ca0;
    }
  }
}

.env-list {
  background: #fafafa;
  border-radius: 6px;
  border: 1px solid #eee;
  overflow: hidden;

  .env-item {
    display: flex;
    align-items: center;
    padding: 10px 12px;
    border-bottom: 1px solid #eee;

    &:last-child {
      border-bottom: none;
    }

    .env-key {
      font-family: monospace;
      font-weight: 500;
      font-size: 13px;
      color: #7c6eb8;
      min-width: 120px;
    }

    .env-value {
      flex: 1;
      font-family: monospace;
      font-size: 13px;
      color: #666;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .delete-btn {
      padding: 6px 10px;
      background: transparent;
      color: #999;
      border: none;
      border-radius: 4px;
      cursor: pointer;

      &:hover {
        background: #fee;
        color: #d00;
      }
    }
  }

  .env-empty {
    padding: 24px;
    text-align: center;
    color: #999;
    font-size: 13px;
  }
}

.keybindings-list {
  background: white;
  border-radius: 8px;
  border: 1px solid #e0e0e0;
  overflow: hidden;

  .keybinding-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 16px;
    border-bottom: 1px solid #eee;

    &:last-child {
      border-bottom: none;
    }

    .keybinding-action {
      font-size: 14px;
      color: #333;
    }

    .keybinding-key {
      font-family: monospace;
      background: #f0f0f0;
      padding: 4px 10px;
      border-radius: 4px;
      font-size: 13px;
      color: #666;
    }
  }
}

.settings-note {
  margin-top: 16px;
  font-size: 13px;
  color: #888;
  display: flex;
  align-items: center;
  gap: 8px;

  i {
    color: #7c6eb8;
  }

  &.success {
    color: #16a34a;

    i {
      color: #16a34a;
    }
  }
}

.wallpaper-preview-container {
  margin-bottom: 16px;

  .wallpaper-preview {
    width: 320px;
    height: 180px;
    border-radius: 8px;
    border: 2px solid #e0e0e0;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }
}

.wallpaper-actions {
  display: flex;
  gap: 12px;
  margin-bottom: 8px;

  .wallpaper-btn {
    padding: 10px 16px;
    background: #7c6eb8;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    transition: background 0.2s;

    &:hover {
      background: #6a5ca0;
    }

    &.secondary {
      background: #f5f5f5;
      color: #666;
      border: 1px solid #ddd;

      &:hover {
        background: #e0e0e0;
      }
    }
  }
}

// Node.js panel styles
.nodejs-panel {
  overflow-y: auto;
}

.nodejs-loading {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 20px;
  color: #666;

  i {
    color: #7c6eb8;
  }
}

.nodejs-error {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 12px 16px;
  background: #fef2f2;
  border: 1px solid #fecaca;
  border-radius: 8px;
  color: #dc2626;
  margin-bottom: 16px;

  i {
    margin-top: 2px;
  }

  span {
    flex: 1;
    font-size: 13px;
    word-break: break-all;
  }
}

.nodejs-section {
  background: white;
  border-radius: 8px;
  border: 1px solid #e0e0e0;
  margin-bottom: 16px;
  overflow: hidden;

  .nodejs-section-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    background: #f8f8f8;
    border-bottom: 1px solid #e0e0e0;
    font-weight: 500;
    color: #333;

    i {
      color: #68a063;
      width: 18px;
      text-align: center;

      &.fa-robot,
      &.fa-sliders {
        color: #7c6eb8;
      }
    }
  }

  .nodejs-section-content {
    padding: 16px;
  }
}

.status-row {
  display: flex;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid #f0f0f0;

  &:last-of-type {
    border-bottom: none;
  }

  .status-label {
    width: 80px;
    font-size: 13px;
    color: #666;
  }

  .status-value {
    flex: 1;
    font-size: 14px;
    color: #333;

    &.installed {
      color: #16a34a;

      i {
        margin-right: 6px;
      }
    }

    &.not-installed {
      color: #dc2626;

      i {
        margin-right: 6px;
      }
    }

    .source-badge {
      display: inline-block;
      font-size: 11px;
      padding: 2px 8px;
      border-radius: 10px;
      margin-left: 8px;
      font-weight: 500;

      &.system {
        background: #dbeafe;
        color: #1d4ed8;
      }

      &.bundled {
        background: #f3e8ff;
        color: #7c3aed;
      }
    }

    &.path {
      font-family: monospace;
      font-size: 12px;
      color: #666;
      word-break: break-all;
    }
  }
}

.nodejs-actions {
  display: flex;
  gap: 12px;
  margin-top: 16px;

  .action-btn {
    padding: 10px 16px;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    transition: background 0.2s;

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }

    &.primary {
      background: #7c6eb8;
      color: white;

      &:hover:not(:disabled) {
        background: #6a5ca0;
      }
    }

    &.secondary {
      background: #f5f5f5;
      color: #666;
      border: 1px solid #ddd;

      &:hover:not(:disabled) {
        background: #e0e0e0;
      }
    }
  }
}

.section-hint {
  margin-top: 16px;
  font-size: 12px;
  color: #888;
  display: flex;
  align-items: flex-start;
  gap: 8px;

  i {
    color: #7c6eb8;
    margin-top: 2px;
  }

  code {
    background: #f0f0f0;
    padding: 2px 6px;
    border-radius: 4px;
    font-family: monospace;
    font-size: 12px;
  }
}

// Claude Code settings styles
.cc-setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 0;
  border-bottom: 1px solid #f0f0f0;

  &:last-child {
    border-bottom: none;
  }
}

.cc-setting-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.cc-setting-label {
  font-size: 13px;
  font-weight: 500;
  color: #333;
}

.cc-setting-desc {
  font-size: 11px;
  color: #888;
}

.cc-setting-select,
.cc-setting-input {
  padding: 4px 8px;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 13px;
  background: #fff;
  min-width: 120px;
  outline: none;
  transition: border-color 0.2s;

  &:focus {
    border-color: #7c6eb8;
  }
}

.cc-toggle-switch {
  position: relative;
  display: inline-block;

  input {
    display: none;
  }
}

.cc-toggle-slider {
  display: inline-block;
  width: 36px;
  height: 20px;
  border-radius: 10px;
  background: #d5d0d0;
  position: relative;
  cursor: pointer;
  transition: background 0.2s;

  &::after {
    content: '';
    position: absolute;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    top: 2px;
    left: 2px;
    transition: left 0.2s;
  }
}

.cc-toggle-switch input:checked + .cc-toggle-slider {
  background: #7c6eb8;

  &::after {
    left: 18px;
  }
}

.command-list {
  margin-top: 12px;
  background: #f8f8f8;
  border-radius: 6px;
  border: 1px solid #eee;
  overflow: hidden;

  .command-item {
    display: flex;
    align-items: center;
    padding: 10px 12px;
    border-bottom: 1px solid #eee;

    &:last-child {
      border-bottom: none;
    }

    code {
      background: #e8e8e8;
      padding: 4px 10px;
      border-radius: 4px;
      font-family: monospace;
      font-size: 13px;
      color: #7c6eb8;
      min-width: 120px;
    }

    span {
      margin-left: 16px;
      font-size: 13px;
      color: #666;
    }
  }
}

.error-message {
  margin-top: 12px;
  padding: 10px 14px;
  background: #fef2f2;
  border: 1px solid #fecaca;
  border-radius: 6px;
  color: #dc2626;
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 8px;

  i {
    font-size: 14px;
  }
}

.success-message {
  margin-top: 12px;
  padding: 10px 14px;
  background: #f0fdf4;
  border: 1px solid #bbf7d0;
  border-radius: 6px;
  color: #16a34a;
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 8px;

  i {
    font-size: 14px;
  }

  code {
    background: #dcfce7;
    padding: 2px 6px;
    border-radius: 3px;
    font-family: monospace;
  }
}

// Download progress styles
.download-progress {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 12px;

  .download-progress-bar {
    flex: 1;
    height: 8px;
    background: #e0e0e0;
    border-radius: 4px;
    overflow: hidden;

    .download-progress-fill {
      height: 100%;
      background: #7c6eb8;
      border-radius: 4px;
      transition: width 0.2s ease;
    }
  }

  .download-progress-text {
    font-size: 13px;
    color: #666;
    min-width: 50px;
    text-align: right;
    font-family: monospace;
  }
}

// Memory panel styles
.memory-panel {
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.memory-content {
  display: flex;
  flex-direction: column;
  flex: 1;
}

.memory-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px 24px;
  background: white;
  border-radius: 8px;
  border: 1px solid #e0e0e0;
  text-align: center;

  .empty-icon {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background: #f0ecf8;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 16px;

    i {
      font-size: 28px;
      color: #7c6eb8;
    }
  }

  h4 {
    margin: 0 0 8px 0;
    font-size: 16px;
    font-weight: 500;
    color: #333;
  }

  p {
    margin: 0 0 20px 0;
    font-size: 13px;
    color: #666;
    max-width: 400px;
    line-height: 1.6;

    code {
      background: #f0f0f0;
      padding: 2px 6px;
      border-radius: 4px;
      font-family: monospace;
      font-size: 12px;
    }
  }
}

.memory-editor {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  background: white;
  border-radius: 8px;
  border: 1px solid #e0e0e0;
  overflow: hidden;

  .editor-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    background: #f8f8f8;
    border-bottom: 1px solid #e0e0e0;

    .file-path {
      display: flex;
      align-items: center;
      gap: 8px;
      font-family: monospace;
      font-size: 13px;
      color: #666;

      i {
        color: #7c6eb8;
      }
    }
  }
}

.memory-textarea {
  flex: 1;
  width: 100%;
  min-height: 300px;
  padding: 16px;
  border: none;
  outline: none;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 13px;
  line-height: 1.6;
  resize: none;
  background: #fafafa;

  &::placeholder {
    color: #aaa;
  }
}

.memory-examples {
  margin-top: 16px;
  background: white;
  border-radius: 8px;
  border: 1px solid #e0e0e0;
  overflow: hidden;

  summary {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    background: #f8f8f8;
    cursor: pointer;
    font-weight: 500;
    color: #333;
    user-select: none;

    &:hover {
      background: #f0f0f0;
    }

    i {
      color: #f59e0b;
    }

    &::marker {
      content: '';
    }

    &::-webkit-details-marker {
      display: none;
    }
  }

  .examples-content {
    padding: 16px;

    p {
      margin: 0 0 12px 0;
      font-size: 13px;
      color: #666;
    }

    ul {
      margin: 0 0 12px 0;
      padding-left: 20px;

      li {
        font-size: 13px;
        color: #666;
        margin-bottom: 8px;
        line-height: 1.5;

        strong {
          color: #333;
        }

        &:last-child {
          margin-bottom: 0;
        }
      }
    }

    .example-note {
      display: flex;
      align-items: flex-start;
      gap: 8px;
      padding: 12px;
      background: #f0ecf8;
      border-radius: 6px;
      margin-bottom: 0;

      i {
        color: #7c6eb8;
        margin-top: 2px;
      }

      code {
        background: #e0d8f0;
        padding: 2px 6px;
        border-radius: 4px;
        font-family: monospace;
        font-size: 12px;
      }
    }
  }
}
</style>
