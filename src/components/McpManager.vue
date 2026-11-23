<template>
  <div class="mcp-manager">
    <!-- Tabs -->
    <div class="mcp-tabs">
      <button
        class="tab-btn"
        :class="{ active: activeTab === 'global' }"
        @click="activeTab = 'global'"
      >
        <i class="fa-solid fa-globe" />
        全局 MCP
        <span class="count">{{ globalMcpList.length + pluginMcpList.length }}</span>
      </button>
      <button
        class="tab-btn"
        :class="{ active: activeTab === 'project' }"
        @click="activeTab = 'project'"
      >
        <i class="fa-solid fa-folder" />
        项目 MCP
        <span class="count">{{ projectsWithMcp.length }}</span>
      </button>
      <button
        class="tab-btn"
        :class="{ active: activeTab === 'install' }"
        @click="activeTab = 'install'"
      >
        <i class="fa-solid fa-plus" />
        安装 MCP
      </button>
      <span class="mcp-tabs-spacer" />
      <span class="mcp-count">{{ totalMcpCount }} 个服务器</span>
      <button
        class="refresh-btn"
        :disabled="loading"
        @click="refreshAll"
      >
        <i
          class="fa-solid"
          :class="loading ? 'fa-spinner fa-spin' : 'fa-rotate'"
        />
        刷新
      </button>
    </div>

    <!-- Global MCP Tab -->
    <template v-if="activeTab === 'global'">
      <div class="mcp-content">
        <!-- Global MCPs -->
        <div
          v-if="globalMcpList.length > 0"
          class="mcp-section"
        >
          <div class="section-header">
            <h4>
              <i class="fa-solid fa-globe" />
              全局 MCP
            </h4>
            <span class="section-count">{{ globalMcpList.length }} 个</span>
          </div>
          <div class="mcp-list">
            <div
              v-for="mcp in globalMcpList"
              :key="mcp.name"
              class="mcp-card"
            >
              <div class="mcp-icon global">
                <i class="fa-solid fa-server" />
              </div>
              <div class="mcp-info">
                <h4>{{ mcp.name }}</h4>
                <p class="mcp-config">
                  {{ formatMcpConfig(mcp.config) }}
                </p>
              </div>
              <div class="mcp-actions">
                <label class="toggle-switch">
                  <input
                    type="checkbox"
                    :checked="mcp.enabled"
                    @change="handleToggleGlobal(mcp.name, ($event.target as HTMLInputElement).checked)"
                  >
                  <span class="slider" />
                </label>
                <button
                  class="action-btn delete"
                  @click="handleDeleteGlobal(mcp.name)"
                >
                  <i class="fa-solid fa-trash" />
                  删除
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- Plugin MCPs -->
        <div
          v-for="(mcps, marketplace) in pluginMcpsByMarketplace"
          :key="marketplace"
          class="mcp-section"
        >
          <div class="section-header">
            <h4>
              <i class="fa-solid fa-puzzle-piece" />
              {{ marketplace }}
            </h4>
            <span class="section-count">{{ mcps.length }} 个</span>
          </div>
          <div class="mcp-list">
            <div
              v-for="mcp in mcps"
              :key="mcp.name"
              class="mcp-card"
            >
              <div class="mcp-icon plugin">
                <i class="fa-solid fa-server" />
              </div>
              <div class="mcp-info">
                <h4>{{ mcp.name }}</h4>
                <p class="mcp-config">
                  {{ formatMcpConfig(mcp.config) }}
                </p>
              </div>
              <div class="mcp-actions">
                <button
                  class="action-btn view-plugin"
                  @click="handleViewPlugin(marketplace)"
                >
                  <i class="fa-solid fa-external-link" />
                  查看插件
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- Empty State -->
        <div
          v-if="globalMcpList.length === 0 && pluginMcpList.length === 0"
          class="empty-state"
        >
          <i class="fa-solid fa-server" />
          <p>暂无 MCP 服务器</p>
          <span class="hint">点击"安装 MCP"添加新的服务器</span>
        </div>
      </div>
    </template>

    <!-- Project MCP Tab -->
    <template v-if="activeTab === 'project'">
      <div class="project-mcp-container">
        <!-- Project List (Left) -->
        <div class="project-list">
          <div class="project-list-header">
            <h4>
              <i class="fa-solid fa-folder-tree" />
              项目列表
            </h4>
          </div>
          <div
            v-if="projectsWithMcp.length === 0"
            class="empty-state small"
          >
            <i class="fa-solid fa-folder-open" />
            <p>暂无包含 MCP 的项目</p>
          </div>
          <div
            v-else
            class="project-items"
          >
            <div
              v-for="project in projectsWithMcp"
              :key="project.path"
              class="project-item"
              :class="{ active: selectedProject === project.path }"
              @click="selectedProject = project.path"
            >
              <i class="fa-solid fa-folder" />
              <span class="project-name">{{ getProjectName(project.path) }}</span>
              <span class="mcp-badge">{{ project.mcp_servers.length }}</span>
            </div>
          </div>
        </div>

        <!-- MCP Details (Right) -->
        <div class="project-mcp-detail">
          <template v-if="selectedProject && selectedProjectData">
            <div class="detail-header">
              <h4>
                <i class="fa-solid fa-server" />
                {{ getProjectName(selectedProject) }}
              </h4>
              <span class="project-path">{{ selectedProject }}</span>
            </div>
            <div class="mcp-list">
              <div
                v-for="mcp in selectedProjectData.mcp_servers"
                :key="mcp.name"
                class="mcp-card"
              >
                <div class="mcp-icon project">
                  <i class="fa-solid fa-server" />
                </div>
                <div class="mcp-info">
                  <h4>{{ mcp.name }}</h4>
                  <p class="mcp-config">
                    {{ formatMcpConfig(mcp.config) }}
                  </p>
                </div>
                <div class="mcp-actions">
                  <label class="toggle-switch">
                    <input
                      type="checkbox"
                      :checked="mcp.enabled"
                      @change="handleToggleProject(selectedProject, mcp.name, ($event.target as HTMLInputElement).checked)"
                    >
                    <span class="slider" />
                  </label>
                  <button
                    class="action-btn delete"
                    @click="handleDeleteProject(selectedProject, mcp.name)"
                  >
                    <i class="fa-solid fa-trash" />
                    删除
                  </button>
                </div>
              </div>
            </div>
          </template>
          <template v-else>
            <div class="empty-state">
              <i class="fa-solid fa-hand-pointer" />
              <p>选择一个项目查看 MCP 配置</p>
            </div>
          </template>
        </div>
      </div>
    </template>

    <!-- Install MCP Tab -->
    <template v-if="activeTab === 'install'">
      <div class="install-mcp-container">
        <div class="install-form">
          <h3>
            <i class="fa-solid fa-plus-circle" />
            添加 MCP 服务器
          </h3>
          <p class="form-hint">
            配置一个新的 MCP 服务器，可添加到全局或指定项目中。
          </p>

          <!-- Target Type Selector -->
          <div class="form-group">
            <label>添加到</label>
            <div class="target-selector">
              <button
                class="target-btn"
                :class="{ active: newMcp.targetType === 'global' }"
                @click="newMcp.targetType = 'global'"
              >
                <i class="fa-solid fa-globe" />
                全局
              </button>
              <button
                class="target-btn"
                :class="{ active: newMcp.targetType === 'project' }"
                @click="newMcp.targetType = 'project'"
              >
                <i class="fa-solid fa-folder" />
                项目
              </button>
            </div>
          </div>

          <!-- Project Selector (shown when target is project) -->
          <div
            v-if="newMcp.targetType === 'project'"
            class="form-group"
          >
            <label>选择项目</label>
            <select
              v-model="newMcp.projectPath"
              class="form-select"
            >
              <option value="">
                -- 请选择项目 --
              </option>
              <option
                v-for="project in allProjects"
                :key="project"
                :value="project"
              >
                {{ getProjectName(project) }}
              </option>
            </select>
            <p
              v-if="allProjects.length === 0"
              class="form-note"
            >
              <i class="fa-solid fa-info-circle" />
              暂无已知项目，请先在 Claude Code 中打开一个项目
            </p>
          </div>

          <div class="form-group">
            <label>服务器名称</label>
            <input
              v-model="newMcp.name"
              type="text"
              placeholder="例如: my-mcp-server"
              class="form-input"
            >
          </div>

          <div class="form-group">
            <label>命令 (command)</label>
            <input
              v-model="newMcp.command"
              type="text"
              placeholder="例如: npx"
              class="form-input"
            >
          </div>

          <div class="form-group">
            <label>参数 (args)</label>
            <input
              v-model="newMcp.argsStr"
              type="text"
              placeholder="例如: -y, @some/mcp-server (用逗号分隔)"
              class="form-input"
            >
          </div>

          <div class="form-group">
            <label>环境变量 (env) - 可选</label>
            <textarea
              v-model="newMcp.envStr"
              placeholder="例如: {&quot;API_KEY&quot;: &quot;your-key&quot;}"
              class="form-textarea"
              rows="3"
            />
          </div>

          <div class="form-actions">
            <button
              class="btn-primary"
              :disabled="!canAddMcp"
              @click="handleAddMcp"
            >
              <i class="fa-solid fa-plus" />
              添加
            </button>
          </div>

          <div class="example-section">
            <h4>
              <i class="fa-solid fa-lightbulb" />
              配置示例
            </h4>
            <pre class="example-code">{{ exampleConfig }}</pre>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ask } from '@tauri-apps/plugin-dialog'

