<script setup lang="ts">
import { watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import FileTree from '../FileTree.vue'
import ChatPanel from '../ChatPanel.vue'
import ContextPanel from '../ContextPanel.vue'
import { useFileTree } from '@/composables/coworks'
import { useChat } from '@/composables/coworks'

const chat = useChat()
const fileTree = useFileTree()

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

// Handle open project
async function handleOpenProject() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择项目目录',
  })

  if (selected) {
    await chat.setProject(selected as string)
  }
}

// Handle select recent project
async function handleSelectProject(path: string) {
  await chat.setProject(path)
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

// Get project display name
function getProjectName(path: string): string {
  return path.split(/[/\\]/).pop() || path
}
</script>

<template>
  <div class="code-mode-view">
    <!-- Left panel: Projects -->
    <div class="left-panel">
      <div class="panel-header">
        <button
          class="open-project-btn"
          @click="handleOpenProject"
        >
          打开项目...
        </button>
      </div>
      <div class="projects-container">
        <div class="section-title">
          最近项目
        </div>
        <div
          v-if="chat.recentProjects.value.length === 0"
          class="empty-hint"
        >
          暂无项目
        </div>
        <div
          v-for="project in chat.recentProjects.value"
          :key="project.path"
          class="project-item"
          :class="{ active: chat.workDir.value === project.path }"
          @click="handleSelectProject(project.path)"
        >
          <span class="project-icon">📁</span>
          <span class="project-name">{{ getProjectName(project.path) }}</span>
        </div>
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
        empty-hint="选择一个项目开始编程对话"
        @send="handleChatSend"
        @cancel="chat.cancelRequest"
        @clear="chat.clearError"
        @update:model="chat.model.value = $event"
      />
      <div
        v-else
        class="empty-placeholder"
      >
        <i class="fa-solid fa-folder-open" />
        <span>选择一个项目开始编程对话</span>
      </div>
    </div>

    <!-- Right panel: Context -->
    <div class="right-panel">
      <!-- File Tree Section -->
      <div class="panel-section file-section">
        <div class="section-header">
          <span class="section-title">文件夹</span>
        </div>
        <div class="file-tree-container">
          <div
            v-if="!chat.workDir.value"
            class="empty-hint"
          >
            请选择项目
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

      <!-- Context Section -->
      <div class="panel-section context-section">
        <div class="section-header">
          <span class="section-title">上下文</span>
        </div>
        <ContextPanel />
      </div>
    </div>
  </div>
</template>

<style scoped>
.code-mode-view {
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

.open-project-btn {
  width: 100%;
  padding: 10px 16px;
  background: rgba(196, 112, 75, 0.10);
  border: 1px solid rgba(196, 112, 75, 0.2);
  border-radius: 8px;
  color: #C4704B;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.open-project-btn:hover {
  background: rgba(196, 112, 75, 0.18);
}

.projects-container {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.section-title {
  padding: 8px 12px;
  font-size: 11px;
  font-weight: 600;
  color: #8B8680;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.project-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  cursor: pointer;
  transition: all 0.15s;
}

.project-item:hover {
  background: rgba(0, 0, 0, 0.03);
}

.project-item.active {
  background: rgba(196, 112, 75, 0.08);
}

.project-icon {
  font-size: 14px;
}

.project-name {
  flex: 1;
  font-size: 13px;
  color: #2D2B2A;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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

.context-section {
  flex-shrink: 0;
}

.section-header {
  padding: 8px 12px;
  border-bottom: 1px solid #E8E5E0;
}

.section-header .section-title {
  padding: 0;
}

.file-tree-container {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
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
