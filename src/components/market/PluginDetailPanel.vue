<template>
  <Teleport to="body">
    <Transition name="slide">
      <div
        v-if="plugin"
        class="detail-panel-overlay"
        @click="$emit('close')"
      >
        <div
          class="detail-panel"
          @click.stop
        >
          <!-- Header -->
          <div class="panel-header">
            <button
              class="close-btn"
              @click="$emit('close')"
            >
              <i class="fa-solid fa-times" />
            </button>
          </div>

          <!-- Content -->
          <div class="panel-content">
            <!-- Plugin Info Section -->
            <div class="plugin-hero">
              <div class="hero-icon">
                <img
                  v-if="iconUrl"
                  :src="iconUrl"
                  alt=""
                >
                <div
                  v-else
                  class="icon-placeholder"
                  :class="categoryClass"
                >
                  <i :class="categoryIcon" />
                </div>
              </div>

              <div class="hero-info">
                <h2 class="plugin-name">
                  {{ plugin.name }}
                </h2>
                <p class="plugin-description">
                  {{ plugin.description || '暂无描述' }}
                </p>

                <div class="plugin-meta">
                  <span
                    v-if="plugin.version"
                    class="meta-item"
                  >
                    <i class="fa-solid fa-tag" />
                    v{{ plugin.version }}
                  </span>
                  <span
                    v-if="plugin.author"
                    class="meta-item"
                  >
                    <i class="fa-solid fa-user" />
                    {{ plugin.author.name }}
                  </span>
                  <span class="meta-item">
                    <i class="fa-solid fa-store" />
                    {{ plugin.marketplaceName }}
                  </span>
                </div>

                <!-- Status -->
                <div class="status-badges">
                  <span
                    v-if="plugin.isInstalled"
                    class="badge installed"
                  >
                    <i class="fa-solid fa-check" />
                    已安装
                    <template v-if="plugin.installedVersion">
                      (v{{ plugin.installedVersion }})
                    </template>
                  </span>
                  <span
                    v-if="plugin.hasUpdate"
                    class="badge update"
                  >
                    <i class="fa-solid fa-arrow-up" />
                    可更新
                  </span>
                  <span
                    v-if="plugin.isEnabled"
                    class="badge enabled"
                  >
                    <i class="fa-solid fa-power-off" />
                    已启用
                  </span>
                </div>

                <!-- Tags -->
                <div
                  v-if="plugin.tags.length > 0"
                  class="plugin-tags"
                >
                  <span
                    v-for="tag in plugin.tags"
                    :key="tag"
                    class="tag"
                  >
                    {{ tag }}
                  </span>
                </div>
              </div>
            </div>

            <!-- Action Buttons -->
            <div class="action-buttons">
              <template v-if="!plugin.isInstalled">
                <button
                  class="action-btn primary"
                  :disabled="installing"
                  @click="handleInstall"
                >
                  <i
                    v-if="installing"
                    class="fa-solid fa-spinner fa-spin"
                  />
                  <i
                    v-else
                    class="fa-solid fa-download"
                  />
                  <span>安装插件</span>
                </button>
              </template>

              <template v-else>
                <button
                  v-if="plugin.hasUpdate"
                  class="action-btn primary"
                  :disabled="updating"
                  @click="handleUpdate"
                >
                  <i
                    v-if="updating"
                    class="fa-solid fa-spinner fa-spin"
                  />
                  <i
                    v-else
                    class="fa-solid fa-rotate"
                  />
                  <span>更新到 v{{ plugin.version }}</span>
                </button>

                <button
                  class="action-btn"
                  :class="plugin.isEnabled ? 'warning' : 'success'"
                  :disabled="toggling"
                  @click="handleToggle"
                >
                  <i
                    v-if="toggling"
                    class="fa-solid fa-spinner fa-spin"
                  />
                  <i
                    v-else
                    :class="plugin.isEnabled ? 'fa-solid fa-pause' : 'fa-solid fa-play'"
                  />
                  <span>{{ plugin.isEnabled ? '禁用' : '启用' }}</span>
                </button>

                <button
                  class="action-btn danger"
                  :disabled="uninstalling"
                  @click="handleUninstall"
                >
                  <i
                    v-if="uninstalling"
                    class="fa-solid fa-spinner fa-spin"
                  />
                  <i
                    v-else
                    class="fa-solid fa-trash"
                  />
                  <span>卸载</span>
                </button>
              </template>
            </div>

            <!-- Highlights Section -->
            <div
              v-if="plugin.meta?.highlights?.length"
              class="section"
            >
              <h3 class="section-title">
                <i class="fa-solid fa-star" />
                亮点功能
              </h3>
              <ul class="highlights-list">
                <li
                  v-for="(highlight, index) in plugin.meta.highlights"
                  :key="index"
                >
                  <i class="fa-solid fa-check" />
                  {{ highlight }}
                </li>
              </ul>
            </div>

            <!-- Applicable Works Section -->
            <div
              v-if="plugin.meta?.applicableWorks?.length"
              class="section"
            >
              <h3 class="section-title">
                <i class="fa-solid fa-briefcase" />
                适用场景
              </h3>
              <div class="works-grid">
                <div
                  v-for="work in plugin.meta.applicableWorks"
                  :key="work.title"
                  class="work-item"
                >
                  <i :class="work.icon || 'fa-solid fa-check-circle'" />
                  <div class="work-info">
                    <h4>{{ work.title }}</h4>
                    <p v-if="work.description">
                      {{ work.description }}
                    </p>
                  </div>
                </div>
              </div>
            </div>

            <!-- Usage Section -->
            <div
              v-if="plugin.meta?.usage"
              class="section"
            >
              <h3 class="section-title">
                <i class="fa-solid fa-book" />
                使用说明
              </h3>
              <div
                class="usage-content"
                v-html="renderedUsage"
              />
            </div>

            <!-- Skills & Agents Section -->
            <div
              v-if="plugin.skills.length > 0 || plugin.agents.length > 0"
              class="section"
            >
              <h3 class="section-title">
                <i class="fa-solid fa-puzzle-piece" />
                包含内容
              </h3>
              <div class="content-lists">
                <div
                  v-if="plugin.skills.length > 0"
                  class="content-list"
                >
                  <h4><i class="fa-solid fa-wand-magic-sparkles" /> Skills</h4>
                  <ul>
                    <li
                      v-for="skill in plugin.skills"
                      :key="skill"
                    >
                      {{ extractName(skill) }}
                    </li>
                  </ul>
                </div>
                <div
                  v-if="plugin.agents.length > 0"
                  class="content-list"
                >
                  <h4><i class="fa-solid fa-robot" /> Agents</h4>
                  <ul>
                    <li
                      v-for="agent in plugin.agents"
                      :key="agent"
                    >
                      {{ extractName(agent) }}
                    </li>
                  </ul>
                </div>
              </div>
            </div>

            <!-- Screenshots Section -->
            <ScreenshotCarousel
              v-if="plugin.meta?.screenshots?.length"
              :screenshots="plugin.meta.screenshots"
            />
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useMarketplaceStore } from '@/stores/marketplaceStore'
import type { PluginInfoExtended, PluginCategory } from '@/types/pluginMarket'
import { CATEGORY_INFO } from '@/types/pluginMarket'
import ScreenshotCarousel from './ScreenshotCarousel.vue'

