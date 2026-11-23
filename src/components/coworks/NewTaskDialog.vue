<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { useApps } from '@/composables/coworks/useApps'
import type { TaskSkillInfo } from '@/types/coworks-persistence'

const { allApps, loadSkillApps, getSkillInfo } = useApps()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'create', task: { name: string; workDir: string; appId?: string; skillInfo?: TaskSkillInfo }): void
}>()

const taskName = ref('')
const workDir = ref('')
const selectedAppId = ref<string | undefined>(undefined)

// Load skill apps on mount
onMounted(() => {
  loadSkillApps()
})

// Handle browse directory
async function handleBrowse() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择工作目录',
  })

  if (selected) {
    workDir.value = selected as string
    // Auto-fill task name if empty
    if (!taskName.value) {
      taskName.value = (selected as string).split(/[/\\]/).pop() || '新任务'
    }
  }
}

// Handle create
function handleCreate() {
  if (!taskName.value.trim() || !workDir.value) {
    return
  }

  // Get skill info if selected app is a skill
  let skillInfo: TaskSkillInfo | undefined
  if (selectedAppId.value) {
    const skillApp = getSkillInfo(selectedAppId.value)
    if (skillApp) {
      skillInfo = {
        skillName: skillApp.skillName,
        skillDescription: skillApp.description, // The skill's description (used as systemPrompt)
        pluginName: skillApp.pluginName,
        pluginPath: skillApp.pluginPath,
        marketplaceName: skillApp.marketplaceName,
      }
    }
  }

  emit('create', {
    name: taskName.value.trim(),
    workDir: workDir.value,
    appId: selectedAppId.value,
    skillInfo,
  })
}

// Handle close
function handleClose() {
  emit('close')
}

// Handle overlay click
function handleOverlayClick(e: MouseEvent) {
  if (e.target === e.currentTarget) {
    handleClose()
  }
}
</script>

<template>
  <div
    class="dialog-overlay"
    @click="handleOverlayClick"
  >
    <div class="dialog">
      <div class="dialog-header">
        <h3>新建任务</h3>
        <button
          class="close-btn"
          @click="handleClose"
        >
          ×
        </button>
      </div>

      <div class="dialog-body">
        <!-- Task name -->
        <div class="form-group">
          <label>任务名称</label>
          <input
            v-model="taskName"
            type="text"
            placeholder="例如：整理文档"
            @keydown.enter="handleCreate"
          >
        </div>

        <!-- Work directory -->
        <div class="form-group">
          <label>工作目录</label>
          <div class="dir-input-group">
            <input
              v-model="workDir"
              type="text"
              placeholder="选择要处理的文件夹"
              readonly
            >
            <button
              class="browse-btn"
              @click="handleBrowse"
            >
              浏览...
            </button>
          </div>
        </div>

        <!-- App selection (optional) -->
        <div class="form-group">
          <label>关联应用 <span class="optional">(可选)</span></label>
          <div class="app-grid">
            <button
              v-for="app in allApps"
              :key="app.id"
              class="app-item"
              :class="{
                selected: selectedAppId === app.id,
                'is-skill': 'isSkill' in app && app.isSkill,
                'is-local-skill': 'isSkill' in app && app.isSkill && 'iconCategory' in app && app.iconCategory === 'local',
                'is-plugin-skill': 'isSkill' in app && app.isSkill && 'iconCategory' in app && app.iconCategory === 'plugin',
              }"
              :title="app.description"
              @click="selectedAppId = selectedAppId === app.id ? undefined : app.id"
            >
              <div
                v-if="'iconType' in app && app.iconType === 'fontawesome'"
                class="app-icon-fa"
                :class="{ local: 'iconCategory' in app && app.iconCategory === 'local', plugin: 'iconCategory' in app && app.iconCategory === 'plugin' }"
              >
                <i :class="app.icon" />
              </div>
              <span
                v-else
                class="app-icon"
              >{{ app.icon }}</span>
              <span class="app-name">{{ app.name }}</span>
              <span
                v-if="'isSkill' in app && app.isSkill"
                class="skill-badge"
                :class="{ local: 'iconCategory' in app && app.iconCategory === 'local' }"
              >
                {{ 'iconCategory' in app && app.iconCategory === 'local' ? 'Local' : 'Skill' }}
              </span>
            </button>
          </div>
        </div>
      </div>

      <div class="dialog-footer">
        <button
          class="cancel-btn"
          @click="handleClose"
        >
          取消
        </button>
        <button
          class="create-btn"
          :disabled="!taskName.trim() || !workDir"
          @click="handleCreate"
        >
          创建任务
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.dialog {
  width: 480px;
  max-width: 90%;
  background: #FFFFFF;
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.12);
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid #E8E5E0;
}

