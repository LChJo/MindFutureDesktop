<script setup lang="ts">
import { ref, computed } from 'vue'
import { useSettingsStore } from '@/stores/settingsStore'

defineProps<{
  model: string
}>()

const emit = defineEmits<{
  (e: 'update:model', model: string): void
}>()

const isOpen = ref(false)
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

  // Append custom providers from settings
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

function selectModel(modelId: string) {
  emit('update:model', modelId)
  isOpen.value = false
}

function getModelName(modelId: string) {
  return models.value.find(m => m.id === modelId)?.name || modelId
}
</script>

<template>
  <div
    class="model-select"
    :class="{ open: isOpen }"
  >
    <button
      class="select-btn"
      @click="isOpen = !isOpen"
    >
      <span class="model-name">{{ getModelName(model) }}</span>
      <span class="dropdown-icon">▼</span>
    </button>

    <div
      v-if="isOpen"
      class="dropdown"
    >
      <button
        v-for="m in models"
        :key="m.id"
        class="dropdown-item"
        :class="{ active: model === m.id }"
        @click="selectModel(m.id)"
      >
        <span class="item-name">{{ m.name }}</span>
        <span class="item-desc">{{ m.description }}</span>
      </button>
    </div>

    <div
      v-if="isOpen"
      class="backdrop"
      @click="isOpen = false"
    />
  </div>
</template>

<style scoped>
.model-select {
  position: relative;
}

.select-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  background: #FFFFFF;
  border: 1px solid #E8E5E0;
  border-radius: 8px;
  color: #2D2B2A;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s;
}

.select-btn:hover {
  background: rgba(0, 0, 0, 0.03);
  border-color: #D0CCC7;
}

.model-select.open .select-btn {
  border-color: rgba(196, 112, 75, 0.5);
}

.dropdown-icon {
  font-size: 10px;
  color: #8B8680;
  transition: transform 0.15s;
}

.model-select.open .dropdown-icon {
  transform: rotate(180deg);
}

.dropdown {
  position: absolute;
  bottom: calc(100% + 4px);
  left: 0;
  min-width: 220px;
  background: #FFFFFF;
  border: 1px solid #E8E5E0;
  border-radius: 8px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.08);
  z-index: 100;
  overflow: hidden;
}

.dropdown-item {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  width: 100%;
  padding: 10px 14px;
  background: transparent;
  border: none;
  color: #2D2B2A;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s;
}

.dropdown-item:hover {
  background: rgba(0, 0, 0, 0.03);
}

.dropdown-item.active {
  background: rgba(196, 112, 75, 0.08);
}

.item-name {
  font-weight: 500;
}

.item-desc {
  font-size: 11px;
  color: #8B8680;
  margin-top: 2px;
}

.backdrop {
  position: fixed;
  inset: 0;
  z-index: 99;
}
</style>
