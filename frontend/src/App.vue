<script setup lang="ts">
import { ref, onMounted } from 'vue'
import Sidebar from './components/Sidebar.vue'
import ThreadContent from './components/ThreadContent.vue'
import AppDialogs from './components/AppDialogs.vue'
import { useThreadList } from './composables/useThreadList'
import { useThreadDetail } from './composables/useThreadDetail'
import { useThreadEvents } from './composables/useThreadEvents'
import { useSearch } from './composables/useSearch'
import { useBackup } from './composables/useBackup'
import { useStats } from './composables/useStats'
import { useConfig } from './composables/useConfig'
import { useDelete } from './composables/useDelete'
import type { ThreadSummary, SearchHit, BackupInfo, SessionEvent } from './api'

const {
  threads,
  total,
  loadingThreads,
  selectedId,
  filters,
  loadThreads,
} = useThreadList()

const {
  loadingDetail,
  detail,
  activeTab,
  selectedThread,
  loadThreadDetail,
  updateTitle,
  archiveSelected,
  updateRuntime,
  backupSelected,
  clearLogs,
} = useThreadDetail(selectedId)

const {
  events,
  eventsTotal,
  loadingEvents,
  expandedEvents,
  eventFilters,
  loadEvents,
  loadAllEvents,
  isEventExpanded,
  toggleEvent,
  eventText,
  saveEvent,
  deleteEvent,
} = useThreadEvents(selectedId)

const {
  searchText,
  searching,
  searchResults,
  runSearch,
  openSearchHit,
} = useSearch()

const {
  backups,
  backupDialog,
  restoring,
  deleting: backupDeleting,
  openBackups,
  restoreBackup,
  deleteBackup,
} = useBackup()

const {
  statsData,
  statsDialog,
  openStats,
} = useStats()

const backingUp = ref(false)
const clearingLogs = ref(false)

const {
  config,
  overview,
  changeDataDirDialog,
  newDataDir,
  changingDataDir,
  browseDialogVisible,
  browseLoading,
  browseResult,
  loadConfig,
  loadOverview,
  openChangeDataDir,
  changeDataDir,
  openBrowse,
  browseTo,
  selectBrowseDir,
} = useConfig(async () => {
  // 清除旧的选中状态，避免旧目录的 ID 残留
  selectedId.value = ''
  detail.value = undefined
  events.value = []
  await Promise.all([loadOverview(), loadThreads()])
})

const {
  deleteDialog,
  deletePreview,
  deleteConfirm,
  deleting,
  previewDelete,
  confirmDelete,
} = useDelete()

onMounted(async () => {
  await Promise.all([loadConfig(), loadOverview(), loadThreads()])
})

async function reloadAll() {
  await Promise.all([loadOverview(), loadThreads()])
}

async function openThread(row: ThreadSummary) {
  selectedId.value = row.id
  activeTab.value = 'timeline'
  await loadThreadDetail(row.id)
  await loadEvents(true)
}

async function handleSearchHit(hit: SearchHit) {
  const result = openSearchHit(hit, threads.value)
  if (result.type === 'existing' && result.thread) {
    await openThread(result.thread)
  } else if (result.type === 'new' && result.threadId) {
    await loadThreadDetail(result.threadId)
    selectedId.value = result.threadId
    await loadEvents(true)
  }
}

async function handleBackupSelected() {
  const success = await backupSelected(backingUp)
  if (success) {
    await openBackups()
  }
}

async function handleClearLogs() {
  const success = await clearLogs(clearingLogs)
  if (success && selectedId.value) {
    await loadThreadDetail(selectedId.value)
  }
}

async function handlePreviewDelete() {
  await previewDelete(selectedId.value)
}

async function handleConfirmDelete() {
  const success = await confirmDelete(selectedId.value)
  if (success) {
    selectedId.value = ''
    detail.value = undefined
    events.value = []
    await reloadAll()
  }
}

async function handleRestoreBackup(item: BackupInfo) {
  const success = await restoreBackup(item)
  if (success) {
    await Promise.all([loadOverview(), loadThreads(), openBackups()])
  }
}

async function handleDeleteBackup(item: BackupInfo) {
  const success = await deleteBackup(item)
  if (success) {
    await openBackups()
  }
}

async function handleArchive(archived: boolean) {
  const success = await archiveSelected(archived)
  if (success && selectedId.value) {
    await Promise.all([loadThreadDetail(selectedId.value), loadThreads()])
  }
}