// Types
interface McpServerConfig {
  command?: string
  args?: string[]
  env?: Record<string, string>
  type?: string
  url?: string
}

interface GlobalMcpInfo {
  name: string
  config: McpServerConfig
  enabled: boolean
  source: string
}

interface ProjectMcpInfo {
  name: string
  config: McpServerConfig
  enabled: boolean
}

interface ProjectInfo {
  path: string
  mcp_servers: ProjectMcpInfo[]
}

// State
const activeTab = ref<'global' | 'project' | 'install'>('global')
const loading = ref(false)
const globalMcpList = ref<GlobalMcpInfo[]>([])
const pluginMcpList = ref<GlobalMcpInfo[]>([])
const projectsWithMcp = ref<ProjectInfo[]>([])
const selectedProject = ref<string | null>(null)
const allProjects = ref<string[]>([])

// New MCP form
const newMcp = ref({
  name: '',
  command: '',
  argsStr: '',
  envStr: '',
  targetType: 'global' as 'global' | 'project',
  projectPath: ''
})

// Computed
const totalMcpCount = computed(() => {
  const globalCount = globalMcpList.value.length + pluginMcpList.value.length
  const projectCount = projectsWithMcp.value.reduce((sum, p) => sum + p.mcp_servers.length, 0)
  return globalCount + projectCount
})

