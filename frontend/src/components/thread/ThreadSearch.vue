<script setup lang="ts">
import { Search } from '@element-plus/icons-vue'
import type { SearchHit, ThreadSummary } from '../../api'

defineProps<{
  selectedThread?: ThreadSummary
  searchText: string
  searching: boolean
  searchResults: SearchHit[]
}>()

const emit = defineEmits<{
  (e: 'search'): void
  (e: 'search-hit', hit: SearchHit): void
  (e: 'update:searchText', value: string): void
}>()
</script>

<template>
  <section class="search-panel">
    <el-input
      :model-value="searchText"
      @update:model-value="(val: string) => emit('update:searchText', val)"
      :prefix-icon="Search"
      clearable
      :disabled="!selectedThread"
      placeholder="搜索当前会话的消息、历史、标题"
      @keyup.enter="emit('search')"
    />
    <el-button type="primary" :loading="searching" :disabled="!selectedThread" @click="emit('search')">搜索</el-button>
  </section>

  <section v-if="searchResults.length" class="search-results">
    <button
      v-for="hit in searchResults"
      :key="`${hit.thread_id}-${hit.source}-${hit.field}-${hit.timestamp}`"
      class="hit"
      @click="emit('search-hit', hit)"
    >
      <strong>{{ hit.title || hit.thread_id }}</strong>
      <span class="muted">{{ hit.source }} · {{ hit.field }} · {{ hit.timestamp || '-' }}</span>
      <span class="prewrap">{{ hit.snippet }}</span>
    </button>
  </section>
</template>

<style scoped>
.search-panel {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.search-results {
  display: grid;
  gap: 8px;
  max-height: 220px;
  overflow: auto;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  background: #fafbfc;
  padding: 10px;
  margin-bottom: 12px;
}

.hit {
  display: grid;
  gap: 4px;
  text-align: left;
  border: 0;
  border-radius: 8px;
  background: transparent;
  padding: 10px 12px;
  color: inherit;
  cursor: pointer;
  transition: all 0.2s ease;
}

.hit:hover {
  background: #eef2ff;
  transform: translateX(4px);
}
</style>
