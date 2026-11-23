import { defineStore } from 'pinia'
import type { TerminalInstance } from '@/types/models'
import { useSettingsStore } from './settingsStore'

export interface TerminalStoreState {
  terminals: TerminalInstance[]
}

export const useTerminalStore = defineStore('terminal', {
  state: (): TerminalStoreState => ({
    terminals: []
  }),

  getters: {
    getTerminalByWindowId: (state) => (windowId: string): TerminalInstance | undefined => {
      return state.terminals.find(t => t.windowId === windowId)
    },

    getTerminalById: (state) => (id: string): TerminalInstance | undefined => {
      return state.terminals.find(t => t.id === id)
    },

    getTerminalByPtyId: (state) => (ptyId: string): TerminalInstance | undefined => {
      return state.terminals.find(t => t.ptyId === ptyId)
    }
  },

  actions: {
    createTerminal(windowId: string, ptyId: string): string {
      const settingsStore = useSettingsStore()
      const terminalId = `term-${Date.now()}`

      const terminal: TerminalInstance = {
        id: terminalId,
        windowId,
        ptyId,
        workingDirectory: settingsStore.defaultWorkingDirectory || '~',
        defaultCli: settingsStore.defaultCli || 'powershell',
        history: []
      }

      this.terminals.push(terminal)
      this.save()
      return terminalId
    },

    destroyTerminal(windowId: string) {
      const index = this.terminals.findIndex(t => t.windowId === windowId)
      if (index !== -1) {
        this.terminals.splice(index, 1)
        this.save()
      }
    },

    addToHistory(terminalId: string, command: string) {
      const terminal = this.terminals.find(t => t.id === terminalId)
      if (terminal) {
        terminal.history.push(command)
        // Keep only last 100 commands
        if (terminal.history.length > 100) {
          terminal.history.shift()
        }
        this.save()
      }
    },

    setWorkingDirectory(terminalId: string, directory: string) {
      const terminal = this.terminals.find(t => t.id === terminalId)
      if (terminal) {
        terminal.workingDirectory = directory
        this.save()
      }
    },

    setCli(terminalId: string, cli: string) {
      const terminal = this.terminals.find(t => t.id === terminalId)
      if (terminal) {
        terminal.defaultCli = cli
        this.save()
      }
    },

    save() {
      localStorage.setItem('mcd-terminals', JSON.stringify(this.terminals))
    },

    load() {
      const saved = localStorage.getItem('mcd-terminals')
      if (saved) {
        try {
          this.terminals = JSON.parse(saved)
        } catch (e) {
          console.error('Failed to load terminals:', e)
        }
      }
    }
  }
})
