<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ask } from '@tauri-apps/plugin-dialog'
import { useFileSystemStore } from '@/stores/fileSystemStore'
import type { FileNode } from '@/types/models'

defineProps<{
  windowId?: string
}>()

const fileSystemStore = useFileSystemStore()

const currentPath = ref<string[]>([])
const selectedFiles = ref<string[]>([])
const clipboard = ref<{ files: string[]; operation: 'copy' | 'cut' }>({ files: [], operation: 'copy' })
const isProcessingOpen = ref(false) // Prevent double-click issues
const renamingFileId = ref<string | null>(null)
const renameValue = ref('')
const previewFile = ref<FileNode | null>(null)

const currentParentId = computed(() => {
  if (currentPath.value.length === 0) {
    return fileSystemStore.rootId
  }
  return currentPath.value[currentPath.value.length - 1]
})

const currentFiles = computed(() => {
  return fileSystemStore.getFilesByParentId(currentParentId.value)
    .sort((a, b) => {
      // Folders first
      if (a.type !== b.type) {
        return a.type === 'folder' ? -1 : 1
      }
      return a.name.localeCompare(b.name)
    })
})

const breadcrumbs = computed(() => {
  const crumbs = [{ id: fileSystemStore.rootId, name: '根目录' }]
  for (const id of currentPath.value) {
    const file = fileSystemStore.getFileById(id)
    if (file) {
      crumbs.push({ id: file.id, name: file.name })
    }
  }
  return crumbs
})

const getFileIcon = (file: FileNode): string => {
  if (file.type === 'folder') return 'fa-solid fa-folder'

  const ext = file.name.split('.').pop()?.toLowerCase()
  const iconMap: Record<string, string> = {
    'txt': 'fa-solid fa-file-lines',
    'md': 'fa-solid fa-file-lines',
    'js': 'fa-solid fa-file-code',
    'ts': 'fa-solid fa-file-code',
    'vue': 'fa-solid fa-file-code',
    'json': 'fa-solid fa-file-code',
    'html': 'fa-solid fa-file-code',
    'css': 'fa-solid fa-file-code',
    'png': 'fa-solid fa-file-image',
    'jpg': 'fa-solid fa-file-image',
    'jpeg': 'fa-solid fa-file-image',
    'gif': 'fa-solid fa-file-image',
    'pdf': 'fa-solid fa-file-pdf',
    'zip': 'fa-solid fa-file-zipper',
    'rar': 'fa-solid fa-file-zipper'
  }
  return iconMap[ext || ''] || 'fa-solid fa-file'
}

const navigateTo = (index: number) => {
  if (index === 0) {
    currentPath.value = []
  } else {
    currentPath.value = currentPath.value.slice(0, index)
  }
  selectedFiles.value = []
}

const openFile = (file: FileNode) => {
  // Prevent double-click from triggering multiple times
  if (isProcessingOpen.value) return
  isProcessingOpen.value = true
  setTimeout(() => { isProcessingOpen.value = false }, 300)

  if (file.type === 'folder') {
    currentPath.value.push(file.id)
    selectedFiles.value = []
  } else {
    previewFile.value = file
  }
}

const selectFile = (file: FileNode, event: MouseEvent) => {
  if (event.ctrlKey || event.metaKey) {
    const index = selectedFiles.value.indexOf(file.id)
    if (index === -1) {
      selectedFiles.value.push(file.id)
    } else {
      selectedFiles.value.splice(index, 1)
    }
  } else if (event.shiftKey && selectedFiles.value.length > 0) {
    const fileIds = currentFiles.value.map(f => f.id)
    const lastSelected = selectedFiles.value[selectedFiles.value.length - 1]
    const lastIndex = fileIds.indexOf(lastSelected)
    const currentIndex = fileIds.indexOf(file.id)
    const start = Math.min(lastIndex, currentIndex)
    const end = Math.max(lastIndex, currentIndex)
    selectedFiles.value = fileIds.slice(start, end + 1)
  } else {
    selectedFiles.value = [file.id]
  }
}

const createFolder = () => {
  const name = prompt('输入文件夹名称:')
  if (name) {
    fileSystemStore.createFolder(currentParentId.value, name)
  }
}

const createFile = () => {
  const name = prompt('输入文件名称:')
  if (name) {
    fileSystemStore.createFile(currentParentId.value, name, '')
  }
}

const deleteSelected = async () => {
  if (selectedFiles.value.length === 0) return
  const confirmed = await ask(`确定删除 ${selectedFiles.value.length} 个项目？`, {
    title: '删除确认',
    kind: 'warning',
  })
  if (confirmed) {
    fileSystemStore.deleteFiles(selectedFiles.value)
    selectedFiles.value = []
  }
}

