// Types matching Rust backend output (devtools.rs)

export interface DevToolsProject {
  id: string
  path: string
  name: string
  session_count: number
  most_recent: number | null
}

export interface SessionSummary {
  id: string
  project_id: string
  created_at: number
  first_message: string | null
  message_count: number
  has_subagents: boolean
  is_ongoing: boolean
  git_branch: string | null
  metrics: SessionMetrics
}

export interface SessionMetrics {
  duration_ms: number
  total_tokens: number
  input_tokens: number
  output_tokens: number
  cache_read_tokens: number
  cache_creation_tokens: number
  message_count: number
  tool_call_count: number
  cost_usd: number
  tool_error_count: number
}

export interface CostBreakdown {
  total: number
  input_cost: number
  output_cost: number
  cache_read_cost: number
  cache_creation_cost: number
}

export interface UsageMetadata {
  input_tokens: number | null
  output_tokens: number | null
  cache_read_input_tokens: number | null
  cache_creation_input_tokens: number | null
}

export interface ToolProgress {
  output_lines: number
  elapsed_ms: number
  data_type: string
}

export interface ToolCallInfo {
  id: string
  name: string
  input: Record<string, unknown>
  is_task: boolean
  task_description: string | null
  progress: ToolProgress | null
}

export interface ToolResultInfo {
  tool_use_id: string
  content: unknown
  is_error: boolean
}

export interface ParsedMessage {
  uuid: string
  parent_uuid: string | null
  message_type: string
  timestamp: string
  role: string | null
  text_content: string
  usage: UsageMetadata | null
  model: string | null
  cwd: string | null
  git_branch: string | null
  is_sidechain: boolean
  is_meta: boolean
  tool_calls: ToolCallInfo[]
  tool_results: ToolResultInfo[]
  cost_usd: number | null
  cost_breakdown: CostBreakdown | null
  subtype: string | null
  message_id: string | null
}

export interface SubagentInfo {
  id: string
  file_path: string
  description: string | null
  parent_task_id: string | null
  start_time: string | null
  end_time: string | null
  duration_ms: number
  metrics: SessionMetrics
  messages: ParsedMessage[]
  subagent_type: string | null
  is_parallel: boolean
}

export const SUBAGENT_TYPE_CONFIG: Record<string, { icon: string; color: string; bg: string }> = {
  Explore:   { icon: 'fa-solid fa-magnifying-glass', color: '#2563EB', bg: 'rgba(37,99,235,0.10)' },
  Plan:      { icon: 'fa-solid fa-map',              color: '#7C3AED', bg: 'rgba(124,58,237,0.10)' },
  Implement: { icon: 'fa-solid fa-code',             color: '#059669', bg: 'rgba(5,150,105,0.10)' },
  Test:      { icon: 'fa-solid fa-flask-vial',       color: '#D97706', bg: 'rgba(217,119,6,0.10)' },
  Review:    { icon: 'fa-solid fa-eye',              color: '#DC2626', bg: 'rgba(220,38,38,0.10)' },
  Fix:       { icon: 'fa-solid fa-wrench',           color: '#CA8A04', bg: 'rgba(202,138,4,0.10)' },
}

export const SUBAGENT_DEFAULT_CONFIG = {
  icon: 'fa-solid fa-sitemap', color: '#C4704B', bg: 'rgba(196,112,75,0.08)',
}

export interface SessionDetail {
  session: SessionSummary
  messages: ParsedMessage[]
  subagents: SubagentInfo[]
  metrics: SessionMetrics
}

// Formatting utilities

export function formatTokensCompact(tokens: number): string {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`
  if (tokens >= 1_000) return `${(tokens / 1_000).toFixed(1)}k`
  return tokens.toString()
}

export function formatCostUsd(cost: number): string {
  if (cost === 0) return '$0.00'
  if (cost >= 0.01) return `$${cost.toFixed(2)}`
  if (cost >= 0.001) return `$${cost.toFixed(3)}`
  return `$${cost.toFixed(4)}`
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`
  const minutes = Math.floor(ms / 60_000)
  const seconds = Math.floor((ms % 60_000) / 1000)
  if (minutes >= 60) {
    const hours = Math.floor(minutes / 60)
    const remainingMinutes = minutes % 60
    return `${hours}h ${remainingMinutes}m`
  }
  return `${minutes}m ${seconds}s`
}

export function formatRelativeTime(timestamp: number): string {
  const now = Date.now()
  const diff = now - timestamp
  const seconds = Math.floor(diff / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)

  if (seconds < 60) return 'just now'
  if (minutes < 60) return `${minutes}m ago`
  if (hours < 24) return `${hours}h ago`
  if (days < 7) return `${days}d ago`
  return new Date(timestamp).toLocaleDateString()
}

export function getToolIcon(toolName: string): string {
  const icons: Record<string, string> = {
    Read: 'fa-solid fa-file-lines',
    Write: 'fa-solid fa-file-pen',
    Edit: 'fa-solid fa-file-code',
    Bash: 'fa-solid fa-terminal',
    Grep: 'fa-solid fa-magnifying-glass',
    Glob: 'fa-solid fa-folder-tree',
    Task: 'fa-solid fa-sitemap',
    TodoRead: 'fa-solid fa-list-check',
    TodoWrite: 'fa-solid fa-square-check',
    WebSearch: 'fa-solid fa-globe',
    WebFetch: 'fa-solid fa-download',
    LSP: 'fa-solid fa-code',
    AskUserQuestion: 'fa-solid fa-circle-question',
    EnterPlanMode: 'fa-solid fa-map',
    ExitPlanMode: 'fa-solid fa-flag-checkered',
    NotebookEdit: 'fa-solid fa-book',
    Skill: 'fa-solid fa-wand-magic-sparkles',
  }
  return icons[toolName] || 'fa-solid fa-wrench'
}

export function getToolSummary(tool: ToolCallInfo): string {
  const input = tool.input as Record<string, unknown>
  switch (tool.name) {
    case 'Read':
      return `Read ${input.file_path || 'file'}`
    case 'Write':
      return `Write ${input.file_path || 'file'}`
    case 'Edit':
      return `Edit ${input.file_path || 'file'}`
    case 'Bash':
      return `$ ${typeof input.command === 'string' ? (input.command.length > 80 ? input.command.slice(0, 80) + '...' : input.command) : 'command'}`
    case 'Grep':
      return `Grep "${input.pattern || ''}" ${input.path || ''}`
    case 'Glob':
      return `Glob ${input.pattern || ''}`
    case 'Task':
      return tool.task_description || 'Task agent'
    case 'WebSearch':
      return `Search: ${input.query || ''}`
    case 'WebFetch':
      return `Fetch: ${input.url || ''}`
    case 'LSP':
      return `LSP ${input.operation || ''}`
    default:
      return tool.name
  }
}
