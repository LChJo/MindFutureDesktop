/**
 * Claude Apps conversation history composable
 * Manages conversation persistence for Claude Apps mode
 * Persists to ~/.MoontonClaudeDesktop/coworks/claude-apps-chats/
 */

import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ChatMessage } from '@/types/coworks'

// Rust API types
interface RustConversationIndex {
  id: string
  title: string
  appName?: string
  createdAt: number
  updatedAt: number
}

interface RustConversation {
  id: string
  title: string
  appName?: string
  messages: ChatMessage[]
  createdAt: number
  updatedAt: number
  sdkSessionId?: string
}

export interface ConversationIndexItem {
  id: string
  title: string
  appName: string | null
  createdAt: number
  updatedAt: number
}

export interface Conversation {
  id: string
  title: string
  appName: string | null
  messages: ChatMessage[]
  createdAt: number
  updatedAt: number
  sdkSessionId?: string
}

function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).substring(2, 11)}`
}

function generateTitle(messages: ChatMessage[]): string {
  const firstUserMessage = messages.find(m => m.role === 'user')
  if (!firstUserMessage) return '新对话'
  const content = firstUserMessage.content.trim()
  if (content.length <= 50) return content
  return content.substring(0, 47) + '...'
}

// Singleton state
const conversations = ref<ConversationIndexItem[]>([])
const currentConversationId = ref<string | null>(null)
const currentConversation = ref<Conversation | null>(null)
const isLoading = ref(false)

export function useClaudeAppsHistory() {
  const sortedConversations = computed(() => {
    return [...conversations.value].sort((a, b) => b.updatedAt - a.updatedAt)
  })

  async function initialize() {
    try {
      const items = await invoke<RustConversationIndex[]>('claude_apps_list_conversations')
      conversations.value = items.map(item => ({
        id: item.id,
        title: item.title,
        appName: item.appName ?? null,
        createdAt: item.createdAt,
        updatedAt: item.updatedAt,
      }))
    } catch (e) {
      console.error('[useClaudeAppsHistory] Failed to load index:', e)
    }
  }

  async function createConversation(appName: string | null): Promise<string> {
    const id = generateId()
    const now = Date.now()

    const conversation: RustConversation = {
      id,
      title: '新对话',
      appName: appName ?? undefined,
      messages: [],
      createdAt: now,
      updatedAt: now,
    }

    await invoke('claude_apps_save_conversation', { conversation })

    conversations.value.push({
      id,
      title: conversation.title,
      appName,
      createdAt: now,
      updatedAt: now,
    })

    currentConversationId.value = id
    currentConversation.value = {
      ...conversation,
      appName,
    }

    return id
  }

  async function loadConversation(id: string): Promise<Conversation | null> {
    try {
      isLoading.value = true
      const conv = await invoke<RustConversation>('claude_apps_load_conversation', { id })
      const result: Conversation = {
        ...conv,
        appName: conv.appName ?? null,
      }
      currentConversationId.value = id
      currentConversation.value = result
      return result
    } catch (e) {
      console.error('[useClaudeAppsHistory] Failed to load conversation:', e)
      return null
    } finally {
      isLoading.value = false
    }
  }

  async function saveCurrentConversation() {
    if (!currentConversation.value) return

    const conv = currentConversation.value
    const updatedAt = Date.now()
    const title = generateTitle(conv.messages)

    const rustConv: RustConversation = {
      id: conv.id,
      title,
      appName: conv.appName ?? undefined,
      messages: conv.messages,
      createdAt: conv.createdAt,
      updatedAt,
      sdkSessionId: conv.sdkSessionId,
    }

    await invoke('claude_apps_save_conversation', { conversation: rustConv })

    // Update local state
    currentConversation.value = { ...conv, title, updatedAt }
    const indexItem = conversations.value.find(c => c.id === conv.id)
    if (indexItem) {
      indexItem.title = title
      indexItem.updatedAt = updatedAt
      indexItem.appName = conv.appName
    }
  }

  async function deleteConversation(id: string) {
    await invoke('claude_apps_delete_conversation', { id })

    const index = conversations.value.findIndex(c => c.id === id)
    if (index !== -1) {
      conversations.value.splice(index, 1)
    }

    if (currentConversationId.value === id) {
      currentConversationId.value = null
      currentConversation.value = null
    }
  }

  function linkToApp(appName: string) {
    if (currentConversation.value) {
      currentConversation.value = {
        ...currentConversation.value,
        appName,
        sdkSessionId: undefined, // workDir 随 appName 变更，旧 session 无法 resume
      }
      const indexItem = conversations.value.find(c => c.id === currentConversation.value?.id)
      if (indexItem) {
        indexItem.appName = appName
      }
    }
  }

  function setSdkSessionId(sdkSessionId: string) {
    if (currentConversation.value) {
      currentConversation.value = {
        ...currentConversation.value,
        sdkSessionId,
      }
    }
  }

  function clearCurrentConversation() {
    currentConversationId.value = null
    currentConversation.value = null
  }

  // Get conversations for a specific app
  function getConversationsForApp(appName: string): ConversationIndexItem[] {
    return conversations.value.filter(c => c.appName === appName)
  }

  // Get unlinked conversations (no app associated)
  function getUnlinkedConversations(): ConversationIndexItem[] {
    return conversations.value.filter(c => !c.appName)
  }

  return {
    conversations: sortedConversations,
    currentConversationId,
    currentConversation,
    isLoading,
    initialize,
    createConversation,
    loadConversation,
    saveCurrentConversation,
    deleteConversation,
    linkToApp,
    setSdkSessionId,
    clearCurrentConversation,
    getConversationsForApp,
    getUnlinkedConversations,
  }
}
