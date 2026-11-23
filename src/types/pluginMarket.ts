// Plugin Market Full - Type Definitions

// Plugin metadata applicable work/scenario
export interface PluginMetaApplicableWork {
  title: string
  description?: string
  icon?: string // Font Awesome icon class
}

// Plugin metadata screenshot
export interface PluginMetaScreenshot {
  url: string
  caption?: string
}

// Plugin metadata from plugin.meta.json
export interface PluginMeta {
  iconUrl?: string
  usage?: string // Markdown formatted
  applicableWorks?: PluginMetaApplicableWork[]
  screenshots?: PluginMetaScreenshot[]
  category?: PluginCategory
  highlights?: string[]
}

// Plugin categories
export type PluginCategory =
  | 'productivity'
  | 'development'
  | 'creative'
  | 'data'
  | 'communication'
  | 'utility'
  | 'other'

// Category display info
export const CATEGORY_INFO: Record<PluginCategory, { label: string; icon: string }> = {
  productivity: { label: '效率工具', icon: 'fa-solid fa-bolt' },
  development: { label: '开发工具', icon: 'fa-solid fa-code' },
  creative: { label: '创意设计', icon: 'fa-solid fa-palette' },
  data: { label: '数据处理', icon: 'fa-solid fa-database' },
  communication: { label: '通讯协作', icon: 'fa-solid fa-comments' },
  utility: { label: '实用工具', icon: 'fa-solid fa-toolbox' },
  other: { label: '其他', icon: 'fa-solid fa-puzzle-piece' }
}

// Plugin author info
export interface PluginAuthor {
  name: string
}

// Extended plugin info with metadata and status
export interface PluginInfoExtended {
  name: string
  source: string
  description: string
  version: string
  author?: PluginAuthor
  skills: string[]
  agents: string[]
  tags: string[]
  marketplaceName: string
  meta?: PluginMeta
  isInstalled: boolean
  isEnabled: boolean
  hasUpdate: boolean
  installedVersion?: string
}

// Filter status options
export type FilterStatus = 'all' | 'installed' | 'not-installed' | 'has-update'

// Filter state for the full market view
export interface PluginFilterState {
  search: string
  status: FilterStatus
  marketplace: string | null // null means all marketplaces
  category: PluginCategory | null // null means all categories
  tags: string[]
}

// Default filter state
export const DEFAULT_FILTER_STATE: PluginFilterState = {
  search: '',
  status: 'all',
  marketplace: null,
  category: null,
  tags: []
}

// Status display info
export const STATUS_INFO: Record<FilterStatus, { label: string; icon: string }> = {
  all: { label: '全部', icon: 'fa-solid fa-globe' },
  installed: { label: '已安装', icon: 'fa-solid fa-check-circle' },
  'not-installed': { label: '未安装', icon: 'fa-solid fa-download' },
  'has-update': { label: '可更新', icon: 'fa-solid fa-arrow-up-circle' }
}

// View mode
export type ViewMode = 'grid' | 'list'

// Sort options
export type SortOption = 'name' | 'updated' | 'installed'

export const SORT_OPTIONS: Record<SortOption, { label: string }> = {
  name: { label: '名称' },
  updated: { label: '更新时间' },
  installed: { label: '安装状态' }
}
