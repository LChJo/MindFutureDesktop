/**
 * Persistence-related type definitions for Coworks
 * For storing chat history, tasks, and projects
 * Migrated from goodable/cowork
 */

import type { ChatMessage } from './coworks'

// Chat history (for Chat mode)
export interface ChatHistory {
  id: string
  title: string // Auto-generated from first message
  messages: ChatMessage[]
  createdAt: number
  updatedAt: number
  sdkSessionId?: string // SDK session ID for conversation continuity
  model?: string // Persisted model choice for this chat session
}

// Chat history index
export interface ChatHistoryIndex {
  items: ChatHistoryIndexItem[]
  lastUpdated: number
}

export interface ChatHistoryIndexItem {
  id: string
  title: string
  createdAt: number
  updatedAt: number
  messageCount: number
}

// Related file tracking (for Cowork mode)
export interface RelatedFile {
  path: string
  operation: 'read' | 'write' | 'edit' | 'delete' | 'move' | 'copy'
  timestamp: number
}

// Work log entry (for Cowork mode)
export interface WorkLog {
  id: string
  action: string // e.g., "Moved 5 files", "Created report.md"
  timestamp: number
  details?: string
}

// Task (for Cowork mode)
export interface CoworksTask {
  id: string
  name: string
  appId?: string // Associated built-in app
  workDir: string // Working directory
  messages: ChatMessage[]
  relatedFiles: RelatedFile[]
  workLogs: WorkLog[]
  createdAt: number
  updatedAt: number
  status: 'active' | 'completed' | 'archived'
  skillInfo?: TaskSkillInfo // Optional skill info for skill-based tasks
  sdkSessionId?: string // SDK session ID for conversation continuity
  model?: string // Persisted model choice for this task session
}

// Task history index
export interface TaskIndex {
  items: TaskIndexItem[]
  lastUpdated: number
}

export interface TaskIndexItem {
  id: string
  name: string
  appId?: string
  workDir: string
  status: 'active' | 'completed' | 'archived'
  createdAt: number
  updatedAt: number
  messageCount: number
}

// Project (for Code mode)
export interface CoworksProject {
  id: string
  name: string
  path: string
  projectType?: string
  createdAt: number
  updatedAt: number
}

// Code project session (for Code mode SDK session persistence)
export interface CodeProjectSession {
  projectPath: string
  sdkSessionId: string
  updatedAt: number
  model?: string // Persisted model choice for this project session
}

// Projects list
export interface ProjectList {
  items: CoworksProject[]
  lastUpdated: number
}

// Built-in application
export interface BuiltinApp {
  id: string
  name: string
  description: string
  icon: string
  mode: 'work' | 'code'
  prompt: string
}

// Skill-based application (from plugin marketplace or local skills)
export interface SkillApp {
  id: string
  name: string
  description: string
  icon: string
  iconType?: 'fontawesome' | 'emoji'
  iconCategory?: 'local' | 'plugin'
  mode: 'work'
  prompt: string
  // Skill-specific fields
  isSkill: true
  skillName: string
  pluginName: string
  pluginPath: string
  marketplaceName: string
}

// Union type for all app types
export type CoworkApp = BuiltinApp | SkillApp

// Helper type guard to check if an app is a skill app
export function isSkillApp(app: CoworkApp): app is SkillApp {
  return 'isSkill' in app && app.isSkill === true
}

// Skill info for tasks (stored when task is created with a skill)
export interface TaskSkillInfo {
  skillName: string
  skillDescription: string // The skill's description (used as systemPrompt)
  pluginName: string
  pluginPath: string
  marketplaceName: string
}
