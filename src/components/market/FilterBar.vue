<template>
  <div class="filter-bar">
    <!-- Search Input -->
    <div class="search-box">
      <i class="fa-solid fa-search" />
      <input
        v-model="searchValue"
        type="text"
        placeholder="搜索插件..."
        @input="debouncedSearch"
      >
      <button
        v-if="searchValue"
        class="clear-btn"
        @click="clearSearch"
      >
        <i class="fa-solid fa-times" />
      </button>
    </div>

    <!-- Status Filter -->
    <div class="filter-dropdown">
      <button
        class="dropdown-trigger"
        @click="showStatusDropdown = !showStatusDropdown"
      >
        <i :class="STATUS_INFO[filters.status].icon" />
        <span>{{ STATUS_INFO[filters.status].label }}</span>
        <i class="fa-solid fa-chevron-down" />
      </button>
      <div
        v-if="showStatusDropdown"
        class="dropdown-menu"
        @click.stop
      >
        <div
          v-for="(info, status) in STATUS_INFO"
          :key="status"
          class="dropdown-item"
          :class="{ active: filters.status === status }"
          @click="selectStatus(status as FilterStatus)"
        >
          <i :class="info.icon" />
          <span>{{ info.label }}</span>
          <span
            v-if="getStatusCount(status as FilterStatus)"
            class="count"
          >
            {{ getStatusCount(status as FilterStatus) }}
          </span>
        </div>
      </div>
    </div>

    <!-- Category Filter -->
    <div class="filter-dropdown">
      <button
        class="dropdown-trigger"
        @click="showCategoryDropdown = !showCategoryDropdown"
      >
        <i :class="selectedCategoryIcon" />
        <span>{{ selectedCategoryLabel }}</span>
        <i class="fa-solid fa-chevron-down" />
      </button>
      <div
        v-if="showCategoryDropdown"
        class="dropdown-menu"
        @click.stop
      >
        <div
          class="dropdown-item"
          :class="{ active: !filters.category }"
          @click="selectCategory(null)"
        >
          <i class="fa-solid fa-globe" />
          <span>全部分类</span>
        </div>
        <div
          v-for="(info, cat) in CATEGORY_INFO"
          :key="cat"
          class="dropdown-item"
          :class="{ active: filters.category === cat }"
          @click="selectCategory(cat as PluginCategory)"
        >
          <i :class="info.icon" />
          <span>{{ info.label }}</span>
        </div>
      </div>
    </div>

    <!-- View Toggle -->
    <div class="view-toggle">
      <button
        class="toggle-btn"
        :class="{ active: viewMode === 'grid' }"
        title="网格视图"
        @click="$emit('update:viewMode', 'grid')"
      >
        <i class="fa-solid fa-table-cells" />
      </button>
      <button
        class="toggle-btn"
        :class="{ active: viewMode === 'list' }"
        title="列表视图"
        @click="$emit('update:viewMode', 'list')"
      >
        <i class="fa-solid fa-list" />
      </button>
    </div>

    <!-- Clear All Filters -->
    <button
      v-if="hasActiveFilters"
      class="clear-filters-btn"
      @click="clearAllFilters"
    >
      <i class="fa-solid fa-filter-circle-xmark" />
      <span>清除筛选</span>
    </button>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useMarketplaceStore } from '@/stores/marketplaceStore'
import type { PluginFilterState, FilterStatus, PluginCategory, ViewMode } from '@/types/pluginMarket'
import { STATUS_INFO, CATEGORY_INFO, DEFAULT_FILTER_STATE } from '@/types/pluginMarket'

const props = defineProps<{
  filters: PluginFilterState
  viewMode: ViewMode
}>()

const emit = defineEmits<{
  'update:filters': [filters: PluginFilterState]
  'update:viewMode': [mode: ViewMode]
}>()

const store = useMarketplaceStore()

const searchValue = ref(props.filters.search)
const showStatusDropdown = ref(false)
const showCategoryDropdown = ref(false)

let searchTimeout: ReturnType<typeof setTimeout> | null = null

// Computed
const selectedCategoryLabel = computed(() =>
  props.filters.category
    ? CATEGORY_INFO[props.filters.category].label
    : '全部分类'
)

const selectedCategoryIcon = computed(() =>
  props.filters.category
    ? CATEGORY_INFO[props.filters.category].icon
    : 'fa-solid fa-shapes'
)

