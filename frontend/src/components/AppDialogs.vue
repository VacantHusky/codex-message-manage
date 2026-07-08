<script setup lang="ts">
import { FolderOpened, Search } from '@element-plus/icons-vue'
import {
  boolLabel,
  formatBytes,
  formatCount,
  formatTimestamp,
  eventTypeName,
  payloadTypeName,
  runtimeLabel,
} from '../utils/format'
import type { DeletePreview, BackupInfo, StatsResponse, BrowseResponse } from '../api'

defineProps<{
  deleteDialog: boolean
  deletePreview?: DeletePreview
  deleteConfirm: boolean
  deleting: boolean
  backupDialog: boolean
  backups: BackupInfo[]
  restoring: string
  backupDeleting: string
  statsDialog: boolean
  statsData?: StatsResponse
  changeDataDirDialog: boolean
  newDataDir: string
  changingDataDir: boolean
  browseDialogVisible: boolean
  browseLoading: boolean
  browseResult?: BrowseResponse
}>()

const emit = defineEmits<{
  (e: 'update:deleteDialog', value: boolean): void
  (e: 'update:deleteConfirm', value: boolean): void
  (e: 'confirm-delete'): void
  (e: 'update:backupDialog', value: boolean): void
  (e: 'restore-backup', item: BackupInfo): void
  (e: 'delete-backup', item: BackupInfo): void
  (e: 'update:statsDialog', value: boolean): void
  (e: 'update:changeDataDirDialog', value: boolean): void
  (e: 'update:newDataDir', value: string): void
  (e: 'change-data-dir'): void
  (e: 'open-browse', path?: string): void
  (e: 'browse-to', path: string): void
  (e: 'select-browse-dir'): void
  (e: 'update:browseDialogVisible', value: boolean): void
}>()
</script>