const pluginMcpsByMarketplace = computed(() => {
  const grouped: Record<string, GlobalMcpInfo[]> = {}
  for (const mcp of pluginMcpList.value) {
    const marketplace = mcp.source.replace('plugin:', '')
    if (!grouped[marketplace]) {
      grouped[marketplace] = []
    }
    grouped[marketplace].push(mcp)
  }
  return grouped
})

const selectedProjectData = computed(() => {
  if (!selectedProject.value) return null
  return projectsWithMcp.value.find(p => p.path === selectedProject.value)
})

const canAddMcp = computed(() => {
  const hasBasicFields = newMcp.value.name.trim() !== '' && newMcp.value.command.trim() !== ''
  if (newMcp.value.targetType === 'project') {
    return hasBasicFields && newMcp.value.projectPath !== ''
  }
  return hasBasicFields
})

const exampleConfig = computed(() => {
  return JSON.stringify({
    command: "npx",
    args: ["-y", "@anthropic/mcp-server-example"],
    env: {
      API_KEY: "your-api-key"
    }
  }, null, 2)
})

// Methods
const loadGlobalMcp = async () => {
  try {
    globalMcpList.value = await invoke<GlobalMcpInfo[]>('get_global_mcp_servers')
  } catch (e) {
    console.error('Failed to load global MCP:', e)
  }
}

const loadPluginMcp = async () => {
  try {
    pluginMcpList.value = await invoke<GlobalMcpInfo[]>('get_plugin_mcp_servers')
  } catch (e) {
    console.error('Failed to load plugin MCP:', e)
  }
}

const loadProjectMcp = async () => {
  try {
    projectsWithMcp.value = await invoke<ProjectInfo[]>('get_projects_with_mcp')
    // Auto-select first project if none selected
    if (!selectedProject.value && projectsWithMcp.value.length > 0) {
      selectedProject.value = projectsWithMcp.value[0].path
    }
  } catch (e) {
    console.error('Failed to load project MCP:', e)
  }
}

const loadAllProjects = async () => {
  try {
    allProjects.value = await invoke<string[]>('get_all_projects')
  } catch (e) {
    console.error('Failed to load all projects:', e)
  }
}

