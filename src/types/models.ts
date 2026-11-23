import type { Component } from 'vue'

// Desktop Icon Types
export interface DesktopIcon {
  id: string
  name: string
  icon: string // Font Awesome class
  position: { x: number; y: number }
  type: 'terminal' | 'file-manager' | 'settings' | 'plugin-market' | 'plugin-market-full' | 'mcp-manager' | 'custom' | 'add-project-link' | 'project-link' | 'plugin-shortcut' | 'setup-guide' | 'env-vars' | 'coworks' | 'llm-router' | 'dynamic-function' | 'dynamic-function-creator' | 'dev-tools' | 'claude-aliases' | 'claude-bot'
  command?: string // 启动参数 or project path for project-link
  pluginInfo?: {
    pluginName: string
    pluginDescription: string
    marketplaceName: string
    skillName: string
    skillDescription?: string
  }
  dynamicFunctionInfo?: {
    functionName: string
    displayName: string
    backendEnabled: boolean
  }
  environmentVariables?: Record<string, string> // 图标专属环境变量
  customArgs?: string // 用户自定义启动命令（完整命令，如 "claude -r"）
}

// Window Types
export type WindowState = 'normal' | 'minimized' | 'maximized'

export interface WindowPosition {
  x: number
  y: number
  width: number
  height: number
}

export interface WindowInstance {
  id: string
  iconId: string
  title: string
  component: Component | string
  state: WindowState
  position: WindowPosition
  zIndex: number
  terminalId?: string
  data?: Record<string, unknown>
}

// File System Types
export type FileNodeType = 'file' | 'folder'

export interface FileNode {
  id: string
  name: string
  type: FileNodeType
  parentId: string | null
  content?: string // for files
  children?: string[] // folder children IDs
  createdAt: Date
  modifiedAt: Date
  size?: number
  mimeType?: string
}

// Terminal Types
export interface TerminalInstance {
  id: string
  windowId: string
  ptyId: string // PTY process ID from backend
  workingDirectory: string
  defaultCli: string // 'powershell', 'cmd', 'claude-code', etc.
  history: string[]
}

// Plugin Types
export interface Plugin {
  id: string
  name: string
  description: string
  version: string
  author: string
  downloads: number
  rating: number
  enabled?: boolean
  configSchema?: Record<string, unknown>
  config?: Record<string, unknown>
}

// Settings Types
export interface Shortcut {
  id: string
  type: 'terminal' | 'file-manager' | 'custom'
  command?: string
  args?: string
}

export interface Settings {
  environmentVariables: Record<string, string>
  defaultCli: string
  claudeCodePath: string
  defaultWorkingDirectory: string
  terminalFontSize: number
  textEditorPath: string
  shortcuts: Shortcut[]
  keybindings: Record<string, string>
}

// Context Menu Types
export interface ContextMenuItem {
  id: string
  label: string
  icon?: string
  action: () => void
  disabled?: boolean
  separator?: boolean
  children?: ContextMenuItem[]
}

export interface ContextMenuState {
  visible: boolean
  x: number
  y: number
  items: ContextMenuItem[]
}
