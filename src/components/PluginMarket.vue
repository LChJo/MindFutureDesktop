<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ask } from '@tauri-apps/plugin-dialog'
import { useMarketplaceStore, type PluginInfo, type MarketplaceInfo } from '@/stores/marketplaceStore'
import { useDesktopStore } from '@/stores/desktopStore'
import { isNewerVersion } from '@/utils/version'
import McpManager from '@/components/McpManager.vue'

// Skill info type from backend
interface SkillInfo {
  name: string
  title: string
  description: string
  path: string
}

// Agent info type from backend
interface AgentInfo {
  name: string
  title: string
  description: string
  file_path: string
  dir_path: string
}

// Plugin skill info type from backend
interface PluginSkillInfo {
  name: string
  title: string
  description: string
  plugin_name: string
  marketplace_name: string
  plugin_path: string
}

// Plugin agent info type from backend
interface PluginAgentInfo {
  name: string
  title: string
  description: string
  plugin_name: string
  marketplace_name: string
}

defineProps<{
  windowId?: string
}>()

const marketplaceStore = useMarketplaceStore()
const desktopStore = useDesktopStore()

// Active tab: 'market', 'skills', 'agents', or 'mcp'
const activeTab = ref<'market' | 'skills' | 'agents' | 'mcp'>('market')

// Skills management
const localSkills = ref<SkillInfo[]>([])
const pluginSkills = ref<PluginSkillInfo[]>([])
const loadingSkills = ref(false)
const loadingPluginSkills = ref(false)
const creatingSkillShortcut = ref<string | null>(null)
const creatingPluginSkillShortcut = ref<string | null>(null)

// Skill sub-tabs: 'local' or marketplace names
const activeSkillTab = ref<string>('local')

// Get unique marketplace names from plugin skills
const skillMarketplaces = computed(() => {
  const names = new Set<string>()
  pluginSkills.value.forEach(skill => {
    names.add(skill.marketplace_name)
  })
  return Array.from(names).sort()
})

// Filtered plugin skills based on active tab
const filteredPluginSkills = computed(() => {
  if (activeSkillTab.value === 'local') {
    return []
  }
  return pluginSkills.value.filter(
    skill => skill.marketplace_name === activeSkillTab.value
  )
})

// Agents management
const localAgents = ref<AgentInfo[]>([])
const pluginAgents = ref<PluginAgentInfo[]>([])
const loadingAgents = ref(false)
const loadingPluginAgents = ref(false)

// Agent sub-tabs: 'local' or marketplace names
const activeAgentTab = ref<string>('local')

// Get unique marketplace names from plugin agents
const agentMarketplaces = computed(() => {
  const names = new Set<string>()
  pluginAgents.value.forEach(agent => {
    names.add(agent.marketplace_name)
  })
  return Array.from(names).sort()
})

// Filtered plugin agents based on active tab
const filteredPluginAgents = computed(() => {
  if (activeAgentTab.value === 'local') {
    return []
  }
  return pluginAgents.value.filter(
    agent => agent.marketplace_name === activeAgentTab.value
  )
})

// Current view: 'sources' or 'plugins'
const currentView = ref<'sources' | 'plugins'>('sources')
const selectedMarketplace = ref<MarketplaceInfo | null>(null)

// Search query
const searchQuery = ref('')

// Plugins from selected marketplace
const marketplacePlugins = ref<PluginInfo[]>([])
const loadingPlugins = ref(false)

// Add marketplace dialog
const showAddDialog = ref(false)
const addSourceType = ref<'github' | 'url'>('github')
const addSourceValue = ref('')
const addError = ref('')
const addLoading = ref(false)

// Plugin action states
const installingPlugin = ref<string | null>(null)
const uninstallingPlugin = ref<string | null>(null)
const creatingShortcut = ref<string | null>(null)
const togglingPlugin = ref<string | null>(null)

// Plugin icons cache (plugin name -> base64 data URL)
const pluginIcons = ref<Record<string, string>>({})

// Skill selection dialog
const showSkillDialog = ref(false)
const skillDialogPlugin = ref<PluginInfo | null>(null)

// Repair dialog
const showRepairDialog = ref(false)
const repairResult = ref<{
  actions: { pluginKey: string; action: string; details: string }[]
  installedPluginsFixed: number
  settingsFixed: number
  totalInstalled: number
} | null>(null)

// Filter plugins by search
const filteredPlugins = computed(() => {
  if (!searchQuery.value) {
    return marketplacePlugins.value
  }
  const query = searchQuery.value.toLowerCase()
  return marketplacePlugins.value.filter(plugin =>
    plugin.name.toLowerCase().includes(query) ||
    plugin.description.toLowerCase().includes(query) ||
    plugin.tags.some(tag => tag.toLowerCase().includes(query))
  )
})

// Load plugin icon (async, non-blocking)
const loadPluginIcon = async (marketplaceName: string, pluginName: string) => {
  if (pluginIcons.value[pluginName]) return // Already cached

  try {
    const icon = await marketplaceStore.getPluginIcon(marketplaceName, pluginName)
    if (icon) {
      pluginIcons.value[pluginName] = icon
    }
  } catch (e) {
    console.error(`Failed to load icon for ${pluginName}:`, e)
  }
}

// Load all plugin icons (background, non-blocking)
const loadAllPluginIcons = async (marketplaceName: string, plugins: PluginInfo[]) => {
  // Load icons in parallel but don't block UI
  plugins.forEach(plugin => {
    loadPluginIcon(marketplaceName, plugin.name)
  })
}

// Load plugins from selected marketplace
const loadMarketplacePlugins = async (marketplaceName: string) => {
  loadingPlugins.value = true
  try {
    marketplacePlugins.value = await marketplaceStore.getMarketplacePlugins(marketplaceName)
    // Load installed plugins to check status
    await marketplaceStore.loadInstalledPlugins()
    // Load plugin icons in background (non-blocking)
    loadAllPluginIcons(marketplaceName, marketplacePlugins.value)
  } catch (e) {
    console.error('Failed to load plugins:', e)
    marketplacePlugins.value = []
  } finally {
    loadingPlugins.value = false
  }
}

// Enter marketplace plugins view
const enterMarketplace = async (marketplace: MarketplaceInfo) => {
  selectedMarketplace.value = marketplace
  currentView.value = 'plugins'
  searchQuery.value = ''
  await loadMarketplacePlugins(marketplace.name)
}

// Back to sources view
const backToSources = () => {
  currentView.value = 'sources'
  selectedMarketplace.value = null
  marketplacePlugins.value = []
  pluginIcons.value = {}
  searchQuery.value = ''
}

// Add marketplace
const handleAddMarketplace = async () => {
  if (!addSourceValue.value.trim()) {
    addError.value = '请输入市场源地址'
    return
  }

  addLoading.value = true
  addError.value = ''

  try {
    await marketplaceStore.addMarketplace(addSourceType.value, addSourceValue.value.trim())
    showAddDialog.value = false
    addSourceValue.value = ''
  } catch (e) {
    addError.value = String(e)
  } finally {
    addLoading.value = false
  }
}

// Update marketplace
const handleUpdateMarketplace = async (name: string, event?: Event) => {
  event?.stopPropagation()
  try {
    await marketplaceStore.updateMarketplace(name)
    // Reload plugins if viewing this marketplace
    if (selectedMarketplace.value?.name === name) {
      await loadMarketplacePlugins(name)
    }
  } catch (e) {
    alert(`更新失败: ${e}`)
  }
}

// Update all marketplaces
const handleUpdateAll = async () => {
  const results = await marketplaceStore.updateAllMarketplaces()
  const failed = results.filter(r => !r.success)
  if (failed.length > 0) {
    alert(`部分更新失败:\n${failed.map(r => `${r.name}: ${r.error}`).join('\n')}`)
  }
}

// Remove marketplace
const handleRemoveMarketplace = async (name: string, event?: Event) => {
  event?.stopPropagation()
  const confirmed = await ask(`确定要删除市场 "${name}" 吗？`, {
    title: '删除确认',
    kind: 'warning',
  })
  if (!confirmed) {
    return
  }

  try {
    await marketplaceStore.removeMarketplace(name)
    // If currently viewing this marketplace, go back
    if (selectedMarketplace.value?.name === name) {
      backToSources()
    }
  } catch (e) {
    alert(`删除失败: ${e}`)
  }
}

