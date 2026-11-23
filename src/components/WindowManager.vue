<script setup lang="ts">
import { useWindowStore } from '@/stores/windowStore'
import Window from './Window.vue'

const windowStore = useWindowStore()

// Check if a window is minimized
const isWindowMinimized = (windowId: string) => {
  const win = windowStore.windows.find(w => w.id === windowId)
  return win && win.state === 'minimized'
}
</script>

<template>
  <div class="window-manager">
    <!-- Render ALL windows, use visibility:hidden for minimized (preserves layout and xterm dimensions) -->
    <div
      v-for="win in windowStore.windows"
      :key="win.id"
      class="window-wrapper"
      :class="{ 'window-minimized': isWindowMinimized(win.id) }"
    >
      <Window :window="win" />
    </div>
  </div>
</template>

<style lang="less" scoped>
.window-manager {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: calc(100% - 48px); // Leave space for taskbar
  pointer-events: none;
  z-index: 100;

  :deep(.window) {
    pointer-events: auto;
  }
}

// Wrapper for windows - allows us to use visibility:hidden instead of display:none
.window-wrapper {
  // Wrapper doesn't affect layout - window inside is absolutely positioned
  position: absolute;
  top: 0;
  left: 0;
  width: 0;
  height: 0;
  overflow: visible;

  // When minimized, use visibility:hidden to preserve element dimensions
  // This is crucial for xterm.js which needs proper dimensions to render
  &.window-minimized {
    visibility: hidden;
    pointer-events: none;

    :deep(.window) {
      pointer-events: none;
    }
  }
}
</style>
