import { reactive, ref } from 'vue'
import { apiGet, type HistoryEntry, type HistoryPage } from '../api'
import { ElMessage } from 'element-plus'

export function useThreadHistory(selectedId: ReturnType<typeof ref<string>>) {
  const historyItems = ref<HistoryEntry[]>([])
  const historyTotal = ref(0)
  const historyPage = ref(1)
  const loadingHistory = ref(false)

  const historyFilters = reactive({
    limit: 20,
  })

  async function loadHistory(reset = false) {
    if (!selectedId.value) return
    if (reset) historyPage.value = 1
    loadingHistory.value = true
    try {
      const offset = (historyPage.value - 1) * historyFilters.limit
      const page = await apiGet<HistoryPage>(`/api/threads/${selectedId.value}/history`, {
        offset,
        limit: historyFilters.limit,
      })
      if (page.items.length === 0 && page.total_matched > 0 && historyPage.value > 1) {
        historyPage.value = Math.ceil(page.total_matched / historyFilters.limit)
        await loadHistory(false)
        return
      }
      historyItems.value = page.items
      historyTotal.value = page.total_matched
    } catch (error) {
      ElMessage.error(messageOf(error))
    } finally {
      loadingHistory.value = false
    }
  }

  async function changeHistoryPage(page: number) {
    historyPage.value = page
    await loadHistory(false)
  }

  function messageOf(error: unknown) {
    return error instanceof Error ? error.message : String(error)
  }

  return {
    historyItems,
    historyTotal,
    historyPage,
    loadingHistory,
    historyFilters,
    loadHistory,
    changeHistoryPage,
  }
}
