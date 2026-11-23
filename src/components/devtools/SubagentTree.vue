<template>
  <div class="subagent-tree">
    <div
      v-for="agent in subagents"
      :key="agent.id"
      class="subagent-card"
    >
      <div
        class="subagent-header"
        @click="toggleExpand(agent.id)"
      >
        <!-- Type badge -->
        <span
          class="type-badge"
          :style="{ color: typeConfig(agent).color, background: typeConfig(agent).bg }"
        >
          <i :class="typeConfig(agent).icon" />
          {{ agent.subagent_type || 'Agent' }}
        </span>

        <!-- Status indicator -->
        <span
          v-if="!agent.end_time"
          class="status-spinner"
          title="Running"
        >
          <i class="fa-solid fa-spinner fa-spin" />
        </span>
        <span
          v-else
          class="status-done"
          title="Completed"
        >
          <i class="fa-solid fa-circle-check" />
        </span>

        <!-- Parallel indicator -->
        <span
          v-if="agent.is_parallel"
          class="parallel-badge"
          title="Parallel execution"
        >
          <i class="fa-solid fa-layer-group" />
        </span>

        <!-- Description -->
        <span
          v-if="agent.description"
          class="subagent-desc"
          :title="agent.description"
        >
          {{ truncate(agent.description, 80) }}
        </span>

        <!-- Metrics pills -->
        <div class="subagent-metrics">
          <span class="metric-pill">
            <i class="fa-solid fa-clock" />
            {{ formatDuration(agent.duration_ms) }}
          </span>
          <span class="metric-pill">
            <i class="fa-solid fa-coins" />
            {{ formatTokensCompact(agent.metrics.total_tokens) }}
          </span>
          <span class="metric-pill">
            {{ formatCostUsd(agent.metrics.cost_usd) }}
          </span>
          <span
            v-if="agent.metrics.tool_call_count > 0"
            class="metric-pill"
          >
            <i class="fa-solid fa-wrench" />
            {{ agent.metrics.tool_call_count }}
          </span>
        </div>

        <!-- Expand chevron -->
        <i
          class="fa-solid expand-chevron"
          :class="expandedAgents.has(agent.id) ? 'fa-chevron-up' : 'fa-chevron-down'"
        />
      </div>

      <!-- Expanded: execution trace -->
      <div
        v-if="expandedAgents.has(agent.id)"
        class="subagent-trace"
      >
        <div class="trace-timeline">
          <div
            v-for="(item, idx) in (traceCache.get(agent.id) || [])"
            :key="idx"
            class="trace-item"
            :class="'trace-' + item.type"
          >
            <div class="trace-dot" />
            <div class="trace-content">
              <template v-if="item.type === 'text'">
                <div class="trace-text">
                  {{ item.text }}
                </div>
              </template>
              <template v-else-if="item.type === 'tool_call'">
                <div class="trace-tool">
                  <i :class="item.icon" />
                  <span class="trace-tool-name">{{ item.toolName }}</span>
                  <span class="trace-tool-summary">{{ item.text }}</span>
                </div>
              </template>
              <template v-else-if="item.type === 'tool_result'">
                <div
                  class="trace-result"
                  :class="{ 'trace-error': item.isError }"
                >
                  <i :class="item.isError ? 'fa-solid fa-xmark' : 'fa-solid fa-check'" />
                  <span>{{ item.text }}</span>
                </div>
              </template>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { SubagentInfo } from '@/types/devtools'
import {
  formatTokensCompact,
  formatCostUsd,
  formatDuration,
  getToolIcon,
  getToolSummary,
  SUBAGENT_TYPE_CONFIG,
  SUBAGENT_DEFAULT_CONFIG,
} from '@/types/devtools'

const props = defineProps<{
  subagents: SubagentInfo[]
}>()

const expandedAgents = ref(new Set<string>())

function toggleExpand(id: string) {
  const next = new Set(expandedAgents.value)
  if (next.has(id)) {
    next.delete(id)
  } else {
    next.add(id)
  }
  expandedAgents.value = next
}

function typeConfig(agent: SubagentInfo) {
  const key = agent.subagent_type || ''
  return SUBAGENT_TYPE_CONFIG[key] || SUBAGENT_DEFAULT_CONFIG
}

interface TraceItem {
  type: 'text' | 'tool_call' | 'tool_result'
  text: string
  icon?: string
  toolName?: string
  isError?: boolean
}

// Memoized trace cache: only recomputes when subagents or expanded set changes
const traceCache = computed(() => {
  const map = new Map<string, TraceItem[]>()
  for (const agent of props.subagents) {
    if (expandedAgents.value.has(agent.id)) {
      map.set(agent.id, buildTrace(agent))
    }
  }
  return map
})

