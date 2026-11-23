<script setup lang="ts">
import { ref, computed } from 'vue'
import { useBotStore } from '@/stores/botStore'
import { useSettingsStore } from '@/stores/settingsStore'

const emit = defineEmits<{
  created: []
  cancel: []
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
const description = ref('')
const model = ref('auto')
const creating = ref(false)

async function handleCreate() {
  if (!name.value.trim() || !description.value.trim()) return
  creating.value = true

  try {
    const bot = await botStore.createBot({
      name: name.value.trim(),
      description: description.value.trim(),
      model: model.value,
    })
    if (bot) {
      emit('created')
    }
  } finally {
    creating.value = false
  }
}
</script>

<template>
  <div class="creation-form">
    <div class="form-header">
      <span>Create Bot</span>
      <button
        class="close-btn"
        @click="emit('cancel')"
      >
        <i class="fa-solid fa-xmark" />
      </button>
    </div>

    <div class="form-body">
      <div class="field">
        <label>名称</label>
        <input
          v-model="name"
          type="text"
          placeholder="给机器人起个名字..."
          maxlength="50"
        >
      </div>

      <div class="field">
        <label>描述</label>
        <textarea
          v-model="description"
          placeholder="描述机器人的职责..."
          rows="3"
        />
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

      <button
        class="create-btn"
        :disabled="!name.trim() || !description.trim() || creating"
        @click="handleCreate"
      >
        <i
          v-if="creating"
          class="fa-solid fa-spinner fa-spin"
        />
        <i
          v-else
          class="fa-solid fa-plus"
        />
        {{ creating ? '创建中...' : 'Create Bot' }}
      </button>
    </div>
  </div>
</template>

<style scoped lang="less">
.creation-form {
  border-bottom: 1px solid #30363d;
  background: #0d1117;
}

.form-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  font-size: 12px;
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

.form-body {
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
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

.create-btn {
  padding: 10px;
  background: #238636;
  border: 1px solid #2ea043;
  border-radius: 6px;
  color: #ffffff;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
  transition: all 0.2s;
  margin-top: 4px;

  &:hover:not(:disabled) {
    background: #2ea043;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}
</style>
