<script setup lang="ts">
import { eventLabel, eventTagType, formatTimestamp } from '../../utils/format'
import type { SessionEvent } from '../../api'

defineProps<{
  event: SessionEvent | null
  editText: string
  editError: string
  savingEdit: boolean
}>()

const visible = defineModel<boolean>('visible', { required: true })

const emit = defineEmits<{
  (e: 'update:editText', value: string): void
  (e: 'validate'): void
  (e: 'save'): void
}>()
</script>

<template>
  <el-dialog
    v-model="visible"
    :title="event ? `编辑节点 #${event.index}` : ''"
    width="720px"
    top="6vh"
    destroy-on-close
  >
    <div v-if="event" class="edit-dialog">
      <div class="edit-dialog-meta">
        <el-tag :type="eventTagType(event)" size="small" effect="plain">
          {{ eventLabel(event) }}
        </el-tag>
        <span v-if="event.role" class="muted">{{ event.role }}</span>
        <span class="muted">{{ formatTimestamp(event.timestamp) }}</span>
      </div>
      <el-input
        :model-value="editText"
        type="textarea"
        :autosize="{ minRows: 14, maxRows: 30 }"
        placeholder="JSON 内容"
        class="edit-textarea"
        @update:model-value="(value: string) => emit('update:editText', value)"
        @input="emit('validate')"
      />
      <p v-if="editError" class="edit-error">{{ editError }}</p>
    </div>
    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" :loading="savingEdit" :disabled="!!editError" @click="emit('save')">
        保存
      </el-button>
    </template>
  </el-dialog>
</template>

<style scoped>
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
