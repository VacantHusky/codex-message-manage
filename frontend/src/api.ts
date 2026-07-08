export interface CountItem {
  name: string
  count: number
}

export interface ManagerConfig {
  data_dir: string
  bind: string
  trash_dir: string
  metadata_path: string
}

export interface Overview {
  data_dir: string
  thread_count: number
  archived_count: number
  session_file_count: number
  total_session_bytes: number
  max_tokens_used: number
  first_created?: string
  last_updated?: string
  by_cwd: CountItem[]
  by_model_provider: CountItem[]
}

export interface ThreadSummary {
  id: string
  rollout_path: string
  resolved_rollout_path?: string
  created_at_text: string
  updated_at_text: string
  recency_at_text: string
  source: string
  thread_source?: string
  model_provider: string
  model?: string
  reasoning_effort?: string
  sandbox_policy: string
  sandbox_type: string
  approval_mode: string
  memory_mode: string
  cwd: string
  title: string
  original_title: string
  tokens_used: number
  session_bytes: number
  has_user_event: boolean
  archived: boolean
  cli_version: string
  first_user_message: string
  preview: string
}

export interface ThreadPage {
  items: ThreadSummary[]
  total: number
  page: number
  page_size: number
}

export interface HistoryEntry {
  ts_text: string
  text: string
}

export interface LogEntry {
  id: number
  ts_text: string
  level: string
  target: string
  body?: string
  file?: string
  line?: number
}

export interface ThreadDetail {
  thread: ThreadSummary
  goal?: {
    objective: string
    status: string
    token_budget?: number
    tokens_used: number
  }
  logs: LogEntry[]
  history: HistoryEntry[]
  file?: {
    path: string
    bytes: number
    modified?: string
  }
}

export interface UpdateRuntimeRequest {
  model_provider: string
  sandbox_type: string
  approval_mode: string
  memory_mode: string
  thread_source?: string
  reasoning_effort?: string
}

export interface SessionEvent {
  index: number
  timestamp: string
  event_type: string
  payload_type?: string
  role?: string
  display_text?: string
  raw: unknown
}

export interface EventPage {
  items: SessionEvent[]
  total_matched: number
  offset: number
  limit: number
}

export interface SearchHit {
  thread_id: string
  title?: string
  cwd?: string
  source: string
  field: string
  timestamp?: string
  snippet: string
}

export interface SearchResponse {
  q: string
  items: SearchHit[]
  limit: number
}

export interface DeletePreview {
  thread_id: string
  files: Array<{ path: string; exists: boolean; bytes?: number }>
  database_rows: Record<string, number>
  history_rows: number
}

export interface ExportResponse {
  filename: string
  content_type: string
  content: string
}

export interface BackupInfo {
  id: string
  thread_id: string
  created_at: string
  note?: string
  path: string
  bytes: number
}

export interface BackupList {
  items: BackupInfo[]
}

export interface SessionSizeItem {
  thread_id: string
  title: string
  cwd: string
  bytes: number
  path?: string
}

export interface StatsResponse {
  by_cwd: CountItem[]
  by_model_provider: CountItem[]
  by_model: CountItem[]
  by_reasoning_effort: CountItem[]
  by_sandbox_policy: CountItem[]
  by_approval_mode: CountItem[]
  by_memory_mode: CountItem[]
  by_thread_source: CountItem[]
  by_cli_version: CountItem[]
  by_day: CountItem[]
  largest_sessions: SessionSizeItem[]
  event_types: CountItem[]
  payload_types: CountItem[]
  model_cache?: ModelCacheSummary
}

export interface ModelCacheSummary {
  fetched_at?: string
  client_version?: string
  models: ModelCacheItem[]
}

export interface ModelCacheItem {
  slug: string
  display_name: string
  default_reasoning_level?: string
  context_window?: number
  supports_parallel_tool_calls: boolean
  supports_image_detail_original: boolean
  supports_search_tool: boolean
  visibility?: string
}

export interface UpdateDataDirResponse {
  ok: boolean
  data_dir: string
  state_db: string
  goals_db: string
  logs_db: string
  sessions_dir: string
  history_path: string
  message: string
}

export interface BrowseItem {
  name: string
  path: string
}

export interface BrowseResponse {
  current: string
  parent?: string
  directories: BrowseItem[]
}

export async function apiGet<T>(path: string, params?: Record<string, unknown>): Promise<T> {
  const url = new URL(path, window.location.origin)
  for (const [key, value] of Object.entries(params ?? {})) {
    if (value !== undefined && value !== null && value !== '') {
      url.searchParams.set(key, String(value))
    }
  }
  const res = await fetch(url)
  return parseResponse<T>(res)
}

export async function apiPost<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(path, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  })
  return parseResponse<T>(res)
}

async function parseResponse<T>(res: Response): Promise<T> {
  const data = await res.json().catch(() => ({}))
  if (!res.ok) {
    throw new Error(data.error ?? `${res.status} ${res.statusText}`)
  }
  return data as T
}
