<script setup lang="ts">
import { computed } from 'vue'
import type { DynamicFunctionInfo, BackendStatus } from '@/stores/dynamicFunctionStore'
import type { ConversationIndexItem } from '@/composables/claudeApps'

const props = defineProps<{
  apps: DynamicFunctionInfo[]
  conversations: ConversationIndexItem[]
  selectedAppName: string | null
  selectedConversationId: string | null
  backendStatuses: Record<string, BackendStatus>
}>()

const emit = defineEmits<{
  (e: 'select-app', appName: string): void
  (e: 'select-conversation', conversationId: string): void
  (e: 'open-app', app: DynamicFunctionInfo): void
  (e: 'reveal-app', app: DynamicFunctionInfo): void
  (e: 'delete-app', app: DynamicFunctionInfo): void
  (e: 'delete-conversation', conversationId: string): void
  (e: 'toggle-backend', appName: string): void
}>()

// Conversations not linked to any app (being created)
const unlinkedConversations = computed(() => {
  return props.conversations
    .filter(c => !c.appName)
    .sort((a, b) => b.updatedAt - a.updatedAt)
})

const DEFAULT_STATUS: BackendStatus = { status: 'stopped', port: null, error: null }

function getStatus(name: string): BackendStatus {
  return props.backendStatuses[name] ?? DEFAULT_STATUS
}

function isTransitioning(name: string): boolean {
  const s = getStatus(name).status
  return s === 'starting' || s === 'stopping'
}

function backendTooltip(name: string): string {
  const bs = getStatus(name)
  switch (bs.status) {
    case 'stopped': return '启动后端服务'
    case 'starting': return '正在启动...'
    case 'running': return `后端运行中 (端口 ${bs.port}) — 点击停止`
    case 'stopping': return '正在停止...'
    case 'error': return `错误: ${bs.error ?? '未知'} — 点击重试`
  }
}

function formatTime(timestamp: number): string {
  const date = new Date(timestamp)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24))

  if (diffDays === 0) {
    return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
  }
  if (diffDays === 1) return '昨天'
  if (diffDays < 7) return `${diffDays}天前`
  return date.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' })
}
</script>

<template>
  <div class="apps-list">
    <!-- Apps section: each app = one row, one conversation -->
    <div
      v-if="apps.length > 0"
      class="section"
    >
      <div class="section-title">
        应用
      </div>
      <div
        v-for="app in apps"
        :key="app.name"
        class="app-item"
        :class="{ selected: selectedAppName === app.name }"
        @click="emit('select-app', app.name)"
      >
        <div class="app-icon">
          <i :class="app.icon" />
        </div>
        <div class="app-info">
          <div class="app-name">
            {{ app.displayName }}
          </div>
          <div class="app-meta">
            <span class="app-version">v{{ app.version }}</span>
            <button
              v-if="app.backendEnabled"
              class="backend-status-btn"
              :class="getStatus(app.name).status"
              :disabled="isTransitioning(app.name)"
              :title="backendTooltip(app.name)"
              @click.stop="emit('toggle-backend', app.name)"
            >
              <i
                class="fa-solid fa-server"
                :class="{ 'fa-pulse': isTransitioning(app.name) }"
              />
            </button>
          </div>
        </div>
        <div class="app-actions">
          <button
            class="action-btn"
            title="运行"
            @click.stop="emit('open-app', app)"
          >
            <i class="fa-solid fa-play" />
          </button>
          <button
            class="action-btn"
            title="打开目录"
            @click.stop="emit('reveal-app', app)"
          >
            <i class="fa-solid fa-folder-open" />
          </button>
          <button
            class="action-btn danger"
            title="删除"
            @click.stop="emit('delete-app', app)"
          >
            <i class="fa-solid fa-trash" />
          </button>
        </div>
      </div>
    </div>

    <!-- Unlinked conversations: apps being created -->
    <div
      v-if="unlinkedConversations.length > 0"
      class="section"
    >
      <div class="section-title">
        创建中
      </div>
      <div
        v-for="conv in unlinkedConversations"
        :key="conv.id"
        class="conv-item"
        :class="{ selected: selectedConversationId === conv.id }"
        @click="emit('select-conversation', conv.id)"
      >
        <div class="conv-icon">
          <i class="fa-solid fa-spinner fa-pulse" />
        </div>
        <div class="conv-info">
          <div class="conv-title">
            {{ conv.title }}
          </div>
          <div class="conv-time">
            {{ formatTime(conv.updatedAt) }}
          </div>
        </div>
        <button
          class="conv-delete"
          title="删除"
          @click.stop="emit('delete-conversation', conv.id)"
        >
          <i class="fa-solid fa-xmark" />
        </button>
      </div>
    </div>

    <!-- Empty state -->
    <div
      v-if="apps.length === 0 && conversations.length === 0"
      class="empty-state"
    >
      <i class="fa-solid fa-cube" />
      <span>暂无应用</span>
      <p>点击 + 开始创建</p>
    </div>
  </div>
