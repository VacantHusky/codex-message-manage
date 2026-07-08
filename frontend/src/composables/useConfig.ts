import { ref } from 'vue'
import { apiGet, apiPost, type ManagerConfig, type Overview, type UpdateDataDirResponse, type BrowseResponse } from '../api'
import { ElMessage } from 'element-plus'

export function useConfig(onDataDirChanged?: () => void | Promise<void>) {
  const config = ref<ManagerConfig>()
  const overview = ref<Overview>()
  const changeDataDirDialog = ref(false)
  const newDataDir = ref('')
  const changingDataDir = ref(false)

  // 文件夹浏览状态
  const browseDialogVisible = ref(false)
  const browseLoading = ref(false)
  const browseResult = ref<BrowseResponse>()

  async function loadConfig() {
    try {
      config.value = await apiGet<ManagerConfig>('/api/config')
    } catch (error) {
      ElMessage.error(messageOf(error))
    }
  }

  async function loadOverview() {
    try {
      overview.value = await apiGet<Overview>('/api/overview')
    } catch (error) {
      ElMessage.error(messageOf(error))
    }
  }

  function openChangeDataDir() {
    newDataDir.value = config.value?.data_dir || ''
    changeDataDirDialog.value = true
  }

  async function openBrowse(path?: string) {
    browseLoading.value = true
    try {
      const params: Record<string, string> = {}
      if (path) params.path = path
      browseResult.value = await apiGet<BrowseResponse>('/api/browse', params)
      browseDialogVisible.value = true
    } catch (error) {
      ElMessage.error(messageOf(error))
    } finally {
      browseLoading.value = false
    }
  }

  async function browseTo(path: string) {
    browseLoading.value = true
    try {
      browseResult.value = await apiGet<BrowseResponse>('/api/browse', { path })
    } catch (error) {
      ElMessage.error(messageOf(error))
    } finally {
      browseLoading.value = false
    }
  }

  function selectBrowseDir() {
    if (browseResult.value) {
      newDataDir.value = browseResult.value.current
    }
    browseDialogVisible.value = false
  }

  async function changeDataDir() {
    if (!newDataDir.value.trim()) {
      ElMessage.warning('请输入数据目录路径')
      return false
    }

    changingDataDir.value = true
    try {
      const result = await apiPost<UpdateDataDirResponse>('/api/config/data-dir', {
        data_dir: newDataDir.value.trim(),
      })
      ElMessage.success(result.message)
      changeDataDirDialog.value = false
      // 刷新 config 以同步最新的 data_dir
      await loadConfig()
      // 直接更新 overview 中的 data_dir
      if (overview.value) {
        overview.value = { ...overview.value, data_dir: result.data_dir }
      }
      // 自动重新加载数据
      if (onDataDirChanged) {
        await onDataDirChanged()
      }
      return true
    } catch (error) {
      ElMessage.error(messageOf(error))
      return false
    } finally {
      changingDataDir.value = false
    }
  }

  function messageOf(error: unknown) {
    return error instanceof Error ? error.message : String(error)
  }

  return {
    config,
    overview,
    changeDataDirDialog,
    newDataDir,
    changingDataDir,
    browseDialogVisible,
    browseLoading,
    browseResult,
    loadConfig,
    loadOverview,
    openChangeDataDir,
    changeDataDir,
    openBrowse,
    browseTo,
    selectBrowseDir,
  }
}