function buildTrace(agent: SubagentInfo): TraceItem[] {
  const items: TraceItem[] = []

  for (const msg of agent.messages) {
    if (msg.message_type === 'assistant') {
      // Text content
      if (msg.text_content && msg.text_content.trim()) {
        items.push({
          type: 'text',
          text: truncate(msg.text_content.trim(), 300),
        })
      }
      // Tool calls
      for (const tc of msg.tool_calls) {
        items.push({
          type: 'tool_call',
          text: truncate(getToolSummary(tc), 150),
          icon: getToolIcon(tc.name),
          toolName: tc.name,
        })
      }
    } else if (msg.message_type === 'user') {
      // Tool results only (skip meta/system prompts)
      for (const tr of msg.tool_results) {
        const content = typeof tr.content === 'string'
          ? tr.content
          : JSON.stringify(tr.content)
        items.push({
          type: 'tool_result',
          text: truncate(content, 150),
          isError: tr.is_error,
        })
      }
    }
  }

  return items
}

function truncate(text: string | null, maxLen: number): string {
  if (!text) return ''
  return text.length > maxLen ? text.slice(0, maxLen) + '...' : text
}
</script>

<style scoped>
.subagent-tree {
  padding: 4px 12px 8px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.subagent-card {
  border: 1px solid #E8E5E0;
  border-radius: 6px;
  background: #FAF9F7;
  overflow: hidden;
}

.subagent-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  cursor: pointer;
  flex-wrap: wrap;
}

.subagent-header:hover {
  background: #F0EDE8;
}

/* Type badge */
.type-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 10px;
  font-weight: 600;
  padding: 2px 7px;
  border-radius: 4px;
  white-space: nowrap;
}

.type-badge i {
  font-size: 9px;
}

/* Status indicators */
.status-spinner {
  color: #D97706;
  font-size: 11px;
}

.status-done {
  color: #16A34A;
  font-size: 11px;
}

/* Parallel badge */
.parallel-badge {
  color: #7C3AED;
  font-size: 10px;
  display: inline-flex;
  align-items: center;
  gap: 2px;
  background: rgba(124, 58, 237, 0.08);
  padding: 1px 5px;
  border-radius: 3px;
}

.subagent-desc {
  font-size: 11px;
  color: #6B6660;
  max-width: 360px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.subagent-metrics {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: auto;
}

.metric-pill {
  font-size: 10px;
  color: #8B8680;
  display: flex;
  align-items: center;
  gap: 3px;
}

.metric-pill i {
  font-size: 9px;
}

.expand-chevron {
  color: #B0ACA7;
  font-size: 10px;
  transition: transform 0.2s ease;
}

.fa-chevron-up.expand-chevron {
  transform: rotate(0deg);
}

.fa-chevron-down.expand-chevron {
  transform: rotate(0deg);
}

/* Execution trace timeline */
.subagent-trace {
  border-top: 1px solid #E8E5E0;
  max-height: 500px;
  overflow-y: auto;
  padding: 8px 12px 8px 16px;
}

.trace-timeline {
  position: relative;
  padding-left: 16px;
  border-left: 2px solid #E8E5E0;
  margin-left: 4px;
}

.trace-item {
  position: relative;
  padding: 3px 0;
}

.trace-dot {
  position: absolute;
  left: -21px;
  top: 8px;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #D5D0CB;
  border: 2px solid #FAF9F7;
}

.trace-tool_call .trace-dot {
  background: #C4704B;
}

.trace-tool_result .trace-dot {
  background: #16A34A;
}

.trace-error .trace-dot,
.trace-tool_result:has(.trace-error) .trace-dot {
  background: #DC2626;
}

.trace-content {
  padding-left: 4px;
}

.trace-text {
  font-size: 11px;
  color: #4A4845;
  line-height: 1.4;
  white-space: pre-wrap;
  word-break: break-word;
}

.trace-tool {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
}

.trace-tool i {
  font-size: 10px;
  color: #C4704B;
}

.trace-tool-name {
  font-weight: 600;
  color: #C4704B;
}

.trace-tool-summary {
  color: #6B6660;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 500px;
}

.trace-result {
  display: flex;
  align-items: flex-start;
  gap: 5px;
  font-size: 10px;
  color: #78B89A;
  line-height: 1.3;
  word-break: break-word;
}

.trace-result i {
  font-size: 9px;
  margin-top: 2px;
  flex-shrink: 0;
}

.trace-result.trace-error {
  color: #DC2626;
}
</style>