// Install plugin
const handleInstallPlugin = async (plugin: PluginInfo) => {
  if (!selectedMarketplace.value) return

  installingPlugin.value = plugin.name
  try {
    await marketplaceStore.installPlugin(selectedMarketplace.value.name, plugin.name)
  } catch (e) {
    alert(`安装失败: ${e}`)
  } finally {
    installingPlugin.value = null
  }
}

// Uninstall plugin
const handleUninstallPlugin = async (plugin: PluginInfo) => {
  if (!selectedMarketplace.value) return

  const confirmed = await ask(`确定要卸载插件 "${plugin.name}" 吗？`, {
    title: '卸载确认',
    kind: 'warning',
  })
  if (!confirmed) {
    return
  }

  uninstallingPlugin.value = plugin.name
  try {
    await marketplaceStore.uninstallPlugin(plugin.name, selectedMarketplace.value.name)
  } catch (e) {
    alert(`卸载失败: ${e}`)
  } finally {
    uninstallingPlugin.value = null
  }
}

// Update plugin (reinstall to get latest version)
const handleUpdatePlugin = async (plugin: PluginInfo) => {
  if (!selectedMarketplace.value) return

  installingPlugin.value = plugin.name
  try {
    // Uninstall then reinstall
    await marketplaceStore.uninstallPlugin(plugin.name, selectedMarketplace.value.name)
    await marketplaceStore.installPlugin(selectedMarketplace.value.name, plugin.name)
  } catch (e) {
    alert(`更新失败: ${e}`)
  } finally {
    installingPlugin.value = null
  }
}

// Check if plugin is installed
const isPluginInstalled = (pluginName: string): boolean => {
  return marketplaceStore.isPluginInstalled(pluginName)
}

// Check if plugin has update available
const hasPluginUpdate = (plugin: PluginInfo): boolean => {
  const installed = marketplaceStore.getInstalledPlugin(plugin.name)
  if (!installed) return false
  // 只有当市场版本 > 已安装版本时才显示更新
  return isNewerVersion(plugin.version, installed.version)
}

// Check if plugin is enabled
const isPluginEnabled = (pluginName: string): boolean => {
  if (!selectedMarketplace.value) return false
  return marketplaceStore.isPluginEnabled(pluginName, selectedMarketplace.value.name)
}

// Toggle plugin enabled/disabled
const handleTogglePlugin = async (plugin: PluginInfo) => {
  if (!selectedMarketplace.value) return

  togglingPlugin.value = plugin.name
  try {
    await marketplaceStore.togglePluginEnabled(plugin.name, selectedMarketplace.value.name)
  } catch (e) {
    alert(`切换失败: ${e}`)
  } finally {
    togglingPlugin.value = null
  }
}

// Extract clean skill name from path (e.g., "./skills/image-generator" -> "image-generator")
const getCleanSkillName = (skillPath: string): string => {
  return skillPath.split('/').pop() || skillPath
}

// Create shortcut for a specific skill
const createShortcutForSkill = (plugin: PluginInfo, skillName: string) => {
  if (!selectedMarketplace.value) return

  creatingShortcut.value = plugin.name
  const cleanName = getCleanSkillName(skillName)
  try {
    const result = desktopStore.addPluginShortcut(
      plugin.name,
      plugin.description,
      selectedMarketplace.value.name,
      skillName // Store the original path for potential future use
    )
    if (result.success) {
      alert(`技能快捷方式已创建: ${cleanName}`)
    } else {
      alert(result.message)
    }
  } catch (e) {
    alert(`创建快捷方式失败: ${e}`)
  } finally {
    creatingShortcut.value = null
  }
}

// Handle skill selection from dialog
const handleSelectSkill = (skillName: string) => {
  if (skillDialogPlugin.value) {
    createShortcutForSkill(skillDialogPlugin.value, skillName)
  }
  closeSkillDialog()
}

// Close skill selection dialog
const closeSkillDialog = () => {
  showSkillDialog.value = false
  skillDialogPlugin.value = null
}

// Repair plugin registry
const handleRepairRegistry = async () => {
  try {
    const result = await marketplaceStore.repairPluginRegistry()
    repairResult.value = result
    showRepairDialog.value = true
  } catch (e) {
    alert(`修复失败: ${e}`)
  }
}

// Close repair dialog
const closeRepairDialog = () => {
  showRepairDialog.value = false
  repairResult.value = null
}

// Close add dialog
const closeAddDialog = () => {
  showAddDialog.value = false
  addSourceValue.value = ''
  addError.value = ''
}

// ============ Skills Management Functions ============

// Load local skills from ~/.claude/skills
const loadLocalSkills = async () => {
  loadingSkills.value = true
  try {
    localSkills.value = await invoke<SkillInfo[]>('get_local_skills')
  } catch (e) {
    console.error('Failed to load local skills:', e)
    localSkills.value = []
  } finally {
    loadingSkills.value = false
  }
}

// Open skill directory in file manager
const handleOpenSkillDirectory = async (skill: SkillInfo) => {
  try {
    await invoke('open_directory', { path: skill.path })
  } catch (e) {
    alert(`打开目录失败: ${e}`)
  }
}

// Create desktop shortcut for a skill
const handleCreateSkillShortcut = (skill: SkillInfo) => {
  creatingSkillShortcut.value = skill.name
  try {
    const result = desktopStore.addPluginShortcut(
      skill.name,
      skill.description || skill.title,
      'local-skills', // Use a special marker for local skills
      skill.name
    )
    if (result.success) {
      alert(`技能快捷方式已创建: ${skill.title}`)
    } else {
      alert(result.message)
    }
  } catch (e) {
    alert(`创建快捷方式失败: ${e}`)
  } finally {
    creatingSkillShortcut.value = null
  }
}

// Check if skill has shortcut
const hasSkillShortcut = (skillName: string): boolean => {
  return desktopStore.hasPluginShortcut(skillName)
}

// Remove skill shortcut
const handleRemoveSkillShortcut = async (skill: SkillInfo) => {
  const confirmed = await ask(`确定要删除 "${skill.title}" 的桌面快捷方式吗？`, {
    title: '删除确认',
    kind: 'warning',
  })
  if (confirmed) {
    desktopStore.removePluginShortcut(skill.name)
  }
}

// ============ Plugin Skills Management Functions ============

// Load plugin skills from installed plugins
const loadPluginSkills = async () => {
  loadingPluginSkills.value = true
  try {
    pluginSkills.value = await invoke<PluginSkillInfo[]>('get_plugin_skills')
  } catch (e) {
    console.error('Failed to load plugin skills:', e)
    pluginSkills.value = []
  } finally {
    loadingPluginSkills.value = false
  }
}

// Navigate to plugin in marketplace
const handleViewPlugin = async (skill: PluginSkillInfo) => {
  // Find the marketplace and enter it
  const marketplace = marketplaceStore.marketplaces.find(
    m => m.name === skill.marketplace_name
  )
  if (marketplace) {
    activeTab.value = 'market'
    await enterMarketplace(marketplace)
    // Set search to find the plugin
    searchQuery.value = skill.plugin_name
  } else {
    alert(`未找到市场源: ${skill.marketplace_name}`)
  }
}

// Create desktop shortcut for a plugin skill
const handleCreatePluginSkillShortcut = (skill: PluginSkillInfo) => {
  creatingPluginSkillShortcut.value = skill.name
  try {
    const result = desktopStore.addPluginShortcut(
      skill.name,
      skill.description || skill.title,
      skill.marketplace_name,
      skill.name
    )
    if (result.success) {
      alert(`技能快捷方式已创建: ${skill.title}`)
    } else {
      alert(result.message)
    }
  } catch (e) {
    alert(`创建快捷方式失败: ${e}`)
  } finally {
    creatingPluginSkillShortcut.value = null
  }
}

// Check if plugin skill has shortcut
const hasPluginSkillShortcut = (skillName: string): boolean => {
  return desktopStore.hasPluginShortcut(skillName)
}

// Remove plugin skill shortcut
const handleRemovePluginSkillShortcut = async (skill: PluginSkillInfo) => {
  const confirmed = await ask(`确定要删除 "${skill.title}" 的桌面快捷方式吗？`, {
    title: '删除确认',
    kind: 'warning',
  })
  if (confirmed) {
    desktopStore.removePluginShortcut(skill.name)
  }
}

// ============ Agents Management Functions ============

// Load local agents from ~/.claude/agents
const loadLocalAgents = async () => {
  loadingAgents.value = true
  try {
    localAgents.value = await invoke<AgentInfo[]>('get_local_agents')
  } catch (e) {
    console.error('Failed to load local agents:', e)
    localAgents.value = []
  } finally {
    loadingAgents.value = false
  }
}

