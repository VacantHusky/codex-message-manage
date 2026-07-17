use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct ThreadQuery {
    pub q: Option<String>,
    pub cwd: Option<String>,
    pub model: Option<String>,
    pub archived: Option<bool>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub size_min: Option<u64>,
    pub size_max: Option<u64>,
    pub token_min: Option<i64>,
    pub token_max: Option<i64>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct EventQuery {
    pub event_type: Option<String>,
    pub payload_type: Option<String>,
    pub role: Option<String>,
    pub q: Option<String>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default)]
    pub regex: bool,
    pub thread_id: Option<String>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ReplaceSearchRequest {
    pub q: String,
    #[serde(default)]
    pub regex: bool,
    pub replacement: String,
    pub confirm: bool,
    pub target: Option<SearchTarget>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SearchTarget {
    pub source: String,
    pub thread_id: String,
    pub event_index: Option<usize>,
    pub history_ts: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateArchiveRequest {
    pub archived: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteRequest {
    pub confirm: bool,
    #[serde(default)]
    pub backup: bool,
}

#[derive(Debug, Deserialize)]
pub struct ClearLogsRequest {
    pub confirm: bool,
}

#[derive(Debug, Deserialize)]
pub struct DeleteEventRequest {
    pub confirm: bool,
    #[serde(default)]
    pub delete_after: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEventRequest {
    pub raw: Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateHistoryRequest {
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteHistoryRequest {
    pub confirm: bool,
}

#[derive(Debug, Deserialize)]
pub struct BackupRequest {
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RestoreRequest {
    pub confirm: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTitleRequest {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRuntimeRequest {
    pub model_provider: String,
    pub sandbox_type: String,
    pub approval_mode: String,
    pub memory_mode: String,
    pub thread_source: Option<String>,
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDataDirRequest {
    pub data_dir: String,
}

#[derive(Debug, Deserialize)]
pub struct BrowseRequest {
    pub path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BrowseResponse {
    pub current: String,
    pub parent: Option<String>,
    pub directories: Vec<BrowseItem>,
}

#[derive(Debug, Serialize)]
pub struct BrowseItem {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ManagerConfig {
    pub data_dir: String,
    pub bind: String,
    pub trash_dir: String,
    pub metadata_path: String,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub by_cwd: Vec<CountItem>,
    pub by_model_provider: Vec<CountItem>,
    pub by_model: Vec<CountItem>,
    pub by_reasoning_effort: Vec<CountItem>,
    pub by_sandbox_policy: Vec<CountItem>,
    pub by_approval_mode: Vec<CountItem>,
    pub by_memory_mode: Vec<CountItem>,
    pub by_thread_source: Vec<CountItem>,
    pub by_cli_version: Vec<CountItem>,
    pub by_day: Vec<CountItem>,
    pub largest_sessions: Vec<SessionSizeItem>,
    pub event_types: Vec<CountItem>,
    pub payload_types: Vec<CountItem>,
    pub model_cache: Option<ModelCacheSummary>,
}

#[derive(Debug, Serialize)]
pub struct ModelCacheSummary {
    pub fetched_at: Option<String>,
    pub client_version: Option<String>,
    pub models: Vec<ModelCacheItem>,
}

#[derive(Debug, Serialize)]
pub struct ModelCacheItem {
    pub slug: String,
    pub display_name: String,
    pub default_reasoning_level: Option<String>,
    pub context_window: Option<i64>,
    pub supports_parallel_tool_calls: bool,
    pub supports_image_detail_original: bool,
    pub supports_search_tool: bool,
    pub visibility: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionSizeItem {
    pub thread_id: String,
    pub title: String,
    pub cwd: String,
    pub bytes: u64,
    pub path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Overview {
    pub data_dir: String,
    pub thread_count: usize,
    pub archived_count: usize,
    pub session_file_count: usize,
    pub total_session_bytes: u64,
    pub max_tokens_used: i64,
    pub first_created: Option<String>,
    pub last_updated: Option<String>,
    pub by_cwd: Vec<CountItem>,
    pub by_model_provider: Vec<CountItem>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CountItem {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct ThreadSummary {
    pub id: String,
    pub rollout_path: String,
    pub resolved_rollout_path: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub created_at_text: String,
    pub updated_at_text: String,
    pub source: String,
    pub thread_source: Option<String>,
    pub model_provider: String,
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    pub sandbox_policy: String,
    pub sandbox_type: String,
    pub approval_mode: String,
    pub memory_mode: String,
    pub cwd: String,
    pub title: String,
    pub original_title: String,
    pub tokens_used: i64,
    pub session_bytes: u64,
    pub has_user_event: bool,
    pub archived: bool,
    pub cli_version: String,
    pub first_user_message: String,
    pub preview: String,
    pub recency_at_ms: i64,
    pub recency_at_text: String,
}

#[derive(Debug, Serialize)]
pub struct ThreadPage {
    pub items: Vec<ThreadSummary>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Debug, Serialize)]
pub struct ThreadDetail {
    pub thread: ThreadSummary,
    pub goal: Option<ThreadGoal>,
    pub logs: Vec<LogEntry>,
    pub history: Vec<HistoryEntry>,
    pub file: Option<FileInfo>,
}

#[derive(Debug, Serialize)]
pub struct ThreadGoal {
    pub goal_id: String,
    pub objective: String,
    pub status: String,
    pub token_budget: Option<i64>,
    pub tokens_used: i64,
    pub time_used_seconds: i64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Serialize)]
pub struct LogEntry {
    pub id: i64,
    pub ts: i64,
    pub ts_nanos: i64,
    pub ts_text: String,
    pub level: String,
    pub target: String,
    pub body: Option<String>,
    pub module_path: Option<String>,
    pub file: Option<String>,
    pub line: Option<i64>,
    pub process_uuid: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HistoryEntry {
    pub session_id: String,
    pub ts: i64,
    pub ts_text: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct HistoryPage {
    pub items: Vec<HistoryEntry>,
    pub total_matched: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub path: String,
    pub bytes: u64,
    pub modified: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EventPage {
    pub items: Vec<SessionEvent>,
    pub total_matched: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Serialize)]
pub struct SessionEvent {
    pub index: usize,
    pub timestamp: String,
    pub event_type: String,
    pub payload_type: Option<String>,
    pub role: Option<String>,
    pub display_text: Option<String>,
    pub raw: Value,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub q: String,
    pub items: Vec<SearchHit>,
    pub total: usize,
    pub thread_count: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub thread_id: String,
    pub title: Option<String>,
    pub cwd: Option<String>,
    pub source: String,
    pub field: String,
    pub timestamp: Option<String>,
    pub snippet: String,
    pub replaceable: bool,
    pub event_index: Option<usize>,
    pub history_ts: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ReplaceSearchResponse {
    pub ok: bool,
    pub replaced_items: usize,
    pub replaced_matches: usize,
    pub thread_count: usize,
    pub backup_id: String,
}

#[derive(Debug, Serialize)]
pub struct DeletePreview {
    pub thread_id: String,
    pub files: Vec<FileImpact>,
    pub database_rows: BTreeMap<String, i64>,
    pub history_rows: i64,
}

#[derive(Debug, Serialize)]
pub struct FileImpact {
    pub path: String,
    pub exists: bool,
    pub bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ThreadWriteResult {
    pub ok: bool,
    pub thread_id: String,
    pub message: String,
    pub backup_dir: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BackupInfo {
    pub id: String,
    pub thread_id: String,
    pub created_at: String,
    pub note: Option<String>,
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct BackupList {
    pub items: Vec<BackupInfo>,
}

#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub filename: String,
    pub content_type: String,
    pub content: String,
}
