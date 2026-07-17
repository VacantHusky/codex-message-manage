<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import VirtualList from '../VirtualList.vue'
import ThreadHeader from './thread/ThreadHeader.vue'
import ThreadSearch from './thread/ThreadSearch.vue'
import ThreadEditDrawer from './thread/ThreadEditDrawer.vue'
import EventDetailDrawer from './thread/EventDetailDrawer.vue'
import HistoryEditDrawer from './thread/HistoryEditDrawer.vue'
import EventEditDialog from './thread/EventEditDialog.vue'
import TimelineTab from './thread/TimelineTab.vue'
import HistoryTab from './thread/HistoryTab.vue'
import {
  summaryText,
  formatBytes,
  copyText,
} from '../utils/format'
import type { ThreadSummary, SearchHit, SessionEvent } from '../api'
import type { HistoryEntry } from '../api'
import type { UpdateRuntimeRequest } from '../api'

const props = defineProps<{
  loadingDetail: boolean
  detail: any
  selectedThread?: ThreadSummary
  events: SessionEvent[]
  eventsTotal: number
  eventPage: number
  loadingEvents: boolean
  eventFilters: {
    event_type: string
    payload_type: string
    role: string
    q: string
    limit: number
  }
  historyItems: HistoryEntry[]
  historyTotal: number
  historyPage: number
  loadingHistory: boolean
  historyFilters: {
    limit: number
  }
  activeTab: string
  searchText: string
  searching: boolean
  searchResults: SearchHit[]
  backingUp: boolean
  clearingLogs: boolean
  saveEvent: (index: number, raw: unknown) => Promise<boolean>
  saveTitle: (title: string) => Promise<boolean>
  saveRuntime: (request: UpdateRuntimeRequest) => Promise<boolean>
  saveArchive: (archived: boolean) => Promise<boolean>
}>()

const emit = defineEmits<{
  (e: 'backup'): void
  (e: 'delete'): void
  (e: 'clear-logs'): void
  (e: 'load-events', reset: boolean): void
  (e: 'change-event-page', page: number): void
  (e: 'load-history', reset: boolean): void
  (e: 'change-history-page', page: number): void
  (e: 'event-saved'): void
  (e: 'history-saved', item: HistoryEntry, text: string): void
  (e: 'delete-history', item: HistoryEntry): void
  (e: 'delete-event', event: SessionEvent): void
  (e: 'search'): void
  (e: 'search-hit', hit: SearchHit): void
  (e: 'update:searchText', value: string): void
  (e: 'update:activeTab', value: string): void
}>()

// event detail drawer
const detailVisible = ref(false)
const detailEvent = ref<SessionEvent | null>(null)

function openDetail(event: SessionEvent) {
  detailEvent.value = event
  detailVisible.value = true
}

const detailJson = computed(() => {
  if (!detailEvent.value) return ''
  return JSON.stringify(detailEvent.value.raw, null, 2)
})

const detailFullText = computed(() => {
  return detailEvent.value?.display_text ?? '(无内容)'
})

const runtimeForm = reactive<UpdateRuntimeRequest>({
  model_provider: 'openai',
  sandbox_type: 'disabled',
  approval_mode: 'on-request',
  memory_mode: 'enabled',
  thread_source: 'user',
  reasoning_effort: '',
})
const threadEditVisible = ref(false)
const titleText = ref('')
const archivedValue = ref(false)
const savingRuntime = ref(false)
const historyEditVisible = ref(false)
const editingHistory = ref<HistoryEntry | null>(null)
const historyText = ref('')

watch(
  () => props.selectedThread,
  (thread) => {
    titleText.value = thread?.title || ''
    archivedValue.value = thread?.archived ?? false
    runtimeForm.model_provider = thread?.model_provider || 'openai'
    runtimeForm.sandbox_type = thread?.sandbox_type || 'disabled'
    runtimeForm.approval_mode = thread?.approval_mode || 'on-request'
    runtimeForm.memory_mode = thread?.memory_mode || 'enabled'
    runtimeForm.thread_source = thread?.thread_source || 'user'
    runtimeForm.reasoning_effort = thread?.reasoning_effort || ''
  },
  { immediate: true },
)

async function handleSaveRuntime() {
  savingRuntime.value = true
  try {
    const titleOk = await props.saveTitle(titleText.value)
    if (!titleOk) return
    const runtimeOk = await props.saveRuntime({ ...runtimeForm })
    if (!runtimeOk) return
    const archiveOk = archivedValue.value === props.selectedThread?.archived
      || await props.saveArchive(archivedValue.value)
    if (archiveOk) threadEditVisible.value = false
  } finally {
    savingRuntime.value = false
  }
}

