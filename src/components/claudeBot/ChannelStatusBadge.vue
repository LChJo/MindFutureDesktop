<script setup lang="ts">
import { computed } from 'vue'
import type { ChannelStatus } from '@/types/claude-bot'

const props = defineProps<{
  statuses: ChannelStatus[]
}>()

const platformIcons: Record<string, string> = {
  telegram: 'fa-brands fa-telegram',
  discord: 'fa-brands fa-discord',
  feishu: 'fa-solid fa-feather',
}

const connectedPlatforms = computed(() => {
  return props.statuses.filter(s => s.connected)
})
</script>

<template>
  <div
    v-if="connectedPlatforms.length > 0"
    class="channel-badges"
  >
    <span
      v-for="status in connectedPlatforms"
      :key="status.platform"
      class="channel-badge"
      :title="`${status.platform} 已连接`"
    >
      <i :class="platformIcons[status.platform] || 'fa-solid fa-tower-broadcast'" />
    </span>
  </div>
</template>

<style scoped lang="less">
.channel-badges {
  display: flex;
  align-items: center;
  gap: 4px;
}

.channel-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border-radius: 3px;
  background: rgba(63, 185, 80, 0.15);
  color: #3fb950;
  font-size: 9px;
}
</style>
