import { ref } from 'vue'
import { apiPost, type DeletePreview } from '../api'
import { ElMessage, ElMessageBox } from 'element-plus'

export function useDelete() {
  const deleteDialog = ref(false)
  const deletePreview = ref<DeletePreview>()
  const deleteConfirm = ref(false)
  const deleting = ref(false)

  async function previewDelete(selectedId: string) {
    if (!selectedId) return
    try {
      await ElMessageBox.confirm('删除会修改 Codex 原始数据，且不会自动备份。', '删除确认', {
        confirmButtonText: '查看影响范围',
        cancelButtonText: '取消',
        type: 'warning',
      })
      deletePreview.value = await apiPost<DeletePreview>(
        `/api/threads/${selectedId}/delete/preview`,
        {},
      )
      deleteConfirm.value = false
      deleteDialog.value = true
    } catch {
      // user cancelled
    }
  }

  async function confirmDelete(selectedId: string) {
    if (!selectedId || !deleteConfirm.value) return
    deleting.value = true
    try {
      const result = await apiPost<{ message: string; backup_dir?: string }>(
        `/api/threads/${selectedId}/delete`,
        { confirm: true },
      )
      ElMessage.success(result.message)
      deleteDialog.value = false
      return true
    } catch (error) {
      ElMessage.error(messageOf(error))
      return false
    } finally {
      deleting.value = false
    }
  }

  function messageOf(error: unknown) {
    return error instanceof Error ? error.message : String(error)
  }

  return {
    deleteDialog,
    deletePreview,
    deleteConfirm,
    deleting,
    previewDelete,
    confirmDelete,
  }
}
