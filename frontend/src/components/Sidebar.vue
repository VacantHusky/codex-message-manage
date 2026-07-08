<script setup lang="ts">
import { computed, watch } from 'vue'
import {
  FolderOpened,
  Refresh,
  Search,
  View,
} from '@element-plus/icons-vue'
import { useThreadList } from '../composables/useThreadList'
import {
  fullTitle,
  compactTitle,
  formatBytes,
  formatCompactCount,
  formatCount,
  formatTimestamp,
  runtimeLabel,
} from '../utils/format'
import type { Overview, ThreadSummary } from '../api'

const props = defineProps<{
  overview?: Overview
  loadingThreads: boolean
  threads: ThreadSummary[]
  total: number
  filters: {
    q: string
    cwd: string
    model: string
    date_range: string[]
    token_range: number[]
    page: number
    page_size: number
  }
  selectedId: string
}>()

const emit = defineEmits<{
  (e: 'select-thread', thread: ThreadSummary): void
  (e: 'reload'): void
  (e: 'open-stats'): void
  (e: 'open-backups'): void
  (e: 'open-change-data-dir'): void
  (e: 'load-threads'): void
}>()

function rowClassName({ row }: { row: ThreadSummary }) {
  return row.id === props.selectedId ? 'selected-row' : ''
}

function handleFilterChange() {
  props.filters.page = 1
  emit('load-threads')
}

const tokenMax = computed(() => props.overview?.max_tokens_used ?? 0)
const tokenSliderMax = computed(() => Math.max(tokenMax.value, 1000))
const tokenStep = computed(() => (tokenSliderMax.value >= 1_000_000 ? 10_000 : 1000))

watch(
  tokenMax,
  (max) => {
    if (max > 0 && props.filters.token_range.length !== 2) {
      props.filters.token_range = [0, max]
    }
  },
  { immediate: true },
)
</script>

<template>
  <aside class="sidebar">
    <section class="topbar">
      <div class="topbar-info">
        <h1>Codex 数据管理器</h1>
        <p class="muted mono truncate">{{ overview?.data_dir ?? 'loading...' }}</p>
      </div>
      <div class="top-actions">
        <el-button @click="emit('open-change-data-dir')" :icon="FolderOpened">切换目录</el-button>
        <el-button @click="emit('open-stats')" :icon="View">统计</el-button>
        <el-button @click="emit('open-backups')" :icon="FolderOpened">恢复</el-button>
        <el-button :icon="Refresh" circle @click="emit('reload')" />
      </div>
    </section>

    <section class="stats" v-if="overview">
      <div class="stat-card">
        <strong>{{ formatCount(overview.thread_count) }}</strong>
        <span>线程</span>
      </div>
      <div class="stat-card">
        <strong>{{ formatCount(overview.session_file_count) }}</strong>
        <span>文件</span>
      </div>
      <div class="stat-card">
        <strong>{{ formatBytes(overview.total_session_bytes) }}</strong>
        <span>会话体积</span>
      </div>
    </section>

    <section class="filters">
      <el-input
        v-model="filters.q"
        :prefix-icon="Search"
        clearable
        placeholder="筛选标题、项目、首条消息"
        @keyup.enter="handleFilterChange"
        @clear="handleFilterChange"
      />
      <div class="filter-row model-filter-row">
        <el-select v-model="filters.cwd" clearable filterable placeholder="项目目录">
          <el-option
            v-for="item in overview?.by_cwd ?? []"
            :key="item.name"
            :label="`${item.name} (${item.count})`"
            :value="item.name"
          />
        </el-select>
        <el-select v-model="filters.model" clearable placeholder="模型/来源">
          <el-option
            v-for="item in overview?.by_model_provider ?? []"
            :key="item.name"
            :label="`${item.name} (${item.count})`"
            :value="item.name"
          />
        </el-select>
      </div>
      <div class="filter-row filter-action-row">
        <el-date-picker
          v-model="filters.date_range"
          type="daterange"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          value-format="YYYY-MM-DD"
          clearable
        />
        <el-button type="primary" :icon="Search" @click="handleFilterChange">
          筛选
        </el-button>
      </div>
      <div class="token-filter">
        <div class="token-filter-head">
          <span>Tokens</span>
          <strong>{{ formatCompactCount(filters.token_range[0]) }} - {{ formatCompactCount(filters.token_range[1]) }}</strong>
        </div>
        <el-slider
          v-model="filters.token_range"
          range
          :min="0"
          :max="tokenSliderMax"
          :step="tokenStep"
          :format-tooltip="formatCompactCount"
        />
      </div>
    </section>

    <el-table
      class="thread-table"
      v-loading="loadingThreads"
      :data="threads"
      height="100%"
      highlight-current-row
      :row-class-name="rowClassName"
      @row-click="(row: ThreadSummary) => emit('select-thread', row)"
    >
      <el-table-column label="会话">
        <template #default="{ row }">
          <div class="thread-title truncate" :title="fullTitle(row)">
            {{ compactTitle(row) }}
          </div>
          <div class="thread-meta truncate">{{ row.cwd }}</div>
          <div class="thread-meta" :title="row.recency_at_text || row.updated_at_text">
            {{ formatTimestamp(row.recency_at_text || row.updated_at_text) }}
            <el-tag v-if="row.archived" size="small" type="warning">归档</el-tag>
          </div>
          <div class="thread-tags">
            <el-tag size="small" effect="plain">{{ row.model || row.model_provider }}</el-tag>
            <el-tag v-if="row.reasoning_effort" size="small" effect="plain" type="success">
              {{ row.reasoning_effort }}
            </el-tag>
            <el-tag v-if="row.cli_version" size="small" effect="plain" type="info">
              {{ row.cli_version }}
            </el-tag>
            <el-tag size="small" effect="plain" type="warning">
              {{ runtimeLabel(row.sandbox_type) }}
            </el-tag>
          </div>
        </template>
      </el-table-column>
      <el-table-column width="92" align="right">
        <template #default="{ row }">
          <span class="muted" :title="`${formatCount(row.tokens_used)} tokens`">
            {{ formatCompactCount(row.tokens_used) }}
          </span>
        </template>
      </el-table-column>
    </el-table>
    <el-pagination
      class="pager"
      layout="prev, pager, next"
      :page-size="filters.page_size"
      :total="total"
      v-model:current-page="filters.page"
      @current-change="emit('load-threads')"
    />
  </aside>
