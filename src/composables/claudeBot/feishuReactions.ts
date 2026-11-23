import { invoke } from '@tauri-apps/api/core'

/** Emoji used by Rust listener to acknowledge receipt of a message. */
export const FEISHU_ACK_REACTION = 'Status_PrivateMessage'

/** Emoji shown while the bot is processing a message. */
export const FEISHU_PROCESSING_REACTION = 'StatusFlashOfInspiration'

/**
 * Clear the acknowledgment reaction (`Status_PrivateMessage`) added by the
 * Rust Feishu listener.  Returns the number of reactions actually deleted.
 *
 * This is a targeted clear (emoji_type filter) so it won't accidentally
 * remove the processing reaction or any user-added reactions.
 */
export function clearFeishuAckReaction(botId: string, messageId: string): Promise<number> {
  return invoke<number>('feishu_clear_app_reactions', {
    botId,
    messageId,
    emojiType: FEISHU_ACK_REACTION,
  })
}
