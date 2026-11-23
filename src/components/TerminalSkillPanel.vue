<script setup lang="ts">
import { ref, computed } from 'vue'

interface SkillInfo {
  skillName: string
  skillDescription: string
  pluginName: string
  pluginDescription: string
  pluginVersion: string
  marketplaceName: string
  agents: string[]
  mcpServers: Record<string, unknown>
}

const props = defineProps<{
  skillInfo: SkillInfo
}>()

const isCollapsed = ref(false)

const togglePanel = () => {
  isCollapsed.value = !isCollapsed.value
}

// Format agent name from path (e.g., "./agents/my-agent.md" -> "my-agent")
const formatAgentName = (agent: string) => {
  const name = agent.split('/').pop() || agent
  return name.replace(/\.md$/, '')
}

// Check if there are MCP servers
const hasMcpServers = computed(() => {
  return props.skillInfo.mcpServers && Object.keys(props.skillInfo.mcpServers).length > 0
})
</script>

<template>
  <div
    class="terminal-skill-panel"
    :class="{ collapsed: isCollapsed }"
  >
    <!-- Toggle button -->
    <div
      class="panel-toggle"
      :title="isCollapsed ? 'Show info' : 'Hide info'"
      @click="togglePanel"
    >
      <i :class="isCollapsed ? 'fa fa-chevron-right' : 'fa fa-chevron-left'" />
    </div>

    <!-- Panel content -->
    <div
      v-show="!isCollapsed"
      class="panel-content"
    >
      <!-- Skill info -->
      <div class="info-section">
        <h3><i class="fa fa-magic" /> Skill</h3>
        <div class="info-name">
          {{ skillInfo.skillName }}
        </div>
        <div
          v-if="skillInfo.skillDescription"
          class="info-desc"
        >
          {{ skillInfo.skillDescription }}
        </div>
      </div>

      <!-- Plugin info -->
      <div class="info-section">
        <h3><i class="fa fa-puzzle-piece" /> Plugin</h3>
        <div class="info-name">
          {{ skillInfo.pluginName }}
        </div>
        <div
          v-if="skillInfo.pluginVersion"
          class="info-version"
        >
          v{{ skillInfo.pluginVersion }}
        </div>
        <div
          v-if="skillInfo.pluginDescription"
          class="info-desc"
        >
          {{ skillInfo.pluginDescription }}
        </div>
        <div class="info-source">
          <i class="fa fa-store" /> {{ skillInfo.marketplaceName }}
        </div>
      </div>

      <!-- Agents list -->
      <div
        v-if="skillInfo.agents?.length"
        class="info-section"
      >
        <h3><i class="fa fa-robot" /> Agents ({{ skillInfo.agents.length }})</h3>
        <div class="info-list">
          <div
            v-for="agent in skillInfo.agents"
            :key="agent"
            class="list-item"
          >
            <i class="fa fa-user-gear" />
            {{ formatAgentName(agent) }}
          </div>
        </div>
      </div>

      <!-- MCP Servers -->
      <div
        v-if="hasMcpServers"
        class="info-section"
      >
        <h3><i class="fa fa-server" /> MCP Servers</h3>
        <div class="info-list">
          <div
            v-for="(_, name) in skillInfo.mcpServers"
            :key="name"
            class="list-item"
          >
            <i class="fa fa-plug" />
            {{ name }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="less" scoped>
.terminal-skill-panel {
  display: flex;
  flex-shrink: 0;
  background: #252526;
  border-right: 1px solid #3c3c3c;
  position: relative;
  width: 220px;
  transition: width 0.2s ease;

  &.collapsed {
    width: 24px;

    .panel-content {
      display: none;
    }
  }
}

.panel-toggle {
  width: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  color: #858585;
  flex-shrink: 0;

  &:hover {
    color: #fff;
    background: #2a2d2e;
  }
}

.panel-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  overflow-x: hidden;
  min-width: 0;
  padding: 8px 0;

  &::-webkit-scrollbar {
    width: 6px;
  }

  &::-webkit-scrollbar-thumb {
    background: #555;
    border-radius: 3px;
  }
}

.info-section {
  padding: 8px 12px;
  border-bottom: 1px solid #3c3c3c;

  &:last-child {
    border-bottom: none;
  }

  h3 {
    font-size: 11px;
    font-weight: 600;
    color: #858585;
    text-transform: uppercase;
    margin: 0 0 8px 0;
    display: flex;
    align-items: center;
    gap: 6px;

    i {
      font-size: 12px;
      color: #569cd6;
    }
  }

  .info-name {
    font-size: 14px;
    font-weight: 500;
    color: #e0e0e0;
    margin-bottom: 4px;
  }

  .info-version {
    font-size: 11px;
    color: #6a9955;
    margin-bottom: 4px;
  }

  .info-desc {
    font-size: 12px;
    color: #9d9d9d;
    line-height: 1.4;
    margin-bottom: 6px;
  }

  .info-source {
    font-size: 11px;
    color: #858585;
    display: flex;
    align-items: center;
    gap: 4px;

    i {
      color: #c586c0;
    }
  }
}

.info-list {
  display: flex;
  flex-direction: column;
  gap: 4px;

  .list-item {
    font-size: 12px;
    color: #cccccc;
    padding: 4px 8px;
    background: #2d2d2d;
    border-radius: 4px;
    display: flex;
    align-items: center;
    gap: 6px;

    i {
      font-size: 11px;
      color: #858585;
    }
  }
}
</style>
