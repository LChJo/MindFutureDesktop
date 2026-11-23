/**
 * Type definitions for Coworks frontend
 * Migrated from goodable/cowork
 */

// App modes - Chat (pure conversation), Code (project development), Cowork (file management)
export type AppMode = 'chat' | 'code' | 'cowork' | 'claudeapps'

// File entry from Rust backend
export interface FileEntry {
  name: string
  path: string
  is_dir: boolean
  size: number
  modified: number | null
}

// Tree node for file tree component
export interface TreeNode extends FileEntry {
  children?: TreeNode[]
  expanded?: boolean
  loading?: boolean
}

// Chat message types
export interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: number
  toolUsages?: ToolUsage[]
}

// Tool usage display
export interface ToolUsage {
  id: string
  toolName: string
  toolId: string
  input?: Record<string, unknown>
  summary?: string
  status: 'pending' | 'running' | 'completed' | 'error'
}

// Sidecar output message types
// sessionId 用于关联消息到正确的对话

export interface TextDeltaMessage {
  type: 'text_delta'
  sessionId: string
  seq?: number
  content: string
}

export interface ToolUseMessage {
  type: 'tool_use'
  sessionId: string
  seq?: number
  toolName: string
  toolId: string
  input: Record<string, unknown>
}

export interface ToolResultMessage {
  type: 'tool_result'
  sessionId: string
  seq?: number
  toolName: string
  toolId: string
  summary: string
}

export interface ProgressMessage {
  type: 'progress'
  sessionId: string
  seq?: number
  stage: string
  message?: string
}

export interface DoneMessage {
  type: 'done'
  sessionId: string
  seq?: number
}

export interface ErrorMessage {
  type: 'error'
  sessionId: string
  seq?: number
  message: string
}

export interface PongMessage {
  type: 'pong'
}

export interface CancelledMessage {
  type: 'cancelled'
  sessionId: string
  seq?: number
}

export interface ReadyMessage {
  type: 'ready'
}

export interface SessionStartedMessage {
  type: 'session_started'
  sessionId: string
  seq?: number
  sdkSessionId: string
}

export type SidecarOutput =
  | TextDeltaMessage
  | ToolUseMessage
  | ToolResultMessage
  | ProgressMessage
  | DoneMessage
  | ErrorMessage
  | PongMessage
  | CancelledMessage
  | ReadyMessage
  | SessionStartedMessage

// Config from Rust backend
export interface CoworksConfig {
  model: string | null
  last_work_dir: string | null
  recent_workspaces: string | null
}
