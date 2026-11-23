<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { homeDir, join } from '@tauri-apps/api/path'
import { mkdir, exists } from '@tauri-apps/plugin-fs'
import { useDesktopStore } from '@/stores/desktopStore'
import { useWindowStore } from '@/stores/windowStore'

interface DynamicFunctionInfo {
  name: string
  displayName: string
  version: string
  description: string
  icon: string
  backendEnabled: boolean
  path: string
}

defineProps<{
  windowId?: string
  windowData?: Record<string, unknown>
}>()

const desktopStore = useDesktopStore()
const windowStore = useWindowStore()

const functions = ref<DynamicFunctionInfo[]>([])
const loading = ref(true)
const refreshing = ref(false)

async function loadFunctions() {
  try {
    functions.value = await invoke<DynamicFunctionInfo[]>('df_list_functions')
  } catch (e) {
    console.error('Failed to list dynamic functions:', e)
  } finally {
    loading.value = false
    refreshing.value = false
  }
}

async function refreshFunctions() {
  refreshing.value = true
  await loadFunctions()

  // Sync desktop icons with discovered functions
  for (const func of functions.value) {
    const hasIcon = desktopStore.icons.some(
      i => i.type === 'dynamic-function' && i.dynamicFunctionInfo?.functionName === func.name
    )
    if (!hasIcon) {
      desktopStore.addDynamicFunction(
        func.name,
        func.displayName,
        func.icon,
        func.backendEnabled
      )
    }
  }
}

async function openFunction(func: DynamicFunctionInfo) {
  // Ensure icon exists on desktop
  const existingIcon = desktopStore.icons.find(
    i => i.type === 'dynamic-function' && i.dynamicFunctionInfo?.functionName === func.name
  )

  if (existingIcon) {
    windowStore.openWindow(existingIcon, {
      functionName: func.name,
      functionPath: func.path,
      backendPort: null
    })
  } else {
    // Create icon first, then open
    const result = desktopStore.addDynamicFunction(
      func.name,
      func.displayName,
      func.icon,
      func.backendEnabled
    )
    if (result.icon) {
      windowStore.openWindow(result.icon, {
        functionName: func.name,
        functionPath: func.path,
        backendPort: null
      })
    }
  }
}

async function deleteFunction(func: DynamicFunctionInfo) {
  try {
    await invoke('df_delete_function', { name: func.name })
    desktopStore.removeDynamicFunction(func.name)
    await loadFunctions()
  } catch (e) {
    console.error('Failed to delete function:', e)
  }
}

async function openCreatorTerminal() {
  const home = await homeDir()
  const dfDir = await join(home, '.MoontonClaudeDesktop', 'dynamic-functions')

  // Ensure directory exists
  const dirExists = await exists(dfDir)
  if (!dirExists) {
    await mkdir(dfDir, { recursive: true })
  }

  const prompt = `请在 ${dfDir} 目录下创建一个动态功能。

要求：
1. 创建一个子目录，目录名为功能名（英文小写，用连字符分隔）
2. 在子目录中创建以下文件：
   - manifest.json: 包含 name, displayName, version, description, icon, window, capabilities, backend 字段
   - frontend/index.html: 前端入口页面
   - frontend/app.js: 前端逻辑
   - frontend/style.css: 样式
   - backend/server.js: (可选) Node.js HTTP 服务，读取 PORT 和 HOST 环境变量

manifest.json 示例：
{
  "name": "my-function",
  "displayName": "我的功能",
  "version": "1.0.0",
  "description": "功能描述",
  "icon": "fa-solid fa-cube",
  "window": { "width": 800, "height": 600, "minWidth": 400, "minHeight": 300 },
  "capabilities": { "network": false, "localStorage": false },
  "backend": { "enabled": false }
}

iframe 内可通过 window.mcd 对象调用：
- mcd.backend(path, options) - 请求后端 API
- mcd.invoke('mcd:readFile', {path}) - 读取函数目录内文件
- mcd.invoke('mcd:writeFile', {path, content}) - 写入函数目录内文件
- mcd.setTitle(title) - 设置窗口标题
- mcd.notify(message, level) - 显示通知

请问你想创建什么样的动态功能？`

  const claudeCommand = `claude "${prompt.replace(/"/g, '\\"').replace(/\n/g, '\\n')}"`

  windowStore.openTerminalWithCwd(dfDir, '创建动态功能', claudeCommand)
}

async function revealInExplorer(func: DynamicFunctionInfo) {
  try {
    await invoke('reveal_in_explorer', { path: func.path })
  } catch (e) {
    console.error('Failed to reveal in explorer:', e)
  }
}

onMounted(() => {
  loadFunctions()
})
</script>

