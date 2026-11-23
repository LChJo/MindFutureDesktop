<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useBotStore, buildSystemPrompt } from '@/stores/botStore'
import { useSettingsStore } from '@/stores/settingsStore'
import { useBotSession } from '@/composables/claudeBot/useBotSession'

const props = defineProps<{
  botId: string
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const botStore = useBotStore()
const settingsStore = useSettingsStore()

const builtinModels = [
  { id: 'Sonnet', name: 'Sonnet', description: '快速且智能' },
  { id: 'Opus', name: 'Opus', description: '最强大的模型' },
  { id: 'Haiku', name: 'Haiku', description: '最快速度' },
]

const models = computed(() => {
  const anthropicModel = settingsStore.environmentVariables?.['ANTHROPIC_MODEL']
  const autoDescription = anthropicModel
    ? `自动 (${anthropicModel})`
    : '自动选择最佳模型 (Sonnet)'

  const base = [
    { id: 'auto', name: 'Auto', description: autoDescription },
    ...builtinModels,
  ]

  const customProviders = settingsStore.coworksModelConfig?.providers || []
  const builtinIds = new Set(['auto', 'Sonnet', 'Opus', 'Haiku'])
  const custom = customProviders
    .filter(p => !builtinIds.has(p.id))
    .map(p => ({
      id: p.id,
      name: p.name,
      description: p.baseUrl ? `${p.model} (自定义)` : p.model,
    }))

  return [...base, ...custom]
})

const name = ref('')
const model = ref('auto')
const description = ref('')
const saving = ref(false)
const restarting = ref(false)
const saved = ref(false)

const bot = computed(() => botStore.bots.find(b => b.id === props.botId))
const needsRestart = computed(() => saved.value && bot.value?.status !== 'stopped')

function loadFromBot() {
  if (!bot.value) return
  name.value = bot.value.name
  model.value = bot.value.model
  description.value = bot.value.description
  saved.value = false
}

onMounted(loadFromBot)
watch(() => props.botId, loadFromBot)

async function handleSave() {
  if (!name.value.trim() || !description.value.trim()) return
  saving.value = true
  try {
    await botStore.updateBot(props.botId, {
      name: name.value.trim(),
      model: model.value,
      description: description.value.trim(),
      systemPrompt: buildSystemPrompt(name.value.trim(), description.value.trim()),
    })
    saved.value = true
  } finally {
    saving.value = false
  }
}

async function handleRestart() {
  restarting.value = true
  try {
    const botSession = useBotSession()
    await botSession.stopSession(props.botId)
    await botStore.updateBotStatus(props.botId, 'idle')
  } finally {
    restarting.value = false
  }
  emit('close')
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="dialog-overlay"
      @click.self="emit('close')"
    >
      <div class="dialog">
        <div class="dialog-header">
          <span>编辑机器人</span>
          <button
            class="close-btn"
            @click="emit('close')"
          >
            <i class="fa-solid fa-xmark" />
          </button>
        </div>

        <div class="dialog-body">
          <div class="field">
            <label>名称</label>
            <input
              v-model="name"
              type="text"
              placeholder="机器人名称..."
              maxlength="50"
            >
          </div>

          <div class="field">
            <label>模型</label>
            <select v-model="model">
              <option
                v-for="m in models"
                :key="m.id"
                :value="m.id"
              >
                {{ m.name }} — {{ m.description }}
              </option>
            </select>
          </div>

          <div class="field">
            <label>描述词</label>
            <textarea
              v-model="description"
              placeholder="描述机器人的职责..."
              rows="5"
            />
          </div>

          <div
            v-if="needsRestart"
            class="restart-notice"
          >
            <i class="fa-solid fa-circle-info" />
            修改已保存，重启后生效
          </div>

          <div class="actions">
            <button
              v-if="needsRestart"
              class="restart-btn"
              :disabled="restarting"
              @click="handleRestart"
            >
              <i
                v-if="restarting"
                class="fa-solid fa-spinner fa-spin"
              />
              <i
                v-else
                class="fa-solid fa-rotate-right"
              />
              {{ restarting ? '重启中...' : '重启' }}
            </button>
            <button
              class="save-btn"
              :disabled="!name.trim() || !description.trim() || saving"
              @click="handleSave"
            >
              <i
                v-if="saving"
                class="fa-solid fa-spinner fa-spin"
              />
              <i
                v-else
                class="fa-solid fa-floppy-disk"
              />
              {{ saving ? '保存中...' : '保存' }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped lang="less">
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
}

.dialog {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 10px;
  width: 420px;
  max-width: 90vw;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  display: flex;
  flex-direction: column;
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  font-size: 13px;
  font-weight: 600;
  color: #58a6ff;
  border-bottom: 1px solid #30363d;
}

.close-btn {
  background: none;
  border: none;
  color: #8b949e;
  cursor: pointer;
  padding: 4px;
  font-size: 14px;

  &:hover {
    color: #e6edf3;
  }
}

.dialog-body {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;

  label {
    font-size: 11px;
    color: #8b949e;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  input,
  textarea,
  select {
    padding: 8px 10px;
    background: #0d1117;
    border: 1px solid #30363d;
    border-radius: 6px;
    color: #e6edf3;
    font-size: 13px;
    font-family: inherit;
    resize: vertical;

    &:focus {
      outline: none;
      border-color: #58a6ff;
    }

    &::placeholder {
      color: #6e7681;
    }
  }

  select {
    cursor: pointer;
  }
}

.restart-notice {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  background: rgba(210, 153, 34, 0.1);
  border: 1px solid rgba(210, 153, 34, 0.3);
  border-radius: 6px;
  color: #d29922;
  font-size: 12px;

  i {
    flex-shrink: 0;
  }
}

.actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.save-btn {
  padding: 8px 16px;
  background: #238636;
  border: 1px solid #2ea043;
  border-radius: 6px;
  color: #ffffff;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
  transition: all 0.2s;

  &:hover:not(:disabled) {
    background: #2ea043;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.restart-btn {
  padding: 8px 16px;
  background: rgba(210, 153, 34, 0.15);
  border: 1px solid rgba(210, 153, 34, 0.4);
  border-radius: 6px;
  color: #d29922;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
  transition: all 0.2s;

  &:hover:not(:disabled) {
    background: rgba(210, 153, 34, 0.25);
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}
</style>
