<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { useWindowStore } from '@/stores/windowStore'
import { useDesktopStore } from '@/stores/desktopStore'
import { useMarketplaceStore } from '@/stores/marketplaceStore'
import { useAutoUpdate } from '@/composables/useAutoUpdate'

const windowStore = useWindowStore()
const desktopStore = useDesktopStore()
const marketplaceStore = useMarketplaceStore()

const {
  checking: updateChecking,
  downloading: updateDownloading,
  lastCheckResult: updateResult,
  error: updateError,
  checkAndPrompt,
} = useAutoUpdate()

const showStartMenu = ref(false)
const showAboutDialog = ref(false)
const currentTime = ref('')
const currentDate = ref('')
const appVersion = ref('')

// Widget data
const installedCount = computed(() => marketplaceStore.installedCount)
const updatableCount = computed(() => marketplaceStore.updatableCount)
const totalPluginCount = computed(() => marketplaceStore.allPluginsExtended.length)

const toggleStartMenu = () => {
  showStartMenu.value = !showStartMenu.value
}

const closeStartMenu = () => {
  showStartMenu.value = false
}

const focusWindow = (windowId: string) => {
  const win = windowStore.windows.find(w => w.id === windowId)
  if (win) {
    if (win.state === 'minimized') {
      windowStore.restoreWindow(windowId)
    }
    windowStore.focusWindow(windowId)
  }
}

const openPluginStore = () => {
  const pluginMarketIcon = desktopStore.icons.find(icon => icon.type === 'plugin-market-full')
  if (pluginMarketIcon) {
    windowStore.openWindow(pluginMarketIcon)
  }
  closeStartMenu()
}

const openSetupGuide = () => {
  const setupGuideIcon = {
    id: 'setup-guide-widget',
    name: '环境检查',
    icon: 'fa-solid fa-screwdriver-wrench',
    position: { x: 0, y: 0 },
    type: 'setup-guide' as const
  }
  windowStore.openWindow(setupGuideIcon)
  closeStartMenu()
}

const openEnvVars = () => {
  const envVarsIcon = {
    id: 'env-vars-widget',
    name: '环境变量',
    icon: 'fa-solid fa-key',
    position: { x: 0, y: 0 },
    type: 'env-vars' as const
  }
  windowStore.openWindow(envVarsIcon, { initialTab: 'env' })
  closeStartMenu()
}

const openModelSettings = () => {
  const settingsIcon = desktopStore.icons.find(icon => icon.type === 'settings') ?? {
    id: 'settings',
    name: '设置',
    icon: 'fa-solid fa-gear',
    position: { x: 0, y: 0 },
    type: 'settings' as const
  }
  windowStore.openWindow(settingsIcon, { initialTab: 'coworks' })
  closeStartMenu()
}

const updateClock = () => {
  const now = new Date()
  currentTime.value = now.toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit'
  })
  currentDate.value = now.toLocaleDateString('zh-CN', {
    month: 'short',
    day: 'numeric'
  })
}

// Task completion notifications
interface TaskNotification {
  id: number
  windowId: string
  title: string
}

const notifications = ref<TaskNotification[]>([])

watch(
  () => windowStore.windows.map(w => ({
    id: w.id,
    status: w.data?.terminalStatus,
    title: w.title,
  })),
  (newVal, oldVal) => {
    if (!oldVal) return
    for (const win of newVal) {
      if (win.status === 'completed') {
        const prev = oldVal.find(w => w.id === win.id)
        if (prev?.status !== 'completed') {
          const notifId = Date.now()
          notifications.value = [...notifications.value, {
            id: notifId,
            windowId: win.id,
            title: win.title,
          }]
          setTimeout(() => {
            notifications.value = notifications.value.filter(n => n.id !== notifId)
          }, 5000)
        }
      }
    }
  },
  { deep: true }
)

const dismissNotification = (notifId: number) => {
  notifications.value = notifications.value.filter(n => n.id !== notifId)
}

const handleNotificationClick = (windowId: string, notifId: number) => {
  focusWindow(windowId)
  dismissNotification(notifId)
}

onMounted(async () => {
  updateClock()
  setInterval(updateClock, 1000)
  appVersion.value = await getVersion()
})
</script>

