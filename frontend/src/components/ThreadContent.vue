<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import {
  CopyDocument,
  Delete,
  EditPen,
  FolderOpened,
  Search,
  View,
} from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import VirtualList from '../VirtualList.vue'
import { useThreadDetail } from '../composables/useThreadDetail'
import { useThreadEvents } from '../composables/useThreadEvents'
import { useSearch } from '../composables/useSearch'
import {
  fullTitle,
  compactTitle,
  summaryText,
  formatBytes,
  formatCount,
  eventTagType,
  eventTypeName,
  payloadTypeName,
  eventLabel,
  copyText,
  formatTimestamp,
  eventSummary,
  eventTypeOptions,
  payloadTypeOptions,
  runtimeLabel,
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
  loadingEvents: boolean
  eventFilters: {
    event_type: string
    payload_type: string
    role: string
    q: string
    limit: number
  }
  activeTab: string
  expandedEvents: Set<number>
  searchText: string
  searching: boolean
  searchResults: SearchHit[]
  backingUp: boolean
  clearingLogs: boolean
  saveEvent: (index: number, raw: unknown) => Promise<boolean>
  saveTitle: (title: string) => Promise<boolean>
  saveRuntime: (request: UpdateRuntimeRequest) => Promise<boolean>
}>()

const emit = defineEmits<{
  (e: 'backup'): void
  (e: 'archive', archived: boolean): void
  (e: 'delete'): void
  (e: 'clear-logs'): void
  (e: 'load-events', reset: boolean): void
  (e: 'load-all-events'): void
  (e: 'event-saved'): void
  (e: 'history-saved', item: HistoryEntry, text: string): void
  (e: 'delete-history', item: HistoryEntry): void
  (e: 'delete-event', event: SessionEvent): void
  (e: 'toggle-event', event: SessionEvent): void
  (e: 'search'): void
  (e: 'search-hit', hit: SearchHit): void
  (e: 'update:searchText', value: string): void
  (e: 'update:activeTab', value: string): void
}>()

function getEventHeight() {
  return 48
}

function isEventExpanded(event: SessionEvent) {
  return props.expandedEvents?.has(event.index) ?? false
}

function eventText(event: SessionEvent) {
  const text = event.display_text ?? ''
  if (isEventExpanded(event) || text.length <= 900) return text
  return `${text.slice(0, 900)}...`
}

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
const savingRuntime = ref(false)
const historyEditVisible = ref(false)
const editingHistory = ref<HistoryEntry | null>(null)
const historyText = ref('')

watch(
  () => props.selectedThread,
  (thread) => {
    titleText.value = thread?.title || ''
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
    if (runtimeOk) {
      threadEditVisible.value = false
    }
  } finally {
    savingRuntime.value = false
  }
}

