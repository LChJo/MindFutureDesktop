<template>
  <div
    class="plugin-card"
    :class="{
      installed: plugin.isInstalled,
      'has-update': plugin.hasUpdate
    }"
    @click="$emit('click', plugin)"
  >
    <!-- Plugin Icon -->
    <div class="card-icon">
      <img
        v-if="iconUrl"
        :src="iconUrl"
        alt=""
        class="icon-image"
      >
      <div
        v-else
        class="icon-placeholder"
        :class="categoryClass"
      >
        <i :class="categoryIcon" />
      </div>
      <!-- Status badge -->
      <div
        v-if="plugin.isInstalled"
        class="status-badge installed"
      >
        <i class="fa-solid fa-check" />
      </div>
      <div
        v-else-if="plugin.hasUpdate"
        class="status-badge update"
      >
        <i class="fa-solid fa-arrow-up" />
      </div>
    </div>

    <!-- Plugin Info -->
    <div class="card-content">
      <h3 class="plugin-name">
        {{ plugin.name }}
      </h3>
      <p class="plugin-description">
        {{ plugin.description || '暂无描述' }}
      </p>

      <!-- Meta Info -->
      <div class="plugin-meta">
        <span
          v-if="plugin.version"
          class="meta-item version"
        >
          <i class="fa-solid fa-tag" />
          {{ plugin.version }}
        </span>
        <span
          v-if="plugin.author"
          class="meta-item author"
        >
          <i class="fa-solid fa-user" />
          {{ plugin.author.name }}
        </span>
      </div>

      <!-- Tags -->
      <div
        v-if="plugin.tags.length > 0"
        class="plugin-tags"
      >
        <span
          v-for="tag in displayTags"
          :key="tag"
          class="tag"
        >
          {{ tag }}
        </span>
        <span
          v-if="plugin.tags.length > 3"
          class="tag more"
        >
          +{{ plugin.tags.length - 3 }}
        </span>
      </div>
    </div>

    <!-- Quick Actions -->
    <div
      class="card-actions"
      @click.stop
    >
      <button
        v-if="!plugin.isInstalled"
        class="action-btn install"
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
        <span>安装</span>
      </button>

      <template v-else>
        <button
          v-if="plugin.hasUpdate"
          class="action-btn update"
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
          <span>更新</span>
        </button>

        <!-- Enable/Disable Toggle -->
        <label
          class="toggle-switch"
          @click.stop
        >
          <input
            type="checkbox"
            :checked="plugin.isEnabled"
            :disabled="toggling"
            @change="handleToggle"
          >
          <span class="toggle-slider" />
        </label>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useMarketplaceStore } from '@/stores/marketplaceStore'
import type { PluginInfoExtended, PluginCategory } from '@/types/pluginMarket'
import { CATEGORY_INFO } from '@/types/pluginMarket'

const props = defineProps<{
  plugin: PluginInfoExtended
}>()

const emit = defineEmits<{
  click: [plugin: PluginInfoExtended]
  installed: [plugin: PluginInfoExtended]
  uninstalled: [plugin: PluginInfoExtended]
  toggled: [plugin: PluginInfoExtended, enabled: boolean]
}>()

const store = useMarketplaceStore()

const iconUrl = ref<string | null>(null)
const installing = ref(false)
const updating = ref(false)
const toggling = ref(false)

// Display only first 3 tags
const displayTags = computed(() => props.plugin.tags.slice(0, 3))

// Get category or default
const category = computed<PluginCategory>(() =>
  props.plugin.meta?.category || 'other'
)

const categoryClass = computed(() => `category-${category.value}`)

const categoryIcon = computed(() =>
  CATEGORY_INFO[category.value]?.icon || 'fa-solid fa-puzzle-piece'
)

// Load plugin icon
onMounted(async () => {
  const icon = await store.getPluginIcon(props.plugin.marketplaceName, props.plugin.name)
  if (icon) {
    iconUrl.value = icon
  }
})

async function handleInstall() {
  installing.value = true
  try {
    await store.installPlugin(props.plugin.marketplaceName, props.plugin.name)
    store.updatePluginStatus(props.plugin.name, props.plugin.marketplaceName, {
      isInstalled: true,
      isEnabled: true,
      installedVersion: props.plugin.version
    })
    emit('installed', props.plugin)
  } catch (e) {
    console.error('Failed to install plugin:', e)
  } finally {
    installing.value = false
  }
}