const refreshAll = async () => {
  loading.value = true
  try {
    await Promise.all([loadGlobalMcp(), loadPluginMcp(), loadProjectMcp(), loadAllProjects()])
  } finally {
    loading.value = false
  }
}

const formatMcpConfig = (config: McpServerConfig): string => {
  if (config.command) {
    const args = config.args ? config.args.join(' ') : ''
    return `${config.command} ${args}`.trim()
  }
  if (config.type && config.url) {
    return `${config.type}: ${config.url}`
  }
  return JSON.stringify(config)
}

const getProjectName = (path: string): string => {
  const parts = path.replace(/\\/g, '/').split('/')
  return parts[parts.length - 1] || path
}

const handleToggleGlobal = async (name: string, enabled: boolean) => {
  try {
    await invoke('toggle_global_mcp', { name, enabled })
    await loadGlobalMcp()
  } catch (e) {
    console.error('Failed to toggle global MCP:', e)
    alert(`操作失败: ${e}`)
  }
}

const handleDeleteGlobal = async (name: string) => {
  const confirmed = await ask(`确定要删除 MCP 服务器 "${name}" 吗？`, {
    title: '删除确认',
    kind: 'warning',
  })
  if (!confirmed) return
  try {
    await invoke('delete_global_mcp', { name })
    await loadGlobalMcp()
  } catch (e) {
    console.error('Failed to delete global MCP:', e)
    alert(`删除失败: ${e}`)
  }
}

const handleToggleProject = async (projectPath: string, name: string, enabled: boolean) => {
  try {
    await invoke('toggle_project_mcp', { projectPath, name, enabled })
    await loadProjectMcp()
  } catch (e) {
    console.error('Failed to toggle project MCP:', e)
    alert(`操作失败: ${e}`)
  }
}

const handleDeleteProject = async (projectPath: string, name: string) => {
  const confirmed = await ask(`确定要删除项目 MCP 服务器 "${name}" 吗？`, {
    title: '删除确认',
    kind: 'warning',
  })
  if (!confirmed) return
  try {
    await invoke('delete_project_mcp', { projectPath, name })
    await loadProjectMcp()
  } catch (e) {
    console.error('Failed to delete project MCP:', e)
    alert(`删除失败: ${e}`)
  }
}

const handleViewPlugin = (marketplace: string) => {
  // Emit event to switch to plugin market and filter by marketplace
  // For now, just show an alert
  alert(`请在Claude管理中管理 "${marketplace}" 的 MCP 配置`)
}

const handleAddMcp = async () => {
  if (!canAddMcp.value) return

  const config: McpServerConfig = {
    command: newMcp.value.command.trim()
  }

  // Parse args
  if (newMcp.value.argsStr.trim()) {
    config.args = newMcp.value.argsStr.split(',').map(s => s.trim()).filter(s => s)
  }

  // Parse env
  if (newMcp.value.envStr.trim()) {
    try {
      config.env = JSON.parse(newMcp.value.envStr)
    } catch (e) {
      alert('环境变量格式错误，请使用有效的 JSON 格式')
      return
    }
  }

  try {
    const name = newMcp.value.name.trim()

    if (newMcp.value.targetType === 'project') {
      // Add to project
      const projectPath = newMcp.value.projectPath
      await invoke('add_project_mcp', {
        projectPath,
        name,
        config
      })
      // Switch to project tab and refresh
      activeTab.value = 'project'
      selectedProject.value = projectPath
      await loadProjectMcp()
      // Reset form
      newMcp.value = { name: '', command: '', argsStr: '', envStr: '', targetType: 'global', projectPath: '' }
      alert('MCP 服务器已添加到项目')
    } else {
      // Add to global
      await invoke('add_global_mcp', { name, config })
      // Reset form
      newMcp.value = { name: '', command: '', argsStr: '', envStr: '', targetType: 'global', projectPath: '' }
      // Switch to global tab and refresh
      activeTab.value = 'global'
      await loadGlobalMcp()
      alert('MCP 服务器已添加到全局')
    }
  } catch (e) {
    console.error('Failed to add MCP:', e)
    alert(`添加失败: ${e}`)
  }
}

// Lifecycle
onMounted(() => {
  refreshAll()
})

// Watch tab changes to refresh data
watch(activeTab, (tab) => {
  if (tab === 'project') {
    loadProjectMcp()
  }
})
</script>

