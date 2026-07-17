import type { ThreadSummary, SessionEvent } from '../api'

export function fullTitle(thread?: ThreadSummary) {
  return thread?.title || thread?.first_user_message || thread?.id || '-'
}

export function compactTitle(thread?: ThreadSummary, maxLength = 80) {
  const value = fullTitle(thread).replace(/\s+/g, ' ').trim()
  if (value.length <= maxLength) return value
  return `${value.slice(0, maxLength)}...`
}

export function summaryText(thread?: ThreadSummary) {
  const value = thread?.first_user_message || thread?.preview || thread?.title || ''
  return value.trim()
}

export function formatBytes(bytes?: number) {
  if (bytes === undefined || Number.isNaN(bytes)) return '-'
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB']
  let value = bytes / 1024
  let unit = units[0]
  for (let i = 1; i < units.length && value >= 1024; i += 1) {
    value /= 1024
    unit = units[i]
  }
  return `${value.toFixed(value >= 10 ? 1 : 2)} ${unit}`
}

export function formatCount(value?: number) {
  if (value === undefined || Number.isNaN(value)) return '-'
  return new Intl.NumberFormat('zh-CN').format(value)
}

export function formatCompactCount(value?: number) {
  if (value === undefined || Number.isNaN(value)) return '-'
  if (Math.abs(value) < 1000) return formatCount(value)
  const units = ['K', 'M', 'B']
  let next = value / 1000
  let unit = units[0]
  for (let i = 1; i < units.length && Math.abs(next) >= 1000; i += 1) {
    next /= 1000
    unit = units[i]
  }
  return `${next.toFixed(next >= 10 ? 1 : 2)}${unit}`
}

export function runtimeLabel(value?: string) {
  if (!value) return '-'
  const labels: Record<string, string> = {
    disabled: '无沙箱',
    'read-only': '只读',
    'workspace-write': '工作区写入',
    'danger-full-access': '完全访问',
    enabled: '启用',
    disabled_memory: '禁用',
    untrusted: '不信任',
    'on-failure': '失败时确认',
    'on-request': '按需确认',
    never: '不确认',
    user: '用户',
    cli: 'CLI',
  }
  return labels[value] ?? value
}

export function boolLabel(value: boolean) {
  return value ? '是' : '否'
}

export function eventTagType(event: SessionEvent) {
  if (event.payload_type === 'token_count') return 'info'
  if (event.payload_type === 'message') return event.role === 'user' ? 'primary' : 'success'
  if (event.payload_type?.includes('tool') || event.payload_type?.includes('function')) return 'warning'
  if (event.event_type === 'event_msg') return 'info'
  return ''
}

export const payloadTypeLabels: Record<string, string> = {
  message: '消息',
  function_call: '函数调用',
  function_call_output: '函数输出',
  custom_tool_call: '自定义工具调用',
  custom_tool_call_output: '自定义工具输出',
  reasoning: '推理',
  user_message: '用户消息',
  agent_message: '助手消息',
  task_started: '任务开始',
  task_complete: '任务完成',
  token_count: 'Token 统计',
}

export const eventTypeLabels: Record<string, string> = {
  response_item: '响应项',
  event_msg: '事件消息',
  turn_context: '回合上下文',
  session_meta: '会话元信息',
  compacted: '压缩记录',
}

export const eventTypeOptions = [
  'response_item',
  'event_msg',
  'turn_context',
  'session_meta',
  'compacted',
]

export const payloadTypeOptions = [
  'message',
  'function_call',
  'function_call_output',
  'custom_tool_call',
  'custom_tool_call_output',
  'reasoning',
  'user_message',
  'agent_message',
  'task_started',
  'task_complete',
  'token_count',
]

export function payloadTypeName(type?: string) {
  if (!type) return ''
  return payloadTypeLabels[type] ?? type
}

export function eventTypeName(type: string) {
  return eventTypeLabels[type] ?? type
}

export function searchSourceName(source: string) {
  const labels: Record<string, string> = {
    threads: '会话信息',
    history: '历史',
    events: '时间线',
  }
  return labels[source] ?? source
}

export function searchFieldName(field: string) {
  const labels: Record<string, string> = {
    title: '标题',
    cwd: '项目目录',
    first_user_message: '首条用户消息',
    preview: '预览',
    text: '正文',
  }
  const translated = labels[field] ?? payloadTypeName(field)
  return translated === field ? eventTypeName(field) : translated
}

export function eventLabel(event: SessionEvent) {
  return event.payload_type ? payloadTypeName(event.payload_type) : eventTypeName(event.event_type)
}

export async function copyText(text?: string): Promise<boolean> {
  if (!text) return false
  try {
    await navigator.clipboard.writeText(text)
    return true
  } catch {
    return false
  }
}

export function formatTimestamp(ts?: string): string {
  if (!ts) return '-'
  const date = new Date(ts)
  if (Number.isNaN(date.getTime())) return ts
  const now = Date.now()
  const diff = now - date.getTime()
  if (diff < 0) return ts
  if (diff < 60_000) return '刚刚'
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)} 分钟前`
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)} 小时前`
  if (diff < 604_800_000) return `${Math.floor(diff / 86_400_000)} 天前`
  const y = date.getFullYear()
  const m = String(date.getMonth() + 1).padStart(2, '0')
  const d = String(date.getDate()).padStart(2, '0')
  const hh = String(date.getHours()).padStart(2, '0')
  const mm = String(date.getMinutes()).padStart(2, '0')
  return `${y}-${m}-${d} ${hh}:${mm}`
}

export function formatDateTime(ts?: string): string {
  if (!ts) return '-'
  const date = new Date(ts)
  if (Number.isNaN(date.getTime())) return ts
  const y = date.getFullYear()
  const m = String(date.getMonth() + 1).padStart(2, '0')
  const d = String(date.getDate()).padStart(2, '0')
  const hh = String(date.getHours()).padStart(2, '0')
  const mm = String(date.getMinutes()).padStart(2, '0')
  const ss = String(date.getSeconds()).padStart(2, '0')
  return `${y}-${m}-${d} ${hh}:${mm}:${ss}`
}

export function eventSummary(event: SessionEvent): string {
  const text = event.display_text ?? ''
  if (!text) return '(空)'
  const first = text.split('\n')[0]
  return first.length > 120 ? `${first.slice(0, 120)}…` : first
}

export function messageOf(error: unknown) {
  return error instanceof Error ? error.message : String(error)
}
