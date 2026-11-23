<template>
  <div class="plugin-market-full">
    <!-- Sidebar -->
    <MarketplaceSidebar
      :selected-marketplace="filters.marketplace"
      :selected-category="filters.category"
      :loading="store.loadingExtended"
      @select-marketplace="handleSelectMarketplace"
      @select-category="handleSelectCategory"
      @refresh="handleRefresh"
      @add-marketplace="showAddDialog = true"
    />

    <!-- Main Content -->
    <div class="main-content">
      <!-- Filter Bar -->
      <FilterBar
        v-model:filters="filters"
        v-model:view-mode="viewMode"
      />

      <!-- Plugin Grid/List -->
      <div
        class="plugin-container"
        :class="viewMode"
      >
        <!-- Loading State -->
        <div
          v-if="store.loadingExtended"
          class="loading-state"
        >
          <i class="fa-solid fa-spinner fa-spin" />
          <p>加载插件列表...</p>
        </div>

        <!-- Empty State -->
        <div
          v-else-if="filteredPlugins.length === 0"
          class="empty-state"
        >
          <i class="fa-solid fa-box-open" />
          <p>没有找到匹配的插件</p>
          <span class="hint">尝试修改筛选条件或添加新的市场源</span>
        </div>

        <!-- Plugin Grid -->
        <div
          v-else
          class="plugin-grid"
          :class="viewMode"
        >
          <PluginCard
            v-for="plugin in filteredPlugins"
            :key="`${plugin.name}@${plugin.marketplaceName}`"
            :plugin="plugin"
            @click="selectedPlugin = plugin"
          />
        </div>
      </div>
    </div>

    <!-- Detail Panel -->
    <PluginDetailPanel
      :plugin="selectedPlugin"
      @close="selectedPlugin = null"
      @updated="handlePluginUpdated"
    />

    <!-- Add Marketplace Dialog -->
    <Teleport to="body">
      <div
        v-if="showAddDialog"
        class="dialog-overlay"
        @click="showAddDialog = false"
      >
        <div
          class="dialog"
          @click.stop
        >
          <div class="dialog-header">
            <h3>添加市场源</h3>
            <button
              class="close-btn"
              @click="showAddDialog = false"
            >
              <i class="fa-solid fa-times" />
            </button>
          </div>

          <div class="dialog-body">
            <!-- Source Type -->
            <div class="form-group">
              <label>源类型</label>
              <div class="source-type-selector">
                <label :class="{ active: addSourceType === 'github' }">
                  <input
                    v-model="addSourceType"
                    type="radio"
                    value="github"
                  >
                  <i class="fa-brands fa-github" />
                  <span>GitHub</span>
                </label>
                <label :class="{ active: addSourceType === 'url' }">
                  <input
                    v-model="addSourceType"
                    type="radio"
                    value="url"
                  >
                  <i class="fa-solid fa-link" />
                  <span>URL</span>
                </label>
              </div>
            </div>

            <!-- Source Value -->
            <div class="form-group">
              <label>{{ addSourceType === 'github' ? 'GitHub 仓库' : '仓库 URL' }}</label>
              <input
                v-model="addSourceValue"
                type="text"
                :placeholder="addSourceType === 'github' ? 'owner/repo' : 'https://github.com/owner/repo'"
              >
              <span class="hint">
                {{ addSourceType === 'github'
                  ? '输入 GitHub 仓库地址，如: anthropics/claude-plugins'
                  : '输入完整的仓库 URL' }}
              </span>
            </div>
          </div>

          <div class="dialog-footer">
            <button
              class="btn cancel"
              @click="showAddDialog = false"
            >
              取消
            </button>
            <button
              class="btn primary"
              :disabled="!addSourceValue || adding"
              @click="handleAddMarketplace"
            >
              <i
                v-if="adding"
                class="fa-solid fa-spinner fa-spin"
              />
              <span>添加</span>
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useMarketplaceStore } from '@/stores/marketplaceStore'
import type { PluginInfoExtended, PluginFilterState, PluginCategory, ViewMode } from '@/types/pluginMarket'
import { DEFAULT_FILTER_STATE } from '@/types/pluginMarket'

import MarketplaceSidebar from './market/MarketplaceSidebar.vue'
import FilterBar from './market/FilterBar.vue'
import PluginCard from './market/PluginCard.vue'
import PluginDetailPanel from './market/PluginDetailPanel.vue'

const store = useMarketplaceStore()

// State
const filters = ref<PluginFilterState>({ ...DEFAULT_FILTER_STATE })
const viewMode = ref<ViewMode>('grid')
const selectedPlugin = ref<PluginInfoExtended | null>(null)

// Add Marketplace Dialog
const showAddDialog = ref(false)
const addSourceType = ref<'github' | 'url'>('github')
const addSourceValue = ref('')
const adding = ref(false)

