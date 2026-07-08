import { ref, computed } from 'vue'
import { apiGet, apiPost, type ThreadDetail, type ThreadSummary, type UpdateRuntimeRequest } from '../api'
import { ElMessage, ElMessageBox } from 'element-plus'

export function useThreadDetail(selectedId: ReturnType<typeof ref<string>>) {
  const loadingDetail = ref(false)
  const detail = ref<ThreadDetail>()
  const activeTab = ref('timeline')

  const selectedThread = computed(() => detail.value?.thread)

  async function loadThreadDetail(id: string) {
    loadingDetail.value = true
    try {
      detail.value = await apiGet<ThreadDetail>(`/api/threads/${id}`)
    } catch (error) {
      ElMessage.error(messageOf(error))
    } finally {
      loadingDetail.value = false
    }
  }

  async function updateTitle(title: string) {
    if (!selectedThread.value) return
    try {
      await apiPost(`/api/threads/${selectedThread.value.id}/title`, {
        title,
      })
      ElMessage.success('标题已更新')
      return true
    } catch (error) {
      ElMessage.error(messageOf(error))
      return false
    }
  }

  async function archiveSelected(archived: boolean) {
    if (!selectedId.value) return
    try {
      await apiPost(`/api/threads/${selectedId.value}/archive`, { archived })
      ElMessage.success(archived ? '已归档' : '已取消归档')
      return true
    } catch (error) {
      ElMessage.error(messageOf(error))
      return false
    }
  }

  async function updateRuntime(request: UpdateRuntimeRequest) {
    if (!selectedId.value) return false
    try {
      const result = await apiPost<{ message: string }>(
        `/api/threads/${selectedId.value}/runtime`,
        request,
      )
      ElMessage.success(result.message)
      return true
    } catch (error) {
      ElMessage.error(messageOf(error))
      return false
    }
  }

  async function backupSelected(backingUp: ReturnType<typeof ref<boolean>>) {
    if (!selectedId.value) return
    backingUp.value = true
    try {
      const result = await apiPost<{ message: string; backup_dir?: string }>(
        `/api/threads/${selectedId.value}/backup`,
        { note: fullTitle(selectedThread.value) },
      )
      ElMessage.success(result.backup_dir ? `${result.message}: ${result.backup_dir}` : result.message)
      return true
    } catch (error) {
      ElMessage.error(messageOf(error))
      return false
    } finally {
      backingUp.value = false
    }
  }

  async function clearLogs(clearingLogs: ReturnType<typeof ref<boolean>>) {
    if (!selectedId.value) return
    try {
      await ElMessageBox.confirm(
        '清理会删除当前会话在 logs_2.sqlite 中的日志行，且不会自动备份。',
        '清理日志确认',
        {
          confirmButtonText: '清理日志',
          cancelButtonText: '取消',
          type: 'warning',
        },
      )
      clearingLogs.value = true
      const result = await apiPost<{ message: string }>(
        `/api/threads/${selectedId.value}/logs/clear`,
        { confirm: true },
      )
      ElMessage.success(result.message)
      return true
    } catch (error) {
      if (error instanceof Error) ElMessage.error(error.message)
      return false
    } finally {
      clearingLogs.value = false
    }
  }

  function fullTitle(thread?: ThreadSummary) {
    return thread?.title || thread?.first_user_message || thread?.id || '-'
  }

  function messageOf(error: unknown) {
    return error instanceof Error ? error.message : String(error)
  }

  return {
    loadingDetail,
    detail,
    activeTab,
    selectedThread,
    loadThreadDetail,
    updateTitle,
    archiveSelected,
    updateRuntime,
    backupSelected,
    clearLogs,
    fullTitle,
  }
}