// Open agents directory in file manager
const handleOpenAgentsDirectory = async (agent: AgentInfo) => {
  try {
    await invoke('open_directory', { path: agent.dir_path })
  } catch (e) {
    alert(`打开目录失败: ${e}`)
  }
}

// Load plugin agents from installed plugins
const loadPluginAgents = async () => {
  loadingPluginAgents.value = true
  try {
    pluginAgents.value = await invoke<PluginAgentInfo[]>('get_plugin_agents')
  } catch (e) {
    console.error('Failed to load plugin agents:', e)
    pluginAgents.value = []
  } finally {
    loadingPluginAgents.value = false
  }
}

// Navigate to plugin in marketplace (for plugin agents)
const handleViewAgentPlugin = async (agent: PluginAgentInfo) => {
  const marketplace = marketplaceStore.marketplaces.find(
    m => m.name === agent.marketplace_name
  )
  if (marketplace) {
    activeTab.value = 'market'
    await enterMarketplace(marketplace)
    searchQuery.value = agent.plugin_name
  } else {
    alert(`未找到市场源: ${agent.marketplace_name}`)
  }
}

// Watch for tab changes to load skills/agents
watch(activeTab, async (newTab) => {
  if (newTab === 'skills') {
    if (localSkills.value.length === 0) {
      await loadLocalSkills()
    }
    if (pluginSkills.value.length === 0) {
      await loadPluginSkills()
    }
  }
  if (newTab === 'agents') {
    if (localAgents.value.length === 0) {
      await loadLocalAgents()
    }
    if (pluginAgents.value.length === 0) {
      await loadPluginAgents()
    }
  }
})

onMounted(async () => {
  // Initialize and auto-repair if needed (e.g., missing installed_plugins.json)
  await marketplaceStore.initializeAndRepairIfNeeded()
})
</script>

