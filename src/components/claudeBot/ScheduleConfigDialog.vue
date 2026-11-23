<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useBotStore } from '@/stores/botStore'
import { useBotScheduler } from '@/composables/claudeBot/useBotScheduler'
import { migrateScheduleConfig } from '@/types/claude-bot'
import type { BotScheduleConfig, FixedTimeEntry, IntervalEntry } from '@/types/claude-bot'

const props = defineProps<{
  botId: string
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const botStore = useBotStore()
const scheduler = useBotScheduler()
const saving = ref(false)

const config = ref<BotScheduleConfig>({
  intervalEnabled: false,
  intervals: [],
  fixedTimeEnabled: false,
  fixedTimes: [],
  message: '',
})

// === 添加/编辑间隔子弹窗 ===
const editingInterval = ref<{ index: number; intervalMinutes: number; message?: string; chatId?: string } | null>(null)

function openAddIntervalDialog() {
  editingInterval.value = { index: -1, intervalMinutes: 60, message: '', chatId: '' }
}

function openEditIntervalDialog(index: number) {
  const entry = config.value.intervals[index]
  editingInterval.value = {
    index,
    intervalMinutes: entry.intervalMinutes,
    message: entry.message ?? '',
    chatId: entry.chatId ?? '',
  }
}

function confirmIntervalEntry() {
  if (!editingInterval.value) return
  const { index, intervalMinutes, message, chatId } = editingInterval.value
  if (!intervalMinutes || intervalMinutes <= 0) return

  const entry: IntervalEntry = {
    intervalMinutes,
    ...(message?.trim() ? { message: message.trim() } : {}),
    ...(chatId?.trim() ? { chatId: chatId.trim() } : {}),
  }
  const intervals = [...config.value.intervals]

  if (index === -1) {
    intervals.push(entry)
  } else {
    intervals[index] = entry
  }

  config.value = { ...config.value, intervals }
  editingInterval.value = null
  botStore.saveScheduleConfig(props.botId, config.value).catch(console.error)
}

function removeInterval(index: number) {
  const intervals = config.value.intervals.filter((_, i) => i !== index)
  config.value = { ...config.value, intervals }
}

// === 添加/编辑时间子弹窗 ===
const editingEntry = ref<{ index: number; time: string; repeatDays?: number[]; message?: string; chatId?: string } | null>(null)

const dayLabels = ['日', '一', '二', '三', '四', '五', '六']

function openAddTimeDialog() {
  editingEntry.value = { index: -1, time: '09:00', repeatDays: undefined, message: '', chatId: '' }
}

function openEditTimeDialog(index: number) {
  const entry = config.value.fixedTimes[index]
  editingEntry.value = {
    index,
    time: entry.time,
    repeatDays: entry.repeatDays ? [...entry.repeatDays] : undefined,
    message: entry.message ?? '',
    chatId: entry.chatId ?? '',
  }
}

function toggleEditDay(day: number) {
  if (!editingEntry.value) return
  // 当前为"每天"（undefined）时，点击某天直接选中仅该天，而非从全7天中移除
  if (!editingEntry.value.repeatDays?.length) {
    editingEntry.value = { ...editingEntry.value, repeatDays: [day] }
    return
  }
  const current = [...editingEntry.value.repeatDays]
  const idx = current.indexOf(day)
  if (idx >= 0) {
    current.splice(idx, 1)
  } else {
    current.push(day)
    current.sort()
  }
  const days = current.length === 0 || current.length === 7 ? undefined : current
  editingEntry.value = { ...editingEntry.value, repeatDays: days }
}

function isEditDayActive(day: number): boolean {
  if (!editingEntry.value) return false
  return !editingEntry.value.repeatDays?.length || editingEntry.value.repeatDays.includes(day)
}

function getRepeatSummary(days?: number[]): string {
  if (!days?.length) return '每天'
  const sorted = [...days].sort()
  if (sorted.length === 5 && !sorted.includes(0) && !sorted.includes(6)) return '工作日'
  if (sorted.length === 2 && sorted.includes(0) && sorted.includes(6)) return '周末'
  return '每' + sorted.map(d => '周' + dayLabels[d]).join('、')
}

const editRepeatSummary = computed(() => {
  if (!editingEntry.value) return ''
  return getRepeatSummary(editingEntry.value.repeatDays)
})

function confirmTimeEntry() {
  if (!editingEntry.value) return
  const { index, time, repeatDays, message, chatId } = editingEntry.value
  if (!time || !/^\d{2}:\d{2}$/.test(time)) return

  const entry: FixedTimeEntry = {
    time,
    repeatDays,
    ...(message?.trim() ? { message: message.trim() } : {}),
    ...(chatId?.trim() ? { chatId: chatId.trim() } : {}),
  }
  const times = [...config.value.fixedTimes]

  if (index === -1) {
    // 新增 - 检查重复
    if (times.some(e => e.time === time)) {
      editingEntry.value = null
      return
    }
    times.push(entry)
  } else {
    // 编辑
    times[index] = entry
  }

  times.sort((a, b) => a.time.localeCompare(b.time))
  config.value = { ...config.value, fixedTimes: times }
  editingEntry.value = null
  botStore.saveScheduleConfig(props.botId, config.value).catch(console.error)
}

function removeFixedTime(index: number) {
  const times = config.value.fixedTimes.filter((_, i) => i !== index)
  config.value = { ...config.value, fixedTimes: times }
}

// === 加载配置 ===
async function loadConfig() {
  const loaded = await botStore.loadScheduleConfig(props.botId)
  config.value = migrateScheduleConfig(loaded)
}

// === 提示消息 ===
const hintMessage = ref('')
let hintTimer: ReturnType<typeof setTimeout> | null = null

function showHint(msg: string) {
  hintMessage.value = msg
  if (hintTimer) clearTimeout(hintTimer)
  hintTimer = setTimeout(() => { hintMessage.value = '' }, 3000)
}

// === 启用/停止 handlers ===
async function handleIntervalEnable() {
  if (!config.value.intervals.length) {
    showHint('请先添加至少一条间隔配置')
    return
  }
  saving.value = true
  try {
    const newConfig = { ...config.value, intervalEnabled: true }
    await botStore.saveScheduleConfig(props.botId, newConfig)
    config.value = newConfig
    const bot = botStore.bots.find(b => b.id === props.botId)
    if (bot) await scheduler.startSchedule(bot, newConfig)
  } finally {
    saving.value = false
  }
}

async function handleIntervalStop() {
  saving.value = true
  try {
    const newConfig = { ...config.value, intervalEnabled: false }
    await botStore.saveScheduleConfig(props.botId, newConfig)
    config.value = newConfig
    const bot = botStore.bots.find(b => b.id === props.botId)
    if (bot) await scheduler.startSchedule(bot, newConfig)
  } finally {
    saving.value = false
  }
}

async function handleFixedTimeEnable() {
  if (!config.value.fixedTimes.length) {
    showHint('请先添加至少一个时间点')
    return
  }
  saving.value = true
  try {
    const newConfig = { ...config.value, fixedTimeEnabled: true }
    await botStore.saveScheduleConfig(props.botId, newConfig)
    config.value = newConfig
    const bot = botStore.bots.find(b => b.id === props.botId)
    if (bot) await scheduler.startSchedule(bot, newConfig)
  } finally {
    saving.value = false
  }
}

async function handleFixedTimeStop() {
  saving.value = true
  try {
    const newConfig = { ...config.value, fixedTimeEnabled: false }
    await botStore.saveScheduleConfig(props.botId, newConfig)
    config.value = newConfig
    const bot = botStore.bots.find(b => b.id === props.botId)
    if (bot) await scheduler.startSchedule(bot, newConfig)
  } finally {
    saving.value = false
  }
}

function handleOverlayClick(e: MouseEvent) {
  if ((e.target as HTMLElement).classList.contains('schedule-dialog-overlay')) {
    emit('close')
  }
}

watch(() => props.visible, (v) => {
  if (v) loadConfig()
})

onMounted(() => {
  if (props.visible) loadConfig()
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="schedule-dialog-overlay"
      @click="handleOverlayClick"
    >
      <div class="schedule-dialog">
        <!-- Header -->
        <div class="dialog-header">
          <div class="header-title">
            <i class="fa-solid fa-clock" />
            定时配置
          </div>
          <button
            class="close-btn"
            @click="emit('close')"
          >
            <i class="fa-solid fa-xmark" />
          </button>
        </div>

        <!-- Body -->
        <div class="dialog-body">
          <!-- 固定间隔 Section -->
          <div class="schedule-section">
            <div class="section-header">
              <span class="section-title">
                <i class="fa-solid fa-rotate" />
                固定间隔
              </span>
              <div class="section-status">
                <span
                  v-if="config.intervalEnabled"
                  class="status-active"
                >
                  <i class="fa-solid fa-circle" /> 运行中
                </span>
                <span
                  v-else
                  class="status-inactive"
                >
                  <i class="fa-regular fa-circle" /> 未启用
                </span>
                <button
                  v-if="config.intervalEnabled"
                  class="section-stop-btn"
                  :disabled="saving"
                  @click="handleIntervalStop"
                >
                  停止
                </button>
                <button
                  v-else
                  class="section-enable-btn"
                  :disabled="saving"
                  @click="handleIntervalEnable"
                >
                  启用
                </button>
              </div>
            </div>
            <!-- 间隔条目列表 -->
            <div
              v-if="config.intervals.length"
              class="time-entries"
            >
              <div
                v-for="(entry, index) in config.intervals"
                :key="index"
                class="time-entry"
                @click="openEditIntervalDialog(index)"
              >
                <span class="entry-time">{{ entry.intervalMinutes }}<span class="entry-unit">分钟</span></span>
                <span class="entry-repeat">每隔 {{ entry.intervalMinutes }} 分钟</span>
                <span
                  v-if="entry.message || entry.chatId"
                  class="entry-custom-badge"
                >
                  <i
                    v-if="entry.message"
                    class="fa-solid fa-comment"
                    title="自定义消息"
                  />
                  <i
                    v-if="entry.chatId"
                    class="fa-solid fa-paper-plane"
                    title="指定 Chat ID"
                  />
                </span>
                <button
                  class="entry-remove"
                  @click.stop="removeInterval(index)"
                >
                  <i class="fa-solid fa-xmark" />
                </button>
              </div>
            </div>

            <button
              class="add-time-btn"
              @click="openAddIntervalDialog"
            >
              <i class="fa-solid fa-plus" />
              添加间隔
            </button>
          </div>

          <div class="section-divider" />

          <!-- 指定时间 Section -->
          <div class="schedule-section">
            <div class="section-header">
              <span class="section-title">
                <i class="fa-regular fa-clock" />
                指定时间
              </span>
              <div class="section-status">
                <span
                  v-if="config.fixedTimeEnabled"
                  class="status-active"
                >
                  <i class="fa-solid fa-circle" /> 运行中
                </span>
                <span
                  v-else
                  class="status-inactive"
                >
                  <i class="fa-regular fa-circle" /> 未启用
                </span>
                <button
                  v-if="config.fixedTimeEnabled"
                  class="section-stop-btn"
                  :disabled="saving"
                  @click="handleFixedTimeStop"
                >
                  停止
                </button>
                <button
                  v-else
                  class="section-enable-btn"
                  :disabled="saving"
                  @click="handleFixedTimeEnable"
                >
                  启用
                </button>
              </div>
            </div>

            <!-- 时间条目列表 -->
            <div
              v-if="config.fixedTimes.length"
              class="time-entries"
            >
              <div
                v-for="(entry, index) in config.fixedTimes"
                :key="index"
                class="time-entry"
                @click="openEditTimeDialog(index)"
              >
                <span class="entry-time">{{ entry.time }}</span>
                <span class="entry-repeat">{{ getRepeatSummary(entry.repeatDays) }}</span>
                <span
                  v-if="entry.message || entry.chatId"
                  class="entry-custom-badge"
                >
                  <i
                    v-if="entry.message"
                    class="fa-solid fa-comment"
                    title="自定义消息"
                  />
                  <i
                    v-if="entry.chatId"
                    class="fa-solid fa-paper-plane"
                    title="指定 Chat ID"
                  />
                </span>
                <button
                  class="entry-remove"
                  @click.stop="removeFixedTime(index)"
                >
                  <i class="fa-solid fa-xmark" />
                </button>
              </div>
            </div>

            <button
              class="add-time-btn"
              @click="openAddTimeDialog"
            >
              <i class="fa-solid fa-plus" />
              添加时间点
            </button>
          </div>

          <div class="section-divider" />

          <!-- Message -->
          <label class="field">
            <span class="field-label">默认消息 <span class="field-hint">（未单独配置消息时使用）</span></span>
            <textarea
              :value="config.message"
              rows="4"
              placeholder="输入定时发送的消息内容..."
              @input="config = { ...config, message: ($event.target as HTMLTextAreaElement).value }"
            />
          </label>

          <!-- 提示消息 -->
          <Transition name="hint-fade">
            <div
              v-if="hintMessage"
              class="hint-bar"
            >
              <i class="fa-solid fa-circle-exclamation" />
              {{ hintMessage }}
            </div>
          </Transition>
        </div>

        <!-- Footer -->
        <div class="dialog-footer">
          <button
            class="close-dialog-btn"
            @click="emit('close')"
          >
            关闭
          </button>
        </div>
      </div>

      <!-- 添加/编辑间隔子弹窗 -->
      <div
        v-if="editingInterval"
        class="time-subdialog-overlay"
        @click.self="editingInterval = null"
      >
        <div class="time-subdialog">
          <div class="subdialog-header">
            <span>{{ editingInterval.index === -1 ? '添加间隔' : '编辑间隔' }}</span>
            <button
              class="close-btn"
              @click="editingInterval = null"
            >
              <i class="fa-solid fa-xmark" />
            </button>
          </div>
          <div class="subdialog-body">
            <label class="field">
              <span class="field-label">间隔（分钟）</span>
              <input
                type="number"
                :value="editingInterval.intervalMinutes"
                min="1"
                placeholder="60"
                @input="editingInterval = { ...editingInterval!, intervalMinutes: Number(($event.target as HTMLInputElement).value) }"
                @blur="editingInterval = { ...editingInterval!, intervalMinutes: Math.max(1, editingInterval!.intervalMinutes || 60) }"
              >
            </label>
            <label class="field">
              <span class="field-label">消息内容 <span class="field-hint">（留空则使用默认消息）</span></span>
              <textarea
                :value="editingInterval.message ?? ''"
                rows="3"
                placeholder="留空则使用默认消息..."
                @input="editingInterval = { ...editingInterval!, message: ($event.target as HTMLTextAreaElement).value }"
              />
            </label>
            <label class="field">
              <span class="field-label">目标 Chat ID <span class="field-hint">（可选）</span></span>
              <input
                :value="editingInterval.chatId ?? ''"
                placeholder="oc_a1b2c3..."
                @input="editingInterval = { ...editingInterval!, chatId: ($event.target as HTMLInputElement).value }"
              >
            </label>
          </div>
          <div class="subdialog-footer">
            <button
              class="cancel-btn"
              @click="editingInterval = null"
            >
              取消
            </button>
            <button
              class="confirm-btn"
              @click="confirmIntervalEntry"
            >
              确定
            </button>
          </div>
        </div>
      </div>

      <!-- 添加/编辑时间子弹窗 -->
      <div
        v-if="editingEntry"
        class="time-subdialog-overlay"
        @click.self="editingEntry = null"
      >
        <div class="time-subdialog">
          <div class="subdialog-header">
            <span>{{ editingEntry.index === -1 ? '添加时间点' : '编辑时间点' }}</span>
            <button
              class="close-btn"
              @click="editingEntry = null"
            >
              <i class="fa-solid fa-xmark" />
            </button>
          </div>
          <div class="subdialog-body">
            <label class="field">
              <span class="field-label">时间</span>
              <input
                type="time"
                :value="editingEntry.time"
                @input="editingEntry = { ...editingEntry!, time: ($event.target as HTMLInputElement).value }"
              >
            </label>
            <div class="repeat-section">
              <span class="field-label">重复</span>
              <div class="day-selector">
                <button
                  v-for="(label, index) in dayLabels"
                  :key="index"
                  class="day-btn"
                  :class="{ active: isEditDayActive(index) }"
                  @click="toggleEditDay(index)"
                >
                  {{ label }}
                </button>
              </div>
              <span class="repeat-hint">{{ editRepeatSummary }}</span>
            </div>
            <label class="field">
              <span class="field-label">消息内容 <span class="field-hint">（留空则使用默认消息）</span></span>
              <textarea
                :value="editingEntry.message ?? ''"
                rows="3"
                placeholder="留空则使用默认消息..."
                @input="editingEntry = { ...editingEntry!, message: ($event.target as HTMLTextAreaElement).value }"
              />
            </label>
            <label class="field">
              <span class="field-label">目标 Chat ID <span class="field-hint">（可选）</span></span>
              <input
                :value="editingEntry.chatId ?? ''"
                placeholder="oc_a1b2c3..."
                @input="editingEntry = { ...editingEntry!, chatId: ($event.target as HTMLInputElement).value }"
              >
            </label>
          </div>
          <div class="subdialog-footer">
            <button
              class="cancel-btn"
              @click="editingEntry = null"
            >
              取消
            </button>
            <button
              class="confirm-btn"
              @click="confirmTimeEntry"
            >
              确定
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped lang="less">
.schedule-dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10001;
}

.schedule-dialog {
  width: 460px;
  max-height: 80vh;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  display: flex;
  flex-direction: column;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid #30363d;
}

.header-title {
  font-size: 14px;
  font-weight: 600;
  color: #e6edf3;
  display: flex;
  align-items: center;
  gap: 8px;
}

.close-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  color: #8b949e;
  cursor: pointer;
  border-radius: 4px;

  &:hover {
    background: #21262d;
    color: #e6edf3;
  }
}

.dialog-body {
  padding: 20px;
  overflow-y: auto;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

// === Section 布局 ===
.schedule-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.section-title {
  font-size: 13px;
  font-weight: 600;
  color: #e6edf3;
  display: flex;
  align-items: center;
  gap: 6px;
}

.section-status {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-active {
  font-size: 11px;
  color: #3fb950;
  i { font-size: 6px; vertical-align: middle; }
}

.status-inactive {
  font-size: 11px;
  color: #8b949e;
  i { font-size: 6px; vertical-align: middle; }
}

.section-enable-btn {
  padding: 4px 12px;
  background: #238636;
  border: 1px solid rgba(240, 246, 252, 0.1);
  border-radius: 4px;
  color: #fff;
  cursor: pointer;
  font-size: 11px;
  font-weight: 600;

  &:hover:not(:disabled) { background: #2ea043; }
  &:disabled { opacity: 0.5; cursor: not-allowed; }
}

.section-stop-btn {
  padding: 4px 12px;
  background: #da3633;
  border: 1px solid rgba(240, 246, 252, 0.1);
  border-radius: 4px;
  color: #fff;
  cursor: pointer;
  font-size: 11px;
  font-weight: 600;

  &:hover:not(:disabled) { background: #f85149; }
  &:disabled { opacity: 0.5; cursor: not-allowed; }
}

.section-divider {
  height: 1px;
  background: #21262d;
}

// === Field 通用 ===
.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-label {
  font-size: 12px;
  color: #8b949e;
}

.field input,
.field textarea {
  padding: 8px 12px;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  color: #e6edf3;
  font-size: 13px;

  &:focus {
    outline: none;
    border-color: #58a6ff;
  }

  &::placeholder {
    color: #484f58;
  }
}

.field input[type="number"] {
  font-family: 'Courier New', monospace;
}

.field textarea {
  resize: vertical;
  min-height: 80px;
  font-family: inherit;
  line-height: 1.5;
}

// === 时间条目列表 ===
.time-entries {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.time-entry {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  cursor: pointer;
  transition: border-color 0.15s;

  &:hover {
    border-color: #58a6ff;
  }
}

.entry-time {
  font-size: 13px;
  font-weight: 600;
  color: #e6edf3;
  font-family: 'Courier New', monospace;
  min-width: 48px;
}

.entry-unit {
  font-size: 10px;
  color: #8b949e;
  margin-left: 2px;
}

.entry-repeat {
  font-size: 11px;
  color: #8b949e;
  flex: 1;
}

.entry-custom-badge {
  display: flex;
  align-items: center;
  gap: 4px;
  color: #58a6ff;
  font-size: 10px;
  margin-left: auto;
  margin-right: 4px;
}

.field-hint {
  color: #6e7681;
  font-weight: 400;
}

.entry-remove {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  color: #484f58;
  cursor: pointer;
  border-radius: 50%;
  font-size: 10px;
  padding: 0;
  flex-shrink: 0;

  &:hover {
    color: #f85149;
    background: rgba(248, 81, 73, 0.1);
  }
}

.add-time-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 8px;
  background: none;
  border: 1px dashed #30363d;
  border-radius: 6px;
  color: #8b949e;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.15s;

  &:hover {
    border-color: #58a6ff;
    color: #58a6ff;
  }
}

// === Footer ===
.dialog-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 16px 20px;
  border-top: 1px solid #30363d;
}

// === 提示条 ===
.hint-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: rgba(210, 153, 34, 0.1);
  border: 1px solid rgba(210, 153, 34, 0.3);
  border-radius: 6px;
  color: #d29922;
  font-size: 12px;
}

.hint-fade-enter-active,
.hint-fade-leave-active {
  transition: opacity 0.25s;
}
.hint-fade-enter-from,
.hint-fade-leave-to {
  opacity: 0;
}

.close-dialog-btn {
  padding: 8px 20px;
  background: #21262d;
  border: 1px solid #30363d;
  border-radius: 6px;
  color: #e6edf3;
  cursor: pointer;
  font-size: 12px;

  &:hover {
    background: #30363d;
  }
}

// === 时间子弹窗 ===
.time-subdialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10002;
}

.time-subdialog {
  width: 340px;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 10px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.subdialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid #30363d;
  font-size: 13px;
  font-weight: 600;
  color: #e6edf3;
}

.subdialog-body {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.subdialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 16px;
  border-top: 1px solid #30363d;
}

// === 星期选择器 ===
.repeat-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.day-selector {
  display: flex;
  gap: 6px;
}

.day-btn {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: #0d1117;
  border: 1px solid #30363d;
  color: #8b949e;
  cursor: pointer;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
  padding: 0;

  &:hover { border-color: #58a6ff; color: #e6edf3; }
  &.active { background: rgba(88, 166, 255, 0.15); border-color: #58a6ff; color: #58a6ff; }
}

.repeat-hint {
  font-size: 11px;
  color: #6e7681;
}

// === 子弹窗按钮 ===
.cancel-btn {
  padding: 6px 14px;
  background: #21262d;
  border: 1px solid #30363d;
  border-radius: 6px;
  color: #e6edf3;
  cursor: pointer;
  font-size: 12px;

  &:hover { background: #30363d; }
}

.confirm-btn {
  padding: 6px 14px;
  background: #238636;
  border: 1px solid rgba(240, 246, 252, 0.1);
  border-radius: 6px;
  color: #fff;
  cursor: pointer;
  font-size: 12px;
  font-weight: 600;

  &:hover { background: #2ea043; }
}
</style>
