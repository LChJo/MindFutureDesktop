<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { BotDefinition } from '@/types/claude-bot'

interface FileEntry {
  name: string
  path: string
  isDir: boolean
  size: number
  modified: string
}

const props = defineProps<{
  bot: BotDefinition
}>()

const files = ref<FileEntry[]>([])
const loading = ref(true)
const error = ref('')
const currentPath = ref('')
const rootPath = ref('')
let refreshTimer: ReturnType<typeof setInterval> | null = null

const isAtRoot = computed(() => currentPath.value === rootPath.value)

const breadcrumbs = computed(() => {
  if (!currentPath.value || !rootPath.value) return []
  const rel = currentPath.value
    .replace(rootPath.value, '')
    .replace(/^[\\/]/, '')
  if (!rel) return []
  return rel.split(/[\\/]/)
})

const contextMenu = ref<{ show: boolean; x: number; y: number; item: FileEntry | null }>({
  show: false,
  x: 0,
  y: 0,
  item: null,
})

function getFileIcon(item: FileEntry): string {
  if (item.isDir) return 'fa-solid fa-folder'
  const ext = item.name.split('.').pop()?.toLowerCase()
  switch (ext) {
    case 'js':
    case 'ts':
    case 'jsx':
    case 'tsx':
    case 'py':
    case 'rs':
    case 'go':
    case 'c':
    case 'cpp':
    case 'java':
    case 'rb':
    case 'sh':
    case 'bat':
      return 'fa-solid fa-file-code'
    case 'html':
    case 'css':
    case 'scss':
    case 'less':
    case 'vue':
    case 'svelte':
      return 'fa-solid fa-file-code'
    case 'json':
    case 'xml':
    case 'yaml':
    case 'yml':
    case 'toml':
    case 'csv':
      return 'fa-solid fa-file-alt'
    case 'md':
    case 'txt':
    case 'log':
      return 'fa-solid fa-file-lines'
    case 'png':
    case 'jpg':
    case 'jpeg':
    case 'gif':
    case 'svg':
    case 'webp':
    case 'ico':
    case 'bmp':
      return 'fa-solid fa-file-image'
    case 'pdf':
      return 'fa-solid fa-file-pdf'
    case 'zip':
    case 'tar':
    case 'gz':
    case 'rar':
    case '7z':
      return 'fa-solid fa-file-zipper'
    case 'mp3':
    case 'wav':
    case 'flac':
    case 'ogg':
      return 'fa-solid fa-file-audio'
    case 'mp4':
    case 'avi':
    case 'mkv':
    case 'mov':
    case 'webm':
      return 'fa-solid fa-file-video'
    case 'doc':
    case 'docx':
      return 'fa-solid fa-file-word'
    case 'xls':
    case 'xlsx':
      return 'fa-solid fa-file-excel'
    case 'ppt':
    case 'pptx':
      return 'fa-solid fa-file-powerpoint'
    default:
      return 'fa-solid fa-file'
  }
}

