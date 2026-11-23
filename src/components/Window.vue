<script setup lang="ts">
import { computed, ref, onUnmounted } from 'vue'
import { useWindowStore } from '@/stores/windowStore'
import type { WindowInstance } from '@/types/models'

const props = defineProps<{
  window: WindowInstance
}>()

const windowStore = useWindowStore()

const isDragging = ref(false)
const isResizing = ref(false)
const dragOffset = ref({ x: 0, y: 0 })
const resizeStartPos = ref({ x: 0, y: 0 })
const resizeStartSize = ref({ width: 0, height: 0 })

const windowStyle = computed(() => {
  if (props.window.state === 'maximized') {
    return {
      top: '0px',
      left: '0px',
      width: '100vw',
      height: 'calc(100vh - 48px)', // Leave space for taskbar
      zIndex: props.window.zIndex,
      borderRadius: '0'
    }
  }
  return {
    top: props.window.position.y + 'px',
    left: props.window.position.x + 'px',
    width: props.window.position.width + 'px',
    height: props.window.position.height + 'px',
    zIndex: props.window.zIndex
  }
})

const isActive = computed(() => {
  return windowStore.activeWindowId === props.window.id
})

const startDrag = (event: MouseEvent) => {
  if (props.window.state === 'maximized') return

  isDragging.value = true
  dragOffset.value = {
    x: event.clientX - props.window.position.x,
    y: event.clientY - props.window.position.y
  }

  document.addEventListener('mousemove', onDrag)
  document.addEventListener('mouseup', stopDrag)
}

const onDrag = (event: MouseEvent) => {
  if (!isDragging.value) return

  const newX = Math.max(0, event.clientX - dragOffset.value.x)
  const newY = Math.max(0, event.clientY - dragOffset.value.y)

  windowStore.updateWindowPosition(props.window.id, {
    x: newX,
    y: newY
  })
}

const stopDrag = () => {
  isDragging.value = false
  document.removeEventListener('mousemove', onDrag)
  document.removeEventListener('mouseup', stopDrag)
}

const startResize = (event: MouseEvent) => {
  if (props.window.state === 'maximized') return

  event.preventDefault()
  event.stopPropagation()

  isResizing.value = true
  resizeStartPos.value = { x: event.clientX, y: event.clientY }
  resizeStartSize.value = {
    width: props.window.position.width,
    height: props.window.position.height
  }

  document.addEventListener('mousemove', onResize)
  document.addEventListener('mouseup', stopResize)
}

const onResize = (event: MouseEvent) => {
  if (!isResizing.value) return

  const deltaX = event.clientX - resizeStartPos.value.x
  const deltaY = event.clientY - resizeStartPos.value.y

  // Minimum size: dynamic minWidth (or 600) x 400 to ensure terminal displays correctly
  const minWidth = (props.window.data?.minWidth as number) || 600
  const newWidth = Math.max(minWidth, resizeStartSize.value.width + deltaX)
  const newHeight = Math.max(400, resizeStartSize.value.height + deltaY)

  windowStore.updateWindowPosition(props.window.id, {
    width: newWidth,
    height: newHeight
  })
}

const stopResize = () => {
  isResizing.value = false
  document.removeEventListener('mousemove', onResize)
  document.removeEventListener('mouseup', stopResize)
}

const bringToFront = () => {
  windowStore.focusWindow(props.window.id)
}

const minimize = () => {
  windowStore.minimizeWindow(props.window.id)
}

const maximize = () => {
  windowStore.maximizeWindow(props.window.id)
}

const close = () => {
  windowStore.closeWindow(props.window.id)
}

onUnmounted(() => {
  document.removeEventListener('mousemove', onDrag)
  document.removeEventListener('mouseup', stopDrag)
  document.removeEventListener('mousemove', onResize)
  document.removeEventListener('mouseup', stopResize)
})
</script>