<template>
  <!-- 删除确认对话框 -->
  <el-dialog
    :model-value="deleteDialog"
    @update:model-value="(val: boolean) => emit('update:deleteDialog', val)"
    title="删除影响范围"
    width="760px"
  >
    <template v-if="deletePreview">
      <el-alert
        type="warning"
        :closable="false"
        title="删除会移除会话文件和数据库记录，且不会自动备份；需要保留恢复点时，请先点击详情页的备份按钮。"
      />
      <h3>文件</h3>
      <el-table :data="deletePreview.files" max-height="220">
        <el-table-column prop="path" label="路径" />
        <el-table-column label="状态" width="90">
          <template #default="{ row }">{{ row.exists ? '存在' : '缺失' }}</template>
        </el-table-column>
        <el-table-column label="大小" width="120">
          <template #default="{ row }">{{ formatBytes(row.bytes) }}</template>
        </el-table-column>
      </el-table>
      <h3>数据库</h3>
      <div class="db-impact">
        <span v-for="(count, name) in deletePreview.database_rows" :key="name">
          {{ name }}: <strong>{{ count }}</strong>
        </span>
        <span>history.jsonl: <strong>{{ deletePreview.history_rows }}</strong></span>
      </div>
      <el-checkbox
        :model-value="deleteConfirm"
        @update:model-value="(val: boolean) => emit('update:deleteConfirm', val)"
      >
        我确认删除，并已知晓会修改原始 Codex 数据
      </el-checkbox>
    </template>
    <template #footer>
      <el-button @click="emit('update:deleteDialog', false)">取消</el-button>
      <el-button
        type="danger"
        :disabled="!deleteConfirm"
        :loading="deleting"
        @click="emit('confirm-delete')"
      >
        执行删除
      </el-button>
    </template>
  </el-dialog>

  <!-- 备份恢复对话框 -->
  <el-dialog
    :model-value="backupDialog"
    @update:model-value="(val: boolean) => emit('update:backupDialog', val)"
    title="备份与恢复"
    width="900px"
  >
    <el-table :data="backups" max-height="460" class="backup-table">
      <el-table-column label="时间" width="170">
        <template #default="{ row }">
          <span class="cell-text" :title="row.created_at">{{ formatTimestamp(row.created_at) }}</span>
        </template>
      </el-table-column>
      <el-table-column label="线程" min-width="200">
        <template #default="{ row }">
          <span class="cell-text mono" :title="row.thread_id">{{ row.thread_id }}</span>
        </template>
      </el-table-column>
      <el-table-column label="大小" width="90" align="right">
        <template #default="{ row }">{{ formatBytes(row.bytes) }}</template>
      </el-table-column>
      <el-table-column label="备注" min-width="160">
        <template #default="{ row }">
          <span class="cell-text" :title="row.note || '-'">{{ row.note || '-' }}</span>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="180" align="center" fixed="right">
        <template #default="{ row }">
          <el-button
            size="small"
            type="primary"
            plain
            :loading="restoring === row.id"
            @click="emit('restore-backup', row)"
          >
            恢复
          </el-button>
          <el-button
            size="small"
            type="danger"
            plain
            :loading="backupDeleting === row.id"
            @click="emit('delete-backup', row)"
          >
            删除
          </el-button>
        </template>
      </el-table-column>
    </el-table>
  </el-dialog>

  <!-- 统计对话框 -->
  <el-dialog
    :model-value="statsDialog"
    @update:model-value="(val: boolean) => emit('update:statsDialog', val)"
    title="统计"
    width="1180px"
    class="stats-dialog"
  >
    <div v-if="statsData" class="stats-body">
      <section class="stats-grid">
        <div>
          <h3>项目</h3>
          <p v-for="item in statsData.by_cwd.slice(0, 10)" :key="item.name">
            <span class="truncate">{{ item.name }}</span>
            <strong>{{ item.count }}</strong>
          </p>
        </div>
        <div>
          <h3>模型来源</h3>
          <p v-for="item in statsData.by_model_provider" :key="item.name">
            <span>{{ item.name }}</span>
            <strong>{{ item.count }}</strong>
          </p>
        </div>
        <div>
          <h3>模型</h3>
          <p v-for="item in statsData.by_model" :key="item.name">
            <span>{{ item.name }}</span>
            <strong>{{ item.count }}</strong>
          </p>
        </div>
        <div>
          <h3>推理强度</h3>
          <p v-for="item in statsData.by_reasoning_effort" :key="item.name">
            <span>{{ item.name }}</span>
            <strong>{{ item.count }}</strong>
          </p>
        </div>
        <div>
          <h3>沙箱</h3>
          <p v-for="item in statsData.by_sandbox_policy" :key="item.name">
            <span>{{ runtimeLabel(item.name) }}</span>
            <strong>{{ item.count }}</strong>
          </p>
        </div>
        <div>
          <h3>确认模式</h3>
          <p v-for="item in statsData.by_approval_mode" :key="item.name">
            <span>{{ runtimeLabel(item.name) }}</span>
            <strong>{{ item.count }}</strong>
          </p>
        </div>
        <div>
          <h3>记忆模式</h3>
          <p v-for="item in statsData.by_memory_mode" :key="item.name">
            <span>{{ runtimeLabel(item.name) }}</span>
            <strong>{{ item.count }}</strong>
          </p>
        </div>
        <div>
          <h3>线程来源</h3>
          <p v-for="item in statsData.by_thread_source" :key="item.name">
            <span>{{ runtimeLabel(item.name) }}</span>
            <strong>{{ item.count }}</strong>
          </p>
        </div>
        <div>
          <h3>CLI 版本</h3>
          <p v-for="item in statsData.by_cli_version" :key="item.name">
            <span>{{ item.name || '-' }}</span>
            <strong>{{ item.count }}</strong>
          </p>
        </div>
        <div>
          <h3>事件类型</h3>
          <p v-for="item in statsData.event_types.slice(0, 10)" :key="item.name">
            <span>{{ eventTypeName(item.name) }}</span>
            <strong>{{ item.count }}</strong>
          </p>
        </div>
        <div>
          <h3>载荷类型</h3>
          <p v-for="item in statsData.payload_types.slice(0, 10)" :key="item.name">
            <span>{{ payloadTypeName(item.name) }}</span>
            <strong>{{ item.count }}</strong>
          </p>
        </div>
      </section>
      <template v-if="statsData.model_cache">
        <h3>模型缓存</h3>
        <div class="model-cache-meta">
          <span>客户端版本: <strong>{{ statsData.model_cache.client_version || '-' }}</strong></span>
          <span>拉取时间: <strong>{{ formatTimestamp(statsData.model_cache.fetched_at) }}</strong></span>
          <span>模型数: <strong>{{ formatCount(statsData.model_cache.models.length) }}</strong></span>
        </div>
        <el-table :data="statsData.model_cache.models" max-height="260">
          <el-table-column prop="display_name" label="模型" min-width="160" />
          <el-table-column prop="slug" label="Slug" min-width="150" />
          <el-table-column label="上下文" width="110">
            <template #default="{ row }">{{ formatCount(row.context_window) }}</template>
          </el-table-column>
          <el-table-column label="默认推理" width="100">
            <template #default="{ row }">{{ row.default_reasoning_level || '-' }}</template>
          </el-table-column>
          <el-table-column label="并行工具" width="90">
            <template #default="{ row }">{{ boolLabel(row.supports_parallel_tool_calls) }}</template>
          </el-table-column>
          <el-table-column label="图片原始" width="90">
            <template #default="{ row }">{{ boolLabel(row.supports_image_detail_original) }}</template>
          </el-table-column>
          <el-table-column label="搜索" width="70">
            <template #default="{ row }">{{ boolLabel(row.supports_search_tool) }}</template>
          </el-table-column>
        </el-table>
      </template>
      <h3>最大会话</h3>
      <el-table :data="statsData.largest_sessions" max-height="260">
        <el-table-column label="标题">
          <template #default="{ row }">
            <span class="cell-text" :title="row.title">{{ row.title }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="cwd" label="项目" />
        <el-table-column label="大小" width="120">
          <template #default="{ row }">{{ formatBytes(row.bytes) }}</template>
        </el-table-column>
      </el-table>
    </div>
    <div v-else v-loading="true" class="stats-loading">
      <span>加载中...</span>
    </div>
  </el-dialog>

  <!-- 切换数据目录对话框 -->
  <el-dialog
    :model-value="changeDataDirDialog"
    @update:model-value="(val: boolean) => emit('update:changeDataDirDialog', val)"
    title="切换数据目录"
    width="600px"
  >
    <el-alert
      type="success"
      :closable="false"
      title="切换后会自动重新加载数据，无需重启应用"
    />
    <div style="margin-top: 12px;">
      <p style="margin-bottom: 6px; color: #606266;">输入数据目录路径：</p>
      <div style="display: flex; gap: 8px;">
        <el-input
          :model-value="newDataDir"
          @update:model-value="(val: string) => emit('update:newDataDir', val)"
          placeholder="如: D:\codex-save 或 C:\Users\用户名\.codex\sessions"
          :prefix-icon="FolderOpened"
          style="flex: 1;"
        />
        <el-button :icon="Search" @click="emit('open-browse', newDataDir || undefined)">
          浏览
        </el-button>
      </div>
      <p style="margin-top: 6px; color: #909399; font-size: 12px;">
        文件夹中应包含 state_*.sqlite、goals_*.sqlite、logs_*.sqlite 等数据库文件
      </p>
    </div>
    <template #footer>
      <el-button @click="emit('update:changeDataDirDialog', false)">取消</el-button>
      <el-button
        type="primary"
        :loading="changingDataDir"
        @click="emit('change-data-dir')"
      >
        确认切换
      </el-button>
    </template>
  </el-dialog>

  <!-- 文件夹浏览对话框 -->
  <el-dialog
    :model-value="browseDialogVisible"
    @update:model-value="(val: boolean) => emit('update:browseDialogVisible', val)"
    title="选择文件夹"
    width="560px"
  >
    <div v-if="browseResult" class="browse-panel">
      <div class="browse-current">
        <span class="browse-label">当前路径：</span>
        <span class="browse-path">{{ browseResult.current }}</span>
      </div>
      <div class="browse-actions">
        <el-button
          size="small"
          :disabled="!browseResult.parent"
          @click="browseResult.parent && emit('browse-to', browseResult.parent)"
        >
          ↑ 上级
        </el-button>
        <el-button size="small" type="primary" @click="emit('select-browse-dir')">
          选择此目录
        </el-button>
      </div>
      <el-divider style="margin: 10px 0;" />
      <div class="browse-list" v-loading="browseLoading">
        <div
          v-if="browseResult.directories.length === 0"
          class="browse-empty"
        >
          此目录下没有子文件夹
        </div>
        <div
          v-for="dir in browseResult.directories"
          :key="dir.path"
          class="browse-item"
          @click="emit('browse-to', dir.path)"
        >
          <FolderOpened style="width: 16px; height: 16px; color: #e6a23c; flex-shrink: 0;" />
          <span>{{ dir.name }}</span>
        </div>
      </div>
    </div>
  </el-dialog>
</template>

<style scoped>
h3 {
  font-size: 13px;
  margin: 10px 0 6px;
  color: #606266;
  font-weight: 600;
}

h3:first-child {
  margin-top: 0;
}

.db-impact {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-bottom: 10px;
  font-size: 13px;
}

.stats-body {
  min-height: 320px;
}

.stats-loading {
  min-height: 200px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 12px;
}

.stats-grid > div {
  border: 1px solid #ebeef5;
  border-radius: 8px;
  padding: 10px;
  background: #fafbfc;
}

.stats-grid p {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  line-height: 1.8;
  margin: 0;
  font-size: 13px;
}

.model-cache-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 18px;
  margin-bottom: 8px;
  color: #697386;
  font-size: 13px;
}

.cell-text {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mono {
  font-family: "JetBrains Mono", "SFMono-Regular", Consolas, monospace;
  font-size: 12px;
}

.browse-panel {
  font-size: 14px;
}

.browse-current {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #f5f7fa;
  border-radius: 6px;
  margin-bottom: 8px;
}

.browse-label {
  color: #909399;
  flex-shrink: 0;
}

.browse-path {
  color: #303133;
  font-family: monospace;
  word-break: break-all;
}

.browse-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.browse-list {
  max-height: 320px;
  overflow-y: auto;
  border: 1px solid #ebeef5;
  border-radius: 6px;
}

.browse-empty {
  padding: 20px;
  text-align: center;
  color: #909399;
}

.browse-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 12px;
  cursor: pointer;
  transition: background 0.15s;
}

.browse-item:hover {
  background: #f0f2f5;
}

.browse-item + .browse-item {
  border-top: 1px solid #f0f0f0;
}
</style>