function getIconColor(item: FileEntry): string {
  if (item.isDir) return '#54aeff'
  const ext = item.name.split('.').pop()?.toLowerCase()
  switch (ext) {
    case 'js':
    case 'jsx':
      return '#f1e05a'
    case 'ts':
    case 'tsx':
      return '#3178c6'
    case 'py':
      return '#3572a5'
    case 'html':
      return '#e34c26'
    case 'css':
    case 'scss':
    case 'less':
      return '#563d7c'
    case 'vue':
      return '#41b883'
    case 'json':
      return '#a9a9a9'
    case 'md':
    case 'txt':
      return '#8b949e'
    case 'png':
    case 'jpg':
    case 'jpeg':
    case 'gif':
    case 'svg':
    case 'webp':
      return '#d4a0ff'
    case 'pdf':
      return '#f85149'
    default:
      return '#8b949e'
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

async function loadFiles(path?: string) {
  loading.value = true
  error.value = ''

  try {
    if (!rootPath.value) {
      const outputDir: string = await invoke('bot_ensure_output_directory', { botId: props.bot.id })
      rootPath.value = outputDir
    }

    const targetPath = path || rootPath.value
    files.value = await invoke<FileEntry[]>('list_directory', { path: targetPath })
    currentPath.value = targetPath
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
    files.value = []
  } finally {
    loading.value = false
  }
}

async function handleDoubleClick(item: FileEntry) {
  if (item.isDir) {
    await loadFiles(item.path)
  } else {
    try {
      await invoke('open_file_default', { path: item.path })
    } catch (e) {
      console.error('Failed to open file:', e)
    }
  }
}

async function goUp() {
  if (isAtRoot.value) return
  const parent = currentPath.value.replace(/[\\/][^\\/]+$/, '')
  if (parent && parent !== currentPath.value) {
    await loadFiles(parent)
  }
}

async function goToRoot() {
  if (rootPath.value) {
    await loadFiles(rootPath.value)
  }
}

async function openInExplorer() {
  try {
    await invoke('reveal_in_explorer', { path: currentPath.value })
  } catch (e) {
    console.error('Failed to reveal in explorer:', e)
  }
}

function showContextMenu(event: MouseEvent, item: FileEntry) {
  event.preventDefault()
  contextMenu.value = {
    show: true,
    x: event.clientX,
    y: event.clientY,
    item,
  }
}

function hideContextMenu() {
  contextMenu.value.show = false
}

async function handleContextAction(action: string) {
  const item = contextMenu.value.item
  if (!item) return
  hideContextMenu()

  switch (action) {
    case 'open':
      await invoke('open_file_default', { path: item.path })
      break
    case 'reveal':
      await invoke('reveal_in_explorer', { path: item.path })
      break
    case 'copy-path':
      navigator.clipboard.writeText(item.path)
      break
  }
}

onMounted(() => {
  document.addEventListener('click', hideContextMenu)
  loadFiles()
  refreshTimer = setInterval(() => loadFiles(), 5000)
})

onUnmounted(() => {
  document.removeEventListener('click', hideContextMenu)
  if (refreshTimer) {
    clearInterval(refreshTimer)
  }
})
</script>

<template>
  <div class="bot-output-viewer">
    <!-- Toolbar -->
    <div class="output-toolbar">
      <div class="toolbar-left">
        <button
          v-if="!isAtRoot"
          class="toolbar-btn"
          title="返回上级"
          @click="goUp"
        >
          <i class="fa-solid fa-arrow-left" />
        </button>
        <button
          v-if="!isAtRoot"
          class="toolbar-btn"
          title="返回根目录"
          @click="goToRoot"
        >
          <i class="fa-solid fa-house" />
        </button>
        <span class="toolbar-label">
          <i class="fa-solid fa-folder-open" />
          <span v-if="breadcrumbs.length === 0">output</span>
          <template v-else>
            <span
              class="breadcrumb-link"
              @click="goToRoot"
            >output</span>
            <template
              v-for="(crumb, idx) in breadcrumbs"
              :key="idx"
            >
              <span class="breadcrumb-sep">/</span>
              <span class="breadcrumb-current">{{ crumb }}</span>
            </template>
          </template>
        </span>
      </div>
      <div class="toolbar-right">
        <span
          v-if="files.length > 0"
          class="file-count"
        >{{ files.length }} 项</span>
        <button
          class="toolbar-btn"
          title="刷新"
          @click="loadFiles()"
        >
          <i class="fa-solid fa-rotate-right" />
        </button>
        <button
          class="toolbar-btn"
          title="在资源管理器中打开"
          @click="openInExplorer"
        >
          <i class="fa-solid fa-folder-open" />
        </button>
      </div>
    </div>

    <!-- Loading -->
    <div
      v-if="loading && files.length === 0"
      class="state-msg"
    >
      <i class="fa-solid fa-spinner fa-spin" />
      加载中...
    </div>

    <!-- Error -->
    <div
      v-else-if="error"
      class="state-msg state-error"
    >
      <i class="fa-solid fa-exclamation-triangle" />
      <span>{{ error }}</span>
      <button @click="loadFiles()">
        重试
      </button>
    </div>

    <!-- Empty -->
    <div
      v-else-if="files.length === 0"
      class="state-msg state-empty"
    >
      <i class="fa-solid fa-folder-open empty-icon" />
      <p>AI 生成的文件将显示在这里</p>
      <p class="sub">
        在对话中让机器人创建文件，文件会自动出现
      </p>
    </div>

    <!-- File Grid -->
    <div
      v-else
      class="file-grid"
      @click.self="hideContextMenu"
    >
      <div
        v-for="item in files"
        :key="item.path"
        class="file-item"
        :title="`${item.name}\n${item.isDir ? '文件夹' : formatSize(item.size)}\n${item.modified}`"
        @dblclick="handleDoubleClick(item)"
        @contextmenu="showContextMenu($event, item)"
      >
        <i
          :class="getFileIcon(item)"
          class="file-icon"
          :style="{ color: getIconColor(item) }"
        />
        <span class="file-name">{{ item.name }}</span>
        <span
          v-if="!item.isDir"
          class="file-size"
        >{{ formatSize(item.size) }}</span>
      </div>
    </div>

    <!-- Context Menu -->
    <div
      v-if="contextMenu.show"
      class="context-menu"
      :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
    >
      <div
        class="menu-item"
        @click="handleContextAction('open')"
      >
        <i class="fa-solid fa-arrow-up-right-from-square" /> 打开
      </div>
      <div
        class="menu-item"
        @click="handleContextAction('reveal')"
      >
        <i class="fa-solid fa-folder-open" /> 在资源管理器中显示
      </div>
      <div
        class="menu-item"
        @click="handleContextAction('copy-path')"
      >
        <i class="fa-solid fa-copy" /> 复制路径
      </div>
    </div>
  </div>
</template>

<style scoped lang="less">
.bot-output-viewer {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #0d1117;
  position: relative;
}

.output-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  border-bottom: 1px solid #30363d;
  background: #161b22;
  gap: 8px;
}

.toolbar-left,
.toolbar-right {
  display: flex;
  align-items: center;
  gap: 4px;
}

.toolbar-btn {
  width: 26px;
  height: 26px;
  background: none;
  border: 1px solid transparent;
  border-radius: 4px;
  color: #8b949e;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;

  &:hover {
    background: #21262d;
    color: #e6edf3;
    border-color: #30363d;
  }
}

.toolbar-label {
  font-size: 12px;
  color: #8b949e;
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: 4px;
}

.breadcrumb-link {
  cursor: pointer;
  color: #58a6ff;
  &:hover { text-decoration: underline; }
}

.breadcrumb-sep {
  color: #484f58;
  margin: 0 2px;
}

.breadcrumb-current {
  color: #e6edf3;
}

.file-count {
  font-size: 11px;
  color: #6e7681;
  margin-right: 4px;
}

.state-msg {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: #6e7681;
  font-size: 13px;
}

.state-error {
  color: #f85149;

  button {
    margin-top: 8px;
    padding: 6px 16px;
    background: rgba(248, 81, 73, 0.1);
    border: 1px solid rgba(248, 81, 73, 0.2);
    border-radius: 4px;
    color: #f85149;
    cursor: pointer;
    &:hover { background: rgba(248, 81, 73, 0.2); }
  }
}

.state-empty {
  .empty-icon {
    font-size: 36px;
    color: #30363d;
    margin-bottom: 4px;
  }

  p {
    margin: 0;
    line-height: 1.5;
  }

  .sub {
    font-size: 11px;
    color: #484f58;
  }
}

.file-grid {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
  gap: 4px;
  align-content: start;
}

.file-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 10px 6px 8px;
  border-radius: 6px;
  cursor: pointer;
  user-select: none;
  transition: background 0.1s;

  &:hover {
    background: #161b22;
  }

  &:active {
    background: #1c2128;
  }
}

.file-icon {
  font-size: 28px;
}

.file-name {
  font-size: 11px;
  color: #e6edf3;
  text-align: center;
  word-break: break-all;
  line-height: 1.3;
  max-height: 2.6em;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.file-size {
  font-size: 10px;
  color: #484f58;
}

.context-menu {
  position: fixed;
  z-index: 1000;
  background: #1c2128;
  border: 1px solid #30363d;
  border-radius: 6px;
  padding: 4px 0;
  min-width: 180px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
}

.menu-item {
  padding: 6px 12px;
  font-size: 12px;
  color: #e6edf3;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;

  &:hover {
    background: #30363d;
  }

  i {
    width: 14px;
    text-align: center;
    color: #8b949e;
  }
}
</style>