<template>
  <div class="plugin-market">
    <!-- Tab Navigation -->
    <div class="tab-bar">
      <button
        class="tab-item"
        :class="{ active: activeTab === 'market' }"
        @click="activeTab = 'market'"
      >
        <i class="fa-solid fa-store" />
        插件市场
      </button>
      <button
        class="tab-item"
        :class="{ active: activeTab === 'skills' }"
        @click="activeTab = 'skills'"
      >
        <i class="fa-solid fa-wand-magic-sparkles" />
        Skill 管理
      </button>
      <button
        class="tab-item"
        :class="{ active: activeTab === 'agents' }"
        @click="activeTab = 'agents'"
      >
        <i class="fa-solid fa-robot" />
        Agent 管理
      </button>
      <button
        class="tab-item"
        :class="{ active: activeTab === 'mcp' }"
        @click="activeTab = 'mcp'"
      >
        <i class="fa-solid fa-server" />
        MCP 管理
      </button>
    </div>

    <!-- Market Tab Content -->
    <template v-if="activeTab === 'market'">
      <!-- Header -->
      <div class="market-header">
        <button
          v-if="currentView === 'plugins'"
          class="back-btn"
          @click="backToSources"
        >
          <i class="fa-solid fa-arrow-left" />
        </button>
        <div class="header-title">
          <h3 v-if="currentView === 'sources'">
            <i class="fa-solid fa-store" />
            插件市场
          </h3>
          <h3 v-else>
            <i class="fa-solid fa-puzzle-piece" />
            {{ selectedMarketplace?.name }}
          </h3>
        </div>
        <div
          v-if="currentView === 'plugins'"
          class="search-box"
        >
          <i class="fa-solid fa-search" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="搜索插件..."
          >
        </div>
      </div>

      <!-- Content -->
      <div class="market-content">
        <!-- Marketplace Sources View -->
        <template v-if="currentView === 'sources'">
          <div
            v-if="marketplaceStore.loading"
            class="loading-state"
          >
            <i class="fa-solid fa-spinner fa-spin" />
            <p>加载中...</p>
          </div>

          <div
            v-else
            class="sources-content"
          >
            <!-- Source List -->
            <div class="source-list">
              <div
                v-for="marketplace in marketplaceStore.marketplaces"
                :key="marketplace.name"
                class="source-card clickable"
                @click="enterMarketplace(marketplace)"
              >
                <div class="source-icon">
                  <i class="fa-solid fa-store" />
                </div>
                <div class="source-info">
                  <h4>{{ marketplace.name }}</h4>
                  <p class="source-url">
                    {{ marketplaceStore.getSourceDisplay(marketplace.source) }}
                  </p>
                  <div class="source-meta">
                    <span>
                      <i class="fa-solid fa-puzzle-piece" />
                      {{ marketplace.pluginCount }} 个插件
                    </span>
                    <span v-if="marketplace.version">
                      <i class="fa-solid fa-code-branch" />
                      v{{ marketplace.version }}
                    </span>
                    <span>
                      <i class="fa-solid fa-clock" />
                      {{ marketplaceStore.formatLastUpdated(marketplace.lastUpdated) }}
                    </span>
                  </div>
                </div>
                <div class="source-actions">
                  <button
                    class="action-btn update"
                    :disabled="marketplaceStore.updating === marketplace.name"
                    @click="handleUpdateMarketplace(marketplace.name, $event)"
                  >
                    <i
                      class="fa-solid"
                      :class="marketplaceStore.updating === marketplace.name ? 'fa-spinner fa-spin' : 'fa-rotate'"
                    />
                  </button>
                  <button
                    class="action-btn delete"
                    @click="handleRemoveMarketplace(marketplace.name, $event)"
                  >
                    <i class="fa-solid fa-trash" />
                  </button>
                </div>
              </div>

              <div
                v-if="marketplaceStore.marketplaces.length === 0"
                class="empty-state"
              >
                <i class="fa-solid fa-store" />
                <p>暂无市场源，请添加一个</p>
              </div>
            </div>

            <!-- Actions -->
            <div class="sources-footer">
              <button
                class="add-btn"
                @click="showAddDialog = true"
              >
                <i class="fa-solid fa-plus" />
                添加市场源
              </button>
              <button
                v-if="marketplaceStore.marketplaces.length > 0"
                class="update-all-btn"
                :disabled="marketplaceStore.updating !== null"
                @click="handleUpdateAll"
              >
                <i class="fa-solid fa-rotate" />
                全部更新
              </button>
              <button
                class="repair-btn"
                :disabled="marketplaceStore.repairing"
                @click="handleRepairRegistry"
              >
                <i
                  class="fa-solid"
                  :class="marketplaceStore.repairing ? 'fa-spinner fa-spin' : 'fa-wrench'"
                />
                {{ marketplaceStore.repairing ? '修复中...' : '修复' }}
              </button>
            </div>
          </div>
        </template>

        <!-- Plugins View (二级界面) -->
        <template v-else>
          <div
            v-if="loadingPlugins"
            class="loading-state"
          >
            <i class="fa-solid fa-spinner fa-spin" />
            <p>加载插件列表...</p>
          </div>

          <div
            v-else-if="filteredPlugins.length === 0"
            class="empty-state"
          >
            <i class="fa-solid fa-puzzle-piece" />
            <p v-if="marketplacePlugins.length === 0">
              该市场源暂无插件
            </p>
            <p v-else>
              未找到匹配的插件
            </p>
          </div>

          <div
            v-else
            class="plugin-list"
          >
            <div
              v-for="plugin in filteredPlugins"
              :key="plugin.name"
              class="plugin-card"
              :class="{
                installed: isPluginInstalled(plugin.name)
              }"
            >
              <div class="plugin-icon">
                <img
                  v-if="pluginIcons[plugin.name]"
                  :src="pluginIcons[plugin.name]"
                  :alt="plugin.name"
                  class="icon-image"
                >
                <i
                  v-else
                  class="fa-solid fa-puzzle-piece"
                />
                <span
                  v-if="isPluginInstalled(plugin.name)"
                  class="installed-badge"
                >
                  <i class="fa-solid fa-check" />
                </span>
              </div>
              <div class="plugin-info">
                <div class="plugin-header">
                  <h4>{{ plugin.name }}</h4>
                  <span
                    v-if="isPluginInstalled(plugin.name)"
                    class="status-tag installed"
                  >已安装</span>
                  <span
                    v-if="hasPluginUpdate(plugin)"
                    class="status-tag update"
                  >有更新</span>
                </div>
                <p>{{ plugin.description }}</p>
                <div class="plugin-meta">
                  <span v-if="plugin.version">
                    <i class="fa-solid fa-code-branch" />
                    v{{ plugin.version }}
                  </span>
                  <span v-if="plugin.author?.name">
                    <i class="fa-solid fa-user" />
                    {{ plugin.author.name }}
                  </span>
                </div>
                <div
                  v-if="plugin.tags.length > 0"
                  class="plugin-tags"
                >
                  <span
                    v-for="tag in plugin.tags.slice(0, 5)"
                    :key="tag"
                    class="tag"
                  >
                    {{ tag }}
                  </span>
                </div>
              </div>
              <div class="plugin-actions">
                <!-- Not installed: show install button -->
                <template v-if="!isPluginInstalled(plugin.name)">
                  <button
                    class="action-btn install"
                    :disabled="installingPlugin === plugin.name"
                    @click="handleInstallPlugin(plugin)"
                  >
                    <i
                      class="fa-solid"
                      :class="installingPlugin === plugin.name ? 'fa-spinner fa-spin' : 'fa-download'"
                    />
                    {{ installingPlugin === plugin.name ? '安装中' : '安装' }}
                  </button>
                </template>

                <!-- Installed: show toggle, update, uninstall buttons -->
                <template v-else>
                  <!-- Enable/Disable toggle -->
                  <div class="toggle-wrapper">
                    <label class="toggle-switch">
                      <input
                        type="checkbox"
                        :checked="isPluginEnabled(plugin.name)"
                        :disabled="togglingPlugin === plugin.name"
                        @change="handleTogglePlugin(plugin)"
                      >
                      <span class="toggle-slider" />
                    </label>
                    <span class="toggle-label">{{ isPluginEnabled(plugin.name) ? '已启用' : '已停用' }}</span>
                  </div>
                  <button
                    v-if="hasPluginUpdate(plugin)"
                    class="action-btn update"
                    :disabled="installingPlugin === plugin.name"
                    @click="handleUpdatePlugin(plugin)"
                  >
                    <i
                      class="fa-solid"
                      :class="installingPlugin === plugin.name ? 'fa-spinner fa-spin' : 'fa-rotate'"
                    />
                    {{ installingPlugin === plugin.name ? '更新中' : '更新' }}
                  </button>
                  <button
                    class="action-btn uninstall"
                    :disabled="uninstallingPlugin === plugin.name"
                    @click="handleUninstallPlugin(plugin)"
                  >
                    <i
                      class="fa-solid"
                      :class="uninstallingPlugin === plugin.name ? 'fa-spinner fa-spin' : 'fa-trash'"
                    />
                    {{ uninstallingPlugin === plugin.name ? '卸载中' : '卸载' }}
                  </button>
                </template>
              </div>
            </div>
          </div>
        </template>
      </div>
    </template>

    <!-- Skills Tab Content -->
    <template v-if="activeTab === 'skills'">
      <div class="skills-header">
        <div class="header-title">
          <h3>
            <i class="fa-solid fa-wand-magic-sparkles" />
            Skill 管理
          </h3>
          <span class="skill-count">{{ localSkills.length + pluginSkills.length }} 个技能</span>
        </div>
        <button
          class="refresh-btn"
          :disabled="loadingSkills || loadingPluginSkills"
          @click="() => { loadLocalSkills(); loadPluginSkills(); }"
        >
          <i
            class="fa-solid"
            :class="loadingSkills || loadingPluginSkills ? 'fa-spinner fa-spin' : 'fa-rotate'"
          />
          刷新
        </button>
      </div>

      <!-- Skill Sub-tabs -->
      <div class="skill-sub-tabs">
        <button
          class="sub-tab"
          :class="{ active: activeSkillTab === 'local' }"
          @click="activeSkillTab = 'local'"
        >
          <i class="fa-solid fa-folder" />
          本地
          <span class="count">{{ localSkills.length }}</span>
        </button>
        <button
          v-for="mpName in skillMarketplaces"
          :key="mpName"
          class="sub-tab"
          :class="{ active: activeSkillTab === mpName }"
          @click="activeSkillTab = mpName"
        >
          <i class="fa-solid fa-store" />
          {{ mpName }}
          <span class="count">{{ pluginSkills.filter(s => s.marketplace_name === mpName).length }}</span>
        </button>
      </div>

      <div class="skills-content">
        <!-- Local Skills (when local tab is active) -->
        <template v-if="activeSkillTab === 'local'">
          <div
            v-if="loadingSkills"
            class="loading-state"
          >
            <i class="fa-solid fa-spinner fa-spin" />
            <p>加载中...</p>
          </div>

          <div
            v-else-if="localSkills.length === 0"
            class="empty-state"
          >
            <i class="fa-solid fa-wand-magic-sparkles" />
            <p>暂无本地 Skills</p>
            <p class="hint">
              在 ~/.claude/skills 目录下创建包含 SKILL.md 的文件夹
            </p>
          </div>

          <div
            v-else
            class="skill-list"
          >
            <div
              v-for="skill in localSkills"
              :key="skill.name"
              class="skill-card"
            >
              <div class="skill-icon local">
                <i class="fa-solid fa-wand-magic-sparkles" />
              </div>
              <div class="skill-info">
                <h4>{{ skill.title }}</h4>
                <p>{{ skill.description || '暂无描述' }}</p>
                <div class="skill-meta">
                  <span class="skill-name">
                    <i class="fa-solid fa-folder" />
                    {{ skill.name }}
                  </span>
                </div>
              </div>
              <div class="skill-actions">
                <button
                  class="action-btn open-dir"
                  @click="handleOpenSkillDirectory(skill)"
                >
                  <i class="fa-solid fa-folder-open" />
                  打开目录
                </button>
                <button
                  v-if="!hasSkillShortcut(skill.name)"
                  class="action-btn shortcut"
                  :disabled="creatingSkillShortcut === skill.name"
                  @click="handleCreateSkillShortcut(skill)"
                >
                  <i
                    class="fa-solid"
                    :class="creatingSkillShortcut === skill.name ? 'fa-spinner fa-spin' : 'fa-wand-magic-sparkles'"
                  />
                  快捷使用
                </button>
                <button
                  v-else
                  class="action-btn shortcut-remove"
                  @click="handleRemoveSkillShortcut(skill)"
                >
                  <i class="fa-solid fa-xmark" />
                  移除快捷
                </button>
              </div>
            </div>
          </div>
        </template>

        <!-- Plugin Skills (when marketplace tab is active) -->
        <template v-else>
          <div
            v-if="loadingPluginSkills"
            class="loading-state"
          >
            <i class="fa-solid fa-spinner fa-spin" />
            <p>加载中...</p>
          </div>

          <div
            v-else-if="filteredPluginSkills.length === 0"
            class="empty-state"
          >
            <i class="fa-solid fa-puzzle-piece" />
            <p>该市场暂无 Skills</p>
          </div>

          <div
            v-else
            class="skill-list"
          >
            <div
              v-for="skill in filteredPluginSkills"
              :key="`${skill.marketplace_name}-${skill.plugin_name}-${skill.name}`"
              class="skill-card"
            >
              <div class="skill-icon plugin">
                <i class="fa-solid fa-puzzle-piece" />
              </div>
              <div class="skill-info">
                <h4>{{ skill.title }}</h4>
                <p>{{ skill.description || '暂无描述' }}</p>
                <div class="skill-meta">
                  <span class="skill-name">
                    <i class="fa-solid fa-cube" />
                    {{ skill.plugin_name }}
                  </span>
                </div>
              </div>
              <div class="skill-actions">
                <button
                  class="action-btn view-plugin"
                  @click="handleViewPlugin(skill)"
                >
                  <i class="fa-solid fa-eye" />
                  查看插件
                </button>
                <button
                  v-if="!hasPluginSkillShortcut(skill.name)"
                  class="action-btn shortcut"
                  :disabled="creatingPluginSkillShortcut === skill.name"
                  @click="handleCreatePluginSkillShortcut(skill)"
                >
                  <i
                    class="fa-solid"
                    :class="creatingPluginSkillShortcut === skill.name ? 'fa-spinner fa-spin' : 'fa-wand-magic-sparkles'"
                  />
                  快捷使用
                </button>
                <button
                  v-else
                  class="action-btn shortcut-remove"
                  @click="handleRemovePluginSkillShortcut(skill)"
                >
                  <i class="fa-solid fa-xmark" />
                  移除快捷
                </button>
              </div>
            </div>
          </div>
        </template>
      </div>
    </template>

    <!-- Agents Tab Content -->
    <template v-if="activeTab === 'agents'">
      <div class="agents-header">
        <div class="header-title">
          <h3>
            <i class="fa-solid fa-robot" />
            Agent 管理
          </h3>
          <span class="agent-count">{{ localAgents.length + pluginAgents.length }} 个代理</span>
        </div>
        <button
          class="refresh-btn"
          :disabled="loadingAgents || loadingPluginAgents"
          @click="() => { loadLocalAgents(); loadPluginAgents(); }"
        >
          <i
            class="fa-solid"
            :class="loadingAgents || loadingPluginAgents ? 'fa-spinner fa-spin' : 'fa-rotate'"
          />
          刷新
        </button>
      </div>

      <!-- Agent Sub-tabs -->
      <div class="agent-sub-tabs">
        <button
          class="sub-tab"
          :class="{ active: activeAgentTab === 'local' }"
          @click="activeAgentTab = 'local'"
        >
          <i class="fa-solid fa-folder" />
          本地
          <span class="count">{{ localAgents.length }}</span>
        </button>
        <button
          v-for="mpName in agentMarketplaces"
          :key="mpName"
          class="sub-tab"
          :class="{ active: activeAgentTab === mpName }"
          @click="activeAgentTab = mpName"
        >
          <i class="fa-solid fa-store" />
          {{ mpName }}
          <span class="count">{{ pluginAgents.filter(a => a.marketplace_name === mpName).length }}</span>
        </button>
      </div>

      <div class="agents-content">
        <!-- Local Agents (when local tab is active) -->
        <template v-if="activeAgentTab === 'local'">
          <div
            v-if="loadingAgents"
            class="loading-state"
          >
            <i class="fa-solid fa-spinner fa-spin" />
            <p>加载中...</p>
          </div>

          <div
            v-else-if="localAgents.length === 0"
            class="empty-state"
          >
            <i class="fa-solid fa-robot" />
            <p>暂无本地 Agents</p>
            <p class="hint">
              在 ~/.claude/agents 目录下创建 .md 文件
            </p>
          </div>

          <div
            v-else
            class="agent-list"
          >
            <div
              v-for="agent in localAgents"
              :key="agent.name"
              class="agent-card"
            >
              <div class="agent-icon local">
                <i class="fa-solid fa-robot" />
              </div>
              <div class="agent-info">
                <h4>{{ agent.title }}</h4>
                <p>{{ agent.description || '暂无描述' }}</p>
                <div class="agent-meta">
                  <span class="agent-file">
                    <i class="fa-solid fa-file" />
                    {{ agent.name }}.md
                  </span>
                </div>
              </div>
              <div class="agent-actions">
                <button
                  class="action-btn open-dir"
                  @click="handleOpenAgentsDirectory(agent)"
                >
                  <i class="fa-solid fa-folder-open" />
                  打开目录
                </button>
              </div>
            </div>
          </div>
        </template>

        <!-- Plugin Agents (when marketplace tab is active) -->
        <template v-else>
          <div
            v-if="loadingPluginAgents"
            class="loading-state"
          >
            <i class="fa-solid fa-spinner fa-spin" />
            <p>加载中...</p>
          </div>

          <div
            v-else-if="filteredPluginAgents.length === 0"
            class="empty-state"
          >
            <i class="fa-solid fa-robot" />
            <p>该市场暂无 Agents</p>
          </div>

          <div
            v-else
            class="agent-list"
          >
            <div
              v-for="agent in filteredPluginAgents"
              :key="`${agent.marketplace_name}-${agent.plugin_name}-${agent.name}`"
              class="agent-card"
            >
              <div class="agent-icon plugin">
                <i class="fa-solid fa-robot" />
              </div>
              <div class="agent-info">
                <h4>{{ agent.title }}</h4>
                <p>{{ agent.description || '暂无描述' }}</p>
                <div class="agent-meta">
                  <span class="agent-file">
                    <i class="fa-solid fa-cube" />
                    {{ agent.plugin_name }}
                  </span>
                </div>
              </div>
              <div class="agent-actions">
                <button
                  class="action-btn view-plugin"
                  @click="handleViewAgentPlugin(agent)"
                >
                  <i class="fa-solid fa-eye" />
                  查看插件
                </button>
              </div>
            </div>
          </div>
        </template>
      </div>
    </template>

    <!-- MCP Tab Content -->
    <template v-if="activeTab === 'mcp'">
      <McpManager />
    </template>

    <!-- Add Marketplace Dialog -->
    <Teleport to="body">
      <div
        v-if="showAddDialog"
        class="dialog-overlay"
        @click="closeAddDialog"
      >
        <div
          class="dialog"
          @click.stop
        >
          <div class="dialog-header">
            <h3>添加市场源</h3>
            <button
              class="close-btn"
              @click="closeAddDialog"
            >
              <i class="fa-solid fa-xmark" />
            </button>
          </div>
          <div class="dialog-body">
            <div class="source-type-selector">
              <label :class="{ active: addSourceType === 'github' }">
                <input
                  v-model="addSourceType"
                  type="radio"
                  value="github"
                >
                <i class="fa-brands fa-github" />
                GitHub 仓库
              </label>
              <label :class="{ active: addSourceType === 'url' }">
                <input
                  v-model="addSourceType"
                  type="radio"
                  value="url"
                >
                <i class="fa-solid fa-link" />
                自定义 URL
              </label>
            </div>

            <div class="input-group">
              <label v-if="addSourceType === 'github'">
                仓库地址 (owner/repo)
              </label>
              <label v-else>
                marketplace.json URL
              </label>
              <input
                v-model="addSourceValue"
                type="text"
                :placeholder="addSourceType === 'github' ? 'anthropics/skills' : 'https://example.com/.claude-plugin/marketplace.json'"
                @keyup.enter="handleAddMarketplace"
              >
            </div>

            <div
              v-if="addSourceType === 'github'"
              class="recommended-source"
            >
              <span class="recommend-label">推荐:</span>
              <button
                class="recommend-btn"
                @click="addSourceValue = 'anthropics/claude-plugins-official'"
              >
                <i class="fa-brands fa-github" />
                anthropics/claude-plugins-official
              </button>
            </div>

            <div
              v-if="addError"
              class="error-message"
            >
              <i class="fa-solid fa-circle-exclamation" />
              {{ addError }}
            </div>
          </div>
          <div class="dialog-footer">
            <button
              class="btn-cancel"
              @click="closeAddDialog"
            >
              取消
            </button>
            <button
              class="btn-confirm"
              :disabled="addLoading"
              @click="handleAddMarketplace"
            >
              <i
                v-if="addLoading"
                class="fa-solid fa-spinner fa-spin"
              />
              {{ addLoading ? '添加中...' : '添加' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Skill Selection Dialog -->
    <Teleport to="body">
      <div
        v-if="showSkillDialog && skillDialogPlugin"
        class="dialog-overlay"
        @click="closeSkillDialog"
      >
        <div
          class="dialog skill-dialog"
          @click.stop
        >
          <div class="dialog-header">
            <h3>选择技能</h3>
            <button
              class="close-btn"
              @click="closeSkillDialog"
            >
              <i class="fa-solid fa-xmark" />
            </button>
          </div>
          <div class="dialog-body">
            <p class="skill-dialog-hint">
              <i class="fa-solid fa-info-circle" />
              选择要创建桌面快捷方式的技能
            </p>
            <div class="skill-list">
              <div
                v-for="skill in skillDialogPlugin.skills"
                :key="skill"
                class="skill-item"
                @click="handleSelectSkill(skill)"
              >
                <i class="fa-solid fa-wand-magic-sparkles" />
                <span class="skill-name">{{ getCleanSkillName(skill) }}</span>
                <i class="fa-solid fa-chevron-right" />
              </div>
            </div>
          </div>
          <div class="dialog-footer">
            <button
              class="btn-cancel"
              @click="closeSkillDialog"
            >
              取消
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Repair Result Dialog -->
    <Teleport to="body">
      <div
        v-if="showRepairDialog && repairResult"
        class="dialog-overlay"
        @click="closeRepairDialog"
      >
        <div
          class="dialog repair-dialog"
          @click.stop
        >
          <div class="dialog-header">
            <h3>
              <i class="fa-solid fa-wrench" />
              修复完成
            </h3>
            <button
              class="close-btn"
              @click="closeRepairDialog"
            >
              <i class="fa-solid fa-xmark" />
            </button>
          </div>
          <div class="dialog-body">
            <div class="repair-summary">
              <div class="summary-item">
                <span class="label">已安装插件数</span>
                <span class="value">{{ repairResult.totalInstalled }}</span>
              </div>
              <div class="summary-item">
                <span class="label">安装记录修复</span>
                <span
                  class="value"
                  :class="{ highlight: repairResult.installedPluginsFixed > 0 }"
                >
                  {{ repairResult.installedPluginsFixed }}
                </span>
              </div>
              <div class="summary-item">
                <span class="label">启用设置修复</span>
                <span
                  class="value"
                  :class="{ highlight: repairResult.settingsFixed > 0 }"
                >
                  {{ repairResult.settingsFixed }}
                </span>
              </div>
            </div>

            <div
              v-if="repairResult.actions.length > 0"
              class="repair-actions"
            >
              <h4>修复操作详情</h4>
              <div class="action-list">
                <div
                  v-for="(action, index) in repairResult.actions"
                  :key="index"
                  class="action-item"
                  :class="action.action.toLowerCase()"
                >
                  <i
                    class="fa-solid"
                    :class="{
                      'fa-plus': action.action === 'Added',
                      'fa-minus': action.action === 'Removed',
                      'fa-check': action.action === 'Fixed'
                    }"
                  />
                  <span class="action-plugin">{{ action.pluginKey }}</span>
                  <span class="action-detail">{{ action.details }}</span>
                </div>
              </div>
            </div>

            <div
              v-else
              class="no-actions"
            >
              <i class="fa-solid fa-check-circle" />
              <p>所有配置均已同步，无需修复</p>
            </div>
          </div>
          <div class="dialog-footer">
            <button
              class="btn-confirm"
              @click="closeRepairDialog"
            >
              确定
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style lang="less" scoped>
.plugin-market {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #f5f5f5;
}

.tab-bar {
  display: flex;
  background: white;
  border-bottom: 1px solid #e0e0e0;
  padding: 0 16px;
  flex-shrink: 0;

  .tab-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 14px 20px;
    border: none;
    background: transparent;
    cursor: pointer;
    font-size: 14px;
    color: #666;
    position: relative;
    transition: color 0.2s;

    i {
      font-size: 16px;
    }

    &:hover {
      color: #7c6eb8;
    }

    &.active {
      color: #7c6eb8;
      font-weight: 500;

      &::after {
        content: '';
        position: absolute;
        bottom: 0;
        left: 0;
        right: 0;
        height: 2px;
        background: #7c6eb8;
      }
    }
  }
}

