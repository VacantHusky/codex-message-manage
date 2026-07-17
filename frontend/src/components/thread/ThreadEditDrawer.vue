<script setup lang="ts">
import type { UpdateRuntimeRequest } from '../../api'

defineProps<{
  titleText: string
  archived: boolean
  runtimeForm: UpdateRuntimeRequest
  savingRuntime: boolean
}>()

const visible = defineModel<boolean>('visible', { required: true })

const emit = defineEmits<{
  (e: 'update:titleText', value: string): void
  (e: 'update:archived', value: boolean): void
  (e: 'save'): void
}>()
</script>

<template>
  <el-drawer
    v-model="visible"
    title="编辑会话"
    size="420px"
    direction="rtl"
  >
    <el-form label-width="92px" class="thread-edit-form">
      <el-form-item label="标题">
        <el-input
          :model-value="titleText"
          clearable
          placeholder="留空表示恢复原始标题"
          @update:model-value="(value: string) => emit('update:titleText', value)"
        />
      </el-form-item>
      <el-form-item label="归档">
        <el-switch
          :model-value="archived"
          active-text="已归档"
          inactive-text="未归档"
          @update:model-value="(value: boolean) => emit('update:archived', value)"
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
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" :loading="savingRuntime" @click="emit('save')">
        保存
      </el-button>
    </template>
  </el-drawer>
</template>

<style scoped>
.thread-edit-form :deep(.el-select) {
  width: 100%;
}
</style>