<template>
  <div class="taskbar">
    <!-- Start Button -->
    <div class="taskbar-start">
      <button
        class="start-button"
        @click="toggleStartMenu"
      >
        <i class="fa-solid fa-grip" />
      </button>
    </div>

    <!-- Window List -->
    <div class="taskbar-windows">
      <div
        v-for="win in windowStore.windows"
        :key="win.id"
        class="taskbar-item"
        :class="{
          active: win.id === windowStore.activeWindowId,
          minimized: win.state === 'minimized'
        }"
        @click="focusWindow(win.id)"
      >
        <i class="fa-solid fa-window-maximize" />
        <span>{{ win.title }}</span>
        <i
          v-if="win.data?.terminalStatus === 'completed'"
          class="fa-solid fa-check taskbar-status-done"
        />
        <i
          v-else-if="win.data?.terminalWorking"
          class="fa-solid fa-circle-notch fa-spin taskbar-status-working"
        />
      </div>
    </div>

    <!-- System Tray -->
    <div class="taskbar-tray">
      <div class="clock">
        <span class="time">{{ currentTime }}</span>
        <span class="date">{{ currentDate }}</span>
      </div>
    </div>

    <!-- Start Menu -->
    <Teleport to="body">
      <div
        v-if="showStartMenu"
        class="start-menu-overlay"
        @click="closeStartMenu"
      />
      <div
        v-if="showStartMenu"
        class="start-menu"
      >
        <div class="start-menu-header">
          <h3>Moonton Claude Desktop</h3>
        </div>
        <!-- Widget Interface -->
        <div class="start-menu-widget">
          <!-- Shortcuts Section -->
          <div class="widget-shortcuts">
            <div
              class="shortcut-item"
              @click="openPluginStore"
            >
              <div class="shortcut-icon">
                <i class="fa-solid fa-store" />
              </div>
              <span class="shortcut-label">插件商店</span>
            </div>
            <div
              class="shortcut-item env-check"
              @click="openSetupGuide"
            >
              <div class="shortcut-icon env">
                <i class="fa-solid fa-screwdriver-wrench" />
              </div>
              <span class="shortcut-label">环境检查</span>
            </div>
            <div
              class="shortcut-item env-vars"
              @click="openEnvVars"
            >
              <div class="shortcut-icon vars">
                <i class="fa-solid fa-key" />
              </div>
              <span class="shortcut-label">环境变量</span>
            </div>
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
          <!-- Status Section -->
          <div class="widget-status">
            <h4 class="status-title">
              插件状态
            </h4>
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
            </div>
          </div>
        </div>
        <div class="start-menu-footer">
          <button
            class="about-btn"
            @click="showAboutDialog = true; closeStartMenu()"
          >
            <i class="fa-solid fa-circle-info" />
            关于
          </button>
        </div>
      </div>
    </Teleport>

    <!-- Task Completion Toast Notifications -->
    <Teleport to="body">
      <TransitionGroup
        name="toast"
        tag="div"
        class="toast-container"
      >
        <div
          v-for="notif in notifications"
          :key="notif.id"
          class="toast-notification"
          @click="handleNotificationClick(notif.windowId, notif.id)"
        >
          <i class="fa-solid fa-circle-check toast-icon" />
          <span class="toast-text">{{ notif.title }} 任务已完成</span>
          <button
            class="toast-close"
            @click.stop="dismissNotification(notif.id)"
          >
            <i class="fa-solid fa-xmark" />
          </button>
        </div>
      </TransitionGroup>
    </Teleport>

    <!-- About Dialog -->
    <Teleport to="body">
      <div
        v-if="showAboutDialog"
        class="about-overlay"
        @click="showAboutDialog = false"
      />
      <div
        v-if="showAboutDialog"
        class="about-dialog"
      >
        <div class="about-header">
          <h3>关于 Moonton Claude Desktop</h3>
          <button
            class="close-btn"
            @click="showAboutDialog = false"
          >
            <i class="fa-solid fa-xmark" />
          </button>
        </div>
        <div class="about-content">
          <div class="about-logo">
            <i class="fa-solid fa-desktop" />
          </div>
          <div class="about-info">
            <p class="app-name">
              Moonton Claude Desktop
            </p>
            <p class="version">
              版本 {{ appVersion }}
            </p>
          </div>
        </div>
        <div class="about-update">
          <div class="about-update-row">
            <span class="update-label">当前版本</span>
            <span class="update-value">{{ updateResult?.current_version || appVersion }}</span>
          </div>
          <div
            v-if="updateResult?.remote_version && updateResult?.update_available"
            class="about-update-row"
          >
            <span class="update-label">最新版本</span>
            <span class="update-value new">{{ updateResult.remote_version }}</span>
          </div>
          <button
            class="update-btn"
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
          <div
            v-if="updateResult && !updateResult.update_available && !updateChecking && !updateError"
            class="update-ok"
          >
            <i class="fa-solid fa-check-circle" /> 已是最新版本
          </div>
          <div
            v-if="updateError"
            class="update-error"
          >
            <i class="fa-solid fa-exclamation-circle" /> {{ updateError }}
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style lang="less" scoped>
.taskbar {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  height: 48px;
  background: rgba(30, 45, 70, 0.85);
  backdrop-filter: blur(20px);
  display: flex;
  align-items: center;
  padding: 0 8px;
  gap: 8px;
  z-index: 9999;
}

