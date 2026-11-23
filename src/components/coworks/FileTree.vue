<script setup lang="ts">
import type { TreeNode } from '@/types/coworks'

const props = defineProps<{
  nodes: TreeNode[]
  selectedPath: string
  level?: number
  getFileIcon: (node: TreeNode) => string
}>()

const emit = defineEmits<{
  (e: 'toggle', node: TreeNode): void
  (e: 'select', node: TreeNode): void
  (e: 'open', node: TreeNode): void
}>()

const level = props.level ?? 0

// Get emoji icon for file type
function getEmoji(node: TreeNode): string {
  if (node.is_dir) {
    return node.expanded ? '📂' : '📁'
  }

  const ext = node.name.split('.').pop()?.toLowerCase()
  const iconMap: Record<string, string> = {
    // Documents
    md: '📝',
    txt: '📄',
    pdf: '📕',
    doc: '📘',
    docx: '📘',
    xls: '📊',
    xlsx: '📊',
    ppt: '📙',
    pptx: '📙',
    // Code
    js: '🟨',
    ts: '🔷',
    jsx: '⚛️',
    tsx: '⚛️',
    vue: '💚',
    py: '🐍',
    rs: '🦀',
    go: '🔵',
    java: '☕',
    // Data
    json: '📋',
    yaml: '📋',
    yml: '📋',
    xml: '📋',
    csv: '📊',
    // Images
    png: '🖼️',
    jpg: '🖼️',
    jpeg: '🖼️',
    gif: '🖼️',
    svg: '🖼️',
    // Archives
    zip: '📦',
    tar: '📦',
    gz: '📦',
    rar: '📦',
  }

  return iconMap[ext || ''] || '📄'
}
</script>

<template>
  <div class="file-tree">
    <div
      v-for="node in nodes"
      :key="node.path"
      class="tree-item"
    >
      <div
        class="tree-row"
        :class="{ selected: node.path === selectedPath }"
        :style="{ paddingLeft: `${level * 16 + 8}px` }"
        @click="emit('select', node)"
        @dblclick="node.is_dir ? emit('toggle', node) : emit('open', node)"
      >
        <span
          v-if="node.is_dir"
          class="expand-icon"
          @click.stop="emit('toggle', node)"
        >
          <template v-if="node.loading">⏳</template>
          <template v-else>{{ node.expanded ? '▼' : '▶' }}</template>
        </span>
        <span
          v-else
          class="expand-icon-placeholder"
        />
        <span class="file-icon">{{ getEmoji(node) }}</span>
        <span class="file-name">{{ node.name }}</span>
      </div>

      <FileTree
        v-if="node.is_dir && node.expanded && node.children"
        :nodes="node.children"
        :selected-path="selectedPath"
        :level="level + 1"
        :get-file-icon="getFileIcon"
        @toggle="emit('toggle', $event)"
        @select="emit('select', $event)"
        @open="emit('open', $event)"
      />
    </div>
  </div>
</template>

<style scoped>
.file-tree {
  font-size: 13px;
  user-select: none;
}

.tree-row {
  display: flex;
  align-items: center;
  padding: 4px 8px;
  cursor: pointer;
  border-radius: 4px;
  transition: background-color 0.15s;
}

.tree-row:hover {
  background-color: rgba(0, 0, 0, 0.03);
}

.tree-row.selected {
  background-color: rgba(196, 112, 75, 0.08);
}

.expand-icon {
  width: 16px;
  font-size: 10px;
  color: #8B8680;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}

.expand-icon-placeholder {
  width: 16px;
}

.file-icon {
  margin-right: 6px;
  font-size: 14px;
}

.file-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
