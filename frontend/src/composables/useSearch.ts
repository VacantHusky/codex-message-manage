import { ref } from 'vue'
import { apiGet, type SearchHit, type SearchResponse, type ThreadSummary } from '../api'
import { ElMessage } from 'element-plus'

export function useSearch() {
  const searchText = ref('')
  const searching = ref(false)
  const searchResults = ref<SearchHit[]>([])

  async function runSearch(threadId?: string) {
    const q = searchText.value.trim()
    if (!q) {
      searchResults.value = []
      return
    }
    searching.value = true
    try {
      const data = await apiGet<SearchResponse>('/api/search', { q, thread_id: threadId, limit: 120 })
      searchResults.value = data.items
    } catch (error) {
      ElMessage.error(messageOf(error))
    } finally {
      searching.value = false
    }
  }

  function openSearchHit(hit: SearchHit, threads: ThreadSummary[]) {
    const row = threads.find((item) => item.id === hit.thread_id)
    if (row) {
      return { type: 'existing', thread: row }
    }
    return { type: 'new', threadId: hit.thread_id }
  }

  function messageOf(error: unknown) {
    return error instanceof Error ? error.message : String(error)
  }

  return {
    searchText,
    searching,
    searchResults,
    runSearch,
    openSearchHit,
  }
}
