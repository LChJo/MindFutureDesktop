<template>
  <!-- Widget Panel - Always visible in top-right -->
  <div
    class="widget-panel"
    :class="{ expanded: isExpanded }"
    @mouseenter="isExpanded = true"
    @mouseleave="isExpanded = false"
  >
    <!-- Status Section - Always visible (on top) -->
    <div class="panel-section status-section">
      <h3 class="section-title">
        插件状态
      </h3>
      <div class="status-list">
        <div class="status-item">
          <i class="fa-solid fa-puzzle-piece" />
          <span class="status-label">已安装插件</span>
          <span class="status-value">{{ installedCount }}</span>
        </div>
        <div
          v-if="updatableCount > 0"
          class="status-item"
        >
          <i class="fa-solid fa-arrow-up" />
          <span class="status-label">可更新</span>
          <span class="status-value update">{{ updatableCount }}</span>
        </div>
        <div class="status-item">
          <i class="fa-solid fa-globe" />
          <span class="status-label">市场总数</span>
          <span class="status-value">{{ totalPluginCount }}</span>
        </div>
        <div
          v-if="envModel"
          class="status-item stacked"
        >
          <div class="status-header">
            <i class="fa-solid fa-robot" />
            <span class="status-label">模型</span>
          </div>
          <span
            class="status-value model"
            :title="envModel"
          >{{ envModel }}</span>
        </div>
      </div>
    </div>

    <!-- Shortcuts Section - Expands downward when hovered -->
    <div
      class="panel-section shortcuts-section"
      :class="{ collapsed: !isExpanded }"
    >
      <div class="shortcuts-grid">
        <div
          class="shortcut-item"
          @click="openPluginStore"
        >
          <div class="shortcut-icon">
            <i class="fa-solid fa-store" />
          </div>
          <span class="shortcut-label">插件商店</span>
        </div>
        <!-- Environment Check -->
        <div
          class="shortcut-item env-check"
          @click="openSetupGuide"
        >
          <div class="shortcut-icon env">
            <i class="fa-solid fa-screwdriver-wrench" />
          </div>
          <span class="shortcut-label">环境检查</span>
        </div>
        <!-- Environment Variables -->
        <div
          class="shortcut-item env-vars"
          @click="openEnvVars"
        >
          <div class="shortcut-icon vars">
            <i class="fa-solid fa-key" />
          </div>
          <span class="shortcut-label">环境变量</span>
        </div>
        <!-- Model Settings -->
        <div
          class="shortcut-item model-settings"
          @click="openModelSettings"
        >
          <div class="shortcut-icon model">
            <i class="fa-solid fa-sliders" />
          </div>
          <span class="shortcut-label">对话模型</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useMarketplaceStore } from '@/stores/marketplaceStore'
import { useWindowStore } from '@/stores/windowStore'
import { useDesktopStore } from '@/stores/desktopStore'
import { useSettingsStore } from '@/stores/settingsStore'

const marketplaceStore = useMarketplaceStore()
const windowStore = useWindowStore()
const desktopStore = useDesktopStore()
const settingsStore = useSettingsStore()

// State
const isExpanded = ref(false)

// Computed
const installedCount = computed(() => marketplaceStore.installedCount)
const updatableCount = computed(() => marketplaceStore.updatableCount)
const totalPluginCount = computed(() => marketplaceStore.allPluginsExtended.length)
const envModel = computed(() => settingsStore.environmentVariables['ANTHROPIC_MODEL'] || '')

// Methods
function openPluginStore() {
  // Find the plugin-market-full desktop icon and open it
  const pluginMarketIcon = desktopStore.icons.find(icon => icon.type === 'plugin-market-full')
  if (pluginMarketIcon) {
    windowStore.openWindow(pluginMarketIcon)
  }
}

function openSetupGuide() {
  // Create a virtual icon for setup guide window
  const setupGuideIcon = {
    id: 'setup-guide-widget',
    name: '环境检查',
    icon: 'fa-solid fa-screwdriver-wrench',
    position: { x: 0, y: 0 },
    type: 'setup-guide' as const
  }
  windowStore.openWindow(setupGuideIcon)
}