.dialog-header h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: #2D2B2A;
}

.close-btn {
  background: none;
  border: none;
  color: #8B8680;
  font-size: 20px;
  cursor: pointer;
  padding: 4px 8px;
  transition: color 0.15s;
}

.close-btn:hover {
  color: #2D2B2A;
}

.dialog-body {
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.form-group label {
  font-size: 13px;
  font-weight: 500;
  color: #2D2B2A;
}

.optional {
  font-weight: normal;
  color: #8B8680;
}

.form-group input {
  padding: 10px 12px;
  background: #FAF9F7;
  border: 1px solid #E8E5E0;
  border-radius: 6px;
  color: #2D2B2A;
  font-size: 14px;
}

.form-group input:focus {
  outline: none;
  border-color: rgba(196, 112, 75, 0.5);
}

.form-group input::placeholder {
  color: #B0ACA7;
}

.dir-input-group {
  display: flex;
  gap: 8px;
}

.dir-input-group input {
  flex: 1;
}

.browse-btn {
  padding: 10px 16px;
  background: rgba(196, 112, 75, 0.10);
  border: 1px solid rgba(196, 112, 75, 0.2);
  border-radius: 6px;
  color: #C4704B;
  font-size: 13px;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s;
}

.browse-btn:hover {
  background: rgba(196, 112, 75, 0.18);
}

.app-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  max-height: 240px;
  overflow-y: auto;
  padding-right: 4px;
}

.app-grid::-webkit-scrollbar {
  width: 4px;
}

.app-grid::-webkit-scrollbar-track {
  background: transparent;
}

.app-grid::-webkit-scrollbar-thumb {
  background: #D0CCC7;
  border-radius: 2px;
}

.app-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 12px 8px;
  background: #FAF9F7;
  border: 1px solid #E8E5E0;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
  position: relative;
}

.app-item:hover {
  background: rgba(0, 0, 0, 0.03);
}

.app-item.selected {
  background: rgba(16, 185, 129, 0.10);
  border-color: rgba(16, 185, 129, 0.4);
}

.app-item.is-skill {
  border-color: rgba(34, 197, 94, 0.3);
}

.app-item.is-skill:hover {
  background: rgba(34, 197, 94, 0.1);
}

.app-item.is-skill.selected {
  background: rgba(34, 197, 94, 0.2);
  border-color: rgba(34, 197, 94, 0.6);
}

.app-item.is-local-skill {
  border-color: rgba(124, 110, 184, 0.3);
}

.app-item.is-local-skill:hover {
  background: rgba(124, 110, 184, 0.1);
}

.app-item.is-local-skill.selected {
  background: rgba(124, 110, 184, 0.2);
  border-color: rgba(124, 110, 184, 0.6);
}

.app-icon {
  font-size: 20px;
}

.app-icon-fa {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #7c6eb8 0%, #a490c4 100%);

  &.local {
    background: linear-gradient(135deg, #7c6eb8 0%, #a490c4 100%);
  }

  &.plugin {
    background: linear-gradient(135deg, #22c55e 0%, #4ade80 100%);
  }

  i {
    font-size: 16px;
    color: white;
  }
}

.app-name {
  font-size: 11px;
  color: #2D2B2A;
  text-align: center;
}

.skill-badge {
  position: absolute;
  top: 2px;
  right: 2px;
  font-size: 8px;
  padding: 1px 4px;
  background: rgba(34, 197, 94, 0.3);
  border-radius: 3px;
  color: #22c55e;
  text-transform: uppercase;

  &.local {
    background: rgba(124, 110, 184, 0.3);
    color: #a490c4;
  }
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 16px 20px;
  border-top: 1px solid #E8E5E0;
}

.cancel-btn,
.create-btn {
  padding: 10px 20px;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.cancel-btn {
  background: transparent;
  border: 1px solid #E8E5E0;
  color: #2D2B2A;
}

.cancel-btn:hover {
  background: rgba(0, 0, 0, 0.03);
}

.create-btn {
  background: #10b981;
  border: none;
  color: #FFFFFF;
}

.create-btn:hover:not(:disabled) {
  background: #059669;
}

.create-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
