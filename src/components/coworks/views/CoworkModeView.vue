<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import FileTree from '../FileTree.vue'
import ChatPanel from '../ChatPanel.vue'
import TaskHistoryList from '../TaskHistoryList.vue'
import NewTaskDialog from '../NewTaskDialog.vue'
import { useFileTree } from '@/composables/coworks'
import { useChat } from '@/composables/coworks'
import type { TaskSkillInfo } from '@/types/coworks-persistence'

const chat = useChat()
const fileTree = useFileTree()

// Dialog states
const showNewTaskDialog = ref(false)

// Watch workDir changes and load file tree
watch(() => chat.workDir.value, async (newDir) => {
  if (newDir) {
    await fileTree.setRootPath(newDir)
    await fileTree.loadRoot()
  }
}, { immediate: true })

// Watch file changes and refresh file tree
watch(() => chat.lastFileChange.value, async (newTimestamp) => {
  if (newTimestamp > 0 && chat.workDir.value) {
    await fileTree.refresh()
  }
})

// Handle create new task
function handleShowNewTask() {
  showNewTaskDialog.value = true
}

// Handle task created
async function handleTaskCreated(task: {
  name: string
  workDir: string
  appId?: string
  skillInfo?: TaskSkillInfo
}) {
  showNewTaskDialog.value = false
  await chat.createNewTask(task.name, task.workDir, task.appId, task.skillInfo)
}

// Handle select task
async function handleSelectTask(id: string) {
  await chat.loadTask(id)
}

// Handle delete task
async function handleDeleteTask(id: string) {
  await chat.deleteTask(id)
}

// Handle file tree toggle
async function handleTreeToggle(node: any) {
  await fileTree.toggleExpand(node)
}

// Handle file tree select
function handleTreeSelect(node: any) {
  fileTree.selectNode(node)
}

// Handle file open with system default application
async function handleOpenFile(path: string) {
  try {
    await invoke('open_file_default', { path })
  } catch (error) {
    console.error('Failed to open file:', error)
  }
}

// Handle file tree open (double-click on file)
function handleTreeOpen(node: any) {
  if (!node.is_dir) {
    handleOpenFile(node.path)
  }
}

// Handle chat send
async function handleChatSend(message: string, attachments: any[] = []) {
  const attachmentInfos = attachments.map(a => ({ path: a.path, name: a.name }))
  await chat.sendMessage(message, attachmentInfos.length > 0 ? attachmentInfos : undefined)
}

// Get directory name from path
function getDirName(path: string): string {
  return path.split(/[/\\]/).pop() || path
}

// Get icon for generated file based on extension
function getGeneratedFileIcon(fileName: string): string {
  const ext = fileName.split('.').pop()?.toLowerCase()
  const iconMap: Record<string, string> = {
    // Images
    png: '🖼️',
    jpg: '🖼️',
    jpeg: '🖼️',
    gif: '🖼️',
    webp: '🖼️',
    svg: '🖼️',
    // Documents
    pdf: '📄',
    doc: '📝',
    docx: '📝',
    txt: '📄',
    md: '📝',
    // Code
    js: '📜',
    ts: '📜',
    py: '📜',
    json: '📋',
    // Audio/Video
    mp3: '🎵',
    mp4: '🎬',
    wav: '🎵',
  }
  return iconMap[ext || ''] || '📄'
}
</script>