const copySelected = () => {
  if (selectedFiles.value.length === 0) return
  clipboard.value = { files: [...selectedFiles.value], operation: 'copy' }
}

const cutSelected = () => {
  if (selectedFiles.value.length === 0) return
  clipboard.value = { files: [...selectedFiles.value], operation: 'cut' }
}

const paste = () => {
  if (clipboard.value.files.length === 0) return

  if (clipboard.value.operation === 'copy') {
    fileSystemStore.copyFiles(clipboard.value.files, currentParentId.value)
  } else {
    fileSystemStore.moveFiles(clipboard.value.files, currentParentId.value)
    clipboard.value = { files: [], operation: 'copy' }
  }
}

const startRename = (file: FileNode) => {
  renamingFileId.value = file.id
  renameValue.value = file.name
}

const finishRename = () => {
  if (renamingFileId.value && renameValue.value.trim()) {
    fileSystemStore.renameFile(renamingFileId.value, renameValue.value.trim())
  }
  renamingFileId.value = null
  renameValue.value = ''
}

const handleDragStart = (event: DragEvent, file: FileNode) => {
  const filesToDrag = selectedFiles.value.includes(file.id)
    ? selectedFiles.value
    : [file.id]
  event.dataTransfer!.setData('fileIds', JSON.stringify(filesToDrag))
  event.dataTransfer!.effectAllowed = 'move'
}

const handleDrop = (event: DragEvent, targetFile?: FileNode) => {
  event.preventDefault()
  const fileIds = JSON.parse(event.dataTransfer!.getData('fileIds') || '[]')
  if (fileIds.length === 0) return

  const targetId = targetFile?.type === 'folder'
    ? targetFile.id
    : currentParentId.value

  // Don't allow dropping on itself
  if (fileIds.includes(targetId)) return

  fileSystemStore.moveFiles(fileIds, targetId)
}

const formatDate = (date: Date): string => {
  return new Date(date).toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  })
}

const formatSize = (size?: number): string => {
  if (size === undefined) return '-'
  if (size < 1024) return `${size} B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`
  return `${(size / 1024 / 1024).toFixed(1)} MB`
}

onMounted(() => {
  fileSystemStore.load()
})
</script>

<template>
  <div class="file-manager">
    <!-- Toolbar -->
    <div class="toolbar">
      <button
        class="toolbar-btn"
        title="新建文件夹"
        @click="createFolder"
      >
        <i class="fa-solid fa-folder-plus" />
      </button>
      <button
        class="toolbar-btn"
        title="新建文件"
        @click="createFile"
      >
        <i class="fa-solid fa-file-circle-plus" />
      </button>
      <div class="toolbar-separator" />
      <button
        class="toolbar-btn"
        :disabled="selectedFiles.length === 0"
        title="剪切"
        @click="cutSelected"
      >
        <i class="fa-solid fa-scissors" />
      </button>
      <button
        class="toolbar-btn"
        :disabled="selectedFiles.length === 0"
        title="复制"
        @click="copySelected"
      >
        <i class="fa-solid fa-copy" />
      </button>
      <button
        class="toolbar-btn"
        :disabled="clipboard.files.length === 0"
        title="粘贴"
        @click="paste"
      >
        <i class="fa-solid fa-paste" />
      </button>
      <div class="toolbar-separator" />
      <button
        class="toolbar-btn danger"
        :disabled="selectedFiles.length === 0"
        title="删除"
        @click="deleteSelected"
      >
        <i class="fa-solid fa-trash" />
      </button>
    </div>

    <!-- Breadcrumb -->
    <div class="breadcrumb">
      <span
        v-for="(crumb, index) in breadcrumbs"
        :key="crumb.id"
        class="breadcrumb-item"
        @click="navigateTo(index)"
      >
        {{ crumb.name }}
        <i
          v-if="index < breadcrumbs.length - 1"
          class="fa-solid fa-chevron-right"
        />
      </span>
    </div>

    <!-- File List -->
    <div
      class="file-list"
      @drop="handleDrop($event)"
      @dragover.prevent
    >
      <div
        v-for="file in currentFiles"
        :key="file.id"
        class="file-item"
        :class="{ selected: selectedFiles.includes(file.id) }"
        draggable="true"
        @click="selectFile(file, $event)"
        @dblclick="openFile(file)"
        @dragstart="handleDragStart($event, file)"
        @drop.stop="handleDrop($event, file)"
        @dragover.prevent
        @contextmenu.prevent="startRename(file)"
      >
        <div class="file-icon">
          <i :class="getFileIcon(file)" />
        </div>
        <div class="file-info">
          <div
            v-if="renamingFileId === file.id"
            class="file-rename"
          >
            <input
              ref="renameInput"
              v-model="renameValue"
              autofocus
              @blur="finishRename"
              @keyup.enter="finishRename"
              @keyup.esc="renamingFileId = null"
              @click.stop
            >
          </div>
          <span
            v-else
            class="file-name"
          >{{ file.name }}</span>
          <span class="file-meta">
            {{ file.type === 'folder' ? '文件夹' : formatSize(file.size) }}
            · {{ formatDate(file.modifiedAt) }}
          </span>
        </div>
      </div>

      <div
        v-if="currentFiles.length === 0"
        class="empty-folder"
      >
        <i class="fa-solid fa-folder-open" />
        <p>文件夹为空</p>
      </div>
    </div>

    <!-- File Preview Modal -->
    <Teleport to="body">
      <div
        v-if="previewFile"
        class="preview-overlay"
        @click="previewFile = null"
      >
        <div
          class="preview-modal"
          @click.stop
        >
          <div class="preview-header">
            <span>{{ previewFile.name }}</span>
            <button @click="previewFile = null">
              <i class="fa-solid fa-xmark" />
            </button>
          </div>
          <div class="preview-content">
            <pre v-if="previewFile.content">{{ previewFile.content }}</pre>
            <p
              v-else
              class="empty-file"
            >
              文件内容为空
            </p>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style lang="less" scoped>
