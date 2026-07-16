<script setup lang="ts">
import { Delete, EditPen } from '@element-plus/icons-vue'
import { formatDateTime } from '../../utils/format'
import type { HistoryEntry } from '../../api'

defineProps<{
  historyItems: HistoryEntry[]
  historyTotal: number
  historyPage: number
  loadingHistory: boolean
  historyFilters: {
    limit: number
  }
}>()

const emit = defineEmits<{
  (e: 'load-history', reset: boolean): void
  (e: 'change-history-page', page: number): void
  (e: 'edit-history', item: HistoryEntry): void
  (e: 'delete-history', item: HistoryEntry): void
}>()
</script>

<template>
  <el-table
    v-loading="loadingHistory"
    :data="historyItems"
    class="history-table"
    height="100%"
    virtual-scroll
  >
    <el-table-column label="时间" width="220">
      <template #default="{ row }">
        <span :title="row.ts_text">{{ formatDateTime(row.ts_text) }}</span>
      </template>
    </el-table-column>
    <el-table-column label="内容">
      <template #default="{ row }">
        <span class="prewrap">{{ row.text }}</span>
      </template>
    </el-table-column>
    <el-table-column label="操作" width="150" align="right">
      <template #default="{ row }">
        <el-button size="small" text :icon="EditPen" @click="emit('edit-history', row)">
          编辑
        </el-button>
        <el-button size="small" text type="danger" :icon="Delete" @click="emit('delete-history', row)">
          删除
        </el-button>
      </template>
    </el-table-column>
  </el-table>

  <div class="load-more">
    <span class="muted">共 {{ historyTotal }} 条</span>
    <el-pagination
      background
      layout="prev, pager, next, sizes"
      :current-page="historyPage"
      :page-size="historyFilters.limit"
      :page-sizes="[10, 20, 50, 100]"
      :total="historyTotal"
      @current-change="(page: number) => emit('change-history-page', page)"
      @size-change="(size: number) => { historyFilters.limit = size; emit('load-history', true) }"
    />
  </div>
</template>

<style scoped>
.history-table {
  flex: 1 1 auto;
  min-height: 0;
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