const props = defineProps<{
  plugin: PluginInfoExtended | null
}>()

const emit = defineEmits<{
  close: []
  updated: [plugin: PluginInfoExtended]
}>()

const store = useMarketplaceStore()

const iconUrl = ref<string | null>(null)
const installing = ref(false)
const updating = ref(false)
const toggling = ref(false)
const uninstalling = ref(false)

// Computed
const category = computed<PluginCategory>(() =>
  props.plugin?.meta?.category || 'other'
)

const categoryClass = computed(() => `category-${category.value}`)

const categoryIcon = computed(() =>
  CATEGORY_INFO[category.value]?.icon || 'fa-solid fa-puzzle-piece'
)

// Simple markdown to HTML (improved rendering)
const renderedUsage = computed(() => {
  if (!props.plugin?.meta?.usage) return ''

  const lines = props.plugin.meta.usage.split('\n')
  const result: string[] = []
  let inCodeBlock = false
  let codeBlockContent: string[] = []
  let inList = false
  let listItems: string[] = []

  const flushList = () => {
    if (listItems.length > 0) {
      result.push(`<ul class="usage-list">${listItems.join('')}</ul>`)
      listItems = []
      inList = false
    }
  }

  for (const line of lines) {
    // Handle code blocks
    if (line.startsWith('```')) {
      if (inCodeBlock) {
        result.push(`<pre class="code-block"><code>${codeBlockContent.join('\n')}</code></pre>`)
        codeBlockContent = []
        inCodeBlock = false
      } else {
        flushList()
        inCodeBlock = true
      }
      continue
    }

    if (inCodeBlock) {
      codeBlockContent.push(line)
      continue
    }

    // Handle headers
    if (line.startsWith('### ')) {
      flushList()
      result.push(`<h5 class="usage-h5">${line.slice(4)}</h5>`)
      continue
    }
    if (line.startsWith('## ')) {
      flushList()
      result.push(`<h4 class="usage-h4">${line.slice(3)}</h4>`)
      continue
    }
    if (line.startsWith('# ')) {
      flushList()
      result.push(`<h3 class="usage-h3">${line.slice(2)}</h3>`)
      continue
    }

    // Handle list items
    if (line.startsWith('- ')) {
      inList = true
      const content = line.slice(2)
        .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
        .replace(/`(.+?)`/g, '<code>$1</code>')
      listItems.push(`<li>${content}</li>`)
      continue
    }

    // Handle numbered list items
    if (/^\d+\. /.test(line)) {
      inList = true
      const content = line.replace(/^\d+\. /, '')
        .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
        .replace(/`(.+?)`/g, '<code>$1</code>')
      listItems.push(`<li>${content}</li>`)
      continue
    }

    // Flush list if we hit a non-list line
    if (inList && line.trim() !== '') {
      flushList()
    }

    // Handle empty lines
    if (line.trim() === '') {
      flushList()
      continue
    }

    // Handle regular text
    const text = line
      .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*(.+?)\*/g, '<em>$1</em>')
      .replace(/`(.+?)`/g, '<code>$1</code>')
    result.push(`<p class="usage-text">${text}</p>`)
  }

  // Flush any remaining list
  flushList()

  return result.join('')
})

