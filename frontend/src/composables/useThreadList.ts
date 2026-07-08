import { ref, reactive } from 'vue'
import { apiGet, type ThreadPage, type ThreadSummary } from '../api'
import { ElMessage } from 'element-plus'

export function useThreadList() {
  const threads = ref<ThreadSummary[]>([])
  const total = ref(0)
  const loadingThreads = ref(false)
  const selectedId = ref('')

  const filters = reactive({
    q: '',
    cwd: '',
    model: '',
    date_range: [] as string[],
    token_range: [] as number[],
    page: 1,
    page_size: 30,
  })

  async function loadThreads() {
    loadingThreads.value = true
    try {
      const page = await apiGet<ThreadPage>('/api/threads', {
        q: filters.q,
        cwd: filters.cwd,
        model: filters.model,
        date_from: filters.date_range?.[0],
        date_to: filters.date_range?.[1],
        token_min: filters.token_range?.length === 2 ? Math.round(filters.token_range[0]) : undefined,
        token_max: filters.token_range?.length === 2 ? Math.round(filters.token_range[1]) : undefined,
        page: filters.page,
        page_size: filters.page_size,
      })
      threads.value = page.items
      total.value = page.total
      if (!selectedId.value && page.items.length) {
        return page.items[0]
      }
      return null
    } catch (error) {
      ElMessage.error(messageOf(error))
      return null
    } finally {
      loadingThreads.value = false
    }
  }

  function messageOf(error: unknown) {
    return error instanceof Error ? error.message : String(error)
  }

  return {
    threads,
    total,
    loadingThreads,
    selectedId,
    filters,
    loadThreads,
  }
}
