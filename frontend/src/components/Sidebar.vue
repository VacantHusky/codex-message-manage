<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import {
  CaretBottom,
  CaretRight,
  FolderOpened,
  InfoFilled,
  Management,
  Refresh,
  Search,
  Setting,
} from '@element-plus/icons-vue'
import {
  fullTitle,
  compactTitle,
  formatBytes,
  formatCompactCount,
  formatCount,
  formatTimestamp,
  runtimeLabel,
} from '../utils/format'
import { apiGet, type BackupInfo, type Overview, type ThreadPage, type ThreadSummary } from '../api'

type SidebarPanel = 'sessions' | 'search' | 'backup' | 'info' | 'settings'

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
  backups: BackupInfo[]
  restoring: string
  backupDeleting: string
}>()

const emit = defineEmits<{
  (e: 'select-thread', thread: ThreadSummary): void
  (e: 'reload'): void
  (e: 'open-stats'): void
  (e: 'open-backups'): void
  (e: 'load-backups'): void
  (e: 'restore-backup', item: BackupInfo): void
  (e: 'delete-backup', item: BackupInfo): void
  (e: 'open-change-data-dir'): void
  (e: 'load-threads'): void
}>()

const activePanel = ref<SidebarPanel>('sessions')
const filtersExpanded = ref(false)
const replaceExpanded = ref(false)
const searchText = ref('')
const replaceText = ref('')
const searchHasQuery = computed(() => searchText.value.trim().length > 0)
const searchThreads = ref<ThreadSummary[]>([])
const searchTotal = ref(0)
const searchPage = ref(1)
const searchPageSize = 30
const loadingSearch = ref(false)
const searchResultLabel = computed(() => `${formatCount(searchTotal.value)} 个结果，包含于 ${formatCount(searchTotal.value)} 个会话中`)
let searchTimer: ReturnType<typeof window.setTimeout> | undefined

const panelTitle = computed(() => {
  const titles: Record<SidebarPanel, string> = {
    sessions: '会话管理',
    search: '搜索',
    backup: '备份',
    info: '信息',
    settings: '设置',
  }
  return titles[activePanel.value]
})

function rowClassName({ row }: { row: ThreadSummary }) {
  return row.id === props.selectedId ? 'selected-row' : ''
}

function handleFilterChange() {
  props.filters.page = 1
  emit('load-threads')
}

async function loadSearchResults(reset = false) {
  const q = searchText.value.trim()
  if (reset) searchPage.value = 1
  if (!q) {
    searchThreads.value = []
    searchTotal.value = 0
    return
  }
  loadingSearch.value = true
  try {
    const data = await apiGet<ThreadPage>('/api/threads', {
      q,
      page: searchPage.value,
      page_size: searchPageSize,
    })
    searchThreads.value = data.items
    searchTotal.value = data.total
  } finally {
    loadingSearch.value = false
  }
}

function handleSearchChange() {
  void loadSearchResults(true)
}

