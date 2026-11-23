import type { Terminal } from '@xterm/xterm'
import type { FitAddon } from '@xterm/addon-fit'
import { invoke } from '@tauri-apps/api/core'

export function useTerminalPty() {
  let terminal: Terminal | null = null
  let fitAddon: FitAddon | null = null
  let ptyId: string | null = null

  // ---- Write buffer (RAF-based batching) ----
  let writeBuffer: Uint8Array[] = []
  let flushScheduled = false
  let rafId: number | null = null

  function onPtyData(chunk: Uint8Array) {
    writeBuffer.push(chunk)
    if (!flushScheduled) {
      flushScheduled = true
      rafId = requestAnimationFrame(flush)
    }
  }

  function flush() {
    flushScheduled = false
    if (!terminal || writeBuffer.length === 0) return

    const totalLength = writeBuffer.reduce((sum, c) => sum + c.length, 0)
    const combined = new Uint8Array(totalLength)
    let offset = 0
    for (const chunk of writeBuffer) {
      combined.set(chunk, offset)
      offset += chunk.length
    }
    writeBuffer = []
    terminal.write(combined)
  }

  // ---- Debounced resize (no coordination flags) ----
  let resizeTimer: ReturnType<typeof setTimeout> | null = null
  let isComposing = false

  function setComposing(value: boolean) {
    isComposing = value
  }

  function scheduleResize() {
    if (isComposing) return
    if (resizeTimer) clearTimeout(resizeTimer)
    resizeTimer = setTimeout(doResize, 150)
  }

  async function doResize() {
    if (!fitAddon || !terminal) return
    const container = terminal.element?.parentElement
    if (!container || container.clientWidth === 0 || container.clientHeight === 0) return

    const dims = fitAddon.proposeDimensions()
    if (!dims) return

    // Skip no-op resize
    if (dims.cols === terminal.cols && dims.rows === terminal.rows) return

    // 1. Let xterm apply the new viewport size (fit() recalculates internally)
    try {
      fitAddon.fit()
    } catch {
      // ignore fit errors
    }

    // 2. Sync PTY using xterm's actual cols/rows, await to ensure
    //    SIGWINCH reaches the TUI before we refresh the display
    await syncPtySize(terminal.cols, terminal.rows)

    // 3. Force re-render all visible rows to clear stale rendering artifacts
    terminal.refresh(0, terminal.rows - 1)
  }

  // ---- PTY size sync ----
  let lastCols = 0
  let lastRows = 0

  async function syncPtySize(cols: number, rows: number) {
    if (!ptyId || (cols === lastCols && rows === lastRows)) return
    lastCols = cols
    lastRows = rows
    try {
      await invoke('resize_terminal', { id: ptyId, cols, rows })
    } catch (e) {
      console.error('Failed to resize PTY:', e)
    }
  }

  // ---- Lifecycle ----
  let killed = false

  function bind(term: Terminal, fit: FitAddon) {
    terminal = term
    fitAddon = fit
  }

  function setPtyId(id: string) {
    ptyId = id
  }

  function getPtyId(): string | null {
    return ptyId
  }

  function write(data: string) {
    if (!ptyId || killed) return
    invoke('write_terminal', { id: ptyId, data })
  }

  async function kill() {
    if (!ptyId || killed) return
    killed = true
    try {
      await invoke('kill_terminal', { id: ptyId })
    } catch (e) {
      console.error('Failed to kill terminal:', e)
    }
    ptyId = null
  }

  function destroy() {
    if (rafId !== null) {
      cancelAnimationFrame(rafId)
      rafId = null
    }
    writeBuffer = []
    flushScheduled = false
    if (resizeTimer) {
      clearTimeout(resizeTimer)
      resizeTimer = null
    }
    terminal = null
    fitAddon = null
  }

  return {
    onPtyData,
    flush,
    scheduleResize,
    doResize,
    syncPtySize,
    setComposing,
    bind,
    setPtyId,
    getPtyId,
    write,
    kill,
    destroy,
  }
}
