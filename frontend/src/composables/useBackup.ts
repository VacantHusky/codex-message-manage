import { ref } from 'vue'
import { apiGet, apiPost, type BackupInfo, type BackupList } from '../api'
import { ElMessage, ElMessageBox } from 'element-plus'

export function useBackup() {
  const backups = ref<BackupInfo[]>([])
  const backupDialog = ref(false)
  const restoring = ref('')
  const deleting = ref('')

  async function openBackups() {
    backupDialog.value = true
    const data = await apiGet<BackupList>('/api/backups')
    backups.value = data.items
  }

  async function restoreBackup(item: BackupInfo) {
    try {
      await ElMessageBox.confirm(
        '恢复会把备份中的会话文件、history 和 SQLite 文件复制回原路径。',
        '恢复确认',
        {
          confirmButtonText: '恢复',
          cancelButtonText: '取消',
          type: 'warning',
        },
      )
      restoring.value = item.id
      await apiPost(`/api/backups/${item.id}/restore`, { confirm: true })
      ElMessage.success('已恢复备份')
      return true
    } catch (error) {
      if (error instanceof Error) ElMessage.error(error.message)
      return false
    } finally {
      restoring.value = ''
    }
  }

  async function deleteBackup(item: BackupInfo) {
    try {
      await ElMessageBox.confirm(
        `删除备份 ${item.id} 将移除备份目录中的所有文件，且不可恢复。`,
        '删除备份确认',
        {
          confirmButtonText: '删除',
          cancelButtonText: '取消',
          type: 'warning',
        },
      )
      deleting.value = item.id
      await apiPost(`/api/backups/${item.id}/delete`, { confirm: true })
      backups.value = backups.value.filter(b => b.id !== item.id)
      ElMessage.success('备份已删除')
      return true
    } catch {
      return false
    } finally {
      deleting.value = ''
    }
  }

  return {
    backups,
    backupDialog,
    restoring,
    deleting,
    openBackups,
    restoreBackup,
    deleteBackup,
  }
}
