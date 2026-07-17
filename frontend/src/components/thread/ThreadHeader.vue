<script setup lang="ts">
import { Delete, EditPen, FolderOpened } from '@element-plus/icons-vue'
import {
  compactTitle,
  formatCount,
  formatTimestamp,
  fullTitle,
  runtimeLabel,
} from '../../utils/format'
import type { ThreadSummary } from '../../api'

defineProps<{
  selectedThread: ThreadSummary
  backingUp: boolean
}>()

const emit = defineEmits<{
  (e: 'edit'): void
  (e: 'backup'): void
  (e: 'delete'): void
}>()
</script>

<template>
  <header class="detail-header">
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
      <el-button @click="emit('edit')" :icon="EditPen">编辑</el-button>
      <el-button :loading="backingUp" @click="emit('backup')">备份</el-button>
      <el-button type="danger" :icon="Delete" @click="emit('delete')">删除</el-button>
    </div>
  </header>
</template>

<style scoped>
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

.actions :deep(.el-button + .el-button) {
  margin-left: 0;
}
</style>