</template>

<style scoped>
.mono {
  margin: 0;
}


.sidebar {
  display: flex;
  flex-direction: column;
  border-right: 1px solid #e2e8f0;
  background: #ffffff;
  padding: 16px;
  min-height: 0;
  overflow: hidden;
  box-shadow: 2px 0 8px rgba(0, 0, 0, 0.04);
}

.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  flex: 0 0 auto;
  gap: 12px;
}

.topbar-info {
  min-width: 0;
  flex: 1 1 220px;
}

.topbar-info h1 {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  margin: 0;
  font-size: 22px;
  line-height: 1.3;
  font-weight: 700;
}

.top-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  flex-wrap: wrap;
  flex: 0 0 auto;
  gap: 0;
}

.stats {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  flex: 0 0 auto;
  gap: 12px;
  margin: 8px 0;
}

.stat-card {
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  padding: 10px;
  background: linear-gradient(135deg, #fafbfc 0%, #f4f6f8 100%);
  text-align: center;
  transition: all 0.2s ease;
}

.stat-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

.stat-card strong {
  display: block;
  font-size: 20px;
  font-weight: 700;
  color: #667eea;
}

.stat-card span {
  color: #697386;
  font-size: 12px;
  margin-top: 4px;
  display: block;
}

.filters {
  display: grid;
  flex: 0 0 auto;
  gap: 10px;
  margin-bottom: 12px;
}

.filter-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.model-filter-row {
  grid-template-columns: minmax(0, 0.85fr) minmax(0, 1.15fr);
}

.filter-action-row {
  grid-template-columns: 1fr auto;
}

.filter-row :deep(.el-date-editor),
.filter-row :deep(.el-input-number) {
  width: 100%;
}

.token-filter {
  min-width: 0;
}

.token-filter-head {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  color: #697386;
  font-size: 12px;
}

.token-filter-head strong {
  color: #2d3748;
  font-weight: 600;
}

.token-filter :deep(.el-slider) {
  --el-slider-main-bg-color: #667eea;
  margin: 0 8px;
}

.thread-title {
  font-weight: 600;
  color: #2d3748;
}

.thread-meta {
  color: #697386;
  font-size: 12px;
}

.thread-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 5px;
}

.thread-table {
  flex: 1 1 auto;
  min-height: 0;
}

.pager {
  flex: 0 0 auto;
  justify-content: center;
  margin-top: 12px;
}

:deep(.selected-row) {
  --el-table-tr-bg-color: #eef2ff;
}

:deep(.el-table) {
  --el-table-border-color: #e2e8f0;
}

:deep(.el-table th) {
  background-color: #f7f8fa !important;
  font-weight: 600;
}
</style>
