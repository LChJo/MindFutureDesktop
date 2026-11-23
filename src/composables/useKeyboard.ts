import { onMounted, onUnmounted } from 'vue'
import { useWindowStore } from '@/stores/windowStore'
import { useDesktopStore } from '@/stores/desktopStore'

export function useKeyboard() {
  const windowStore = useWindowStore()
  const desktopStore = useDesktopStore()

  const openTerminal = () => {
    const terminalIcon = desktopStore.icons.find(i => i.type === 'terminal')
    if (terminalIcon) {
      windowStore.openWindow(terminalIcon)
    }
  }

  const openFileManager = () => {
    const fileManagerIcon = desktopStore.icons.find(i => i.type === 'file-manager')
    if (fileManagerIcon) {
      windowStore.openWindow(fileManagerIcon)
    }
  }

  const closeActiveWindow = () => {
    if (windowStore.activeWindowId) {
      windowStore.closeWindow(windowStore.activeWindowId)
    }
  }

  const switchWindow = () => {
    windowStore.switchToNextWindow()
  }

  const keyMap: Record<string, () => void> = {
    'ctrl+alt+t': openTerminal,
    'ctrl+alt+f': openFileManager,
    'ctrl+w': closeActiveWindow,
    'alt+tab': switchWindow
  }

  const parseKeyCombo = (event: KeyboardEvent): string => {
    const parts: string[] = []

    if (event.ctrlKey) parts.push('ctrl')
    if (event.altKey) parts.push('alt')
    if (event.shiftKey) parts.push('shift')
    if (event.metaKey) parts.push('meta')

    // Handle special keys
    const key = event.key.toLowerCase()
    if (!['control', 'alt', 'shift', 'meta'].includes(key)) {
      parts.push(key === ' ' ? 'space' : key)
    }

    return parts.join('+')
  }

  const handleKeyDown = (event: KeyboardEvent) => {
    const keyCombo = parseKeyCombo(event)

    // Check if this key combo is mapped
    if (keyMap[keyCombo]) {
      event.preventDefault()
      event.stopPropagation()
      keyMap[keyCombo]()
    }
  }

  onMounted(() => {
    window.addEventListener('keydown', handleKeyDown)
  })

  onUnmounted(() => {
    window.removeEventListener('keydown', handleKeyDown)
  })

  return {
    openTerminal,
    openFileManager,
    closeActiveWindow,
    switchWindow
  }
}