.tab-content-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #999;
  padding: 60px 20px;

  i {
    font-size: 64px;
    margin-bottom: 20px;
    color: #ddd;
  }

  h3 {
    margin: 0 0 8px 0;
    font-size: 18px;
    font-weight: 500;
    color: #666;
  }

  p {
    margin: 0;
    font-size: 14px;
  }
}

// Skills Tab Styles
.skills-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px;
  background: white;
  border-bottom: 1px solid #e0e0e0;

  .header-title {
    display: flex;
    align-items: center;
    gap: 12px;

    h3 {
      margin: 0;
      font-size: 16px;
      font-weight: 500;
      display: flex;
      align-items: center;
      gap: 8px;
      color: #333;

      i {
        color: #7c6eb8;
      }
    }

    .skill-count {
      font-size: 13px;
      color: #888;
      background: #f0f0f0;
      padding: 4px 10px;
      border-radius: 12px;
    }

    .skills-path {
      font-size: 12px;
      color: #888;
      font-family: monospace;
      background: #f0f0f0;
      padding: 4px 8px;
      border-radius: 4px;
    }
  }

  .refresh-btn {
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    display: flex;
    align-items: center;
    gap: 6px;
    background: #f0ecf8;
    color: #7c6eb8;
    transition: background 0.2s;

    &:hover:not(:disabled) {
      background: #e0d8f0;
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }
}