// Methods
function extractName(path: string): string {
  // Extract filename from path like "./skills/xxx.md" -> "xxx"
  const name = path.split('/').pop() || path
  return name.replace(/\.(md|txt)$/, '')
}

async function loadIcon() {
  if (!props.plugin) return
  iconUrl.value = await store.getPluginIcon(props.plugin.marketplaceName, props.plugin.name)
}

async function handleInstall() {
  if (!props.plugin) return
  installing.value = true
  try {
    await store.installPlugin(props.plugin.marketplaceName, props.plugin.name)
    store.updatePluginStatus(props.plugin.name, props.plugin.marketplaceName, {
      isInstalled: true,
      isEnabled: true,
      installedVersion: props.plugin.version
    })
    emit('updated', props.plugin)
  } catch (e) {
    console.error('Failed to install plugin:', e)
  } finally {
    installing.value = false
  }
}

async function handleUpdate() {
  if (!props.plugin) return
  updating.value = true
  try {
    await store.uninstallPlugin(props.plugin.name, props.plugin.marketplaceName)
    await store.installPlugin(props.plugin.marketplaceName, props.plugin.name)
    store.updatePluginStatus(props.plugin.name, props.plugin.marketplaceName, {
      hasUpdate: false,
      installedVersion: props.plugin.version
    })
    emit('updated', props.plugin)
  } catch (e) {
    console.error('Failed to update plugin:', e)
  } finally {
    updating.value = false
  }
}

async function handleToggle() {
  if (!props.plugin) return
  toggling.value = true
  try {
    await store.togglePluginEnabled(props.plugin.name, props.plugin.marketplaceName)
    const newEnabled = !props.plugin.isEnabled
    store.updatePluginStatus(props.plugin.name, props.plugin.marketplaceName, {
      isEnabled: newEnabled
    })
    emit('updated', props.plugin)
  } catch (e) {
    console.error('Failed to toggle plugin:', e)
  } finally {
    toggling.value = false
  }
}