<template>
  <div class="cowork-mode-view">
    <!-- Left panel: Tasks -->
    <div class="left-panel">
      <div class="panel-header">
        <button
          class="new-task-btn"
          @click="handleShowNewTask"
        >
          + 新建任务
        </button>
      </div>
      <div class="tasks-container">
        <TaskHistoryList
          :tasks="chat.tasks.value"
          :current-id="chat.contextId.value"
          :get-display-name="chat.getTaskDisplayName"
          @select="handleSelectTask"
          @delete="handleDeleteTask"
        />
      </div>
    </div>

    <!-- Main panel: Chat -->
    <div class="main-panel">
      <ChatPanel
        v-if="chat.contextId.value"
        :messages="chat.messages.value"
        :is-loading="chat.isLoading.value"
        :error="chat.error.value"
        :model="chat.model.value"
        empty-hint="新建任务或选择历史任务开始工作"
        @send="handleChatSend"
        @cancel="chat.cancelRequest"
        @clear="chat.clearError"
        @update:model="chat.model.value = $event"
      />
      <div
        v-else
        class="empty-placeholder"
      >
        <i class="fa-solid fa-list-check" />
        <span>新建任务或选择历史任务开始工作</span>
      </div>
    </div>

    <!-- Right panel: Work context -->
    <div class="right-panel">
      <!-- File Tree Section -->
      <div class="panel-section file-section">
        <div class="section-header">
          <span class="section-title">文件夹</span>
          <span
            v-if="chat.workDir.value"
            class="dir-name"
          >{{ getDirName(chat.workDir.value) }}</span>
        </div>
        <div class="file-tree-container">
          <div
            v-if="!chat.workDir.value"
            class="empty-hint"
          >
            请先选择任务
          </div>
          <div
            v-else-if="fileTree.loading.value"
            class="loading"
          >
            加载中...
          </div>
          <div
            v-else-if="fileTree.error.value"
            class="error"
          >
            {{ fileTree.error.value }}
          </div>
          <FileTree
            v-else
            :nodes="fileTree.rootNodes.value"
            :selected-path="fileTree.selectedPath.value"
            :get-file-icon="fileTree.getFileIcon"
            @toggle="handleTreeToggle"
            @select="handleTreeSelect"
            @open="handleTreeOpen"
          />
        </div>
      </div>

      <!-- Related Files Section -->
      <div class="panel-section related-section">
        <div class="section-header">
          <span class="section-title">相关文件</span>
        </div>
        <div class="related-content">
          <!-- Generated file (highlighted) -->
          <div
            v-if="chat.generatedFile.value"
            class="generated-file"
          >
            <div class="generated-file-header">
              <span class="generated-icon">✨</span>
              <span class="generated-label">生成的文件</span>
            </div>
            <div
              class="generated-file-item clickable"
              :title="chat.generatedFile.value.path"
              @click="handleOpenFile(chat.generatedFile.value.path)"
            >
              <span class="file-icon">{{ getGeneratedFileIcon(chat.generatedFile.value.name) }}</span>
              <span class="file-name">{{ chat.generatedFile.value.name }}</span>
            </div>
          </div>
          <!-- Other related files -->
          <div
            v-if="chat.relatedFiles.value.length === 0 && !chat.generatedFile.value"
            class="empty-hint"
          >
            尚无文件操作
          </div>
          <div
            v-else-if="chat.relatedFiles.value.length > 0"
            class="related-list"
          >
            <div
              v-for="file in chat.relatedFiles.value"
              :key="file.path + file.operation"
              class="related-item clickable"
              :title="file.path"
              @click="handleOpenFile(file.path)"
            >
              <span class="operation-icon">
                {{ file.operation === 'read' ? '📖' : file.operation === 'write' ? '✏️' : file.operation === 'edit' ? '📝' : '📄' }}
              </span>
              <span class="file-name">{{ file.path.split(/[/\\]/).pop() }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- New Task Dialog -->
    <NewTaskDialog
      v-if="showNewTaskDialog"
      @close="showNewTaskDialog = false"
      @create="handleTaskCreated"
    />
  </div>
</template>

<style scoped>
.cowork-mode-view {
  display: flex;
  height: 100%;
  width: 100%;
  background: #FAF9F7;
}

.left-panel {
  width: 180px;
  display: flex;
  flex-direction: column;
  border-right: 1px solid #E8E5E0;
  background: #FFFFFF;
}

.panel-header {
  padding: 12px;
  border-bottom: 1px solid #E8E5E0;
}

.new-task-btn {
  width: 100%;
  padding: 10px 16px;
  background: rgba(16, 185, 129, 0.10);
  border: 1px solid rgba(16, 185, 129, 0.2);
  border-radius: 8px;
  color: #10b981;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.new-task-btn:hover {
  background: rgba(16, 185, 129, 0.18);
}

.tasks-container {
  flex: 1;
  overflow-y: auto;
}

.main-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.right-panel {
  width: 240px;
  display: flex;
  flex-direction: column;
  border-left: 1px solid #E8E5E0;
  background: #FFFFFF;
}

.panel-section {
  display: flex;
  flex-direction: column;
  border-bottom: 1px solid #E8E5E0;
}

.file-section {
  flex: 1;
  min-height: 0;
}

.related-section {
  flex: 1;
  min-height: 120px;
  max-height: 300px;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid #E8E5E0;
}

.section-title {
  font-size: 11px;
  font-weight: 600;
  color: #8B8680;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.dir-name {
  font-size: 11px;
  color: #10b981;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 120px;
}

.file-tree-container {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.related-content {
  overflow-y: auto;
  flex: 1;
}

.related-list {
  padding: 8px 0;
}

.related-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 12px;
  font-size: 12px;
}

.clickable {
  cursor: pointer;
  transition: background-color 0.15s;
}

.clickable:hover {
  background-color: rgba(0, 0, 0, 0.04);
}

.operation-icon {
  font-size: 12px;
}

.file-name {
  color: #2D2B2A;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.empty-hint {
  padding: 12px;
  text-align: center;
  color: #8B8680;
  font-size: 12px;
}

.empty-placeholder {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: #B0ACA7;
  font-size: 14px;
}

.empty-placeholder i {
  font-size: 32px;
  opacity: 0.5;
}

.generated-file {
  padding: 8px 12px;
  background: rgba(196, 112, 75, 0.06);
  border-bottom: 1px solid rgba(196, 112, 75, 0.15);
}

.generated-file-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 6px;
}

.generated-icon {
  font-size: 12px;
}

.generated-label {
  font-size: 10px;
  color: #C4704B;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: 600;
}

.generated-file-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  background: rgba(196, 112, 75, 0.08);
  border-radius: 6px;
}

.generated-file-item.clickable:hover {
  background: rgba(196, 112, 75, 0.15);
}

.generated-file-item .file-icon {
  font-size: 16px;
}

.generated-file-item .file-name {
  color: #2D2B2A;
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.loading,
.error {
  padding: 12px;
  text-align: center;
  font-size: 12px;
}

.loading {
  color: #8B8680;
}

.error {
  color: #D94F4F;
}
</style>
