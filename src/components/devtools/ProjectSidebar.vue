<template>
  <div class="sidebar">
    <!-- Header -->
    <div class="sidebar-header">
      <h3>
        <i class="fa-solid fa-chart-line" />
        DevTools
      </h3>
      <button
        class="refresh-btn"
        :disabled="store.isLoadingProjects"
        @click="store.loadProjects()"
      >
        <i
          class="fa-solid"
          :class="store.isLoadingProjects ? 'fa-spinner fa-spin' : 'fa-rotate'"
        />
      </button>
    </div>

    <!-- Search -->
    <div class="sidebar-search">
      <i class="fa-solid fa-magnifying-glass" />
      <input
        v-model="store.searchQuery"
        type="text"
        placeholder="Search projects..."
      >
    </div>

    <!-- Project List -->
    <div class="project-list">
      <div
        v-if="store.isLoadingProjects"
        class="loading-hint"
      >
        <i class="fa-solid fa-spinner fa-spin" /> Loading...
      </div>

      <div
        v-for="project in store.filteredProjects"
        :key="project.id"
        class="project-item"
        :class="{ active: store.selectedProjectId === project.id }"
      >
        <div
          class="project-header"
          @click="toggleProject(project.id)"
        >
          <i
            class="fa-solid"
            :class="expandedProjects.has(project.id) ? 'fa-chevron-down' : 'fa-chevron-right'"
          />
          <span
            class="project-name"
            :title="project.path"
          >{{ project.name }}</span>
          <span class="project-count">{{ project.session_count }}</span>
        </div>

        <!-- Sessions -->
        <div
          v-if="expandedProjects.has(project.id)"
          class="session-list"
        >
          <div
            v-if="store.isLoadingSessions && store.selectedProjectId === project.id"
            class="loading-hint"
          >
            <i class="fa-solid fa-spinner fa-spin" />
          </div>
          <div
            v-for="session in projectSessions(project.id)"
            :key="session.id"
            class="session-item"
            :class="{ active: store.selectedSessionId === session.id }"
            @click="store.selectSession(session.id, project.id)"
          >
            <div class="session-header">
              <span
                v-if="session.is_ongoing"
                class="ongoing-dot"
                title="Active session"
              />
              <span class="session-preview">
                {{ session.first_message || session.id.slice(0, 8) }}
              </span>
            </div>
            <div class="session-meta">
              <span
                class="meta-item"
                :title="`${session.metrics.total_tokens} tokens`"
              >
                <i class="fa-solid fa-coins" />
                {{ formatTokensCompact(session.metrics.total_tokens) }}
              </span>
              <span
                class="meta-item"
                :title="`$${session.metrics.cost_usd.toFixed(4)}`"
              >
                {{ formatCostUsd(session.metrics.cost_usd) }}
              </span>
              <span class="meta-item">
                {{ formatRelativeTime(session.created_at) }}
              </span>
            </div>
          </div>

          <div
            v-if="!store.isLoadingSessions && projectSessions(project.id).length === 0"
            class="no-sessions"
          >
            No sessions
          </div>
        </div>
      </div>

      <div
        v-if="!store.isLoadingProjects && store.filteredProjects.length === 0"
        class="empty-hint"
      >
        No projects found
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useDevtoolsStore } from '@/stores/devtoolsStore'
import { formatTokensCompact, formatCostUsd, formatRelativeTime } from '@/types/devtools'
import type { SessionSummary } from '@/types/devtools'

const store = useDevtoolsStore()
const expandedProjects = ref(new Set<string>())
const sessionCache = ref<Record<string, SessionSummary[]>>({})

async function toggleProject(projectId: string) {
  const next = new Set(expandedProjects.value)
  if (next.has(projectId)) {
    next.delete(projectId)
  } else {
    next.add(projectId)
    // Cache current project sessions before switching
    if (store.selectedProjectId && store.selectedProjectId !== projectId) {
      sessionCache.value = { ...sessionCache.value, [store.selectedProjectId]: store.sessions }
    }
    await store.selectProject(projectId)
  }
  expandedProjects.value = next
}

function projectSessions(projectId: string): SessionSummary[] {
  if (store.selectedProjectId === projectId) {
    return store.sessions
  }
  return sessionCache.value[projectId] || []
}
</script>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px;
  border-bottom: 1px solid #E8E5E0;
}

.sidebar-header h3 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: #2D2B2A;
  display: flex;
  align-items: center;
  gap: 8px;
}

.sidebar-header h3 i {
  color: #C4704B;
}

.refresh-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(196, 112, 75, 0.10);
  border: none;
  border-radius: 6px;
  cursor: pointer;
  color: #C4704B;
  font-size: 12px;
  transition: all 0.15s;
}

.refresh-btn:hover:not(:disabled) {
  background: rgba(196, 112, 75, 0.18);
}

.refresh-btn:disabled {
  opacity: 0.5;
}

.sidebar-search {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid #E8E5E0;
}

.sidebar-search i {
  color: #B0ACA7;
  font-size: 12px;
}

.sidebar-search input {
  flex: 1;
  background: none;
  border: none;
  color: #2D2B2A;
  outline: none;
  font-size: 12px;
}

.sidebar-search input::placeholder {
  color: #B0ACA7;
}

.project-list {
  flex: 1;
  overflow-y: auto;
}

.project-item {
  border-bottom: 1px solid #F0EDE8;
}

.project-item.active > .project-header {
  background: rgba(196, 112, 75, 0.06);
}

.project-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  cursor: pointer;
  user-select: none;
}

.project-header:hover {
  background: #F5F3EF;
}

.project-header i {
  font-size: 10px;
  color: #8B8680;
  width: 14px;
  text-align: center;
}

.project-name {
  flex: 1;
  font-size: 13px;
  font-weight: 500;
  color: #2D2B2A;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-count {
  font-size: 11px;
  color: #8B8680;
  background: #F0EDE8;
  padding: 1px 6px;
  border-radius: 8px;
}

.session-list {
  padding-left: 20px;
}

.session-item {
  padding: 6px 12px;
  cursor: pointer;
  border-left: 2px solid transparent;
  transition: background 0.15s;
}

.session-item:hover {
  background: #F5F3EF;
}

.session-item.active {
  background: rgba(196, 112, 75, 0.08);
  border-left-color: #C4704B;
}

.session-header {
  display: flex;
  align-items: center;
  gap: 6px;
}

.ongoing-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #16A34A;
  flex-shrink: 0;
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.session-preview {
  font-size: 12px;
  color: #4A4845;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.session-meta {
  display: flex;
  gap: 8px;
  margin-top: 2px;
}

.meta-item {
  font-size: 10px;
  color: #8B8680;
  display: flex;
  align-items: center;
  gap: 3px;
}

.meta-item i {
  font-size: 9px;
}

.loading-hint {
  padding: 12px;
  text-align: center;
  color: #8B8680;
  font-size: 12px;
}

.loading-hint i {
  color: #C4704B;
}

.no-sessions {
  padding: 8px 12px;
  color: #B0ACA7;
  font-size: 11px;
  font-style: italic;
}

.empty-hint {
  padding: 24px 12px;
  text-align: center;
  color: #B0ACA7;
  font-size: 12px;
}
</style>