async function handleUpdate() {
  updating.value = true
  try {
    // Uninstall first
    await store.uninstallPlugin(props.plugin.name, props.plugin.marketplaceName)
    // Then reinstall
    await store.installPlugin(props.plugin.marketplaceName, props.plugin.name)
    store.updatePluginStatus(props.plugin.name, props.plugin.marketplaceName, {
      hasUpdate: false,
      installedVersion: props.plugin.version
    })
  } catch (e) {
    console.error('Failed to update plugin:', e)
  } finally {
    updating.value = false
  }
}

async function handleToggle() {
  toggling.value = true
  try {
    await store.togglePluginEnabled(props.plugin.name, props.plugin.marketplaceName)
    const newEnabled = !props.plugin.isEnabled
    store.updatePluginStatus(props.plugin.name, props.plugin.marketplaceName, {
      isEnabled: newEnabled
    })
    emit('toggled', props.plugin, newEnabled)
  } catch (e) {
    console.error('Failed to toggle plugin:', e)
  } finally {
    toggling.value = false
  }
}
</script>

<style lang="less" scoped>
.plugin-card {
  display: flex;
  flex-direction: column;
  padding: 16px;
  background: white;
  border-radius: 12px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
  cursor: pointer;
  transition: all 0.2s ease;
  border: 1px solid #e8e8e8;

  &:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }

  &.installed {
    border-left: 3px solid #22c55e;
  }

  &.has-update {
    border-left: 3px solid #f59e0b;
  }
}

.card-icon {
  position: relative;
  width: 64px;
  height: 64px;
  margin-bottom: 12px;

  .icon-image {
    width: 100%;
    height: 100%;
    border-radius: 12px;
    object-fit: cover;
  }

  .icon-placeholder {
    width: 100%;
    height: 100%;
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;

    i {
      font-size: 28px;
      color: white;
    }

    // Category gradients
    &.category-productivity {
      background: linear-gradient(135deg, #7c6eb8 0%, #a490c4 100%);
    }
    &.category-development {
      background: linear-gradient(135deg, #3b82f6 0%, #60a5fa 100%);
    }
    &.category-creative {
      background: linear-gradient(135deg, #ec4899 0%, #f472b6 100%);
    }
    &.category-data {
      background: linear-gradient(135deg, #14b8a6 0%, #2dd4bf 100%);
    }
    &.category-communication {
      background: linear-gradient(135deg, #f59e0b 0%, #fbbf24 100%);
    }
    &.category-utility {
      background: linear-gradient(135deg, #6366f1 0%, #818cf8 100%);
    }
    &.category-other {
      background: linear-gradient(135deg, #64748b 0%, #94a3b8 100%);
    }
  }

  .status-badge {
    position: absolute;
    bottom: -4px;
    right: -4px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 2px solid white;

    i {
      font-size: 10px;
      color: white;
    }

    &.installed {
      background: #22c55e;
    }

    &.update {
      background: #f59e0b;
    }
  }
}

.card-content {
  flex: 1;
  min-height: 0;
}

.plugin-name {
  font-size: 15px;
  font-weight: 600;
  color: #1a1a1a;
  margin: 0 0 6px 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.plugin-description {
  font-size: 13px;
  color: #666;
  margin: 0 0 8px 0;
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  min-height: 36px;
}

.plugin-meta {
  display: flex;
  gap: 12px;
  margin-bottom: 8px;

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

.plugin-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-bottom: 12px;

  .tag {
    padding: 2px 8px;
    background: #f0f0f0;
    border-radius: 10px;
    font-size: 11px;
    color: #666;

    &.more {
      background: #e8e8e8;
      color: #888;
    }
  }
}

.card-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-top: 12px;
  border-top: 1px solid #f0f0f0;
}

.action-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border: none;
  border-radius: 6px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s;

  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  &.install {
    background: #7c6eb8;
    color: white;

    &:hover:not(:disabled) {
      background: #6a5ca0;
    }
  }

  &.update {
    background: #f0ecf8;
    color: #7c6eb8;

    &:hover:not(:disabled) {
      background: #e0d8f0;
    }
  }
}

.toggle-switch {
  position: relative;
  display: inline-block;
  width: 40px;
  height: 22px;
  margin-left: auto;

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
    border-radius: 22px;

    &:before {
      position: absolute;
      content: "";
      height: 16px;
      width: 16px;
      left: 3px;
      bottom: 3px;
      background-color: white;
      transition: 0.3s;
      border-radius: 50%;
    }
  }

  input:checked + .toggle-slider {
    background-color: #22c55e;

    &:before {
      transform: translateX(18px);
    }
  }

  input:disabled + .toggle-slider {
    opacity: 0.5;
    cursor: not-allowed;
  }
}
</style>
