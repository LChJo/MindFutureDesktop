import { defineStore } from 'pinia'
import type { FileNode } from '@/types/models'

export interface FileSystemState {
  files: FileNode[]
  rootId: string
}

export const useFileSystemStore = defineStore('fileSystem', {
  state: (): FileSystemState => ({
    files: [],
    rootId: 'root'
  }),

  getters: {
    getFileById: (state) => (id: string): FileNode | undefined => {
      return state.files.find(f => f.id === id)
    },

    getFilesByParentId: (state) => (parentId: string | null): FileNode[] => {
      return state.files.filter(f => f.parentId === parentId)
    },

    getRootFiles: (state): FileNode[] => {
      return state.files.filter(f => f.parentId === state.rootId)
    }
  },

  actions: {
    initializeFileSystem() {
      // Create root folder if not exists
      if (!this.files.find(f => f.id === this.rootId)) {
        const root: FileNode = {
          id: this.rootId,
          name: 'Root',
          type: 'folder',
          parentId: null,
          children: [],
          createdAt: new Date(),
          modifiedAt: new Date()
        }
        this.files.push(root)

        // Create default folders
        this.createFolder(this.rootId, 'Documents')
        this.createFolder(this.rootId, 'Downloads')
        this.createFolder(this.rootId, 'Pictures')
        this.createFolder(this.rootId, 'Projects')

        this.save()
      }
    },

    createFolder(parentId: string, name: string): FileNode {
      const newFolder: FileNode = {
        id: `folder-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
        name,
        type: 'folder',
        parentId,
        children: [],
        createdAt: new Date(),
        modifiedAt: new Date()
      }
      this.files.push(newFolder)

      // Update parent's children
      const parent = this.files.find(f => f.id === parentId)
      if (parent && parent.children) {
        parent.children.push(newFolder.id)
      }

      this.save()
      return newFolder
    },

    createFile(parentId: string, name: string, content: string = ''): FileNode {
      const newFile: FileNode = {
        id: `file-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
        name,
        type: 'file',
        parentId,
        content,
        createdAt: new Date(),
        modifiedAt: new Date(),
        size: content.length
      }
      this.files.push(newFile)

      // Update parent's children
      const parent = this.files.find(f => f.id === parentId)
      if (parent && parent.children) {
        parent.children.push(newFile.id)
      }

      this.save()
      return newFile
    },

    updateFile(fileId: string, updates: Partial<FileNode>) {
      const file = this.files.find(f => f.id === fileId)
      if (file) {
        Object.assign(file, updates, { modifiedAt: new Date() })
        if (updates.content !== undefined) {
          file.size = updates.content.length
        }
        this.save()
      }
    },

    deleteFile(fileId: string) {
      const file = this.files.find(f => f.id === fileId)
      if (!file) return

      // If it's a folder, delete all children recursively
      if (file.type === 'folder' && file.children) {
        for (const childId of file.children) {
          this.deleteFile(childId)
        }
      }

      // Remove from parent's children
      if (file.parentId) {
        const parent = this.files.find(f => f.id === file.parentId)
        if (parent && parent.children) {
          const index = parent.children.indexOf(fileId)
          if (index !== -1) {
            parent.children.splice(index, 1)
          }
        }
      }

      // Remove the file
      const index = this.files.findIndex(f => f.id === fileId)
      if (index !== -1) {
        this.files.splice(index, 1)
      }

      this.save()
    },

    deleteFiles(fileIds: string[]) {
      for (const fileId of fileIds) {
        this.deleteFile(fileId)
      }
    },

    moveFile(fileId: string, newParentId: string) {
      const file = this.files.find(f => f.id === fileId)
      if (!file) return

      // Remove from old parent
      if (file.parentId) {
        const oldParent = this.files.find(f => f.id === file.parentId)
        if (oldParent && oldParent.children) {
          const index = oldParent.children.indexOf(fileId)
          if (index !== -1) {
            oldParent.children.splice(index, 1)
          }
        }
      }

      // Add to new parent
      const newParent = this.files.find(f => f.id === newParentId)
      if (newParent && newParent.children) {
        newParent.children.push(fileId)
      }

      file.parentId = newParentId
      file.modifiedAt = new Date()
      this.save()
    },

    moveFiles(fileIds: string[], newParentId: string) {
      for (const fileId of fileIds) {
        this.moveFile(fileId, newParentId)
      }
    },

    copyFile(fileId: string, targetParentId: string): FileNode | null {
      const file = this.files.find(f => f.id === fileId)
      if (!file) return null

      if (file.type === 'file') {
        return this.createFile(targetParentId, `${file.name} (copy)`, file.content || '')
      } else {
        const newFolder = this.createFolder(targetParentId, `${file.name} (copy)`)
        // Copy children recursively
        if (file.children) {
          for (const childId of file.children) {
            this.copyFile(childId, newFolder.id)
          }
        }
        return newFolder
      }
    },

    copyFiles(fileIds: string[], targetParentId: string) {
      for (const fileId of fileIds) {
        this.copyFile(fileId, targetParentId)
      }
    },

    renameFile(fileId: string, newName: string) {
      const file = this.files.find(f => f.id === fileId)
      if (file) {
        file.name = newName
        file.modifiedAt = new Date()
        this.save()
      }
    },

    getPath(fileId: string): string {
      const parts: string[] = []
      let current = this.files.find(f => f.id === fileId)

      while (current && current.id !== this.rootId) {
        parts.unshift(current.name)
        current = this.files.find(f => f.id === current?.parentId)
      }

      return '/' + parts.join('/')
    },

    getFileByPath(path: string): FileNode | null {
      const parts = path.split('/').filter(Boolean)
      let current: FileNode | undefined = this.files.find(f => f.id === this.rootId)

      for (const part of parts) {
        if (!current || current.type !== 'folder') return null
        const children = this.files.filter(f => f.parentId === current!.id)
        current = children.find(f => f.name === part)
        if (!current) return null
      }

      return current || null
    },

    save() {
      localStorage.setItem('mcd-filesystem', JSON.stringify({
        files: this.files,
        rootId: this.rootId
      }))
    },

    load() {
      const saved = localStorage.getItem('mcd-filesystem')
      if (saved) {
        try {
          const data = JSON.parse(saved)
          this.files = data.files.map((f: FileNode) => ({
            ...f,
            createdAt: new Date(f.createdAt),
            modifiedAt: new Date(f.modifiedAt)
          }))
          this.rootId = data.rootId
        } catch (e) {
          console.error('Failed to load file system:', e)
          this.initializeFileSystem()
        }
      } else {
        this.initializeFileSystem()
      }
    }
  }
})
