/**
 * File tree composable for Coworks
 * Handles file system navigation and tree state
 */

import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { FileEntry, TreeNode } from '@/types/coworks'

export function useFileTree() {
  const rootPath = ref<string>('')
  const rootNodes = ref<TreeNode[]>([])
  const selectedPath = ref<string>('')
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Set root directory and load contents
  async function setRootPath(path: string) {
    rootPath.value = path
    selectedPath.value = ''
    await loadDirectory(path)
  }

  // Load directory contents
  async function loadDirectory(path: string): Promise<TreeNode[]> {
    try {
      const entries = await invoke<FileEntry[]>('coworks_read_directory', { path })
      return entries.map((entry) => ({
        ...entry,
        expanded: false,
        loading: false,
        children: entry.is_dir ? undefined : undefined,
      }))
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to load directory'
      return []
    }
  }

  // Load root directory
  async function loadRoot() {
    if (!rootPath.value) return

    loading.value = true
    error.value = null

    try {
      rootNodes.value = await loadDirectory(rootPath.value)
    } finally {
      loading.value = false
    }
  }

  // Toggle directory expansion
  async function toggleExpand(node: TreeNode) {
    if (!node.is_dir) return

    if (node.expanded) {
      node.expanded = false
      return
    }

    node.loading = true
    try {
      node.children = await loadDirectory(node.path)
      node.expanded = true
    } finally {
      node.loading = false
    }
  }

  // Select a file or directory
  function selectNode(node: TreeNode) {
    selectedPath.value = node.path
  }

  // Refresh current tree
  async function refresh() {
    if (!rootPath.value) return

    // Store expanded paths
    const expandedPaths = new Set<string>()
    function collectExpanded(nodes: TreeNode[]) {
      for (const node of nodes) {
        if (node.expanded) {
          expandedPaths.add(node.path)
          if (node.children) {
            collectExpanded(node.children)
          }
        }
      }
    }
    collectExpanded(rootNodes.value)

    // Reload root
    await loadRoot()

    // Restore expanded state
    async function restoreExpanded(nodes: TreeNode[]) {
      for (const node of nodes) {
        if (expandedPaths.has(node.path)) {
          await toggleExpand(node)
          if (node.children) {
            await restoreExpanded(node.children)
          }
        }
      }
    }
    await restoreExpanded(rootNodes.value)
  }

  // Get file icon based on extension
  function getFileIcon(node: TreeNode): string {
    if (node.is_dir) {
      return node.expanded ? 'folder-open' : 'folder'
    }

    const ext = node.name.split('.').pop()?.toLowerCase()
    const iconMap: Record<string, string> = {
      // Documents
      md: 'file-text',
      txt: 'file-text',
      pdf: 'file-pdf',
      doc: 'file-word',
      docx: 'file-word',
      xls: 'file-excel',
      xlsx: 'file-excel',
      ppt: 'file-powerpoint',
      pptx: 'file-powerpoint',
      // Code
      js: 'file-code',
      ts: 'file-code',
      jsx: 'file-code',
      tsx: 'file-code',
      vue: 'file-code',
      py: 'file-code',
      rs: 'file-code',
      go: 'file-code',
      java: 'file-code',
      // Data
      json: 'file-code',
      yaml: 'file-code',
      yml: 'file-code',
      xml: 'file-code',
      csv: 'file-excel',
      // Images
      png: 'file-image',
      jpg: 'file-image',
      jpeg: 'file-image',
      gif: 'file-image',
      svg: 'file-image',
      // Archives
      zip: 'file-archive',
      tar: 'file-archive',
      gz: 'file-archive',
      rar: 'file-archive',
    }

    return iconMap[ext || ''] || 'file'
  }

  return {
    rootPath,
    rootNodes,
    selectedPath,
    loading,
    error,
    setRootPath,
    loadRoot,
    toggleExpand,
    selectNode,
    refresh,
    getFileIcon,
  }
}