function openThreadEdit() {
  titleText.value = props.selectedThread?.title || ''
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
    <header class="detail-header" v-if="selectedThread">
      <div class="title-block">
        <div class="title-line">
          <h2 :title="fullTitle(selectedThread)">{{ compactTitle(selectedThread, 120) }}</h2>
          <el-tag :type="selectedThread.archived ? 'warning' : 'success'" effect="dark">
            {{ selectedThread.archived ? '已归档' : '未归档' }}
          </el-tag>
        </div>
        <div class="detail-meta">
          <span><el-icon><FolderOpened /></el-icon>{{ selectedThread.cwd }}</span>
          <span class="mono">{{ selectedThread.id }}</span>
          <span>{{ selectedThread.model_provider }} / {{ selectedThread.model || '-' }}</span>
          <span v-if="selectedThread.reasoning_effort">reasoning: {{ selectedThread.reasoning_effort }}</span>
          <span>{{ formatCount(selectedThread.tokens_used) }} tokens</span>
          <span v-if="selectedThread.cli_version">CLI {{ selectedThread.cli_version }}</span>
          <span>最近活跃 {{ formatTimestamp(selectedThread.recency_at_text) }}</span>
        </div>
        <div class="detail-tags">
          <el-tag size="small" effect="plain" type="warning">
            sandbox: {{ runtimeLabel(selectedThread.sandbox_type) }}
          </el-tag>
          <el-tag size="small" effect="plain" type="info">
            approval: {{ runtimeLabel(selectedThread.approval_mode) }}
          </el-tag>
          <el-tag size="small" effect="plain" type="success">
            memory: {{ runtimeLabel(selectedThread.memory_mode) }}
          </el-tag>
          <el-tag v-if="selectedThread.thread_source" size="small" effect="plain">
            source: {{ runtimeLabel(selectedThread.thread_source) }}
          </el-tag>
        </div>
      </div>
      <div class="actions">
        <el-button @click="openThreadEdit" :icon="EditPen">编辑</el-button>
        <el-button :loading="backingUp" @click="emit('backup')">备份</el-button>
        <el-button
          :icon="View"
          @click="emit('archive', !selectedThread.archived)"
        >
          {{ selectedThread.archived ? '取消归档' : '归档' }}
        </el-button>
        <el-button type="danger" :icon="Delete" @click="emit('delete')">删除</el-button>
      </div>
    </header>

    <section class="search-panel">
      <el-input
        :model-value="searchText"
        @update:model-value="(val: string) => emit('update:searchText', val)"
        :prefix-icon="Search"
        clearable
        placeholder="全局搜索消息、历史、标题"
        @keyup.enter="emit('search')"
      />
      <el-button type="primary" :loading="searching" @click="emit('search')">搜索</el-button>
    </section>

    <section v-if="searchResults.length" class="search-results">
      <button
        v-for="hit in searchResults"
        :key="`${hit.thread_id}-${hit.source}-${hit.field}-${hit.timestamp}`"
        class="hit"
        @click="emit('search-hit', hit)"
      >
        <strong>{{ hit.title || hit.thread_id }}</strong>
        <span class="muted">{{ hit.source }} · {{ hit.field }} · {{ hit.timestamp || '-' }}</span>
        <span class="prewrap">{{ hit.snippet }}</span>
      </button>
    </section>

    <el-empty v-if="!selectedThread" description="没有选中的会话" />

    <template v-else>
      <el-tabs :model-value="activeTab" @update:model-value="(val: string) => emit('update:activeTab', val)" class="detail-tabs">
        <el-tab-pane label="时间线" name="timeline">
          <section class="event-toolbar">
            <el-select v-model="eventFilters.event_type" clearable placeholder="事件类型">
              <el-option
                v-for="type in eventTypeOptions"
                :key="type"
                :label="eventTypeName(type)"
                :value="type"
              />
            </el-select>
            <el-select v-model="eventFilters.payload_type" clearable filterable placeholder="载荷类型">
              <el-option
                v-for="type in payloadTypeOptions"
                :key="type"
                :label="payloadTypeName(type)"
                :value="type"
              />
            </el-select>
            <el-select v-model="eventFilters.role" clearable placeholder="角色">
              <el-option label="user" value="user" />
              <el-option label="assistant" value="assistant" />
              <el-option label="developer" value="developer" />
            </el-select>
            <el-input
              v-model="eventFilters.q"
              :prefix-icon="Search"
              clearable
              placeholder="在当前会话内筛选"
              @keyup.enter="emit('load-events', true)"
            />
            <el-button :icon="Search" @click="emit('load-events', true)">应用</el-button>
          </section>

          <VirtualList
            v-if="events.length > 0"
            :items="events"
            :item-height="getEventHeight()"
            class="timeline-virtual"
            v-slot="{ item: event }"
          >
            <div class="timeline-item-wrapper">
              <div class="event-item" @click="openDetail(event)">
                <div class="event-head">
                  <el-tag :type="eventTagType(event)" size="small" effect="plain">
                    {{ eventLabel(event) }}
                  </el-tag>
                  <span v-if="event.role" class="muted">{{ event.role }}</span>
                  <span class="muted">#{{ event.index }}</span>
                  <span class="event-summary">{{ eventSummary(event) }}</span>
                  <span class="event-time muted">{{ formatTimestamp(event.timestamp) }}</span>
                  <div class="event-actions" @click.stop>
                    <el-button size="small" text :icon="CopyDocument" @click="copyWithToast(event.display_text)">
                      复制文本
                    </el-button>
                    <el-button size="small" text :icon="CopyDocument" @click="copyWithToast(JSON.stringify(event.raw, null, 2))">
                      复制 JSON
                    </el-button>
                    <el-button size="small" text :icon="EditPen" @click.stop="openEdit(event)">
                      编辑
                    </el-button>
                    <el-button size="small" text type="danger" :icon="Delete" @click.stop="emit('delete-event', event)">
                      删除
                    </el-button>
                  </div>
                </div>
              </div>
            </div>
          </VirtualList>

          <div class="load-more">
            <span class="muted">{{ events.length }} / {{ eventsTotal }}</span>
            <el-button
              :disabled="events.length >= eventsTotal"
              :loading="loadingEvents"
              @click="emit('load-events', false)"
            >
              加载更多
            </el-button>
            <el-button
              :disabled="events.length >= eventsTotal"
              :loading="loadingEvents"
              @click="emit('load-all-events')"
            >
              加载全部
            </el-button>
          </div>
        </el-tab-pane>

        <el-tab-pane label="历史" name="history">
          <el-table :data="detail?.history ?? []" height="100%" virtual-scroll>
            <el-table-column prop="ts_text" label="时间" width="220" />
            <el-table-column label="内容">
              <template #default="{ row }">
                <span class="prewrap">{{ row.text }}</span>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="150" align="right">
              <template #default="{ row }">
                <el-button size="small" text :icon="EditPen" @click="openHistoryEdit(row)">
                  编辑
                </el-button>
                <el-button size="small" text type="danger" :icon="Delete" @click="emit('delete-history', row)">
                  删除
                </el-button>
              </template>
            </el-table-column>
          </el-table>
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

    <el-drawer
      v-model="threadEditVisible"
      title="编辑会话"
      size="420px"
      direction="rtl"
    >
      <el-form label-width="92px" class="thread-edit-form">
        <el-form-item label="标题">
          <el-input
            v-model="titleText"
            clearable
            placeholder="留空表示恢复原始标题"
          />
        </el-form-item>
        <el-form-item label="模型来源">
          <el-input v-model="runtimeForm.model_provider" placeholder="model_provider" />
        </el-form-item>
        <el-form-item label="沙箱">
          <el-select v-model="runtimeForm.sandbox_type">
            <el-option label="无沙箱" value="disabled" />
            <el-option label="只读" value="read-only" />
            <el-option label="工作区写入" value="workspace-write" />
            <el-option label="完全访问" value="danger-full-access" />
          </el-select>
        </el-form-item>
        <el-form-item label="确认模式">
          <el-select v-model="runtimeForm.approval_mode">
            <el-option label="不信任" value="untrusted" />
            <el-option label="失败时确认" value="on-failure" />
            <el-option label="按需确认" value="on-request" />
            <el-option label="不确认" value="never" />
          </el-select>
        </el-form-item>
        <el-form-item label="记忆">
          <el-select v-model="runtimeForm.memory_mode">
            <el-option label="启用" value="enabled" />
            <el-option label="禁用" value="disabled" />
          </el-select>
        </el-form-item>
        <el-form-item label="线程来源">
          <el-select v-model="runtimeForm.thread_source" clearable>
            <el-option label="用户" value="user" />
            <el-option label="CLI" value="cli" />
            <el-option label="自动" value="auto" />
          </el-select>
        </el-form-item>
        <el-form-item label="推理强度">
          <el-select v-model="runtimeForm.reasoning_effort" clearable>
            <el-option label="low" value="low" />
            <el-option label="medium" value="medium" />
            <el-option label="high" value="high" />
            <el-option label="xhigh" value="xhigh" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="threadEditVisible = false">取消</el-button>
        <el-button type="primary" :loading="savingRuntime" @click="handleSaveRuntime">
          保存
        </el-button>
      </template>
    </el-drawer>

    <el-drawer
      v-model="detailVisible"
      :title="detailEvent ? `#${detailEvent.index} ${eventLabel(detailEvent)}` : ''"
      size="50%"
      direction="rtl"
    >
      <template v-if="detailEvent">
        <div class="drawer-meta">
          <el-tag :type="eventTagType(detailEvent)" size="small" effect="plain">
            {{ eventLabel(detailEvent) }}
          </el-tag>
          <span v-if="detailEvent.role" class="muted">{{ detailEvent.role }}</span>
          <span class="muted">#{{ detailEvent.index }}</span>
          <span class="muted">{{ formatTimestamp(detailEvent.timestamp) }}</span>
        </div>

        <h4>内容</h4>
        <div class="drawer-section">
          <pre class="drawer-text">{{ detailFullText }}</pre>
        </div>

        <div class="drawer-toolbar">
          <el-button size="small" :icon="CopyDocument" @click="copyWithToast(detailFullText)">
            复制文本
          </el-button>
          <el-button size="small" :icon="CopyDocument" @click="copyWithToast(detailJson)">
            复制 JSON
          </el-button>
          <el-button size="small" :icon="EditPen" @click="detailVisible = false; openEdit(detailEvent!)">
            编辑
          </el-button>
          <el-button size="small" type="danger" :icon="Delete" @click="detailVisible = false; emit('delete-event', detailEvent)">
            删除
          </el-button>
        </div>

        <h4>JSON</h4>
        <div class="drawer-section">
          <pre class="drawer-json">{{ detailJson }}</pre>
        </div>
      </template>
    </el-drawer>

    <el-drawer
      v-model="historyEditVisible"
      title="编辑历史"
      size="520px"
      direction="rtl"
    >
      <el-input
        v-model="historyText"
        type="textarea"
        :autosize="{ minRows: 10, maxRows: 24 }"
      />
      <template #footer>
        <el-button @click="historyEditVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSaveHistory">保存</el-button>
      </template>
    </el-drawer>

    <el-dialog
      v-model="editVisible"
      :title="editEvent ? `编辑节点 #${editEvent.index}` : ''"
      width="720px"
      top="6vh"
      destroy-on-close
    >
      <div v-if="editEvent" class="edit-dialog">
        <div class="edit-dialog-meta">
          <el-tag :type="eventTagType(editEvent)" size="small" effect="plain">
            {{ eventLabel(editEvent) }}
          </el-tag>
          <span v-if="editEvent.role" class="muted">{{ editEvent.role }}</span>
          <span class="muted">{{ formatTimestamp(editEvent.timestamp) }}</span>
        </div>
        <el-input
          v-model="editText"
          type="textarea"
          :autosize="{ minRows: 14, maxRows: 30 }"
          placeholder="JSON 内容"
          class="edit-textarea"
          @input="validateEditJson"
        />
        <p v-if="editError" class="edit-error">{{ editError }}</p>
      </div>
      <template #footer>
        <el-button @click="editVisible = false">取消</el-button>
        <el-button type="primary" :loading="savingEdit" :disabled="!!editError" @click="handleSaveEdit">
          保存
        </el-button>
      </template>
    </el-dialog>
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

.detail-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 12px;
  margin-bottom: 16px;
  padding-bottom: 16px;
  border-bottom: 1px solid #e2e8f0;
}

