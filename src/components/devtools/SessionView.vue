<template>
  <div
    ref="scrollContainer"
    class="session-view"
    @scroll="handleScroll"
  >
    <div
      v-for="msg in visibleMessages"
      :key="msg.uuid"
      class="message-block"
      :class="messageClass(msg)"
    >
      <!-- User message: tool_result only (no text) → compact indicator -->
      <template v-if="msg.message_type === 'user' && !msg.is_meta && msg.tool_results.length > 0 && !msg.text_content.trim()">
        <div class="message tool-result-indicator">
          <i class="fa-solid fa-circle-check" />
          <span class="tool-result-text">
            {{ msg.tool_results.length }} tool result{{ msg.tool_results.length > 1 ? 's' : '' }} returned
          </span>
          <span class="message-time">{{ formatTime(msg.timestamp) }}</span>
        </div>
      </template>

      <!-- User message: normal text -->
      <template v-else-if="msg.message_type === 'user' && !msg.is_meta">
        <div class="message user-message">
          <div class="message-header">
            <i class="fa-solid fa-user" />
            <span class="message-label">User</span>
            <span class="message-time">{{ formatTime(msg.timestamp) }}</span>
          </div>
          <div class="message-body user-body">
            {{ msg.text_content }}
          </div>
        </div>
      </template>

      <!-- Assistant message -->
      <template v-else-if="msg.message_type === 'assistant'">
        <div class="message assistant-message">
          <div class="message-header">
            <i class="fa-solid fa-robot" />
            <span class="message-label">Assistant</span>
            <span
              v-if="msg.model"
              class="model-badge"
            >{{ shortModel(msg.model) }}</span>
            <span
              v-if="msg.usage"
              class="token-pill"
              :title="usageTooltip(msg)"
            >
              {{ formatTokensCompact(totalTokens(msg)) }}
            </span>
            <span
              v-if="msg.cost_usd != null"
              class="cost-pill"
              :title="costTooltip(msg)"
            >
              {{ formatCostUsd(msg.cost_usd) }}
            </span>
            <span
              v-for="agentType in messageAgentTypes(msg)"
              :key="agentType"
              class="agent-type-badge"
              :style="{ color: agentTypeConfig(agentType).color, background: agentTypeConfig(agentType).bg }"
            >
              <i :class="agentTypeConfig(agentType).icon" />
              {{ agentType }}
            </span>
            <span class="message-time">{{ formatTime(msg.timestamp) }}</span>
          </div>

          <!-- Text content -->
          <div
            v-if="msg.text_content"
            class="message-body assistant-body"
          >
            {{ msg.text_content }}
          </div>

          <!-- Tool calls with inline subagents -->
          <div
            v-if="msg.tool_calls.length > 0"
            class="tool-calls"
          >
            <template
              v-for="tc in msg.tool_calls"
              :key="tc.id"
            >
              <ToolCallItem
                :tool-call="tc"
                :tool-result="findToolResult(tc.id)"
              />
              <SubagentTree
                v-if="tc.is_task && subagentsForTask(tc.id).length > 0"
                :subagents="subagentsForTask(tc.id)"
                class="inline-subagent"
              />
            </template>
          </div>
        </div>
      </template>

      <!-- System message (single-line indicator) -->
      <template v-else-if="msg.message_type === 'system'">
        <div class="message system-indicator">
          <i class="fa-solid fa-gear" />
          <span class="system-text">{{ msg.text_content }}</span>
          <span class="message-time">{{ formatTime(msg.timestamp) }}</span>
        </div>
      </template>

      <!-- Summary message (context compaction) -->
      <template v-else-if="msg.message_type === 'summary'">
        <div class="message summary-indicator">
          <i class="fa-solid fa-compress" />
          <span class="summary-label">Context Compacted</span>
          <span class="summary-text">{{ msg.text_content }}</span>
          <span class="message-time">{{ formatTime(msg.timestamp) }}</span>
        </div>
      </template>

      <!-- Tool result (meta user message) - shown inline with tool calls above -->
    </div>

    <div
      v-if="visibleMessages.length === 0"
      class="session-empty"
    >
      <i class="fa-solid fa-inbox" />
      <span>此 session 无可显示的消息</span>
    </div>

    <div
      ref="scrollAnchor"
      class="scroll-anchor"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, nextTick } from 'vue'