.skill-sub-tabs {
  display: flex;
  gap: 8px;
  padding: 12px 16px;
  background: white;
  border-bottom: 1px solid #e0e0e0;
  overflow-x: auto;
  flex-shrink: 0;

  &::-webkit-scrollbar {
    height: 4px;
  }

  &::-webkit-scrollbar-thumb {
    background: #ddd;
    border-radius: 2px;
  }

  .sub-tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border: 1px solid #e0e0e0;
    border-radius: 20px;
    background: white;
    cursor: pointer;
    font-size: 13px;
    color: #666;
    white-space: nowrap;
    transition: all 0.2s;

    i {
      font-size: 12px;
    }

    .count {
      background: #f0f0f0;
      padding: 2px 8px;
      border-radius: 10px;
      font-size: 11px;
      font-weight: 500;
    }

    &:hover {
      border-color: #7c6eb8;
      color: #7c6eb8;
    }

    &.active {
      background: #7c6eb8;
      border-color: #7c6eb8;
      color: white;

      .count {
        background: rgba(255, 255, 255, 0.2);
        color: white;
      }
    }
  }
}

.skills-content {
  flex: 1;
  overflow: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.skill-section {
  .section-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
    padding-bottom: 12px;
    border-bottom: 1px solid #e0e0e0;

    h4 {
      margin: 0;
      font-size: 15px;
      font-weight: 500;
      display: flex;
      align-items: center;
      gap: 8px;
      color: #333;

      i {
        color: #7c6eb8;
      }
    }

    .skills-path {
      font-size: 12px;
      color: #888;
      font-family: monospace;
      background: #f0f0f0;
      padding: 4px 8px;
      border-radius: 4px;
    }
  }
}

.loading-state.small,
.empty-state.small {
  padding: 30px 20px;

  i {
    font-size: 32px;
    margin-bottom: 12px;
  }

  p {
    font-size: 13px;
  }
}

