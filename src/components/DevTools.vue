<template>
  <div class="devtools">
    <div class="devtools-sidebar">
      <ProjectSidebar />
    </div>
    <div class="devtools-main">
      <!-- Error banner -->
      <div
        v-if="store.error"
        class="devtools-error"
      >
        <i class="fa-solid fa-triangle-exclamation" />
        <span>{{ store.error }}</span>
        <button
          class="error-dismiss"
          @click="store.clearError()"
        >
          <i class="fa-solid fa-xmark" />
        </button>
      </div>

      <!-- Loading -->
      <div
        v-if="store.isLoadingSession"
        class="devtools-loading"
      >
        <i class="fa-solid fa-spinner fa-spin" />
        <span>Loading session...</span>
      </div>

      <!-- Session content -->
      <template v-else-if="store.activeSession">
        <MetricsPanel
          :metrics="store.activeSession.metrics"
          :session="store.activeSession.session"
          :subagent-count="store.activeSession.subagents.length"
        />
        <SessionView
          :messages="store.activeSession.messages"
          :subagents="store.activeSession.subagents"
          :is-ongoing="store.activeSession.session.is_ongoing"
        />
      </template>

      <!-- Empty state -->
      <div
        v-else
        class="devtools-empty"
      >
        <i class="fa-solid fa-chart-line" />
        <p>Select a project and session to analyze</p>
        <p class="devtools-empty-hint">
          Session logs are read from ~/.claude/projects/
        </p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useDevtoolsStore } from '@/stores/devtoolsStore'
import ProjectSidebar from '@/components/devtools/ProjectSidebar.vue'
import SessionView from '@/components/devtools/SessionView.vue'
import MetricsPanel from '@/components/devtools/MetricsPanel.vue'

defineProps<{
  windowId?: string
  windowData?: Record<string, unknown>
}>()

const store = useDevtoolsStore()
let unlisten: UnlistenFn | null = null

onMounted(async () => {
  await store.loadProjects()
  await store.startWatching()

  unlisten = await listen<string>('devtools-file-change', (event) => {
    store.handleFileChange(event.payload)
  })
})

onUnmounted(async () => {
  if (unlisten) unlisten()
  await store.stopWatching()
})
</script>

<style scoped>
.devtools {
  display: flex;
  height: 100%;
  background: #FAF9F7;
  color: #2D2B2A;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  font-size: 13px;
}

.devtools-sidebar {
  width: 300px;
  min-width: 240px;
  border-right: 1px solid #E8E5E0;
  overflow-y: auto;
  flex-shrink: 0;
  background: #FFFFFF;
}

.devtools-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}

.devtools-error {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #FEF2F2;
  border-bottom: 1px solid #FECACA;
  color: #B91C1C;
  font-size: 12px;
}

.devtools-error .error-dismiss {
  margin-left: auto;
  background: none;
  border: none;
  color: #B91C1C;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 3px;
}

.devtools-error .error-dismiss:hover {
  background: rgba(185, 28, 28, 0.1);
}

.devtools-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 100%;
  color: #8B8680;
}

.devtools-loading i {
  color: #C4704B;
}

.devtools-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #8B8680;
  gap: 8px;
}

.devtools-empty i {
  font-size: 48px;
  margin-bottom: 8px;
  color: #C4704B;
  opacity: 0.4;
}

.devtools-empty p {
  margin: 0;
  font-size: 14px;
}

.devtools-empty-hint {
  font-size: 12px !important;
  color: #B0ACA7;
}
</style>