import type { ParsedMessage, SubagentInfo, ToolResultInfo } from '@/types/devtools'
import { formatTokensCompact, formatCostUsd, SUBAGENT_TYPE_CONFIG, SUBAGENT_DEFAULT_CONFIG } from '@/types/devtools'
import ToolCallItem from '@/components/devtools/ToolCallItem.vue'
import SubagentTree from '@/components/devtools/SubagentTree.vue'

const props = defineProps<{
  messages: ParsedMessage[]
  subagents: SubagentInfo[]
  isOngoing: boolean
}>()

const scrollContainer = ref<HTMLElement | null>(null)
const scrollAnchor = ref<HTMLElement | null>(null)
const isNearBottom = ref(true)
let savedScrollTop = 0

const visibleMessages = computed(() =>
  props.messages.filter((msg) => {
    // Skip meta user messages (tool results) - they're shown inline with tool calls
    if (msg.message_type === 'user' && msg.is_meta) return false
    return true
  })
)

// Build a map of tool_use_id -> ToolResultInfo from all user messages with tool results
const toolResultMap = computed(() => {
  const map = new Map<string, ToolResultInfo>()
  for (const msg of props.messages) {
    if (msg.message_type === 'user' && msg.tool_results.length > 0) {
      for (const tr of msg.tool_results) {
        map.set(tr.tool_use_id, tr)
      }
    }
  }
  return map
})

function findToolResult(toolUseId: string): ToolResultInfo | undefined {
  return toolResultMap.value.get(toolUseId)
}

// Map task tool_use_id -> SubagentInfo[] (uses backend-resolved parent_task_id)
const subagentMap = computed(() => {
  const map = new Map<string, SubagentInfo[]>()
  for (const sa of props.subagents) {
    if (sa.parent_task_id) {
      const existing = map.get(sa.parent_task_id) || []
      map.set(sa.parent_task_id, [...existing, sa])
    }
  }
  return map
})

function subagentsForTask(toolUseId: string): SubagentInfo[] {
  return subagentMap.value.get(toolUseId) || []
}

function messageAgentTypes(msg: ParsedMessage): string[] {
  const types: string[] = []
  for (const tc of msg.tool_calls) {
    if (!tc.is_task) continue
    for (const sa of subagentsForTask(tc.id)) {
      if (sa.subagent_type && !types.includes(sa.subagent_type)) {
        types.push(sa.subagent_type)
      }
    }
  }
  return types
}

function agentTypeConfig(agentType: string): { icon: string; color: string; bg: string } {
  const key = agentType.toLowerCase()
  return SUBAGENT_TYPE_CONFIG[key] || SUBAGENT_DEFAULT_CONFIG
}

function messageClass(msg: ParsedMessage): string {
  if (msg.message_type === 'user' && !msg.is_meta && msg.tool_results.length > 0 && !msg.text_content.trim()) return 'tool-result-block'
  if (msg.message_type === 'user' && !msg.is_meta) return 'user-block'
  if (msg.message_type === 'assistant') return 'assistant-block'
  if (msg.message_type === 'system') return 'system-block'
  if (msg.message_type === 'summary') return 'summary-block'
  return ''
}

function formatTime(timestamp: string): string {
  if (!timestamp) return ''
  try {
    const date = new Date(timestamp)
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
  } catch {
    return ''
  }
}

function shortModel(model: string): string {
  if (model.includes('opus')) return 'Opus'
  if (model.includes('sonnet')) return 'Sonnet'
  if (model.includes('haiku')) return 'Haiku'
  return model.split('-').slice(0, 2).join('-')
}

function totalTokens(msg: ParsedMessage): number {
  if (!msg.usage) return 0
  return (msg.usage.input_tokens || 0) + (msg.usage.output_tokens || 0)
}

function usageTooltip(msg: ParsedMessage): string {
  if (!msg.usage) return ''
  return [
    `Input: ${(msg.usage.input_tokens || 0).toLocaleString()}`,
    `Output: ${(msg.usage.output_tokens || 0).toLocaleString()}`,
  ].join(' | ')
}