function openEnvVars() {
  // Create a virtual icon for environment variables window
  const envVarsIcon = {
    id: 'env-vars-widget',
    name: '环境变量',
    icon: 'fa-solid fa-key',
    position: { x: 0, y: 0 },
    type: 'env-vars' as const
  }
  windowStore.openWindow(envVarsIcon, { initialTab: 'env' })
}

function openModelSettings() {
  const settingsIcon = desktopStore.icons.find(icon => icon.type === 'settings') ?? {
    id: 'settings',
    name: '设置',
    icon: 'fa-solid fa-gear',
    position: { x: 0, y: 0 },
    type: 'settings' as const
  }
  windowStore.openWindow(settingsIcon, { initialTab: 'coworks' })
}

// Load marketplace data on mount
onMounted(async () => {
  // Load plugin data if not already loaded
  if (marketplaceStore.allPluginsExtended.length === 0) {
    await marketplaceStore.loadMarketplaces()
    await marketplaceStore.loadAllPluginsExtended()
  }
})
</script>

<style lang="less" scoped>
.widget-panel {
  position: fixed;
  top: 16px;
  right: 16px;
  width: 180px;
  background: rgba(246, 246, 246, 0.88);
  backdrop-filter: blur(40px);
  border-radius: 14px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  box-shadow:
    0 4px 24px rgba(0, 0, 0, 0.12),
    0 0 0 0.5px rgba(255, 255, 255, 0.6) inset;
  overflow: hidden;
  z-index: 50;  /* 低于 WindowManager 的 100，让窗口可以覆盖小组件 */
}

.panel-section {
  padding: 10px 12px;
}

.shortcuts-section {
  max-height: 280px;  // 4 buttons * ~58px + padding
  overflow: hidden;
  transition: all 0.3s ease;
  border-top: 1px solid rgba(0, 0, 0, 0.05);

  &.collapsed {
    max-height: 0;
    padding-top: 0;
    padding-bottom: 0;
    border-top: none;
    opacity: 0;
  }
}

.section-title {
  font-size: 10px;
  font-weight: 600;
  color: rgba(0, 0, 0, 0.4);
  letter-spacing: 0.3px;
  margin: 0 0 8px 0;
}

.shortcuts-grid {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.shortcut-item {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  background: rgba(0, 0, 0, 0.02);
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s;
  border: 1px solid transparent;

  &:hover {
    background: rgba(175, 82, 222, 0.1);
    border-color: rgba(175, 82, 222, 0.25);
  }

  &.env-check {
    .shortcut-icon.env {
      background: linear-gradient(135deg, #34C759, #28A745);
    }
  }

  &.env-vars {
    .shortcut-icon.vars {
      background: linear-gradient(135deg, #3B82F6, #1D4ED8);
    }
  }

  &.model-settings {
    .shortcut-icon.model {
      background: linear-gradient(135deg, #F59E0B, #D97706);
    }
  }

  .shortcut-icon {
    width: 36px;
    height: 36px;
    border-radius: 9px;
    background: linear-gradient(135deg, #AF52DE, #8944AB);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;

    i {
      font-size: 15px;
      color: white;
    }
  }

  .shortcut-label {
    font-size: 13px;
    color: rgba(0, 0, 0, 0.85);
    font-weight: 500;
  }
}

.status-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 8px;
  border-radius: 6px;

  i {
    width: 16px;
    text-align: center;
    font-size: 11px;
    color: rgba(0, 0, 0, 0.35);
  }

  .status-label {
    flex: 1;
    font-size: 11px;
    color: rgba(0, 0, 0, 0.6);
  }

  .status-value {
    font-size: 12px;
    font-weight: 600;
    color: rgba(0, 0, 0, 0.85);

    &.update {
      color: #f59e0b;
    }
  }

  // 堆叠布局（模型和 API）
  &.stacked {
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 6px 8px;
    background: rgba(0, 0, 0, 0.02);
    margin-top: 4px;

    .status-header {
      display: flex;
      align-items: center;
      gap: 6px;

      i {
        width: auto;
      }
    }

    .status-value {
      width: 100%;
      font-size: 11px;
      font-weight: 500;
      word-break: break-all;
      line-height: 1.4;
      padding-left: 22px;

      &.model {
        color: #10B981;
      }

      &.router {
        color: #8b5cf6;
      }
    }
  }
}
</style>