// Computed
const filteredPlugins = computed(() => {
  let plugins = [...store.allPluginsExtended]

  // Filter by marketplace
  if (filters.value.marketplace) {
    plugins = plugins.filter(p => p.marketplaceName === filters.value.marketplace)
  }

  // Filter by category
  if (filters.value.category) {
    plugins = plugins.filter(p => {
      if (filters.value.category === 'other') {
        return !p.meta?.category
      }
      return p.meta?.category === filters.value.category
    })
  }

  // Filter by status
  switch (filters.value.status) {
    case 'installed':
      plugins = plugins.filter(p => p.isInstalled)
      break
    case 'not-installed':
      plugins = plugins.filter(p => !p.isInstalled)
      break
    case 'has-update':
      plugins = plugins.filter(p => p.hasUpdate)
      break
  }

  // Filter by search
  if (filters.value.search) {
    const query = filters.value.search.toLowerCase()
    plugins = plugins.filter(p =>
      p.name.toLowerCase().includes(query) ||
      p.description.toLowerCase().includes(query) ||
      p.tags.some(tag => tag.toLowerCase().includes(query))
    )
  }

  // Filter by tags
  if (filters.value.tags.length > 0) {
    plugins = plugins.filter(p =>
      filters.value.tags.every(tag => p.tags.includes(tag))
    )
  }

  return plugins
})

// Methods
function handleSelectMarketplace(name: string | null) {
  filters.value = {
    ...filters.value,
    marketplace: name
  }
}

function handleSelectCategory(category: PluginCategory | null) {
  filters.value = {
    ...filters.value,
    category
  }
}

async function handleRefresh() {
  await store.loadMarketplaces()
  await store.loadAllPluginsExtended()
}

async function handleAddMarketplace() {
  if (!addSourceValue.value) return

  adding.value = true
  try {
    await store.addMarketplace(addSourceType.value, addSourceValue.value)
    await store.loadAllPluginsExtended()
    showAddDialog.value = false
    addSourceValue.value = ''
  } catch (e) {
    console.error('Failed to add marketplace:', e)
    alert(`添加失败: ${e}`)
  } finally {
    adding.value = false
  }
}

function handlePluginUpdated(_plugin: PluginInfoExtended) {
  // Refresh the plugin list to get updated status
  store.loadAllPluginsExtended()
}

// Lifecycle
onMounted(async () => {
  // Initialize data
  await store.loadMarketplaces()
  await store.loadAllPluginsExtended()
})
</script>

<style lang="less" scoped>
.plugin-market-full {
  display: flex;
  width: 100%;
  height: 100%;
  background: #f5f5f5;
}

.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.plugin-container {
  flex: 1;
  overflow-y: auto;
  padding: 16px;

  &.list {
    padding: 16px;
  }
}

.loading-state,
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  color: #888;

  i {
    font-size: 48px;
    margin-bottom: 16px;
  }

  p {
    font-size: 16px;
    margin: 0 0 8px 0;
  }

  .hint {
    font-size: 13px;
    color: #aaa;
  }
}

.plugin-grid {
  display: grid;
  gap: 16px;

  &.grid {
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  }

  &.list {
    grid-template-columns: 1fr;
  }
}

// Dialog Styles
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
  z-index: 10100;
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
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px;
  border-bottom: 1px solid #e8e8e8;

  h3 {
    font-size: 18px;
    font-weight: 600;
    color: #333;
    margin: 0;
  }

  .close-btn {
    width: 32px;
    height: 32px;
    border: none;
    border-radius: 50%;
    background: #f5f5f5;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;

    &:hover {
      background: #e8e8e8;
    }
  }
}

.dialog-body {
  padding: 24px;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 16px 24px;
  border-top: 1px solid #e8e8e8;
}

.form-group {
  margin-bottom: 20px;

  &:last-child {
    margin-bottom: 0;
  }

  > label {
    display: block;
    font-size: 14px;
    font-weight: 500;
    color: #333;
    margin-bottom: 8px;
  }

  input[type="text"] {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid #ddd;
    border-radius: 8px;
    font-size: 14px;

    &:focus {
      outline: none;
      border-color: #7c6eb8;
    }
  }

  .hint {
    display: block;
    margin-top: 6px;
    font-size: 12px;
    color: #888;
  }
}

.source-type-selector {
  display: flex;
  gap: 12px;

  label {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    border: 2px solid #e0e0e0;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s;
    flex: 1;

    input {
      display: none;
    }

    &:hover {
      border-color: #ccc;
    }

    &.active {
      border-color: #7c6eb8;
      background: #f0ecf8;
      color: #7c6eb8;
    }

    i {
      font-size: 18px;
    }
  }
}

.btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 10px 20px;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;

  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  &.cancel {
    background: #f5f5f5;
    color: #333;

    &:hover:not(:disabled) {
      background: #e8e8e8;
    }
  }

  &.primary {
    background: #7c6eb8;
    color: white;

    &:hover:not(:disabled) {
      background: #6a5ca0;
    }
  }
}
</style>