<template>
  <div class="df-creator">
    <!-- Header -->
    <div class="df-header">
      <div class="df-header-title">
        <i class="fa-solid fa-wand-magic-sparkles" />
        <span>动态功能</span>
      </div>
      <div class="df-header-actions">
        <button
          class="df-btn df-btn-refresh"
          :disabled="refreshing"
          @click="refreshFunctions"
        >
          <i
            class="fa-solid fa-rotate"
            :class="{ 'fa-spin': refreshing }"
          />
          <span>刷新</span>
        </button>
        <button
          class="df-btn df-btn-create"
          @click="openCreatorTerminal"
        >
          <i class="fa-solid fa-plus" />
          <span>创建</span>
        </button>
      </div>
    </div>

    <!-- Content -->
    <div class="df-content">
      <!-- Loading -->
      <div
        v-if="loading"
        class="df-empty"
      >
        <i class="fa-solid fa-spinner fa-spin" />
        <span>加载中...</span>
      </div>

      <!-- Empty state -->
      <div
        v-else-if="functions.length === 0"
        class="df-empty"
      >
        <i class="fa-solid fa-cube" />
        <span>暂无动态功能</span>
        <p>点击"创建"按钮，用自然语言描述你想要的功能，Claude 将自动生成代码。</p>
      </div>

      <!-- Function list -->
      <div
        v-else
        class="df-list"
      >
        <div
          v-for="func in functions"
          :key="func.name"
          class="df-item"
          @dblclick="openFunction(func)"
        >
          <div class="df-item-icon">
            <i :class="func.icon" />
          </div>
          <div class="df-item-info">
            <div class="df-item-name">
              {{ func.displayName }}
            </div>
            <div class="df-item-desc">
              {{ func.description || func.name }}
            </div>
            <div class="df-item-meta">
              <span class="df-item-version">v{{ func.version }}</span>
              <span
                v-if="func.backendEnabled"
                class="df-item-backend"
              >
                <i class="fa-solid fa-server" /> 后端
              </span>
            </div>
          </div>
          <div class="df-item-actions">
            <button
              class="df-icon-btn"
              title="打开"
              @click.stop="openFunction(func)"
            >
              <i class="fa-solid fa-arrow-up-right-from-square" />
            </button>
            <button
              class="df-icon-btn"
              title="打开目录"
              @click.stop="revealInExplorer(func)"
            >
              <i class="fa-solid fa-folder-open" />
            </button>
            <button
              class="df-icon-btn danger"
              title="删除"
              @click.stop="deleteFunction(func)"
            >
              <i class="fa-solid fa-trash" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="less" scoped>
.df-creator {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: #f9fafb;
}

.df-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  background: #fff;
  border-bottom: 1px solid #e5e7eb;

  .df-header-title {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 18px;
    font-weight: 600;
    color: #1f2937;

    i {
      color: #8b5cf6;
    }
  }

  .df-header-actions {
    display: flex;
    gap: 8px;
  }
}

.df-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;

  &.df-btn-refresh {
    background: #f3f4f6;
    color: #4b5563;

    &:hover:not(:disabled) {
      background: #e5e7eb;
    }

    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }

  &.df-btn-create {
    background: #8b5cf6;
    color: white;

    &:hover {
      background: #7c3aed;
    }
  }
}

.df-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px;
}

.df-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 12px;
  color: #9ca3af;

  i {
    font-size: 48px;
  }

  span {
    font-size: 16px;
    font-weight: 500;
  }

  p {
    font-size: 14px;
    max-width: 400px;
    text-align: center;
    line-height: 1.6;
  }
}

.df-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.df-item {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 16px;
  background: #fff;
  border-radius: 10px;
  border: 1px solid #e5e7eb;
  cursor: pointer;
  transition: all 0.2s;

  &:hover {
    border-color: #8b5cf6;
    box-shadow: 0 2px 8px rgba(139, 92, 246, 0.1);
  }
}

.df-item-icon {
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #f5f3ff;
  border-radius: 10px;
  flex-shrink: 0;

  i {
    font-size: 20px;
    color: #8b5cf6;
  }
}

.df-item-info {
  flex: 1;
  min-width: 0;

  .df-item-name {
    font-size: 15px;
    font-weight: 600;
    color: #1f2937;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .df-item-desc {
    font-size: 13px;
    color: #6b7280;
    margin-top: 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .df-item-meta {
    display: flex;
    gap: 10px;
    margin-top: 4px;

    .df-item-version {
      font-size: 12px;
      color: #9ca3af;
    }

    .df-item-backend {
      font-size: 12px;
      color: #10b981;
      display: flex;
      align-items: center;
      gap: 4px;

      i {
        font-size: 10px;
      }
    }
  }
}

.df-item-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}

.df-icon-btn {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  color: #9ca3af;
  transition: all 0.2s;

  &:hover {
    background: #f3f4f6;
    color: #4b5563;
  }

  &.danger:hover {
    background: #fef2f2;
    color: #ef4444;
  }

  i {
    font-size: 14px;
  }
}
</style>