function scheduleSearchChange() {
  if (searchTimer) window.clearTimeout(searchTimer)
  searchTimer = window.setTimeout(() => {
    handleSearchChange()
  }, 350)
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

watch(activePanel, (panel) => {
  if (panel === 'backup') {
    emit('load-backups')
  }
})
</script>

<template>
  <aside class="sidebar">
    <nav class="sidebar-rail" aria-label="侧边栏导航">
      <div class="rail-main">
        <el-tooltip content="会话管理" placement="right">
          <button
            class="rail-button"
            :class="{ active: activePanel === 'sessions' }"
            type="button"
            aria-label="会话管理"
            @click="activePanel = 'sessions'"
          >
            <el-icon><Management /></el-icon>
          </button>
        </el-tooltip>
        <el-tooltip content="搜索" placement="right">
          <button
            class="rail-button"
            :class="{ active: activePanel === 'search' }"
            type="button"
            aria-label="搜索"
            @click="activePanel = 'search'"
          >
            <el-icon><Search /></el-icon>
          </button>
        </el-tooltip>
        <el-tooltip content="备份" placement="right">
          <button
            class="rail-button"
            :class="{ active: activePanel === 'backup' }"
            type="button"
            aria-label="备份"
            @click="activePanel = 'backup'"
          >
            <el-icon><FolderOpened /></el-icon>
          </button>
        </el-tooltip>
        <el-tooltip content="信息" placement="right">
          <button
            class="rail-button"
            :class="{ active: activePanel === 'info' }"
            type="button"
            aria-label="信息"
            @click="activePanel = 'info'"
          >
            <el-icon><InfoFilled /></el-icon>
          </button>
        </el-tooltip>
      </div>

      <div class="rail-bottom">
        <el-tooltip content="设置" placement="right">
          <button
            class="rail-button"
            :class="{ active: activePanel === 'settings' }"
            type="button"
            aria-label="设置"
            @click="activePanel = 'settings'"
          >
            <el-icon><Setting /></el-icon>
          </button>
        </el-tooltip>
      </div>
    </nav>

    <section class="sidebar-panel">
      <header class="panel-header">
        <div class="panel-title">
          <h1>{{ panelTitle }}</h1>
          <p class="muted mono truncate">{{ overview?.data_dir ?? 'loading...' }}</p>
        </div>
        <el-button :icon="Refresh" circle @click="emit('reload')" />
      </header>

      <section v-if="activePanel === 'info'" class="info-panel">
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
        <el-button class="wide-action" :icon="FolderOpened" @click="emit('open-change-data-dir')">
          切换目录
        </el-button>
        <el-button class="wide-action" @click="emit('open-stats')">查看完整统计</el-button>
      </section>

      <section v-else-if="activePanel === 'settings'" class="settings-panel">
        <el-button class="wide-action" :icon="Refresh" @click="emit('reload')">
          重新加载
        </el-button>
      </section>

      <section v-else-if="activePanel === 'backup'" class="backup-panel">
        <el-table v-if="backups.length" :data="backups" height="100%" class="backup-table">
          <el-table-column label="备份">
            <template #default="{ row }">
              <div class="thread-title truncate" :title="row.note || row.id">
                {{ row.note || row.id }}
              </div>
              <div class="thread-meta truncate mono" :title="row.thread_id">{{ row.thread_id }}</div>
              <div class="thread-meta" :title="row.created_at">
                {{ formatTimestamp(row.created_at) }} · {{ formatBytes(row.bytes) }}
              </div>
            </template>
          </el-table-column>
          <el-table-column width="118" align="right">
            <template #default="{ row }">
              <el-button
                size="small"
                text
                type="primary"
                :loading="restoring === row.id"
                @click="emit('restore-backup', row)"
              >
                恢复
              </el-button>
              <el-button
                size="small"
                text
                type="danger"
                :loading="backupDeleting === row.id"
                @click="emit('delete-backup', row)"
              >
                删除
              </el-button>
            </template>
          </el-table-column>
        </el-table>
        <el-empty v-else description="暂无备份" />
      </section>

      <template v-else>
        <section v-if="activePanel === 'sessions'" class="session-tools">
          <button class="filter-toggle" type="button" @click="filtersExpanded = !filtersExpanded">
            <span>筛选</span>
            <span class="muted">{{ filtersExpanded ? '收起' : '展开' }}</span>
          </button>

          <div v-show="filtersExpanded" class="filters">
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
              <div class="token-slider-wrap">
                <el-slider
                  v-model="filters.token_range"
                  range
                  :min="0"
                  :max="tokenSliderMax"
                  :step="tokenStep"
                  :format-tooltip="formatCompactCount"
                />
              </div>
            </div>
          </div>
        </section>

        <section v-else class="search-panel">
          <div class="search-stack">
            <button
              class="replace-toggle"
              type="button"
              :aria-label="replaceExpanded ? '收起替换' : '展开替换'"
              @click="replaceExpanded = !replaceExpanded"
            >
              <el-icon>
                <CaretBottom v-if="replaceExpanded" />
                <CaretRight v-else />
              </el-icon>
            </button>
            <el-input
              v-model="searchText"
              :prefix-icon="Search"
              clearable
              placeholder="搜索"
              @input="scheduleSearchChange"
              @keyup.enter="handleSearchChange"
              @clear="handleSearchChange"
            />
            <el-input
              v-if="replaceExpanded"
              v-model="replaceText"
              class="replace-input"
              clearable
              placeholder="替换"
            />
          </div>
        </section>

        <el-table
          v-if="activePanel === 'sessions' || searchHasQuery"
          class="thread-table"
          v-loading="activePanel === 'search' ? loadingSearch : loadingThreads"
          :data="activePanel === 'search' ? searchThreads : threads"
          height="100%"
          highlight-current-row
          :row-class-name="rowClassName"
          @row-click="(row: ThreadSummary) => emit('select-thread', row)"
        >
          <el-table-column :label="activePanel === 'search' ? searchResultLabel : '会话'">
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
        <el-empty
          v-else
          class="search-empty"
          description="输入关键词后显示搜索结果"
        />
        <el-pagination
          v-if="activePanel === 'sessions' || searchHasQuery"
          class="pager"
          layout="prev, pager, next"
          :page-size="activePanel === 'search' ? searchPageSize : filters.page_size"
          :total="activePanel === 'search' ? searchTotal : total"
          :current-page="activePanel === 'search' ? searchPage : filters.page"
          @current-change="(page: number) => {
            if (activePanel === 'search') {
              searchPage = page
              loadSearchResults(false)
            } else {
              filters.page = page
              emit('load-threads')
            }
          }"
        />
      </template>
    </section>
  </aside>