<style scoped lang="less">
.mcp-manager {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #f5f5f5;
}

.mcp-tabs {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 12px 20px;
  background: white;
  border-bottom: 1px solid #e0e0e0;

  .tab-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    border: none;
    border-radius: 8px;
    background: transparent;
    cursor: pointer;
    font-size: 14px;
    color: #666;
    transition: all 0.2s;
    flex-shrink: 0;
    white-space: nowrap;

    i {
      font-size: 14px;
    }

    .count {
      background: #f0f0f0;
      padding: 2px 8px;
      border-radius: 10px;
      font-size: 12px;
    }

    &:hover {
      background: #f5f5f5;
    }

    &.active {
      background: #3b82f6;
      color: white;

      .count {
        background: rgba(255, 255, 255, 0.2);
        color: white;
      }
    }
  }

  .mcp-tabs-spacer {
    flex: 1;
  }

  .mcp-count {
    font-size: 13px;
    color: #888;
    background: #f0f0f0;
    padding: 4px 10px;
    border-radius: 12px;
    flex-shrink: 0;
    align-self: center;
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
    background: #eff6ff;
    color: #3b82f6;
    transition: background 0.2s;
    flex-shrink: 0;

    &:hover:not(:disabled) {
      background: #dbeafe;
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }
}

.mcp-content {
  flex: 1;
  overflow: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.mcp-section {
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
    padding-bottom: 8px;
    border-bottom: 1px solid #e0e0e0;

    h4 {
      margin: 0;
      font-size: 14px;
      font-weight: 500;
      display: flex;
      align-items: center;
      gap: 8px;
      color: #333;

      i {
        color: #3b82f6;
      }
    }

    .section-count {
      font-size: 12px;
      color: #888;
    }
  }
}