<template>
  <div
    class="window"
    :class="{
      maximized: window.state === 'maximized',
      active: isActive,
      dragging: isDragging,
      resizing: isResizing
    }"
    :style="windowStyle"
    @mousedown="bringToFront"
  >
    <div
      class="window-header"
      @mousedown="startDrag"
    >
      <span class="window-title">{{ window.title }}</span>
      <!-- Terminal task status indicator -->
      <span
        v-if="window.data?.terminalStatus === 'completed'"
        class="terminal-status completed"
        title="任务已完成"
      >&#10003;</span>
      <span
        v-else-if="window.data?.terminalWorking"
        class="terminal-status working"
        title="正在处理..."
      ><i class="fa-solid fa-circle-notch fa-spin" /></span>
      <div class="window-controls">
        <button
          class="control-btn minimize"
          title="最小化"
          @click.stop="minimize"
        >
          <i class="fa-solid fa-minus" />
        </button>
        <button
          class="control-btn maximize"
          title="最大化"
          @click.stop="maximize"
        >
          <i
            class="fa-solid"
            :class="window.state === 'maximized' ? 'fa-compress' : 'fa-expand'"
          />
        </button>
        <button
          class="control-btn close"
          title="关闭"
          @click.stop="close"
        >
          <i class="fa-solid fa-xmark" />
        </button>
      </div>
    </div>
    <div class="window-content">
      <component
        :is="window.component"
        v-if="typeof window.component !== 'string'"
        :window-id="window.id"
        :window-data="window.data"
      />
      <div
        v-else
        class="component-placeholder"
      >
        {{ window.component }}
      </div>
    </div>
    <div
      v-if="window.state !== 'maximized'"
      class="resize-handle"
      @mousedown="startResize"
    />
  </div>
</template>

<style lang="less" scoped>
.window {
  position: absolute;
  background: rgba(30, 30, 30, 0.95);
  border-radius: 12px;
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.4),
    0 0 0 1px rgba(255, 255, 255, 0.1);
  display: flex;
  flex-direction: column;
  min-width: 300px;
  min-height: 200px;
  overflow: hidden;
  transition: box-shadow 0.3s, transform 0.2s;

  &.maximized {
    border-radius: 0;
  }

  &.active {
    box-shadow:
      0 16px 64px rgba(0, 0, 0, 0.5),
      0 0 0 1px rgba(124, 110, 184, 0.4),
      0 0 40px rgba(124, 110, 184, 0.15);
  }

  &.dragging,
  &.resizing {
    opacity: 0.95;
    user-select: none;
    transform: scale(1.01);
  }
}

.window-header {
  background: linear-gradient(180deg, rgba(45, 50, 65, 0.98) 0%, rgba(35, 40, 55, 0.98) 100%);
  backdrop-filter: blur(20px);
  padding: 0 12px;
  height: 40px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: move;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  flex-shrink: 0;
  overflow: hidden;

  .maximized & {
    cursor: default;
  }
}

.window-title {
  font-size: 13px;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.9);
  overflow: hidden;
  white-space: nowrap;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
  flex: 1;
  min-width: 0;
  mask-image: linear-gradient(to right, black calc(100% - 20px), transparent 100%);
  -webkit-mask-image: linear-gradient(to right, black calc(100% - 20px), transparent 100%);
}

.terminal-status {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
  margin-left: 8px;
  flex-shrink: 0;
  font-weight: 600;

  &.completed {
    color: #10b981;
    background: rgba(16, 185, 129, 0.12);
  }

  &.working {
    color: #D4A843;
    background: rgba(212, 168, 67, 0.12);
    font-size: 10px;
  }
}

.window-controls {
  display: flex;
  gap: 8px;
  margin-left: 12px;
  flex-shrink: 0;

  .control-btn {
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 50%;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;

    i {
      font-size: 10px;
      color: white;
    }

    &.minimize {
      background: #febc2e;

      &:hover {
        background: #e5a829;
      }
    }

    &.maximize {
      background: #28c840;

      &:hover {
        background: #24b439;
      }
    }

    &.close {
      background: #ff5f57;

      &:hover {
        background: #e54942;
      }
    }
  }
}

.window-content {
  flex: 1;
  overflow: hidden; // Changed from auto to hidden to prevent scroll-related issues
  background: #1e1e1e;
  position: relative;

  .component-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: rgba(255, 255, 255, 0.5);
    font-size: 14px;
  }
}

.resize-handle {
  position: absolute;
  bottom: 0;
  right: 0;
  width: 20px;
  height: 20px;
  cursor: nwse-resize;

  &::after {
    content: '';
    position: absolute;
    bottom: 6px;
    right: 6px;
    width: 10px;
    height: 10px;
    border-right: 2px solid rgba(255, 255, 255, 0.2);
    border-bottom: 2px solid rgba(255, 255, 255, 0.2);
    transition: border-color 0.2s;
  }

  &:hover::after {
    border-color: rgba(124, 110, 184, 0.6);
  }
}
</style>
