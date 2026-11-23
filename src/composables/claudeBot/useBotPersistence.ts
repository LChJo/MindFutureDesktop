/**
 * Bot Persistence composable
 * Handles saving/loading bot conversations via Tauri commands
 */

import { invoke } from '@tauri-apps/api/core'
import type { BotChatMessage } from '@/types/claude-bot'

export function useBotPersistence() {
  async function loadConversation(botId: string): Promise<BotChatMessage[]> {
    try {
      const raw: string = await invoke('bot_load_conversation', { botId })
      return JSON.parse(raw)
    } catch (e) {
      console.error(`[useBotPersistence] Failed to load conversation for ${botId}:`, e)
      return []
    }
  }

  async function saveConversation(botId: string, messages: BotChatMessage[]): Promise<void> {
    try {
      await invoke('bot_save_conversation', {
        botId,
        conversation: JSON.stringify(messages),
      })
    } catch (e) {
      console.error(`[useBotPersistence] Failed to save conversation for ${botId}:`, e)
    }
  }

  async function readOutputFile(botId: string, filePath: string): Promise<string> {
    return invoke('bot_read_output_file', { botId, filePath })
  }

  async function writeOutputFile(botId: string, filePath: string, content: string): Promise<void> {
    return invoke('bot_write_output_file', { botId, filePath, content })
  }

  async function ensureOutputDirectory(botId: string): Promise<string> {
    return invoke('bot_ensure_output_directory', { botId })
  }

  return {
    loadConversation,
    saveConversation,
    readOutputFile,
    writeOutputFile,
    ensureOutputDirectory,
  }
}