.taskbar-start {
  .start-button {
    width: 40px;
    height: 40px;
    border: none;
    background: transparent;
    color: white;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 18px;
    transition: background 0.2s;

    &:hover {
      background: rgba(255, 255, 255, 0.1);
    }
  }
}

.taskbar-windows {
  flex: 1;
  display: flex;
  gap: 4px;
  overflow-x: auto;
  padding: 4px 0;
  position: relative;
  mask-image: linear-gradient(
    to right,
    transparent 0px,
    black 12px,
    black calc(100% - 12px),
    transparent 100%
  );
  -webkit-mask-image: linear-gradient(
    to right,
    transparent 0px,
    black 12px,
    black calc(100% - 12px),
    transparent 100%
  );
  padding-left: 12px;
  padding-right: 12px;

  &::-webkit-scrollbar {
    display: none;
  }

  .taskbar-item {
    padding: 6px 12px;
    background: rgba(255, 255, 255, 0.05);
    color: rgba(255, 255, 255, 0.8);
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    white-space: nowrap;
    max-width: 200px;
    transition: all 0.2s;
    border-bottom: 2px solid transparent;

    i {
      font-size: 14px;
    }

    span {
      font-size: 12px;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    &:hover {
      background: rgba(255, 255, 255, 0.1);
    }

    &.active {
      background: rgba(255, 255, 255, 0.15);
      border-bottom-color: #a490c4;
    }

    &.minimized {
      opacity: 0.6;
    }

    .taskbar-status-done {
      color: #10b981;
      font-size: 10px;
      flex-shrink: 0;
    }

    .taskbar-status-working {
      color: #D4A843;
      font-size: 10px;
      flex-shrink: 0;
    }
  }
}

.taskbar-tray {
  display: flex;
  align-items: center;
  padding: 0 8px;

  .clock {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    color: white;
    padding: 4px 8px;
    border-radius: 4px;
    cursor: pointer;

    &:hover {
      background: rgba(255, 255, 255, 0.1);
    }

    .time {
      font-size: 12px;
      font-weight: 500;
    }

    .date {
      font-size: 11px;
      opacity: 0.8;
    }
  }
}

// Toast notifications
.toast-container {
  position: fixed;
  bottom: 56px;
  right: 12px;
  z-index: 10000;
  display: flex;
  flex-direction: column-reverse;
  gap: 8px;
  pointer-events: none;

  > * {
    pointer-events: auto;
  }
}

.toast-notification {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  background: rgba(30, 45, 70, 0.95);
  backdrop-filter: blur(20px);
  border: 1px solid rgba(16, 185, 129, 0.3);
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  cursor: pointer;
  min-width: 200px;
  max-width: 320px;
  transition: background 0.2s;

  &:hover {
    background: rgba(40, 55, 80, 0.95);
  }

  .toast-icon {
    color: #10b981;
    font-size: 16px;
    flex-shrink: 0;
  }

  .toast-text {
    color: rgba(255, 255, 255, 0.9);
    font-size: 12px;
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .toast-close {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.4);
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 4px;
    font-size: 11px;
    flex-shrink: 0;

    &:hover {
      color: white;
      background: rgba(255, 255, 255, 0.1);
    }
  }
}

.toast-enter-active {
  animation: toast-in 0.3s ease-out;
}

.toast-leave-active {
  animation: toast-out 0.3s ease-in;
}

@keyframes toast-in {
  from {
    opacity: 0;
    transform: translateY(12px);
  }
}

@keyframes toast-out {
  to {
    opacity: 0;
    transform: translateX(20px);
  }
}

.start-menu-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 48px;
  z-index: 9998;
}

