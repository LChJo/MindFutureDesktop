<template>
  <div class="marketplace-sidebar">
    <!-- Header -->
    <div class="sidebar-header">
      <i class="fa-solid fa-store" />
      <span>插件市场</span>
    </div>

    <!-- Marketplace Sources Section -->
    <div class="sidebar-section">
      <h3 class="section-title">
        市场源
      </h3>
      <div class="source-list">
        <!-- All Sources -->
        <div
          class="source-item"
          :class="{ active: !selectedMarketplace }"
          @click="$emit('selectMarketplace', null)"
        >
          <i class="fa-solid fa-globe" />
          <span>全部市场</span>
          <span class="count">{{ totalPluginCount }}</span>
        </div>

        <!-- Individual Marketplaces -->
        <div
          v-for="marketplace in marketplaces"
          :key="marketplace.name"
          class="source-item"
          :class="{ active: selectedMarketplace === marketplace.name }"
          @click="$emit('selectMarketplace', marketplace.name)"
        >
          <i :class="getMarketplaceIcon(marketplace)" />
          <span>{{ marketplace.name }}</span>
          <span class="count">{{ marketplace.pluginCount }}</span>
        </div>
      </div>
    </div>

    <!-- Categories Section -->
    <div class="sidebar-section">
      <h3 class="section-title">
        分类
      </h3>
      <div class="category-list">
        <div
          class="category-item"
          :class="{ active: !selectedCategory }"
          @click="$emit('selectCategory', null)"
        >
          <i class="fa-solid fa-shapes" />
          <span>全部分类</span>
        </div>

        <div
          v-for="(info, cat) in availableCategories"
          :key="cat"
          class="category-item"
          :class="{ active: selectedCategory === cat }"
          @click="$emit('selectCategory', cat)"
        >
          <i :class="info?.icon" />
          <span>{{ info?.label }}</span>
          <span
            v-if="getCategoryCount(cat)"
            class="count"
          >{{ getCategoryCount(cat) }}</span>
        </div>
      </div>
    </div>

    <!-- Stats Section -->
    <div class="sidebar-section stats">
      <h3 class="section-title">
        统计
      </h3>
      <div class="stats-list">
        <div class="stat-item">
          <i class="fa-solid fa-download" />
          <span>已安装</span>
          <span class="value">{{ installedCount }}</span>
        </div>
        <div
          v-if="updatableCount > 0"
          class="stat-item"
        >
          <i class="fa-solid fa-arrow-up" />
          <span>可更新</span>
          <span class="value update">{{ updatableCount }}</span>
        </div>
        <div class="stat-item">
          <i class="fa-solid fa-puzzle-piece" />
          <span>总插件</span>
          <span class="value">{{ totalPluginCount }}</span>
        </div>
      </div>
    </div>

    <!-- Actions Section -->
    <div class="sidebar-section actions">
      <button
        class="action-btn refresh"
        :disabled="loading"
        @click="$emit('refresh')"
      >
        <i :class="loading ? 'fa-solid fa-spinner fa-spin' : 'fa-solid fa-rotate'" />
        <span>刷新列表</span>
      </button>
      <button
        class="action-btn add"
        @click="$emit('addMarketplace')"
      >
        <i class="fa-solid fa-plus" />
        <span>添加市场源</span>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useMarketplaceStore } from '@/stores/marketplaceStore'
import type { MarketplaceInfo } from '@/stores/marketplaceStore'
import type { PluginCategory } from '@/types/pluginMarket'
import { CATEGORY_INFO } from '@/types/pluginMarket'

defineProps<{
  selectedMarketplace: string | null
  selectedCategory: PluginCategory | null
  loading: boolean
}>()

defineEmits<{
  selectMarketplace: [name: string | null]
  selectCategory: [category: PluginCategory | null]
  refresh: []
  addMarketplace: []
}>()

const store = useMarketplaceStore()

const marketplaces = computed(() => store.marketplaces)
const totalPluginCount = computed(() => store.allPluginsExtended.length)
const installedCount = computed(() => store.installedCount)
const updatableCount = computed(() => store.updatableCount)

// Only show categories that have plugins
const availableCategories = computed(() => {
  const categories = store.allCategories
  const result: Partial<Record<PluginCategory, { label: string; icon: string }>> = {}

  for (const cat of categories) {
    if (CATEGORY_INFO[cat as PluginCategory]) {
      result[cat as PluginCategory] = CATEGORY_INFO[cat as PluginCategory]
    }
  }

  // Always show 'other' if there are uncategorized plugins
  const uncategorizedCount = store.allPluginsExtended.filter(p => !p.meta?.category).length
  if (uncategorizedCount > 0) {
    result.other = CATEGORY_INFO.other
  }

  return result
})

function getMarketplaceIcon(marketplace: MarketplaceInfo): string {
  if (marketplace.source.source === 'github') {
    return 'fa-brands fa-github'
  }
  return 'fa-solid fa-link'
}

function getCategoryCount(category: string): number {
  if (category === 'other') {
    return store.allPluginsExtended.filter(p => !p.meta?.category).length
  }
  return store.allPluginsExtended.filter(p => p.meta?.category === category).length
}
</script>

<style lang="less" scoped>
.marketplace-sidebar {
  width: 240px;
  background: #fafafa;
  border-right: 1px solid #e8e8e8;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.sidebar-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 16px;
  font-size: 18px;
  font-weight: 600;
  color: #333;
  border-bottom: 1px solid #e8e8e8;

  i {
    color: #7c6eb8;
  }
}

.sidebar-section {
  padding: 16px;
  border-bottom: 1px solid #e8e8e8;

  &:last-child {
    border-bottom: none;
  }

  &.stats {
    background: #f5f5f5;
  }

  &.actions {
    margin-top: auto;
    padding: 12px;
  }
}

.section-title {
  font-size: 12px;
  font-weight: 600;
  color: #888;
  text-transform: uppercase;
  margin: 0 0 12px 0;
  letter-spacing: 0.5px;
}

.source-list,
.category-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.source-item,
.category-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 13px;
  color: #333;
  transition: all 0.2s;

  &:hover {
    background: #f0f0f0;
  }

  &.active {
    background: #7c6eb8;
    color: white;

    i {
      color: white;
    }

    .count {
      background: rgba(255, 255, 255, 0.2);
      color: white;
    }
  }

  i {
    font-size: 14px;
    color: #666;
    width: 18px;
    text-align: center;
  }

  span:not(.count) {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .count {
    font-size: 11px;
    padding: 2px 8px;
    background: #e8e8e8;
    border-radius: 10px;
    color: #666;
  }
}

.stats-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.stat-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0;
  font-size: 13px;
  color: #666;

  i {
    font-size: 12px;
    width: 18px;
    text-align: center;
    color: #888;
  }

  span:not(.value) {
    flex: 1;
  }

  .value {
    font-weight: 600;
    color: #333;

    &.update {
      color: #f59e0b;
    }
  }
}

.action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  width: 100%;
  padding: 10px;
  border: none;
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;
  margin-bottom: 8px;

  &:last-child {
    margin-bottom: 0;
  }

  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  &.refresh {
    background: #f0f0f0;
    color: #333;

    &:hover:not(:disabled) {
      background: #e8e8e8;
    }
  }

  &.add {
    background: #7c6eb8;
    color: white;

    &:hover:not(:disabled) {
      background: #6a5ca0;
    }
  }
}
</style>
