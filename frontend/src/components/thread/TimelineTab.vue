<script setup lang="ts">
import { CopyDocument, Delete, EditPen, Search } from '@element-plus/icons-vue'
import VirtualList from '../../VirtualList.vue'
import {
  eventLabel,
  eventSummary,
  eventTagType,
  eventTypeName,
  eventTypeOptions,
  formatTimestamp,
  payloadTypeName,
  payloadTypeOptions,
} from '../../utils/format'
import type { SessionEvent } from '../../api'

defineProps<{
  events: SessionEvent[]
  eventsTotal: number
  eventPage: number
  eventFilters: {
    event_type: string
    payload_type: string
    role: string
    q: string
    limit: number
  }
}>()

const emit = defineEmits<{
  (e: 'load-events', reset: boolean): void
  (e: 'change-event-page', page: number): void
  (e: 'open-detail', event: SessionEvent): void
  (e: 'copy', text?: string): void
  (e: 'edit-event', event: SessionEvent): void
  (e: 'delete-event', event: SessionEvent): void
}>()

function getEventHeight() {
  return 48
}
</script>

<template>
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
      <div class="event-item" @click="emit('open-detail', event)">
        <div class="event-head">
          <el-tag :type="eventTagType(event)" size="small" effect="plain">
            {{ eventLabel(event) }}
          </el-tag>
          <span v-if="event.role" class="muted">{{ event.role }}</span>
          <span class="muted">#{{ event.index }}</span>
          <span class="event-summary">{{ eventSummary(event) }}</span>
          <span class="event-time muted">{{ formatTimestamp(event.timestamp) }}</span>
          <div class="event-actions" @click.stop>
            <el-button size="small" text :icon="CopyDocument" @click="emit('copy', event.display_text)">
              复制文本
            </el-button>
            <el-button size="small" text :icon="CopyDocument" @click="emit('copy', JSON.stringify(event.raw, null, 2))">
              复制 JSON
            </el-button>
            <el-button size="small" text :icon="EditPen" @click.stop="emit('edit-event', event)">
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
    <span class="muted">共 {{ eventsTotal }} 条</span>
    <el-pagination
      background
      layout="prev, pager, next, sizes"
      :current-page="eventPage"
      :page-size="eventFilters.limit"
      :page-sizes="[10, 12, 15, 20, 50, 100]"
      :total="eventsTotal"
      @current-change="(page: number) => emit('change-event-page', page)"
      @size-change="(size: number) => { eventFilters.limit = size; emit('load-events', true) }"
    />
  </div>
</template>

<style scoped>
.event-toolbar {
  display: grid;
  grid-template-columns: 140px 190px 120px minmax(220px, 1fr) auto;
  flex: 0 0 auto;
  margin: 10px 0 12px;
}

.timeline-virtual {
  flex: 1 1 auto;
  height: auto;
  min-height: 0;
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

.load-more {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: 12px;
}
</style>