</template>

<style lang="less" scoped>
.apps-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  overflow-y: auto;
  padding: 8px;
}

.section {
  margin-bottom: 8px;
}

.section-title {
  font-size: 11px;
  font-weight: 600;
  color: #8B8680;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding: 4px 8px;
  margin-bottom: 4px;
}

.app-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s;

  &:hover {
    background: rgba(0, 0, 0, 0.03);
  }

  &.selected {
    background: rgba(196, 112, 75, 0.08);
  }
}

.app-icon {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(196, 112, 75, 0.10);
  border-radius: 8px;
  flex-shrink: 0;

  i {
    font-size: 16px;
    color: #C4704B;
  }
}

.app-info {
  flex: 1;
  min-width: 0;

  .app-name {
    font-size: 13px;
    font-weight: 500;
    color: #2D2B2A;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .app-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 2px;
  }

  .app-version {
    font-size: 11px;
    color: #8B8680;
  }

  .backend-status-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: none;
    cursor: pointer;
    transition: all 0.15s;
    font-size: 10px;

    &.stopped {
      color: #8B8680;
      &:hover { color: #10b981; }
    }

    &.starting,
    &.stopping {
      color: #D4A843;
      cursor: default;
    }

    &.running {
      color: #10b981;
      &:hover { color: #D94F4F; }
    }

    &.error {
      color: #D94F4F;
      &:hover { color: #D4A843; }
    }

    &:disabled {
      cursor: default;
    }
  }
}

.app-actions {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.15s;

  .app-item:hover & {
    opacity: 1;
  }
}

.action-btn {
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  color: #8B8680;
  transition: all 0.15s;

  &:hover {
    background: rgba(0, 0, 0, 0.05);
    color: #2D2B2A;
  }

  &.danger:hover {
    background: rgba(217, 79, 79, 0.08);
    color: #D94F4F;
  }

  i {
    font-size: 11px;
  }
}

.conv-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;

  &:hover {
    background: rgba(0, 0, 0, 0.03);
  }

  &.selected {
    background: rgba(196, 112, 75, 0.10);
  }
}

.conv-icon {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(196, 112, 75, 0.10);
  border-radius: 6px;
  flex-shrink: 0;

  i {
    font-size: 12px;
    color: #C4704B;
  }
}

.conv-info {
  flex: 1;
  min-width: 0;

  .conv-title {
    font-size: 12px;
    color: #2D2B2A;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .conv-time {
    font-size: 10px;
    color: #8B8680;
    margin-top: 2px;
  }
}

.conv-delete {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  color: #8B8680;
  opacity: 0;
  transition: all 0.15s;
  flex-shrink: 0;

  &:hover {
    background: rgba(217, 79, 79, 0.08);
    color: #D94F4F;
  }

  i {
    font-size: 10px;
  }

  .conv-item:hover & {
    opacity: 1;
  }
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  color: #8B8680;
  text-align: center;

  i {
    font-size: 32px;
    margin-bottom: 12px;
    opacity: 0.5;
  }

  span {
    font-size: 14px;
    font-weight: 500;
    margin-bottom: 4px;
  }

  p {
    font-size: 12px;
    opacity: 0.7;
  }
}
</style>
