<template>
  <div class="metrics-panel">
    <div
      class="metric"
      :title="`Duration: ${formatDuration(metrics.duration_ms)}`"
    >
      <i class="fa-solid fa-clock" />
      <span>{{ formatDuration(metrics.duration_ms) }}</span>
    </div>

    <div
      class="metric"
      :title="tokenTooltip"
    >
      <i class="fa-solid fa-coins" />
      <span>{{ formatTokensCompact(metrics.total_tokens) }} tokens</span>
    </div>

    <div
      class="metric"
      :title="`Cost: $${metrics.cost_usd.toFixed(4)}`"
    >
      <i class="fa-solid fa-dollar-sign" />
      <span>{{ formatCostUsd(metrics.cost_usd) }}</span>
    </div>

    <div class="metric">
      <i class="fa-solid fa-message" />
      <span>{{ metrics.message_count }} msgs</span>
    </div>

    <div class="metric">
      <i class="fa-solid fa-wrench" />
      <span>{{ metrics.tool_call_count }} tools</span>
    </div>

    <div
      v-if="subagentCount > 0"
      class="metric"
    >
      <i class="fa-solid fa-sitemap" />
      <span>{{ subagentCount }} agents</span>
    </div>

    <div
      v-if="metrics.tool_call_count > 0"
      class="metric"
      :title="successRateTooltip"
    >
      <i class="fa-solid fa-circle-check" />
      <span :class="{ 'success-rate-low': successRate < SUCCESS_RATE_WARNING_THRESHOLD }">{{ successRate }}%</span>
    </div>

    <div
      v-if="session.git_branch"
      class="metric branch-badge"
    >
      <i class="fa-solid fa-code-branch" />
      <span>{{ session.git_branch }}</span>
    </div>

    <div
      v-if="session.is_ongoing"
      class="metric ongoing-badge"
    >
      <span class="ongoing-dot" />
      <span>Live</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { SessionMetrics, SessionSummary } from '@/types/devtools'
import { formatTokensCompact, formatCostUsd, formatDuration } from '@/types/devtools'

const props = withDefaults(defineProps<{
  metrics: SessionMetrics
  session: SessionSummary
  subagentCount?: number
}>(), {
  subagentCount: 0,
})

const tokenTooltip = computed(() => {
  const m = props.metrics
  return [
    `Input: ${m.input_tokens.toLocaleString()}`,
    `Output: ${m.output_tokens.toLocaleString()}`,
    `Cache Read: ${m.cache_read_tokens.toLocaleString()}`,
    `Cache Create: ${m.cache_creation_tokens.toLocaleString()}`,
  ].join('\n')
})

const SUCCESS_RATE_WARNING_THRESHOLD = 90

const successRate = computed(() => {
  const total = props.metrics.tool_call_count
  if (total === 0) return 100
  const errors = props.metrics.tool_error_count
  const rate = Math.round(((total - errors) / total) * 100)
  return Math.max(0, Math.min(100, rate))
})

const successRateTooltip = computed(() => {
  const total = props.metrics.tool_call_count
  const errors = props.metrics.tool_error_count
  return `Success: ${total - errors}/${total} tool calls | Errors: ${errors}`
})
</script>

<style scoped>
.metrics-panel {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 8px 16px;
  background: #FFFFFF;
  border-bottom: 1px solid #E8E5E0;
  flex-wrap: wrap;
  min-height: 36px;
}

.metric {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  color: #4A4845;
  white-space: nowrap;
}

.metric i {
  font-size: 11px;
  color: #C4704B;
}

.branch-badge {
  background: #F0EDE8;
  padding: 2px 8px;
  border-radius: 10px;
  font-size: 11px;
  color: #6B6660;
}

.branch-badge i {
  color: #8B8680;
}

.ongoing-badge {
  color: #16A34A;
}

.ongoing-dot {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #16A34A;
  animation: pulse 2s infinite;
}

.success-rate-low {
  color: #DC2626;
  font-weight: 600;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
</style>
