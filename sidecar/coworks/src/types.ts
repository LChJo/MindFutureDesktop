// Input message types
export type CoworkMode = 'chat' | 'code' | 'cowork' | 'claudeapps'

// Plugin info for skill-based tasks
export interface PluginInfo {
  path: string
  skillName: string
  skillDescription: string
}

// Attachment info
export interface AttachmentInfo {
  path: string
  name: string
}

export interface StartMessage {
  type: 'start'
  sessionId: string
  workDir: string
  model: string
  prompt: string
  mode: CoworkMode
  systemPrompt?: string
  usePreset?: boolean    // If true, systemPrompt is appended to claude_code preset instead of replacing it
  plugin?: PluginInfo
  attachments?: AttachmentInfo[]
  sdkSessionId?: string  // SDK session ID for resuming conversation context
}

export interface CancelMessage {
  type: 'cancel'
  sessionId: string
}

export interface PingMessage {
  type: 'ping'
}

export type InputMessage = StartMessage | CancelMessage | PingMessage

// Output message types
export interface TextDeltaOutput {
  type: 'text_delta'
  sessionId: string
  seq?: number
  content: string
}

export interface ToolUseOutput {
  type: 'tool_use'
  sessionId: string
  seq?: number
  toolId: string
  toolName: string
  input: Record<string, unknown>
}

export interface ToolResultOutput {
  type: 'tool_result'
  sessionId: string
  seq?: number
  toolId: string
  summary: string
}

export interface ProgressOutput {
  type: 'progress'
  sessionId: string
  seq?: number
  stage: string
  message?: string
}

export interface DoneOutput {
  type: 'done'
  sessionId: string
  seq?: number
}

export interface ErrorOutput {
  type: 'error'
  sessionId: string
  seq?: number
  message: string
}

export interface CancelledOutput {
  type: 'cancelled'
  sessionId: string
  seq?: number
}

export interface PongOutput {
  type: 'pong'
}

export interface ReadyOutput {
  type: 'ready'
}

export interface SessionStartedOutput {
  type: 'session_started'
  sessionId: string
  seq?: number
  sdkSessionId: string
}

export type OutputMessage =
  | TextDeltaOutput
  | ToolUseOutput
  | ToolResultOutput
  | ProgressOutput
  | DoneOutput
  | ErrorOutput
  | CancelledOutput
  | PongOutput
  | ReadyOutput
  | SessionStartedOutput