function costTooltip(msg: ParsedMessage): string {
  if (!msg.cost_breakdown) return ''
  const b = msg.cost_breakdown
  return [
    `Total: $${b.total.toFixed(4)}`,
    `Input: $${b.input_cost.toFixed(4)}`,
    `Output: $${b.output_cost.toFixed(4)}`,
    `Cache Read: $${b.cache_read_cost.toFixed(4)}`,
    `Cache Create: $${b.cache_creation_cost.toFixed(4)}`,
  ].join('\n')
}

function handleScroll() {
  const el = scrollContainer.value
  if (!el) return
  isNearBottom.value = el.scrollHeight - el.scrollTop - el.clientHeight < 100
}

// Capture scroll position synchronously before DOM update
watch(
  () => props.messages,
  () => {
    const el = scrollContainer.value
    if (el) {
      savedScrollTop = el.scrollTop
    }
  },
  { flush: 'sync' }
)

// Restore scroll position or auto-scroll after DOM update
watch(
  () => props.messages.length,
  async (_newLen, oldLen) => {
    await nextTick()
    const el = scrollContainer.value
    if (!el) return

    if (props.isOngoing && isNearBottom.value) {
      // Auto-scroll for live sessions when near bottom
      scrollAnchor.value?.scrollIntoView({ behavior: 'smooth' })
    } else if (oldLen != null && oldLen > 0 && savedScrollTop > 0) {
      // Restore scroll position on data refresh
      el.scrollTop = savedScrollTop
    }
  }
)
</script>

<style scoped>
.session-view {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.message-block {
  max-width: 100%;
}

.message {
  border-radius: 8px;
  overflow: hidden;
}

.message-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  font-size: 11px;
  color: #8B8680;
}

.message-header i {
  font-size: 11px;
}

.message-label {
  font-weight: 600;
  color: #4A4845;
}

.message-time {
  margin-left: auto;
  font-size: 10px;
  color: #B0ACA7;
}

.model-badge {
  background: rgba(196, 112, 75, 0.10);
  padding: 1px 6px;
  border-radius: 8px;
  font-size: 10px;
  color: #C4704B;
}

.token-pill {
  background: rgba(22, 163, 74, 0.08);
  padding: 1px 6px;
  border-radius: 8px;
  font-size: 10px;
  color: #16A34A;
}

.cost-pill {
  background: rgba(202, 138, 4, 0.10);
  padding: 1px 6px;
  border-radius: 8px;
  font-size: 10px;
  color: #CA8A04;
  cursor: default;
}

.message-body {
  padding: 8px 12px;
  font-size: 13px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}

.user-message {
  background: #EBF0F7;
  border: 1px solid #D5DDE8;
}

.user-message .message-header i {
  color: #4A7FBF;
}

.assistant-message {
  background: #FFFFFF;
  border: 1px solid #E8E5E0;
}

.assistant-message .message-header i {
  color: #C4704B;
}

.assistant-body {
  color: #2D2B2A;
}

.user-body {
  color: #2D2B2A;
}

.tool-calls {
  padding: 4px 12px 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.inline-subagent {
  padding: 0;
  margin-top: 2px;
}

.tool-result-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 12px;
  font-size: 10px;
  color: #78B89A;
}

.tool-result-indicator i {
  font-size: 9px;
  color: #6DAE8E;
}

.tool-result-text {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.system-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 12px;
  font-size: 10px;
  color: #A8A29E;
}

.system-indicator i {
  font-size: 9px;
  color: #B0ACA7;
}

.system-text {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.summary-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  background: rgba(59, 130, 246, 0.06);
  border: 1px solid rgba(59, 130, 246, 0.15);
  border-radius: 6px;
  font-size: 11px;
  color: #6B7280;
}

.summary-indicator i {
  font-size: 10px;
  color: #3B82F6;
}

.summary-label {
  font-weight: 600;
  color: #3B82F6;
  flex-shrink: 0;
}

.summary-text {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #6B7280;
}

.agent-type-badge {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 1px 6px;
  border-radius: 8px;
  font-size: 10px;
  font-weight: 600;
}

.agent-type-badge i {
  font-size: 9px;
}

.scroll-anchor {
  height: 1px;
}

.session-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 48px 16px;
  color: #B0ACA7;
  font-size: 12px;
}

.session-empty i {
  font-size: 24px;
  color: #D5D0CB;
}
</style>