async function handleUninstall() {
  if (!props.plugin) return
  uninstalling.value = true
  try {
    await store.uninstallPlugin(props.plugin.name, props.plugin.marketplaceName)
    store.updatePluginStatus(props.plugin.name, props.plugin.marketplaceName, {
      isInstalled: false,
      isEnabled: false,
      hasUpdate: false,
      installedVersion: undefined
    })
    emit('updated', props.plugin)
    emit('close')
  } catch (e) {
    console.error('Failed to uninstall plugin:', e)
  } finally {
    uninstalling.value = false
  }
}

// Watch for plugin changes
watch(() => props.plugin, () => {
  if (props.plugin) {
    loadIcon()
  } else {
    iconUrl.value = null
  }
}, { immediate: true })
</script>

<style lang="less" scoped>
.detail-panel-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  z-index: 10050;
  display: flex;
  justify-content: flex-end;
}

.detail-panel {
  width: 480px;
  max-width: 100%;
  height: 100%;
  background: white;
  box-shadow: -4px 0 20px rgba(0, 0, 0, 0.1);
  display: flex;
  flex-direction: column;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 12px 16px;
  border-bottom: 1px solid #e8e8e8;

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
    transition: background 0.2s;

    &:hover {
      background: #e8e8e8;
    }
  }
}

.panel-content {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
}

.plugin-hero {
  display: flex;
  gap: 16px;
  margin-bottom: 20px;
}

