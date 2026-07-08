import { ref } from 'vue'
import { apiGet, type StatsResponse } from '../api'
import { ElMessage } from 'element-plus'

export function useStats() {
  const statsData = ref<StatsResponse>()
  const statsDialog = ref(false)

  async function loadStats() {
    try {
      statsData.value = await apiGet<StatsResponse>('/api/stats')
    } catch (error) {
      ElMessage.error(messageOf(error))
    }
  }

  async function openStats() {
    statsDialog.value = true
    if (!statsData.value) {
      await loadStats()
    }
  }

  function messageOf(error: unknown) {
    return error instanceof Error ? error.message : String(error)
  }

  return {
    statsData,
    statsDialog,
    openStats,
  }
}
