<script setup lang="ts">
import { CopyDocument, Delete, EditPen } from '@element-plus/icons-vue'
import {
  eventLabel,
  eventTagType,
  formatTimestamp,
} from '../../utils/format'
import type { SessionEvent } from '../../api'

defineProps<{
  event: SessionEvent | null
  detailJson: string
  detailFullText: string
}>()

const visible = defineModel<boolean>('visible', { required: true })

const emit = defineEmits<{
  (e: 'copy', text?: string): void
  (e: 'edit-event', event: SessionEvent): void
  (e: 'delete-event', event: SessionEvent): void
}>()
</script>

<template>
  <el-drawer
    v-model="visible"
    :title="event ? `#${event.index} ${eventLabel(event)}` : ''"
    size="50%"
    direction="rtl"
  >
    <template v-if="event">
      <div class="drawer-meta">
        <el-tag :type="eventTagType(event)" size="small" effect="plain">
          {{ eventLabel(event) }}
        </el-tag>
        <span v-if="event.role" class="muted">{{ event.role }}</span>
        <span class="muted">#{{ event.index }}</span>
        <span class="muted">{{ formatTimestamp(event.timestamp) }}</span>
      </div>

      <h4>内容</h4>
      <div class="drawer-section">
        <pre class="drawer-text">{{ detailFullText }}</pre>
      </div>

      <div class="drawer-toolbar">
        <el-button size="small" :icon="CopyDocument" @click="emit('copy', detailFullText)">
          复制文本
        </el-button>
        <el-button size="small" :icon="CopyDocument" @click="emit('copy', detailJson)">
          复制 JSON
        </el-button>
        <el-button size="small" :icon="EditPen" @click="visible = false; emit('edit-event', event)">
          编辑
        </el-button>
        <el-button size="small" type="danger" :icon="Delete" @click="visible = false; emit('delete-event', event)">
          删除
        </el-button>
      </div>

      <h4>JSON</h4>
      <div class="drawer-section">
        <pre class="drawer-json">{{ detailJson }}</pre>
      </div>
    </template>
  </el-drawer>
</template>

<style scoped>
.drawer-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
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
</style>
