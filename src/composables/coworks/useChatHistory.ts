/**
 * Chat History composable for Coworks
 * Manages chat history for Chat mode
 * Persists to ~/.coworks/chats/
 */

import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ChatMessage } from '@/types/coworks'
import type { ChatHistory, ChatHistoryIndexItem } from '@/types/coworks-persistence'

// Rust API types
interface RustChatHistoryIndexEntry {
  id: string
  title: string
  createdAt: number
  updatedAt: number
}

interface RustChatHistory {
  id: string
  title: string
  messages: ChatMessage[]
  createdAt: number
  updatedAt: number
  sdkSessionId?: string
  model?: string
}

// Generate unique ID
function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
}

// Generate title from first message
function generateTitle(messages: ChatMessage[]): string {
  const firstUserMessage = messages.find(m => m.role === 'user')
  if (!firstUserMessage) return '新对话'

  // Truncate to 50 characters
  const content = firstUserMessage.content.trim()
  if (content.length <= 50) return content
  return content.substring(0, 47) + '...'
}

// Singleton state
const chatHistories = ref<ChatHistoryIndexItem[]>([])
const currentChatId = ref<string | null>(null)
const currentChat = ref<ChatHistory | null>(null)
const isLoading = ref(false)

export function useChatHistory() {
  // Sorted chat histories (newest first)
  const sortedHistories = computed(() => {
    return [...chatHistories.value].sort((a, b) => b.updatedAt - a.updatedAt)
  })

  // Initialize from storage
  async function initialize() {
    try {
      const items = await invoke<RustChatHistoryIndexEntry[]>('coworks_list_chat_histories')
      chatHistories.value = items.map(item => ({
        id: item.id,
        title: item.title,
        createdAt: item.createdAt,
        updatedAt: item.updatedAt,
        messageCount: 0, // Will be updated when loading full chat
      }))
    } catch (e) {
      console.error('[useChatHistory] Failed to load index:', e)
    }
  }

  // Create a new chat
  async function createChat(): Promise<string> {
    const id = generateId()
    const now = Date.now()

    const chat: RustChatHistory = {
      id,
      title: '新对话',
      messages: [],
      createdAt: now,
      updatedAt: now,
    }

    // Save chat (this also updates the index)
    await invoke('coworks_save_chat_history', { history: chat })

    // Update local state
    chatHistories.value.push({
      id,
      title: chat.title,
      createdAt: now,
      updatedAt: now,
      messageCount: 0,
    })

    // Set as current
    currentChatId.value = id
    currentChat.value = chat

    return id
  }

  // Load a chat by ID
  async function loadChat(id: string): Promise<ChatHistory | null> {
    try {
      isLoading.value = true
      const chat = await invoke<RustChatHistory>('coworks_load_chat_history', { id })
      currentChatId.value = id
      currentChat.value = chat
      return chat
    } catch (e) {
      console.error('[useChatHistory] Failed to load chat:', e)
      return null
    } finally {
      isLoading.value = false
    }
  }

  // Save current chat
  async function saveCurrentChat() {
    if (!currentChat.value) return

    const chat = currentChat.value
    chat.updatedAt = Date.now()
    chat.title = generateTitle(chat.messages)

    // Save chat (this also updates the index)
    await invoke('coworks_save_chat_history', { history: chat })

    // Update local index
    const indexItem = chatHistories.value.find(h => h.id === chat.id)
    if (indexItem) {
      indexItem.title = chat.title
      indexItem.updatedAt = chat.updatedAt
      indexItem.messageCount = chat.messages.length
    }
  }

  // Save a specific chat by reference (for background session persistence)
  async function saveChatRef(chat: RustChatHistory) {
    chat.updatedAt = Date.now()
    chat.title = generateTitle(chat.messages)
    await invoke('coworks_save_chat_history', { history: chat })
    const indexItem = chatHistories.value.find(h => h.id === chat.id)
    if (indexItem) {
      indexItem.title = chat.title
      indexItem.updatedAt = chat.updatedAt
      indexItem.messageCount = chat.messages.length
    }
  }

  // Add message to current chat
  function addMessage(message: ChatMessage) {
    if (!currentChat.value) return
    currentChat.value.messages.push(message)
  }

  // Update last message (for streaming)
  function updateLastMessage(update: Partial<ChatMessage>) {
    if (!currentChat.value || currentChat.value.messages.length === 0) return
    const lastIndex = currentChat.value.messages.length - 1
    currentChat.value.messages[lastIndex] = {
      ...currentChat.value.messages[lastIndex],
      ...update,
    }
  }

  // Get messages of current chat
  const messages = computed(() => currentChat.value?.messages || [])

  // Delete a chat
  async function deleteChat(id: string) {
    // Delete from backend (this also updates the index)
    await invoke('coworks_delete_chat_history', { id })

    // Update local index
    const index = chatHistories.value.findIndex(h => h.id === id)
    if (index !== -1) {
      chatHistories.value.splice(index, 1)
    }

    // Clear current if it was deleted
    if (currentChatId.value === id) {
      currentChatId.value = null
      currentChat.value = null
    }
  }

  // Clear current chat (without deleting from history)
  function clearCurrentChat() {
    currentChatId.value = null
    currentChat.value = null
  }

  // Set SDK session ID for current chat (for conversation continuity)
  function setSdkSessionId(sdkSessionId: string) {
    if (currentChat.value) {
      currentChat.value.sdkSessionId = sdkSessionId
    }
  }

  return {
    chatHistories: sortedHistories,
    currentChatId,
    currentChat,
    messages,
    isLoading,
    initialize,
    createChat,
    loadChat,
    saveCurrentChat,
    saveChatRef,
    addMessage,
    updateLastMessage,
    deleteChat,
    clearCurrentChat,
    setSdkSessionId,
  }
}