</template>

<style scoped>
.mono {
  margin: 0;
}


.sidebar {
  display: flex;
  flex-direction: row;
  border-right: 1px solid #e2e8f0;
  background: #ffffff;
  min-height: 0;
  overflow: hidden;
  box-shadow: 2px 0 8px rgba(0, 0, 0, 0.04);
}

.sidebar-rail {
  display: flex;
  flex: 0 0 56px;
  flex-direction: column;
  align-items: center;
  justify-content: space-between;
  min-height: 0;
  padding: 10px 8px;
  border-right: 1px solid #e2e8f0;
  background: #f8fafc;
}

.rail-main,
.rail-bottom {
  display: grid;
  gap: 8px;
}

.rail-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: #697386;
  cursor: pointer;
  transition: background-color 0.15s ease, color 0.15s ease;
}

.rail-button:hover,
.rail-button.active {
  background: #eef2ff;
  color: #4f46e5;
}

.rail-button .el-icon {
  font-size: 20px;
}

.sidebar-panel {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  padding: 16px;
  overflow: hidden;
}

.panel-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  flex: 0 0 auto;
  gap: 12px;
  margin-bottom: 12px;
}

.panel-title {
  min-width: 0;
}

.panel-title h1 {
  color: #2d3748;
  margin: 0;
  font-size: 20px;
  line-height: 1.3;
  font-weight: 700;
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

.info-panel,
.backup-panel,
.settings-panel {
  display: grid;
  align-content: start;
  gap: 12px;
  min-height: 0;
}

.backup-panel {
  flex: 1 1 auto;
  align-content: stretch;
}

.backup-table {
  min-height: 0;
}

.wide-action {
  justify-content: flex-start;
  width: 100%;
}

.wide-action + .wide-action {
  margin-left: 0;
}

.session-tools {
  display: grid;
  flex: 0 0 auto;
  gap: 10px;
  margin-bottom: 12px;
}

.filter-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  background: #fafbfc;
  color: #2d3748;
  cursor: pointer;
  padding: 8px 10px;
  text-align: left;
}

.filter-toggle:hover {
  border-color: #c7d2fe;
  background: #f5f7ff;
}

.filters {
  display: grid;
  flex: 0 0 auto;
  gap: 10px;
}

.search-panel {
  display: grid;
  flex: 0 0 auto;
  gap: 10px;
  margin-bottom: 12px;
}

.search-stack {
  display: grid;
  grid-template-columns: 24px minmax(0, 1fr);
  gap: 8px;
  align-items: center;
}

.replace-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 32px;
  border: 0;
  background: transparent;
  color: #697386;
  cursor: pointer;
  padding: 0;
}

.replace-toggle:hover {
  color: #4f46e5;
}

.replace-input {
  grid-column: 2;
}

.search-empty {
  flex: 1 1 auto;
  min-height: 0;
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
  overflow: hidden;
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
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.token-slider-wrap {
  box-sizing: border-box;
  min-width: 0;
  padding: 0 8px;
}

.token-slider-wrap :deep(.el-slider) {
  --el-slider-main-bg-color: #667eea;
  width: 100%;
  margin: 0;
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