.start-menu {
  position: fixed;
  bottom: 56px;
  left: 8px;
  width: 220px;
  background: rgba(30, 45, 70, 0.95);
  backdrop-filter: blur(20px);
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 8px 32px rgba(30, 58, 95, 0.4);
  z-index: 10000;

  .start-menu-header {
    padding: 14px 14px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);

    h3 {
      margin: 0;
      color: white;
      font-size: 14px;
      font-weight: 500;
    }
  }

  .start-menu-widget {
    padding: 10px;

    .widget-shortcuts {
      display: flex;
      flex-direction: column;
      gap: 6px;
      margin-bottom: 10px;
      padding-bottom: 10px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);

      .shortcut-item {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 8px 10px;
        background: rgba(255, 255, 255, 0.05);
        border-radius: 8px;
        cursor: pointer;
        transition: all 0.2s;
        border: 1px solid transparent;

        &:hover {
          background: rgba(175, 82, 222, 0.2);
          border-color: rgba(175, 82, 222, 0.3);
        }

        &.env-check .shortcut-icon.env {
          background: linear-gradient(135deg, #34C759, #28A745);
        }

        &.env-vars .shortcut-icon.vars {
          background: linear-gradient(135deg, #3B82F6, #1D4ED8);
        }

        &.model-settings .shortcut-icon.model {
          background: linear-gradient(135deg, #F59E0B, #D97706);
        }

        .shortcut-icon {
          width: 32px;
          height: 32px;
          border-radius: 8px;
          background: linear-gradient(135deg, #AF52DE, #8944AB);
          display: flex;
          align-items: center;
          justify-content: center;
          flex-shrink: 0;

          i {
            font-size: 13px;
            color: white;
          }
        }

        .shortcut-label {
          font-size: 13px;
          color: rgba(255, 255, 255, 0.9);
          font-weight: 500;
        }
      }
    }

    .widget-status {
      .status-title {
        font-size: 10px;
        font-weight: 600;
        color: rgba(255, 255, 255, 0.4);
        letter-spacing: 0.3px;
        margin: 0 0 8px 4px;
      }

      .status-list {
        display: flex;
        flex-direction: column;
        gap: 4px;

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
            color: rgba(255, 255, 255, 0.35);
          }

          .status-label {
            flex: 1;
            font-size: 11px;
            color: rgba(255, 255, 255, 0.6);
          }

          .status-value {
            font-size: 12px;
            font-weight: 600;
            color: rgba(255, 255, 255, 0.9);

            &.update {
              color: #f59e0b;
            }
          }
        }
      }
    }
  }

  .start-menu-footer {
    padding: 10px 12px;
    border-top: 1px solid rgba(255, 255, 255, 0.1);

    .about-btn {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 8px 10px;
      background: transparent;
      border: none;
      color: rgba(255, 255, 255, 0.7);
      cursor: pointer;
      border-radius: 6px;
      font-size: 12px;
      width: 100%;

      &:hover {
        background: rgba(255, 255, 255, 0.1);
        color: white;
      }
    }
  }
}

.about-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  z-index: 10000;
}

.about-dialog {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 320px;
  background: rgba(30, 45, 70, 0.95);
  backdrop-filter: blur(20px);
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  z-index: 10001;

  .about-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);

    h3 {
      margin: 0;
      color: white;
      font-size: 15px;
      font-weight: 500;
    }

    .close-btn {
      background: transparent;
      border: none;
      color: rgba(255, 255, 255, 0.6);
      cursor: pointer;
      padding: 4px;
      border-radius: 4px;
      font-size: 14px;

      &:hover {
        background: rgba(255, 255, 255, 0.1);
        color: white;
      }
    }
  }

  .about-content {
    padding: 24px 20px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;

    .about-logo {
      width: 64px;
      height: 64px;
      background: linear-gradient(135deg, #a490c4 0%, #7c6a9a 100%);
      border-radius: 16px;
      display: flex;
      align-items: center;
      justify-content: center;

      i {
        font-size: 28px;
        color: white;
      }
    }

    .about-info {
      text-align: center;

      .app-name {
        margin: 0 0 4px 0;
        color: white;
        font-size: 16px;
        font-weight: 500;
      }

      .version {
        margin: 0 0 12px 0;
        color: rgba(255, 255, 255, 0.6);
        font-size: 13px;
      }

    }
  }

  .about-update {
    padding: 12px 20px 16px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    display: flex;
    flex-direction: column;
    gap: 8px;

    .about-update-row {
      display: flex;
      justify-content: space-between;
      align-items: center;
      font-size: 12px;

      .update-label { color: rgba(255, 255, 255, 0.45); }
      .update-value { color: rgba(255, 255, 255, 0.85); font-weight: 500; }
      .update-value.new { color: #34C759; }
    }

    .update-btn {
      align-self: flex-start;
      padding: 6px 14px;
      border-radius: 7px;
      background: rgba(175, 82, 222, 0.85);
      color: white;
      font-size: 12px;
      border: none;
      cursor: pointer;
      display: flex;
      align-items: center;
      gap: 6px;
      transition: background 0.2s;

      &:hover:not(:disabled) { background: rgba(175, 82, 222, 1); }
      &:disabled { opacity: 0.5; cursor: not-allowed; }
    }

    .update-ok {
      font-size: 11px;
      color: #34C759;
      display: flex;
      align-items: center;
      gap: 5px;
    }

    .update-error {
      font-size: 11px;
      color: #FF453A;
      display: flex;
      align-items: center;
      gap: 5px;
    }
  }
}
</style>