function openThreadEdit() {
  titleText.value = props.selectedThread?.title || ''
  archivedValue.value = props.selectedThread?.archived ?? false
  runtimeForm.model_provider = props.selectedThread?.model_provider || 'openai'
  runtimeForm.sandbox_type = props.selectedThread?.sandbox_type || 'disabled'
  runtimeForm.approval_mode = props.selectedThread?.approval_mode || 'on-request'
  runtimeForm.memory_mode = props.selectedThread?.memory_mode || 'enabled'
  runtimeForm.thread_source = props.selectedThread?.thread_source || 'user'
  runtimeForm.reasoning_effort = props.selectedThread?.reasoning_effort || ''
  threadEditVisible.value = true
}

function openHistoryEdit(item: HistoryEntry) {
  editingHistory.value = item
  historyText.value = item.text
  historyEditVisible.value = true
}

function handleSaveHistory() {
  if (!editingHistory.value) return
  emit('history-saved', editingHistory.value, historyText.value)
  historyEditVisible.value = false
}

async function copyWithToast(text?: string) {
  if (!text) {
    ElMessage.warning('没有可复制的内容')
    return
  }
  if (await copyText(text)) {
    ElMessage.success('已复制到剪贴板')
  }
}

// edit dialog
const editVisible = ref(false)
const editEvent = ref<SessionEvent | null>(null)
const editText = ref('')
const editError = ref('')
const savingEdit = ref(false)

function openEdit(event: SessionEvent) {
  editEvent.value = event
  editText.value = JSON.stringify(event.raw, null, 2)
  editError.value = ''
  editVisible.value = true
}

function validateEditJson() {
  if (!editText.value.trim()) {
    editError.value = 'JSON 不能为空'
    return false
  }
  try {
    JSON.parse(editText.value)
    editError.value = ''
    return true
  } catch (e) {
    editError.value = e instanceof Error ? e.message : 'JSON 格式不正确'
    return false
  }
}

async function handleSaveEdit() {
  if (!editEvent.value || !validateEditJson()) return
  savingEdit.value = true
  try {
    const raw = JSON.parse(editText.value)
    const ok = await props.saveEvent(editEvent.value.index, raw)
    if (ok) {
      editVisible.value = false
      emit('event-saved')
    }
  } finally {
    savingEdit.value = false
  }
}
</script>