async function handleRuntimeSaved(request: Parameters<typeof updateRuntime>[0]) {
  const success = await updateRuntime(request)
  if (success && selectedId.value) {
    await Promise.all([loadThreadDetail(selectedId.value), loadThreads(), loadOverview()])
  }
  return !!success
}

async function handleTitleSaved(title: string) {
  const success = await updateTitle(title)
  if (success && selectedId.value) {
    await Promise.all([loadThreadDetail(selectedId.value), loadThreads(), loadOverview()])
  }
  return !!success
}

async function handleEventSaved() {
  if (selectedId.value) {
    await Promise.all([loadThreadDetail(selectedId.value), loadEvents(true)])
  }
}

async function handleDeleteEvent(event: SessionEvent) {
  const success = await deleteEvent(event)
  if (success && selectedId.value) {
    await Promise.all([loadThreadDetail(selectedId.value), loadEvents(true)])
  }
}
</script>

<template>
  <main class="app-shell">
    <Sidebar
      :overview="overview"
      :loading-threads="loadingThreads"
      :threads="threads"
      :total="total"
      :filters="filters"
      :selected-id="selectedId"
      @select-thread="openThread"
      @reload="reloadAll"
      @open-stats="openStats"
      @open-backups="openBackups"
      @open-change-data-dir="openChangeDataDir"
      @load-threads="loadThreads"
    />

    <ThreadContent
      :loading-detail="loadingDetail"
      :detail="detail"
      :selected-thread="selectedThread"
      :events="events"
      :events-total="eventsTotal"
      :loading-events="loadingEvents"
      :event-filters="eventFilters"
      :active-tab="activeTab"
      :expanded-events="expandedEvents"
      :search-text="searchText"
      :searching="searching"
      :search-results="searchResults"
      :backing-up="backingUp"
      :clearing-logs="clearingLogs"
      :save-event="saveEvent"
      :save-title="handleTitleSaved"
      :save-runtime="handleRuntimeSaved"
      @backup="handleBackupSelected"
      @archive="handleArchive"
      @delete="handlePreviewDelete"
      @clear-logs="handleClearLogs"
      @load-events="loadEvents"
      @load-all-events="loadAllEvents"
      @event-saved="handleEventSaved"
      @delete-event="handleDeleteEvent"
      @toggle-event="toggleEvent"
      @search="runSearch"
      @search-hit="handleSearchHit"
      @update:search-text="(val: string) => searchText = val"
      @update:active-tab="(val: string) => activeTab = val"
    />

    <AppDialogs
      :delete-dialog="deleteDialog"
      :delete-preview="deletePreview"
      :delete-confirm="deleteConfirm"
      :deleting="deleting"
      :backup-dialog="backupDialog"
      :backups="backups"
      :restoring="restoring"
      :backup-deleting="backupDeleting"
      :stats-dialog="statsDialog"
      :stats-data="statsData"
      :change-data-dir-dialog="changeDataDirDialog"
      :new-data-dir="newDataDir"
      :changing-data-dir="changingDataDir"
      :browse-dialog-visible="browseDialogVisible"
      :browse-loading="browseLoading"
      :browse-result="browseResult"
      @update:delete-dialog="(val: boolean) => deleteDialog = val"
      @update:delete-confirm="(val: boolean) => deleteConfirm = val"
      @confirm-delete="handleConfirmDelete"
      @update:backup-dialog="(val: boolean) => backupDialog = val"
      @restore-backup="handleRestoreBackup"
      @delete-backup="handleDeleteBackup"
      @update:stats-dialog="(val: boolean) => statsDialog = val"
      @update:change-data-dir-dialog="(val: boolean) => changeDataDirDialog = val"
      @update:new-data-dir="(val: string) => newDataDir = val"
      @change-data-dir="changeDataDir"
      @open-browse="openBrowse"
      @browse-to="browseTo"
      @select-browse-dir="selectBrowseDir"
      @update:browse-dialog-visible="(val: boolean) => browseDialogVisible = val"
    />
  </main>
</template>

<style scoped>
.app-shell {
  display: grid;
  grid-template-columns: 480px minmax(0, 1fr);
  height: 100vh;
  min-height: 0;
  overflow: hidden;
  background: linear-gradient(135deg, #f5f7fa 0%, #e4e8ec 100%);
}
</style>