const hasActiveFilters = computed(() =>
  props.filters.search !== '' ||
  props.filters.status !== 'all' ||
  props.filters.category !== null ||
  props.filters.tags.length > 0
)

// Methods
function debouncedSearch() {
  if (searchTimeout) {
    clearTimeout(searchTimeout)
  }
  searchTimeout = setTimeout(() => {
    emit('update:filters', {
      ...props.filters,
      search: searchValue.value
    })
  }, 300)
}

function clearSearch() {
  searchValue.value = ''
  emit('update:filters', {
    ...props.filters,
    search: ''
  })
}

function selectStatus(status: FilterStatus) {
  showStatusDropdown.value = false
  emit('update:filters', {
    ...props.filters,
    status
  })
}

function selectCategory(category: PluginCategory | null) {
  showCategoryDropdown.value = false
  emit('update:filters', {
    ...props.filters,
    category
  })
}

function getStatusCount(status: FilterStatus): number {
  const plugins = store.allPluginsExtended
  switch (status) {
    case 'all':
      return plugins.length
    case 'installed':
      return plugins.filter(p => p.isInstalled).length
    case 'not-installed':
      return plugins.filter(p => !p.isInstalled).length
    case 'has-update':
      return plugins.filter(p => p.hasUpdate).length
    default:
      return 0
  }
}

function clearAllFilters() {
  searchValue.value = ''
  emit('update:filters', { ...DEFAULT_FILTER_STATE })
}

// Close dropdowns on outside click
function handleClickOutside(event: MouseEvent) {
  const target = event.target as HTMLElement
  if (!target.closest('.filter-dropdown')) {
    showStatusDropdown.value = false
    showCategoryDropdown.value = false
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
  if (searchTimeout) {
    clearTimeout(searchTimeout)
  }
})
</script>

<style lang="less" scoped>
.filter-bar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: white;
  border-bottom: 1px solid #e8e8e8;
}

.search-box {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #f5f5f5;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  flex: 1;
  max-width: 300px;

  i {
    color: #999;
    font-size: 14px;
  }

  input {
    border: none;
    outline: none;
    background: transparent;
    flex: 1;
    font-size: 14px;
    color: #333;

    &::placeholder {
      color: #aaa;
    }
  }

  .clear-btn {
    background: none;
    border: none;
    padding: 4px;
    cursor: pointer;
    color: #999;

    &:hover {
      color: #666;
    }
  }
}

.filter-dropdown {
  position: relative;

  .dropdown-trigger {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: white;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    cursor: pointer;
    font-size: 13px;
    color: #333;
    transition: all 0.2s;

    &:hover {
      border-color: #7c6eb8;
    }

    i:first-child {
      font-size: 12px;
      color: #666;
    }

    i:last-child {
      font-size: 10px;
      color: #999;
    }
  }

  .dropdown-menu {
    position: absolute;
    top: 100%;
    left: 0;
    margin-top: 4px;
    background: white;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    min-width: 160px;
    z-index: 100;
    overflow: hidden;

    .dropdown-item {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 10px 12px;
      cursor: pointer;
      font-size: 13px;
      color: #333;
      transition: background 0.2s;

      &:hover {
        background: #f5f5f5;
      }

      &.active {
        background: #f0ecf8;
        color: #7c6eb8;

        i {
          color: #7c6eb8;
        }
      }

      i {
        font-size: 12px;
        color: #666;
        width: 16px;
        text-align: center;
      }

      .count {
        margin-left: auto;
        font-size: 11px;
        padding: 2px 6px;
        background: #f0f0f0;
        border-radius: 10px;
        color: #666;
      }
    }
  }
}

.view-toggle {
  display: flex;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  overflow: hidden;

  .toggle-btn {
    padding: 8px 12px;
    background: white;
    border: none;
    cursor: pointer;
    color: #666;
    transition: all 0.2s;

    &:hover {
      background: #f5f5f5;
    }

    &.active {
      background: #7c6eb8;
      color: white;
    }

    &:first-child {
      border-right: 1px solid #e0e0e0;
    }
  }
}

.clear-filters-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  background: #fee;
  border: 1px solid #fcc;
  border-radius: 8px;
  color: #d00;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s;

  &:hover {
    background: #fdd;
  }
}
</style>