<template>
  <section class="content" v-loading="loadingDetail">
    <ThreadHeader
      v-if="selectedThread"
      :selected-thread="selectedThread"
      :backing-up="backingUp"
      @edit="openThreadEdit"
      @backup="emit('backup')"
      @delete="emit('delete')"
    />

    <ThreadSearch
      :selected-thread="selectedThread"
      :search-text="searchText"
      :searching="searching"
      :search-results="searchResults"
      @search="emit('search')"
      @search-hit="(hit) => emit('search-hit', hit)"
      @update:search-text="(value) => emit('update:searchText', value)"
    />

    <el-empty v-if="!selectedThread" description="没有选中的会话" />

    <template v-else>
      <el-tabs :model-value="activeTab" @update:model-value="(val: string) => emit('update:activeTab', val)" class="detail-tabs">
        <el-tab-pane label="时间线" name="timeline" class="timeline-pane">
          <TimelineTab
            :events="events"
            :events-total="eventsTotal"
            :event-page="eventPage"
            :event-filters="eventFilters"
            @load-events="(reset) => emit('load-events', reset)"
            @change-event-page="(page) => emit('change-event-page', page)"
            @open-detail="openDetail"
            @copy="copyWithToast"
            @edit-event="openEdit"
            @delete-event="(event) => emit('delete-event', event)"
          />
        </el-tab-pane>

        <el-tab-pane label="历史" name="history" class="history-pane">
          <HistoryTab
            :history-items="historyItems"
            :history-total="historyTotal"
            :history-page="historyPage"
            :loading-history="loadingHistory"
            :history-filters="historyFilters"
            @load-history="(reset) => emit('load-history', reset)"
            @change-history-page="(page) => emit('change-history-page', page)"
            @edit-history="openHistoryEdit"
            @delete-history="(item) => emit('delete-history', item)"
          />
        </el-tab-pane>

        <el-tab-pane label="日志" name="logs">
          <section class="tab-toolbar">
            <span class="muted">最近 {{ detail?.logs.length ?? 0 }} 条日志</span>
            <el-button
              type="danger"
              plain
              :disabled="!(detail?.logs.length)"
              :loading="clearingLogs"
              @click="emit('clear-logs')"
            >
              清理日志
            </el-button>
          </section>
          <VirtualList
            v-if="detail?.logs?.length"
            :items="detail.logs"
            :item-height="60"
            class="logs-virtual"
            v-slot="{ item: log }: { item: any }"
          >
            <div class="log-item">
              <span class="log-time mono">{{ log.ts_text }}</span>
              <el-tag :type="log.level === 'ERROR' ? 'danger' : log.level === 'WARN' ? 'warning' : 'info'" size="small">
                {{ log.level }}
              </el-tag>
              <span class="log-target">{{ log.target }}</span>
              <span class="log-body prewrap">{{ log.body }}</span>
            </div>
          </VirtualList>
          <el-empty v-else description="暂无日志" />
        </el-tab-pane>

        <el-tab-pane label="信息" name="info">
          <el-descriptions :column="2" border>
            <el-descriptions-item label="rollout" :span="2">
              <span class="mono info-path">{{ detail?.thread.resolved_rollout_path || detail?.thread.rollout_path }}</span>
            </el-descriptions-item>
            <el-descriptions-item label="文件大小">
              {{ formatBytes(detail?.file?.bytes) }}
            </el-descriptions-item>
            <el-descriptions-item label="创建时间">
              {{ detail?.thread.created_at_text }}
            </el-descriptions-item>
            <el-descriptions-item label="更新时间">
              {{ detail?.thread.updated_at_text }}
            </el-descriptions-item>
            <el-descriptions-item label="最近活跃">
              {{ detail?.thread.recency_at_text }}
            </el-descriptions-item>
            <el-descriptions-item label="模型">
              {{ detail?.thread.model_provider }} / {{ detail?.thread.model || '-' }}
            </el-descriptions-item>
            <el-descriptions-item label="CLI">
              {{ detail?.thread.cli_version || '-' }}
            </el-descriptions-item>
            <el-descriptions-item label="原始沙箱" :span="2">
              <span class="mono info-path">{{ detail?.thread.sandbox_policy || '-' }}</span>
            </el-descriptions-item>
            <el-descriptions-item label="目标" :span="2">
              <span class="prewrap">{{ detail?.goal?.objective || '-' }}</span>
            </el-descriptions-item>
          </el-descriptions>
        </el-tab-pane>
      </el-tabs>
    </template>

    <ThreadEditDrawer
      v-model:visible="threadEditVisible"
      :title-text="titleText"
      :archived="archivedValue"
      :runtime-form="runtimeForm"
      :saving-runtime="savingRuntime"
      @update:title-text="(value) => titleText = value"
      @update:archived="(value) => archivedValue = value"
      @save="handleSaveRuntime"
    />

    <EventDetailDrawer
      v-model:visible="detailVisible"
      :event="detailEvent"
      :detail-json="detailJson"
      :detail-full-text="detailFullText"
      @copy="copyWithToast"
      @edit-event="openEdit"
      @delete-event="(event) => emit('delete-event', event)"
    />

    <HistoryEditDrawer
      v-model:visible="historyEditVisible"
      v-model:history-text="historyText"
      @save="handleSaveHistory"
    />

    <EventEditDialog
      v-model:visible="editVisible"
      :event="editEvent"
      :edit-text="editText"
      :edit-error="editError"
      :saving-edit="savingEdit"
      @update:edit-text="(value) => editText = value"
      @validate="validateEditJson"
      @save="handleSaveEdit"
    />
  </section>
</template>

<style scoped>
.content {
  display: flex;
  flex-direction: column;
  height: 100vh;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  padding: 16px;
  background: #ffffff;
  border-radius: 0;
}

.detail-tabs {
  flex: 1 1 auto;
  min-height: 0;
  overflow: hidden;
  padding: 0;
}

.detail-tabs :deep(.el-tabs__content) {
  height: calc(100% - 55px);
  overflow: hidden;
}

.detail-tabs :deep(.el-tab-pane) {
  height: 100%;
  min-height: 0;
}

.timeline-pane {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.history-pane {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.tab-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin: 10px 0 12px;
}

.logs-virtual {
  height: calc(100% - 52px);
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  position: relative;
  padding-bottom: 1px;
}

.log-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-bottom: 1px solid #f0f0f0;
  height: 100%;
  box-sizing: border-box;
  overflow: hidden;
}

.log-time {
  font-size: 12px;
  color: #697386;
  min-width: 180px;
}

.log-target {
  font-size: 12px;
  color: #4a5568;
  min-width: 120px;
  font-weight: 500;
}

.log-body {
  font-size: 12px;
  color: #2d3748;
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-height: 40px;
}

.info-path {
  overflow-wrap: anywhere;
}

</style>
