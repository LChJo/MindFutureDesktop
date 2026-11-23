<template>
  <div
    class="tool-call"
    :class="{ error: toolResult?.is_error, expanded }"
  >
    <div
      class="tool-header"
      @click="expanded = !expanded"
    >
      <i :class="getToolIcon(toolCall.name)" />
      <span class="tool-name">{{ toolCall.name }}</span>
      <span class="tool-summary">{{ getToolSummary(toolCall) }}</span>
      <span
        v-if="toolCall.progress"
        class="progress-info"
      >
        {{ toolCall.progress.output_lines }} lines · {{ formatDuration(toolCall.progress.elapsed_ms) }}
      </span>
      <span
        v-if="toolResult?.is_error"
        class="error-badge"
      >
        <i class="fa-solid fa-circle-xmark" /> Error
      </span>
      <i
        class="fa-solid expand-icon"
        :class="expanded ? 'fa-chevron-up' : 'fa-chevron-down'"
      />
    </div>

    <div
      v-if="expanded"
      class="tool-detail"
    >
      <!-- Input -->
      <div class="detail-section">
        <div class="detail-label">
          Input
        </div>
        <pre class="detail-content">{{ formatJson(toolCall.input) }}</pre>
      </div>

      <!-- Result -->
      <div
        v-if="toolResult"
        class="detail-section"
      >
        <div
          class="detail-label"
          :class="{ 'error-label': toolResult.is_error }"
        >
          {{ toolResult.is_error ? 'Error' : 'Result' }}
        </div>
        <pre
          class="detail-content"
          :class="{ 'error-content': toolResult.is_error }"
        >{{ formatResult(toolResult.content) }}</pre>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import type { ToolCallInfo, ToolResultInfo } from '@/types/devtools'
import { getToolIcon, getToolSummary, formatDuration } from '@/types/devtools'

defineProps<{
  toolCall: ToolCallInfo
  toolResult?: ToolResultInfo
}>()

const expanded = ref(false)

function formatJson(value: unknown): string {
  if (typeof value === 'string') return value
  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

function formatResult(content: unknown): string {
  if (typeof content === 'string') {
    // Truncate very long results
    return content.length > 3000 ? content.slice(0, 3000) + '\n... (truncated)' : content
  }
  if (Array.isArray(content)) {
    // Handle content block arrays
    return content
      .map((block) => {
        if (typeof block === 'string') return block
        if (block.type === 'text') return block.text
        return JSON.stringify(block, null, 2)
      })
      .join('\n')
  }
  return formatJson(content)
}
</script>

<style scoped>
.tool-call {
  border: 1px solid #E8E5E0;
  border-radius: 6px;
  overflow: hidden;
  background: #FAF9F7;
}

.tool-call.error {
  border-color: #FECACA;
}

.tool-call.expanded {
  border-color: #D5D0C8;
}

.tool-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px;
  cursor: pointer;
  font-size: 12px;
  user-select: none;
}

.tool-header:hover {
  background: #F0EDE8;
}

.tool-header > i:first-child {
  color: #8B8680;
  font-size: 11px;
  width: 16px;
  text-align: center;
}

.tool-name {
  font-weight: 600;
  color: #C4704B;
  flex-shrink: 0;
}

.tool-summary {
  flex: 1;
  color: #6B6660;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 11px;
}

.progress-info {
  color: #9CA3AF;
  font-size: 10px;
  flex-shrink: 0;
  font-family: 'Consolas', 'Monaco', monospace;
}

.error-badge {
  color: #DC2626;
  font-size: 10px;
  display: flex;
  align-items: center;
  gap: 3px;
  flex-shrink: 0;
}

.expand-icon {
  color: #B0ACA7;
  font-size: 10px;
  margin-left: auto;
  flex-shrink: 0;
}

.tool-detail {
  border-top: 1px solid #E8E5E0;
}

.detail-section {
  padding: 6px 10px;
  border-bottom: 1px solid #F0EDE8;
}

.detail-section:last-child {
  border-bottom: none;
}

.detail-label {
  font-size: 10px;
  font-weight: 600;
  color: #8B8680;
  text-transform: uppercase;
  margin-bottom: 4px;
}

.error-label {
  color: #DC2626;
}

.detail-content {
  margin: 0;
  padding: 6px 8px;
  background: #F5F3EF;
  border-radius: 4px;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 11px;
  line-height: 1.4;
  color: #4A4845;
  overflow-x: auto;
  max-height: 300px;
  overflow-y: auto;
  white-space: pre-wrap;
  word-break: break-all;
}

.error-content {
  color: #B91C1C;
  background: #FEF2F2;
}
</style>
