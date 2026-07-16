<script setup lang="ts">
const historyText = defineModel<string>('historyText', { required: true })
const visible = defineModel<boolean>('visible', { required: true })

const emit = defineEmits<{
  (e: 'save'): void
}>()
</script>

<template>
  <el-drawer
    v-model="visible"
    title="编辑历史"
    size="520px"
    direction="rtl"
  >
    <el-input
      v-model="historyText"
      type="textarea"
      :autosize="{ minRows: 10, maxRows: 24 }"
    />
    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" @click="emit('save')">保存</el-button>
    </template>
  </el-drawer>
</template>