.hero-icon {
  width: 80px;
  height: 80px;
  flex-shrink: 0;

  img {
    width: 100%;
    height: 100%;
    border-radius: 16px;
    object-fit: cover;
  }

  .icon-placeholder {
    width: 100%;
    height: 100%;
    border-radius: 16px;
    display: flex;
    align-items: center;
    justify-content: center;

    i {
      font-size: 36px;
      color: white;
    }

    &.category-productivity { background: linear-gradient(135deg, #7c6eb8 0%, #a490c4 100%); }
    &.category-development { background: linear-gradient(135deg, #3b82f6 0%, #60a5fa 100%); }
    &.category-creative { background: linear-gradient(135deg, #ec4899 0%, #f472b6 100%); }
    &.category-data { background: linear-gradient(135deg, #14b8a6 0%, #2dd4bf 100%); }
    &.category-communication { background: linear-gradient(135deg, #f59e0b 0%, #fbbf24 100%); }
    &.category-utility { background: linear-gradient(135deg, #6366f1 0%, #818cf8 100%); }
    &.category-other { background: linear-gradient(135deg, #64748b 0%, #94a3b8 100%); }
  }
}

.hero-info {
  flex: 1;
  min-width: 0;

  .plugin-name {
    font-size: 20px;
    font-weight: 600;
    color: #1a1a1a;
    margin: 0 0 8px 0;
  }

  .plugin-description {
    font-size: 14px;
    color: #666;
    margin: 0 0 12px 0;
    line-height: 1.5;
  }

  .plugin-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin-bottom: 12px;

    .meta-item {
      display: flex;
      align-items: center;
      gap: 4px;
      font-size: 12px;
      color: #888;

      i {
        font-size: 10px;
      }
    }
  }
}

.status-badges {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 12px;

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    border-radius: 12px;
    font-size: 12px;

    &.installed {
      background: #dcfce7;
      color: #16a34a;
    }

    &.update {
      background: #fef3c7;
      color: #d97706;
    }

    &.enabled {
      background: #dbeafe;
      color: #2563eb;
    }
  }
}

.plugin-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;

  .tag {
    padding: 4px 10px;
    background: #f0f0f0;
    border-radius: 12px;
    font-size: 12px;
    color: #666;
  }
}

.action-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  padding: 16px 0;
  border-top: 1px solid #e8e8e8;
  border-bottom: 1px solid #e8e8e8;
  margin-bottom: 20px;
}

.action-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;

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

  &.success {
    background: #dcfce7;
    color: #16a34a;

    &:hover:not(:disabled) {
      background: #bbf7d0;
    }
  }

  &.warning {
    background: #fef3c7;
    color: #d97706;

    &:hover:not(:disabled) {
      background: #fde68a;
    }
  }

  &.danger {
    background: #fee2e2;
    color: #dc2626;

    &:hover:not(:disabled) {
      background: #fecaca;
    }
  }
}

.section {
  margin-bottom: 24px;

  .section-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    font-weight: 600;
    color: #333;
    margin: 0 0 12px 0;

    i {
      color: #7c6eb8;
    }
  }
}

.highlights-list {
  list-style: none;
  margin: 0;
  padding: 0;

  li {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 0;
    color: #444;
    font-size: 14px;

    i {
      color: #22c55e;
      margin-top: 3px;
    }
  }
}

.works-grid {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.work-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px;
  background: #f9f9f9;
  border-radius: 8px;

  i {
    font-size: 20px;
    color: #7c6eb8;
    margin-top: 2px;
  }

  .work-info {
    h4 {
      font-size: 14px;
      font-weight: 600;
      color: #333;
      margin: 0 0 4px 0;
    }

    p {
      font-size: 12px;
      color: #666;
      margin: 0;
    }
  }
}

.usage-content {
  font-size: 14px;
  line-height: 1.6;
  color: #444;

  :deep(.usage-h3) {
    font-size: 16px;
    font-weight: 600;
    color: #1a1a1a;
    margin: 20px 0 10px 0;
    padding-bottom: 6px;
    border-bottom: 1px solid #e8e8e8;

    &:first-child {
      margin-top: 0;
    }
  }

  :deep(.usage-h4) {
    font-size: 14px;
    font-weight: 600;
    color: #333;
    margin: 16px 0 8px 0;
  }

  :deep(.usage-h5) {
    font-size: 13px;
    font-weight: 600;
    color: #555;
    margin: 12px 0 6px 0;
  }

  :deep(.usage-text) {
    margin: 6px 0;
    color: #555;
  }

  :deep(code) {
    background: #f0ecf8;
    color: #7c6eb8;
    padding: 2px 6px;
    border-radius: 4px;
    font-family: 'SF Mono', Monaco, Consolas, monospace;
    font-size: 12px;
  }

  :deep(.code-block) {
    background: #f8f8f8;
    border: 1px solid #e8e8e8;
    padding: 12px 14px;
    border-radius: 8px;
    overflow-x: auto;
    margin: 10px 0;

    code {
      background: none;
      color: #333;
      padding: 0;
      font-size: 13px;
      line-height: 1.5;
    }
  }

  :deep(.usage-list) {
    list-style: none;
    padding: 0;
    margin: 8px 0;

    li {
      position: relative;
      padding: 6px 0 6px 20px;
      color: #444;
      border-bottom: 1px solid #f5f5f5;

      &:last-child {
        border-bottom: none;
      }

      &::before {
        content: '';
        position: absolute;
        left: 0;
        top: 12px;
        width: 6px;
        height: 6px;
        background: #7c6eb8;
        border-radius: 50%;
      }
    }
  }

  :deep(strong) {
    font-weight: 600;
    color: #333;
  }

  :deep(em) {
    font-style: italic;
    color: #666;
  }
}

.content-lists {
  display: flex;
  gap: 20px;

  .content-list {
    flex: 1;

    h4 {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 13px;
      font-weight: 600;
      color: #666;
      margin: 0 0 8px 0;

      i {
        font-size: 12px;
      }
    }

    ul {
      list-style: none;
      margin: 0;
      padding: 0;

      li {
        padding: 6px 0;
        font-size: 13px;
        color: #444;
        border-bottom: 1px solid #f0f0f0;

        &:last-child {
          border-bottom: none;
        }
      }
    }
  }
}

// Transition
.slide-enter-active,
.slide-leave-active {
  transition: all 0.3s ease;
}

.slide-enter-from,
.slide-leave-to {
  opacity: 0;

  .detail-panel {
    transform: translateX(100%);
  }
}
</style>