.mcp-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.mcp-card {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px;
  background: white;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  transition: box-shadow 0.2s;

  &:hover {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }

  .mcp-icon {
    width: 48px;
    height: 48px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;

    i {
      font-size: 20px;
      color: white;
    }

    &.global {
      background: linear-gradient(135deg, #3b82f6 0%, #60a5fa 100%);
    }

    &.plugin {
      background: linear-gradient(135deg, #22c55e 0%, #4ade80 100%);
    }

    &.project {
      background: linear-gradient(135deg, #f59e0b 0%, #fbbf24 100%);
    }
  }

  .mcp-info {
    flex: 1;
    min-width: 0;

    h4 {
      margin: 0 0 4px 0;
      font-size: 15px;
      font-weight: 500;
      color: #333;
    }

    .mcp-config {
      margin: 0;
      font-size: 12px;
      color: #888;
      font-family: monospace;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
  }

  .mcp-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }
}

// Toggle switch
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

  .slider {
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

  input:checked + .slider {
    background-color: #3b82f6;
  }

  input:checked + .slider:before {
    transform: translateX(20px);
  }
}

.action-btn {
  padding: 6px 12px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
  display: flex;
  align-items: center;
  gap: 4px;
  transition: background 0.2s;

  &.delete {
    background: #fee2e2;
    color: #dc2626;

    &:hover {
      background: #fecaca;
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

// Project MCP layout
.project-mcp-container {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.project-list {
  width: 280px;
  background: white;
  border-right: 1px solid #e0e0e0;
  display: flex;
  flex-direction: column;

  .project-list-header {
    padding: 16px;
    border-bottom: 1px solid #e0e0e0;

    h4 {
      margin: 0;
      font-size: 14px;
      font-weight: 500;
      display: flex;
      align-items: center;
      gap: 8px;
      color: #333;

      i {
        color: #3b82f6;
      }
    }
  }

  .project-items {
    flex: 1;
    overflow: auto;
    padding: 8px;
  }

  .project-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.2s;

    i {
      color: #f59e0b;
    }

    .project-name {
      flex: 1;
      font-size: 13px;
      color: #333;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .mcp-badge {
      background: #eff6ff;
      color: #3b82f6;
      padding: 2px 8px;
      border-radius: 10px;
      font-size: 11px;
      font-weight: 500;
    }

    &:hover {
      background: #f5f5f5;
    }

    &.active {
      background: #eff6ff;

      .project-name {
        color: #3b82f6;
        font-weight: 500;
      }
    }
  }
}

.project-mcp-detail {
  flex: 1;
  overflow: auto;
  padding: 20px;
  background: #f5f5f5;

  .detail-header {
    margin-bottom: 16px;
    padding-bottom: 12px;
    border-bottom: 1px solid #e0e0e0;

    h4 {
      margin: 0 0 4px 0;
      font-size: 16px;
      font-weight: 500;
      display: flex;
      align-items: center;
      gap: 8px;
      color: #333;

      i {
        color: #3b82f6;
      }
    }

    .project-path {
      font-size: 12px;
      color: #888;
      font-family: monospace;
    }
  }
}

// Install MCP form
.install-mcp-container {
  flex: 1;
  overflow: auto;
  padding: 20px;
  display: flex;
  justify-content: center;
  align-items: flex-start;
}

.install-form {
  width: 100%;
  max-width: 600px;
  background: white;
  border-radius: 12px;
  padding: 24px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  height: fit-content;

  h3 {
    margin: 0 0 8px 0;
    font-size: 18px;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 10px;
    color: #333;

    i {
      color: #3b82f6;
    }
  }

  .form-hint {
    margin: 0 0 24px 0;
    font-size: 13px;
    color: #888;
  }

  .form-group {
    margin-bottom: 16px;

    label {
      display: block;
      margin-bottom: 6px;
      font-size: 13px;
      font-weight: 500;
      color: #333;
    }
  }

  .target-selector {
    display: flex;
    gap: 8px;

    .target-btn {
      flex: 1;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
      padding: 12px 16px;
      border: 2px solid #e0e0e0;
      border-radius: 8px;
      background: white;
      cursor: pointer;
      font-size: 14px;
      color: #666;
      transition: all 0.2s;

      i {
        font-size: 16px;
      }

      &:hover {
        border-color: #3b82f6;
        color: #3b82f6;
      }

      &.active {
        border-color: #3b82f6;
        background: #eff6ff;
        color: #3b82f6;
        font-weight: 500;
      }
    }
  }

  .form-select {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid #e0e0e0;
    border-radius: 6px;
    font-size: 14px;
    background: white;
    cursor: pointer;
    transition: border-color 0.2s;
    box-sizing: border-box;

    &:focus {
      outline: none;
      border-color: #3b82f6;
    }
  }

  .form-note {
    margin: 8px 0 0 0;
    font-size: 12px;
    color: #f59e0b;
    display: flex;
    align-items: center;
    gap: 6px;

    i {
      font-size: 14px;
    }
  }

  .form-input,
  .form-textarea {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid #e0e0e0;
    border-radius: 6px;
    font-size: 14px;
    font-family: monospace;
    transition: border-color 0.2s;
    box-sizing: border-box;

    &:focus {
      outline: none;
      border-color: #3b82f6;
    }

    &::placeholder {
      color: #aaa;
    }
  }

  .form-textarea {
    resize: vertical;
    min-height: 80px;
    max-height: 150px;
  }

  .form-actions {
    margin-top: 24px;
    display: flex;
    justify-content: flex-end;
  }

  .btn-primary {
    padding: 10px 20px;
    border: none;
    border-radius: 6px;
    background: #3b82f6;
    color: white;
    font-size: 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    transition: background 0.2s;

    &:hover:not(:disabled) {
      background: #2563eb;
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }

  .example-section {
    margin-top: 32px;
    padding-top: 24px;
    border-top: 1px solid #e0e0e0;

    h4 {
      margin: 0 0 12px 0;
      font-size: 14px;
      font-weight: 500;
      display: flex;
      align-items: center;
      gap: 8px;
      color: #666;

      i {
        color: #f59e0b;
      }
    }

    .example-code {
      margin: 0;
      padding: 16px;
      background: #f8fafc;
      border: 1px solid #e2e8f0;
      border-radius: 8px;
      font-size: 13px;
      font-family: monospace;
      color: #334155;
      overflow-x: auto;
    }
  }
}

// Empty state
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  text-align: center;

  i {
    font-size: 48px;
    color: #ddd;
    margin-bottom: 16px;
  }

  p {
    margin: 0;
    font-size: 16px;
    color: #888;
  }

  .hint {
    margin-top: 8px;
    font-size: 13px;
    color: #aaa;
  }

  &.small {
    padding: 30px 20px;

    i {
      font-size: 32px;
      margin-bottom: 12px;
    }

    p {
      font-size: 13px;
    }
  }
}
</style>