.skill-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.skill-card {
  display: flex;
  gap: 16px;
  padding: 20px;
  background: white;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  transition: box-shadow 0.2s;

  &:hover {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }

  .skill-icon {
    width: 56px;
    height: 56px;
    background: linear-gradient(135deg, #7c6eb8 0%, #a490c4 100%);
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;

    i {
      font-size: 24px;
      color: white;
    }

    &.local {
      background: linear-gradient(135deg, #7c6eb8 0%, #a490c4 100%);
    }

    &.plugin {
      background: linear-gradient(135deg, #22c55e 0%, #4ade80 100%);
    }
  }

  .skill-info {
    flex: 1;
    min-width: 0;

    h4 {
      margin: 0 0 6px 0;
      font-size: 16px;
      font-weight: 500;
      color: #333;
    }

    p {
      margin: 0 0 10px 0;
      font-size: 13px;
      color: #666;
      line-height: 1.4;
    }

    .skill-meta {
      display: flex;
      flex-wrap: wrap;
      gap: 12px;

      .skill-name {
        display: flex;
        align-items: center;
        gap: 4px;
        font-size: 12px;
        color: #888;
        font-family: monospace;

        i {
          color: #999;
        }
      }
    }
  }

  .skill-actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    justify-content: center;

    .action-btn {
      padding: 8px 16px;
      border: none;
      border-radius: 6px;
      cursor: pointer;
      font-size: 13px;
      display: flex;
      align-items: center;
      gap: 6px;
      white-space: nowrap;
      transition: background 0.2s;

      &.open-dir {
        background: #f0f0f0;
        color: #666;

        &:hover {
          background: #e0e0e0;
          color: #333;
        }
      }

      &.shortcut {
        background: #e0f2fe;
        color: #0284c7;

        &:hover:not(:disabled) {
          background: #bae6fd;
        }

        &:disabled {
          opacity: 0.6;
          cursor: not-allowed;
        }
      }

      &.shortcut-remove {
        background: #fef3c7;
        color: #d97706;

        &:hover {
          background: #fde68a;
        }
      }

      &.view-plugin {
        background: #dcfce7;
        color: #16a34a;

        &:hover {
          background: #bbf7d0;
        }
      }
    }
  }
}

.empty-state .hint {
  font-size: 12px;
  color: #aaa;
  margin-top: 8px;
}

// Agents Tab Styles
.agents-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px;
  background: white;
  border-bottom: 1px solid #e0e0e0;

  .header-title {
    display: flex;
    align-items: center;
    gap: 12px;

    h3 {
      margin: 0;
      font-size: 16px;
      font-weight: 500;
      display: flex;
      align-items: center;
      gap: 8px;
      color: #333;

      i {
        color: #7c6eb8;
      }
    }

    .agents-path {
      font-size: 12px;
      color: #888;
      font-family: monospace;
      background: #f0f0f0;
      padding: 4px 8px;
      border-radius: 4px;
    }
  }

  .refresh-btn {
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    display: flex;
    align-items: center;
    gap: 6px;
    background: #f0ecf8;
    color: #7c6eb8;
    transition: background 0.2s;

    &:hover:not(:disabled) {
      background: #e0d8f0;
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }
}

.agent-sub-tabs {
  display: flex;
  gap: 8px;
  padding: 12px 16px;
  background: white;
  border-bottom: 1px solid #e0e0e0;
  overflow-x: auto;
  flex-shrink: 0;

  &::-webkit-scrollbar {
    height: 4px;
  }

  &::-webkit-scrollbar-thumb {
    background: #ddd;
    border-radius: 2px;
  }

  .sub-tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border: 1px solid #e0e0e0;
    border-radius: 20px;
    background: white;
    cursor: pointer;
    font-size: 13px;
    color: #666;
    white-space: nowrap;
    transition: all 0.2s;

    i {
      font-size: 12px;
    }

    .count {
      background: #f0f0f0;
      padding: 2px 8px;
      border-radius: 10px;
      font-size: 11px;
      font-weight: 500;
    }

    &:hover {
      border-color: #5b8def;
      color: #5b8def;
    }

    &.active {
      background: #5b8def;
      border-color: #5b8def;
      color: white;

      .count {
        background: rgba(255, 255, 255, 0.2);
        color: white;
      }
    }
  }
}

.agents-content {
  flex: 1;
  overflow: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.agent-section {
  .section-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
    padding-bottom: 12px;
    border-bottom: 1px solid #e0e0e0;

    h4 {
      margin: 0;
      font-size: 15px;
      font-weight: 500;
      display: flex;
      align-items: center;
      gap: 8px;
      color: #333;

      i {
        color: #5b8def;
      }
    }

    .agents-path {
      font-size: 12px;
      color: #888;
      font-family: monospace;
      background: #f0f0f0;
      padding: 4px 8px;
      border-radius: 4px;
    }
  }
}

.agent-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.agent-card {
  display: flex;
  gap: 16px;
  padding: 20px;
  background: white;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  transition: box-shadow 0.2s;

  &:hover {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }

  .agent-icon {
    width: 56px;
    height: 56px;
    background: linear-gradient(135deg, #5b8def 0%, #7ba4f4 100%);
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;

    i {
      font-size: 24px;
      color: white;
    }

    &.local {
      background: linear-gradient(135deg, #5b8def 0%, #7ba4f4 100%);
    }

    &.plugin {
      background: linear-gradient(135deg, #f59e0b 0%, #fbbf24 100%);
    }
  }

  .agent-info {
    flex: 1;
    min-width: 0;

    h4 {
      margin: 0 0 6px 0;
      font-size: 16px;
      font-weight: 500;
      color: #333;
    }

    p {
      margin: 0 0 10px 0;
      font-size: 13px;
      color: #666;
      line-height: 1.4;
    }

    .agent-meta {
      display: flex;
      flex-wrap: wrap;
      gap: 12px;

      .agent-file {
        display: flex;
        align-items: center;
        gap: 4px;
        font-size: 12px;
        color: #888;
        font-family: monospace;

        i {
          color: #999;
        }
      }
    }
  }

  .agent-actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    justify-content: center;

    .action-btn {
      padding: 8px 16px;
      border: none;
      border-radius: 6px;
      cursor: pointer;
      font-size: 13px;
      display: flex;
      align-items: center;
      gap: 6px;
      white-space: nowrap;
      transition: background 0.2s;

      &.open-dir {
        background: #f0f0f0;
        color: #666;

        &:hover {
          background: #e0e0e0;
          color: #333;
        }
      }
    }
  }
}

.market-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px;
  background: white;
  border-bottom: 1px solid #e0e0e0;

  .back-btn {
    width: 36px;
    height: 36px;
    border: none;
    background: #f0f0f0;
    border-radius: 8px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #666;
    flex-shrink: 0;

    &:hover {
      background: #e0e0e0;
      color: #333;
    }
  }

  .header-title {
    flex-shrink: 0;

    h3 {
      margin: 0;
      font-size: 16px;
      font-weight: 500;
      display: flex;
      align-items: center;
      gap: 8px;
      color: #333;

      i {
        color: #7c6eb8;
      }
    }
  }

  .search-box {
    flex: 1;
    position: relative;
    max-width: 300px;
    margin-left: auto;

    i {
      position: absolute;
      left: 12px;
      top: 50%;
      transform: translateY(-50%);
      color: #999;
    }

    input {
      width: 100%;
      padding: 10px 12px 10px 36px;
      border: 1px solid #ddd;
      border-radius: 6px;
      font-size: 14px;
      outline: none;

      &:focus {
        border-color: #7c6eb8;
      }
    }
  }
}

.market-content {
  flex: 1;
  overflow: auto;
  padding: 16px;
}

.plugin-list,
.source-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.source-card {
  display: flex;
  gap: 16px;
  padding: 20px;
  background: white;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  transition: box-shadow 0.2s, transform 0.2s;

  &.clickable {
    cursor: pointer;

    &:hover {
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
      transform: translateY(-2px);
    }
  }
}

.plugin-card {
  display: flex;
  gap: 16px;
  padding: 20px;
  background: white;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  transition: box-shadow 0.2s;

  &:hover {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }

  &.installed {
    border-left: 3px solid #22c55e;
  }
}

.plugin-icon {
  width: 56px;
  height: 56px;
  background: linear-gradient(135deg, #7c6eb8 0%, #a490c4 100%);
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  position: relative;

  .icon-image {
    width: 100%;
    height: 100%;
    border-radius: 12px;
    object-fit: cover;
  }

  i {
    font-size: 24px;
    color: white;
  }

  .installed-badge {
    position: absolute;
    bottom: -4px;
    right: -4px;
    width: 20px;
    height: 20px;
    background: #22c55e;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 2px solid white;

    i {
      font-size: 10px;
      color: white;
    }
  }
}

.source-icon {
  width: 56px;
  height: 56px;
  background: linear-gradient(135deg, #7c6eb8 0%, #a490c4 100%);
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;

  i {
    font-size: 24px;
    color: white;
  }
}

.plugin-info {
  flex: 1;
  min-width: 0;

  .plugin-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;

    h4 {
      margin: 0;
      font-size: 16px;
      font-weight: 500;
      color: #333;
    }

    .status-tag {
      font-size: 11px;
      padding: 2px 8px;
      border-radius: 4px;
      font-weight: 500;

      &.installed {
        background: #dcfce7;
        color: #16a34a;
      }

      &.update {
        background: #fef3c7;
        color: #d97706;
      }
    }
  }

  p {
    margin: 0 0 10px 0;
    font-size: 13px;
    color: #666;
    line-height: 1.4;
  }
}

.source-info {
  flex: 1;
  min-width: 0;

  h4 {
    margin: 0 0 6px 0;
    font-size: 16px;
    font-weight: 500;
    color: #333;
  }

  p {
    margin: 0 0 10px 0;
    font-size: 13px;
    color: #666;
    line-height: 1.4;
  }

  .source-url {
    font-family: monospace;
    font-size: 12px;
    color: #888;
  }
}

.plugin-meta,
.source-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 16px;

  span {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: #888;

    i {
      color: #999;
    }
  }

  .marketplace-badge {
    background: #f0ecf8;
    color: #7c6eb8;
    padding: 2px 8px;
    border-radius: 4px;

    i {
      color: #7c6eb8;
    }
  }
}

.plugin-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 10px;

  .tag {
    font-size: 11px;
    padding: 2px 8px;
    background: #f0f0f0;
    color: #666;
    border-radius: 4px;
  }
}

.source-actions {
  display: flex;
  gap: 8px;
  align-items: center;

  .action-btn {
    width: 36px;
    height: 36px;
    padding: 0;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.2s;

    &.update {
      background: #f0ecf8;
      color: #7c6eb8;

      &:hover:not(:disabled) {
        background: #e0d8f0;
      }

      &:disabled {
        opacity: 0.6;
        cursor: not-allowed;
      }
    }

    &.delete {
      background: #fee;
      color: #d00;

      &:hover {
        background: #fdd;
      }
    }
  }
}

.plugin-actions {
  display: flex;
  flex-direction: column;
  gap: 8px;
  justify-content: center;

  .action-btn {
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
    transition: background 0.2s;

    &.install {
      background: #7c6eb8;
      color: white;

      &:hover:not(:disabled) {
        background: #6a5ca0;
      }

      &:disabled {
        opacity: 0.6;
        cursor: not-allowed;
      }
    }

    &.update {
      background: #f0ecf8;
      color: #7c6eb8;

      &:hover:not(:disabled) {
        background: #e0d8f0;
      }

      &:disabled {
        opacity: 0.6;
        cursor: not-allowed;
      }
    }

    &.uninstall {
      background: #fee;
      color: #d00;

      &:hover:not(:disabled) {
        background: #fdd;
      }

      &:disabled {
        opacity: 0.6;
        cursor: not-allowed;
      }
    }

    &.shortcut {
      background: #e0f2fe;
      color: #0284c7;

      &:hover:not(:disabled) {
        background: #bae6fd;
      }

      &:disabled {
        opacity: 0.6;
        cursor: not-allowed;
      }
    }

    &.shortcut-remove {
      background: #fef3c7;
      color: #d97706;

      &:hover:not(:disabled) {
        background: #fde68a;
      }

      &:disabled {
        opacity: 0.6;
        cursor: not-allowed;
      }
    }
  }

  .toggle-wrapper {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 0;

    .toggle-label {
      font-size: 12px;
      color: #666;
      min-width: 45px;
    }
  }

  .toggle-switch {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 24px;

    input {
      opacity: 0;
      width: 0;
      height: 0;
    }

    .toggle-slider {
      position: absolute;
      cursor: pointer;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background-color: #ccc;
      transition: 0.3s;
      border-radius: 24px;

      &:before {
        position: absolute;
        content: "";
        height: 18px;
        width: 18px;
        left: 3px;
        bottom: 3px;
        background-color: white;
        transition: 0.3s;
        border-radius: 50%;
      }
    }

    input:checked + .toggle-slider {
      background-color: #22c55e;
    }

    input:checked + .toggle-slider:before {
      transform: translateX(20px);
    }

    input:disabled + .toggle-slider {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }
}

.sources-content {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.sources-footer {
  display: flex;
  gap: 12px;
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid #e0e0e0;

  .add-btn,
  .update-all-btn,
  .repair-btn {
    padding: 10px 20px;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    display: flex;
    align-items: center;
    gap: 8px;
    transition: all 0.2s;
  }

  .add-btn {
    background: #7c6eb8;
    color: white;

    &:hover {
      background: #6a5ca0;
    }
  }

  .update-all-btn {
    background: #f0ecf8;
    color: #7c6eb8;

    &:hover:not(:disabled) {
      background: #e0d8f0;
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }

  .repair-btn {
    background: #f5f5f5;
    color: #666;
    margin-left: auto;
    border: 1px solid #ddd;

    &:hover:not(:disabled) {
      background: #e8e8e8;
      border-color: #ccc;
      color: #333;
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }
}

.loading-state,
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  color: #999;

  i {
    font-size: 48px;
    margin-bottom: 16px;
  }

  p {
    font-size: 14px;
  }
}

// Dialog styles
.dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10010;
}

.dialog {
  background: white;
  border-radius: 12px;
  width: 480px;
  max-width: 90vw;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
}

.dialog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 24px;
  border-bottom: 1px solid #e0e0e0;

  h3 {
    margin: 0;
    font-size: 18px;
    font-weight: 500;
  }

  .close-btn {
    width: 32px;
    height: 32px;
    border: none;
    background: transparent;
    cursor: pointer;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #666;

    &:hover {
      background: #f0f0f0;
    }
  }
}

.dialog-body {
  padding: 24px;

  .source-type-selector {
    display: flex;
    gap: 12px;
    margin-bottom: 20px;

    label {
      flex: 1;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
      padding: 12px;
      border: 2px solid #e0e0e0;
      border-radius: 8px;
      cursor: pointer;
      transition: all 0.2s;

      input {
        display: none;
      }

      i {
        font-size: 18px;
      }

      &.active {
        border-color: #7c6eb8;
        background: #f0ecf8;
        color: #7c6eb8;
      }

      &:hover:not(.active) {
        border-color: #ccc;
      }
    }
  }

  .input-group {
    label {
      display: block;
      font-size: 14px;
      font-weight: 500;
      color: #333;
      margin-bottom: 8px;
    }

    input {
      width: 100%;
      padding: 12px;
      border: 1px solid #ddd;
      border-radius: 6px;
      font-size: 14px;
      outline: none;

      &:focus {
        border-color: #7c6eb8;
      }
    }
  }

  .recommended-source {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;

    .recommend-label {
      font-size: 13px;
      color: #666;
    }

    .recommend-btn {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      padding: 6px 12px;
      background: linear-gradient(135deg, #f5f3ff 0%, #ede9fe 100%);
      border: 1px solid #c4b5fd;
      border-radius: 6px;
      font-size: 13px;
      color: #7c3aed;
      cursor: pointer;
      transition: all 0.2s ease;

      &:hover {
        background: linear-gradient(135deg, #ede9fe 0%, #ddd6fe 100%);
        border-color: #a78bfa;
      }

      i {
        font-size: 14px;
      }
    }
  }

  .error-message {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 16px;
    padding: 12px;
    background: #fef2f2;
    color: #dc2626;
    border-radius: 6px;
    font-size: 13px;
  }
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 16px 24px;
  border-top: 1px solid #e0e0e0;

  button {
    padding: 10px 20px;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    display: flex;
    align-items: center;
    gap: 6px;

    &.btn-cancel {
      background: #f0f0f0;
      color: #666;

      &:hover {
        background: #e0e0e0;
      }
    }

    &.btn-confirm {
      background: #7c6eb8;
      color: white;

      &:hover:not(:disabled) {
        background: #6a5ca0;
      }

      &:disabled {
        opacity: 0.6;
        cursor: not-allowed;
      }
    }
  }
}

// Skill selection dialog styles
.skill-dialog {
  width: 400px;

  .skill-dialog-hint {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 16px 0;
    padding: 12px;
    background: #f0ecf8;
    border-radius: 8px;
    color: #7c6eb8;
    font-size: 13px;

    i {
      flex-shrink: 0;
    }
  }

  .skill-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 300px;
    overflow-y: auto;
  }

  .skill-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    background: #f8f8f8;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s;

    &:hover {
      background: #e8e0f0;
      transform: translateX(4px);
    }

    i:first-child {
      color: #7c6eb8;
      font-size: 16px;
    }

    .skill-name {
      flex: 1;
      font-size: 14px;
      font-weight: 500;
      color: #333;
    }

    i:last-child {
      color: #999;
      font-size: 12px;
    }
  }
}

// Repair dialog styles
.repair-dialog {
  width: 500px;

  .dialog-header h3 {
    display: flex;
    align-items: center;
    gap: 10px;

    i {
      color: #d97706;
    }
  }

  .repair-summary {
    display: flex;
    gap: 16px;
    margin-bottom: 24px;

    .summary-item {
      flex: 1;
      padding: 16px;
      background: #f8f8f8;
      border-radius: 8px;
      text-align: center;

      .label {
        display: block;
        font-size: 12px;
        color: #888;
        margin-bottom: 4px;
      }

      .value {
        display: block;
        font-size: 24px;
        font-weight: 600;
        color: #333;

        &.highlight {
          color: #22c55e;
        }
      }
    }
  }

  .repair-actions {
    h4 {
      margin: 0 0 12px 0;
      font-size: 14px;
      font-weight: 500;
      color: #666;
    }

    .action-list {
      max-height: 240px;
      overflow-y: auto;
    }

    .action-item {
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 10px 12px;
      margin-bottom: 8px;
      border-radius: 6px;
      font-size: 13px;

      &.added {
        background: #dcfce7;

        i {
          color: #22c55e;
        }
      }

      &.removed {
        background: #fef2f2;

        i {
          color: #dc2626;
        }
      }

      &.fixed {
        background: #fef3c7;

        i {
          color: #d97706;
        }
      }

      .action-plugin {
        font-weight: 500;
        color: #333;
      }

      .action-detail {
        flex: 1;
        text-align: right;
        color: #888;
        font-size: 12px;
      }
    }
  }

  .no-actions {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 32px;
    color: #22c55e;

    i {
      font-size: 48px;
      margin-bottom: 12px;
    }

    p {
      margin: 0;
      color: #666;
      font-size: 14px;
    }
  }
}
</style>
