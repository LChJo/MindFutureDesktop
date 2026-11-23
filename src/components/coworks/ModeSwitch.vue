<script setup lang="ts">
import type { AppMode } from '@/types/coworks'

defineProps<{
  mode: AppMode
}>()

const emit = defineEmits<{
  (e: 'update:mode', mode: AppMode): void
}>()

const modes: { key: AppMode; icon: string; label: string }[] = [
  { key: 'chat', icon: '💬', label: 'Chat' },
  { key: 'code', icon: '💻', label: 'Code' },
  { key: 'cowork', icon: '📁', label: 'Cowork' },
]
</script>

<template>
  <div class="mode-switch">
    <button
      v-for="m in modes"
      :key="m.key"
      class="mode-btn"
      :class="{ active: mode === m.key }"
      @click="emit('update:mode', m.key)"
    >
      <span class="mode-icon">{{ m.icon }}</span>
      <span class="mode-label">{{ m.label }}</span>
    </button>
  </div>
</template>

<style scoped>
.mode-switch {
  display: flex;
  background: rgba(0, 0, 0, 0.05);
  border-radius: 8px;
  padding: 2px;
}

.mode-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  background: transparent;
  border: none;
  border-radius: 6px;
  color: #8B8680;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s;
}

.mode-btn:hover {
  color: #2D2B2A;
}

.mode-btn.active {
  background: rgba(196, 112, 75, 0.10);
  color: #C4704B;
}

.mode-icon {
  font-size: 14px;
}

.mode-label {
  font-weight: 500;
}
</style>