.title-block {
  display: grid;
  gap: 10px;
  min-width: 0;
  flex: 1 1 420px;
}

.title-line {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.title-line h2 {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #2d3748;
  margin: 0;
  font-size: 20px;
  line-height: 1.35;
  font-weight: 600;
}

.detail-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 10px 16px;
  color: #697386;
  font-size: 13px;
}

.detail-meta span {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  min-width: 0;
}

.detail-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.actions {
  display: flex;
  gap: 8px;
  flex: 0 0 auto;
  flex-wrap: wrap;
  justify-content: flex-end;
  max-width: min(620px, 100%);
}

.thread-edit-form :deep(.el-select) {
  width: 100%;
}

.actions :deep(.el-button + .el-button) {
  margin-left: 0;
}

.search-panel {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.search-results {
  display: grid;
  gap: 8px;
  max-height: 220px;
  overflow: auto;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  background: #fafbfc;
  padding: 10px;
  margin-bottom: 12px;
}

.hit {
  display: grid;
  gap: 4px;
  text-align: left;
  border: 0;
  border-radius: 8px;
  background: transparent;
  padding: 10px 12px;
  color: inherit;
  cursor: pointer;
  transition: all 0.2s ease;
}

.hit:hover {
  background: #eef2ff;
  transform: translateX(4px);
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

.event-toolbar {
  display: grid;
  grid-template-columns: 140px 190px 120px minmax(220px, 1fr) auto;
  margin: 10px 0 18px;
}

.timeline-virtual {
  height: calc(100% - 140px);
  min-height: 200px;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  position: relative;
}

.timeline-item-wrapper {
  padding: 4px 8px;
  height: 100%;
  box-sizing: border-box;
  overflow: hidden;
}

.event-item {
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  background: #fafbfc;
  padding: 4px 10px;
  height: 100%;
  box-sizing: border-box;
  overflow: hidden;
  cursor: pointer;
  transition: all 0.15s ease;
}

.event-item:hover {
  border-color: #667eea;
  box-shadow: 0 2px 8px rgba(102, 126, 234, 0.1);
}

.event-head {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.event-time {
  font-size: 12px;
  margin-left: auto;
  white-space: nowrap;
}

.event-summary {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
  color: #4a5568;
}

.event-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
}

.event-actions :deep(.el-button + .el-button) {
  margin-left: 0;
}

.raw-json {
  max-height: 360px;
  overflow: auto;
  margin: 0;
  font-size: 12px;
  background: #f7f8fa;
  padding: 10px;
  border-radius: 6px;
}

/* drawer styles */
.drawer-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
}

.drawer-meta h4 {
  margin: 0 0 10px;
}

.drawer-section {
  margin-bottom: 16px;
}

.drawer-text {
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 40vh;
  overflow: auto;
  background: #f7f8fa;
  padding: 12px;
  border-radius: 6px;
  font-size: 13px;
  line-height: 1.6;
  margin: 0;
}

.drawer-json {
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 50vh;
  overflow: auto;
  background: #f7f8fa;
  padding: 12px;
  border-radius: 6px;
  font-size: 12px;
  margin: 0;
}

.drawer-toolbar {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}

.load-more {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin-top: 14px;
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

/* edit dialog */
.edit-dialog-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}

.edit-textarea :deep(textarea) {
  font-family: 'Cascadia Code', 'Fira Code', 'JetBrains Mono', Consolas, monospace;
  font-size: 13px;
  line-height: 1.5;
  tab-size: 2;
}

.edit-error {
  color: #f56c6c;
  font-size: 13px;
  margin: 8px 0 0;
}
</style>
