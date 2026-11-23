<script setup lang="ts">
import { computed } from 'vue'
import type { ToolUsage } from '@/types/coworks'

const props = defineProps<{
  usage: ToolUsage
}>()

const toolIcon = computed(() => {
  const icons: Record<string, string> = {
    Read: '📖',
    Write: '✏️',
    Edit: '📝',
    Bash: '💻',
    Glob: '🔍',
    Grep: '🔎',
    LS: '📂',
  }
  return icons[props.usage.toolName] || '🔧'
})

const statusIcon = computed(() => {
  switch (props.usage.status) {
    case 'pending':
      return '⏳'
    case 'running':
      return '🔄'
    case 'completed':
      return '✅'
    case 'error':
      return '❌'
    default:
      return ''
  }
})

const inputSummary = computed(() => {
  const input = props.usage.input
  if (!input) return ''

  // Common input fields to show
  if (input.path) return String(input.path)
  if (input.file_path) return String(input.file_path)
  if (input.filePath) return String(input.filePath)
  if (input.pattern) return String(input.pattern)
  if (input.command) return String(input.command).slice(0, 50)

  return ''
})
</script>

<template>
  <div
    class="tool-usage"
    :class="usage.status"
  >
    <div class="tool-header">
      <span class="tool-icon">{{ toolIcon }}</span>
      <span class="tool-name">{{ usage.toolName }}</span>
      <span class="status-icon">{{ statusIcon }}</span>
    </div>
    <div
      v-if="inputSummary"
      class="tool-input"
    >
      {{ inputSummary }}
    </div>
    <div
      v-if="usage.summary"
      class="tool-result"
    >
      {{ usage.summary }}
    </div>
  </div>
</template>

<style scoped>
.tool-usage {
  background: rgba(0, 0, 0, 0.03);
  border-radius: 6px;
  padding: 8px 12px;
  margin: 8px 0;
  font-size: 12px;
  border-left: 3px solid #B0ACA7;
}

.tool-usage.running {
  border-left-color: #D4A843;
}

.tool-usage.completed {
  border-left-color: #10b981;
}

.tool-usage.error {
  border-left-color: #D94F4F;
}

.tool-header {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 500;
}

.tool-icon {
  font-size: 14px;
}

.tool-name {
  color: #C4704B;
}

.status-icon {
  margin-left: auto;
  font-size: 12px;
}

.tool-input {
  margin-top: 4px;
  color: #2D2B2A;
  font-family: monospace;
  font-size: 11px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tool-result {
  margin-top: 4px;
  color: #8B8680;
  font-size: 11px;
  max-height: 60px;
  overflow: hidden;
}
</style>