.file-manager {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #fafafa;
}

.toolbar {
  display: flex;
  align-items: center;
  padding: 8px 12px;
  background: white;
  border-bottom: 1px solid #e0e0e0;
  gap: 4px;

  .toolbar-btn {
    width: 32px;
    height: 32px;
    border: none;
    background: transparent;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #555;
    transition: all 0.2s;

    &:hover:not(:disabled) {
      background: #f0f0f0;
      color: #333;
    }

    &:disabled {
      opacity: 0.4;
      cursor: not-allowed;
    }

    &.danger:hover:not(:disabled) {
      background: #fee;
      color: #d00;
    }
  }

  .toolbar-separator {
    width: 1px;
    height: 20px;
    background: #ddd;
    margin: 0 8px;
  }
}

.breadcrumb {
  display: flex;
  align-items: center;
  padding: 10px 16px;
  background: white;
  border-bottom: 1px solid #e0e0e0;
  gap: 4px;
  flex-wrap: wrap;

  .breadcrumb-item {
    display: flex;
    align-items: center;
    gap: 4px;
    color: #0066cc;
    cursor: pointer;
    font-size: 13px;

    &:hover {
      text-decoration: underline;
    }

    &:last-child {
      color: #333;
      cursor: default;

      &:hover {
        text-decoration: none;
      }
    }

    i {
      color: #999;
      font-size: 10px;
    }
  }
}

.file-list {
  flex: 1;
  overflow: auto;
  padding: 12px;
}

.file-item {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.15s;
  gap: 12px;

  &:hover {
    background: #f0f0f0;
  }

  &.selected {
    background: #e3f2fd;

    &:hover {
      background: #bbdefb;
    }
  }

  .file-icon {
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;

    i {
      font-size: 28px;
      color: #ffc107;

      &.fa-file,
      &.fa-file-lines,
      &.fa-file-code {
        color: #90a4ae;
      }

      &.fa-file-image {
        color: #26a69a;
      }

      &.fa-file-pdf {
        color: #e53935;
      }
    }
  }

  .file-info {
    flex: 1;
    min-width: 0;

    .file-name {
      display: block;
      font-size: 14px;
      color: #333;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .file-meta {
      display: block;
      font-size: 12px;
      color: #999;
      margin-top: 2px;
    }

    .file-rename {
      input {
        width: 100%;
        padding: 4px 8px;
        border: 1px solid #2196f3;
        border-radius: 4px;
        font-size: 14px;
        outline: none;
      }
    }
  }
}

.empty-folder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 200px;
  color: #999;

  i {
    font-size: 48px;
    margin-bottom: 12px;
  }

  p {
    font-size: 14px;
  }
}

.preview-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10002;
}

.preview-modal {
  width: 600px;
  max-width: 90vw;
  max-height: 80vh;
  background: white;
  border-radius: 8px;
  overflow: hidden;
  display: flex;
  flex-direction: column;

  .preview-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    background: #f5f5f5;
    border-bottom: 1px solid #e0e0e0;

    span {
      font-weight: 500;
    }

    button {
      border: none;
      background: transparent;
      cursor: pointer;
      padding: 4px 8px;
      border-radius: 4px;

      &:hover {
        background: #e0e0e0;
      }
    }
  }

  .preview-content {
    flex: 1;
    overflow: auto;
    padding: 16px;

    pre {
      margin: 0;
      white-space: pre-wrap;
      word-break: break-word;
      font-family: 'Consolas', 'Monaco', monospace;
      font-size: 13px;
      line-height: 1.5;
    }

    .empty-file {
      color: #999;
      text-align: center;
      padding: 40px;
    }
  }
}
</style>
