use std::{
    collections::{BTreeMap, HashMap},
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    str::FromStr,
    sync::RwLock,
    time::{SystemTime, UNIX_EPOCH},
};

/// Windows: 用 MoveFileExW 替换被内存映射锁定的文件
/// 先写入临时文件，再原子替换目标
#[cfg(target_os = "windows")]
fn windows_replace_file(source: &Path, target: &Path) -> Result<(), std::io::Error> {
    use std::os::windows::ffi::OsStrExt;

    unsafe extern "system" {
        fn MoveFileExW(
            lpExistingFileName: *const u16,
            lpNewFileName: *const u16,
            dwFlags: u32,
        ) -> i32;
        fn GetLastError() -> u32;
    }

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_COPY_ALLOWED: u32 = 0x2;

    let tmp = target.with_extension("restore_swap");
    fs::copy(source, &tmp)?;

    let wide_old: Vec<u16> = tmp
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let wide_new: Vec<u16> = target
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let ok = unsafe {
        MoveFileExW(
            wide_old.as_ptr(),
            wide_new.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_COPY_ALLOWED,
        )
    };
    if ok == 0 {
        let err = unsafe { GetLastError() };
        let _ = fs::remove_file(&tmp);
        return Err(std::io::Error::from_raw_os_error(err as i32));
    }
    Ok(())
}

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use thiserror::Error;
use walkdir::WalkDir;

use crate::models::*;

enum RewriteEventAction {
    Keep,
    Delete,
    Replace(String),
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ManagerMetadata {
    #[serde(default)]
    thread_titles: HashMap<String, String>,
    #[serde(default)]
    backups: Vec<BackupManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupManifest {
    id: String,
    thread_id: String,
    created_at: String,
    note: Option<String>,
    path: String,
    files: Vec<BackupFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupFile {
    kind: String,
    original_path: String,
    backup_path: String,
    bytes: u64,
}

#[derive(Debug)]
pub struct AppStore {
    config_path: PathBuf,
    config: ManagerConfig,
    data_dir: RwLock<PathBuf>,
    state_db: RwLock<PathBuf>,
    goals_db: RwLock<PathBuf>,
    logs_db: RwLock<PathBuf>,
    sessions_dir: RwLock<PathBuf>,
    history_path: RwLock<PathBuf>,
    trash_dir: PathBuf,
    metadata_path: PathBuf,
}

impl AppStore {
    pub fn with_config(config_path: PathBuf, config: ManagerConfig) -> Result<Self, StoreError> {
        let data_dir = PathBuf::from(&config.data_dir);
        let data_dir = if data_dir.is_absolute() {
            data_dir
        } else {
            std::env::current_dir()?.join(data_dir)
        };
        let trash_dir = path_from_config(&config.trash_dir)?;
        let metadata_path = path_from_config(&config.metadata_path)?;
        Ok(Self {
            config_path,
            state_db: RwLock::new(data_dir.join("state_5.sqlite")),
            goals_db: RwLock::new(data_dir.join("goals_1.sqlite")),
            logs_db: RwLock::new(data_dir.join("logs_2.sqlite")),
            sessions_dir: RwLock::new(data_dir.join("sessions")),
            history_path: RwLock::new(data_dir.join("history.jsonl")),
            config,
            trash_dir,
            metadata_path,
            data_dir: RwLock::new(data_dir),
        })
    }

    pub fn data_dir(&self) -> String {
        self.data_dir.read().unwrap().display().to_string()
    }

    pub fn config(&self) -> ManagerConfig {
        ManagerConfig {
            data_dir: self.data_dir.read().unwrap().display().to_string(),
            bind: self.config.bind.clone(),
            trash_dir: self.trash_dir.display().to_string(),
            metadata_path: self.metadata_path.display().to_string(),
        }
    }

    pub async fn validate(&self) -> Result<(), StoreError> {
        let data_dir = self.data_dir.read().unwrap();
        if !data_dir.exists() {
            return Err(StoreError::NotFound(format!(
                "data dir {}",
                data_dir.display()
            )));
        }
        let state_db = self.state_db.read().unwrap();
        let goals_db = self.goals_db.read().unwrap();
        let logs_db = self.logs_db.read().unwrap();
        for path in [&*state_db, &*goals_db, &*logs_db] {
            if !path.exists() {
                return Err(StoreError::NotFound(path.display().to_string()));
            }
        }
        Ok(())
    }

    pub async fn overview(&self) -> Result<Overview, StoreError> {
        let threads = self.load_threads().await?;
        let mut by_cwd = count_by(threads.iter().map(|t| t.cwd.clone()));
        let mut by_model_provider = count_by(threads.iter().map(|t| t.model_provider.clone()));
        by_cwd.truncate(12);
        by_model_provider.truncate(12);

        let mut session_file_count = 0usize;
        let mut total_session_bytes = 0u64;
        for path in self.session_files() {
            session_file_count += 1;
            total_session_bytes += path.metadata().map(|m| m.len()).unwrap_or(0);
        }

        Ok(Overview {
            data_dir: self.data_dir(),
            thread_count: threads.len(),
            archived_count: threads.iter().filter(|t| t.archived).count(),
            session_file_count,
            total_session_bytes,
            max_tokens_used: threads.iter().map(|t| t.tokens_used).max().unwrap_or(0),
            first_created: threads
                .iter()
                .map(|t| t.created_at)
                .min()
                .map(timestamp_to_text),
            last_updated: threads
                .iter()
                .map(|t| t.updated_at)
                .max()
                .map(timestamp_to_text),
            by_cwd,
            by_model_provider,
        })
    }

    pub async fn stats(&self) -> Result<StatsResponse, StoreError> {
        let threads = self.load_threads().await?;
        let mut by_cwd = count_by(threads.iter().map(|thread| thread.cwd.clone()));
        let mut by_model_provider =
            count_by(threads.iter().map(|thread| thread.model_provider.clone()));
        let mut by_model = count_by(threads.iter().filter_map(|thread| thread.model.clone()));
        let mut by_reasoning_effort = count_by(
            threads
                .iter()
                .filter_map(|thread| thread.reasoning_effort.clone()),
        );
        let mut by_sandbox_policy =
            count_by(threads.iter().map(|thread| thread.sandbox_type.clone()));
        let mut by_approval_mode =
            count_by(threads.iter().map(|thread| thread.approval_mode.clone()));
        let mut by_memory_mode = count_by(threads.iter().map(|thread| thread.memory_mode.clone()));
        let mut by_thread_source = count_by(
            threads
                .iter()
                .filter_map(|thread| thread.thread_source.clone()),
        );
        let mut by_cli_version = count_by(threads.iter().map(|thread| thread.cli_version.clone()));
        let mut by_day = count_by(threads.iter().map(|thread| {
            thread
                .created_at_text
                .split('T')
                .next()
                .unwrap_or_default()
                .to_string()
        }));

        let mut largest_sessions: Vec<_> = threads
            .iter()
            .map(|thread| {
                let bytes = thread
                    .resolved_rollout_path
                    .as_deref()
                    .and_then(|path| Path::new(path).metadata().ok())
                    .map(|meta| meta.len())
                    .unwrap_or(0);
                SessionSizeItem {
                    thread_id: thread.id.clone(),
                    title: thread.title.clone(),
                    cwd: thread.cwd.clone(),
                    bytes,
                    path: thread.resolved_rollout_path.clone(),
                }
            })
            .collect();
        largest_sessions.sort_by(|a, b| b.bytes.cmp(&a.bytes));

        let mut event_type_counts: HashMap<String, usize> = HashMap::new();
        let mut payload_type_counts: HashMap<String, usize> = HashMap::new();
        for thread in &threads {
            let Some(path) = thread.resolved_rollout_path.as_deref() else {
                continue;
            };
            for_each_jsonl_value(Path::new(path), |_, raw| {
                if let Some(event_type) = raw.get("type").and_then(Value::as_str) {
                    *event_type_counts.entry(event_type.to_string()).or_default() += 1;
                }
                if let Some(payload_type) = raw
                    .get("payload")
                    .and_then(|payload| payload.get("type"))
                    .and_then(Value::as_str)
                {
                    *payload_type_counts
                        .entry(payload_type.to_string())
                        .or_default() += 1;
                }
                Ok(())
            })?;
        }

        by_cwd.truncate(20);
        by_model_provider.truncate(20);
        by_model.truncate(20);
        by_reasoning_effort.truncate(20);
        by_sandbox_policy.truncate(20);
        by_approval_mode.truncate(20);
        by_memory_mode.truncate(20);
        by_thread_source.truncate(20);
        by_cli_version.truncate(20);
        by_day.truncate(60);
        largest_sessions.truncate(20);

        Ok(StatsResponse {
            by_cwd,
            by_model_provider,
            by_model,
            by_reasoning_effort,
            by_sandbox_policy,
            by_approval_mode,
            by_memory_mode,
            by_thread_source,
            by_cli_version,
            by_day,
            largest_sessions,
            event_types: counts_from_map(event_type_counts),
            payload_types: counts_from_map(payload_type_counts),
            model_cache: self.model_cache_summary(),
        })
    }

    pub async fn threads(&self, query: ThreadQuery) -> Result<ThreadPage, StoreError> {
        let q = normalize_query(query.q.as_deref());
        let from_ms = query.date_from.as_deref().and_then(|s| date_to_ms(s, true));
        let to_ms = query.date_to.as_deref().and_then(|s| date_to_ms(s, false));
        let size_min = query.size_min;
        let size_max = query.size_max;
        let token_min = query.token_min;
        let token_max = query.token_max;
        let mut items: Vec<_> = self
            .load_threads()
            .await?
            .into_iter()
            .filter(|thread| {
                if let Some(archived) = query.archived
                    && thread.archived != archived
                {
                    return false;
                }
                if let Some(cwd) = query.cwd.as_deref()
                    && !cwd.is_empty()
                    && thread.cwd != cwd
                {
                    return false;
                }
                if let Some(model) = query.model.as_deref()
                    && !model.is_empty()
                    && thread.model_provider != model
                    && thread.model.as_deref() != Some(model)
                {
                    return false;
                }
                if let Some(from_ms) = from_ms
                    && thread.created_at_ms < from_ms
                {
                    return false;
                }
                if let Some(to_ms) = to_ms
                    && thread.created_at_ms > to_ms
                {
                    return false;
                }
                if let Some(size_min) = size_min
                    && thread.session_bytes < size_min
                {
                    return false;
                }
                if let Some(size_max) = size_max
                    && thread.session_bytes > size_max
                {
                    return false;
                }
                if let Some(token_min) = token_min
                    && thread.tokens_used < token_min
                {
                    return false;
                }
                if let Some(token_max) = token_max
                    && thread.tokens_used > token_max
                {
                    return false;
                }
                if let Some(q) = q.as_deref() {
                    return contains_ci(&thread.id, q)
                        || contains_ci(&thread.title, q)
                        || contains_ci(&thread.cwd, q)
                        || contains_ci(&thread.first_user_message, q)
                        || contains_ci(&thread.preview, q);
                }
                true
            })
            .collect();

        items.sort_by(|a, b| {
            b.recency_at_ms
                .cmp(&a.recency_at_ms)
                .then_with(|| b.id.cmp(&a.id))
        });
        let total = items.len();
        let page_size = query.page_size.unwrap_or(30).clamp(1, 200);
        let page = query.page.unwrap_or(1).max(1);
        let start = (page - 1) * page_size;
        let items = items.into_iter().skip(start).take(page_size).collect();
        Ok(ThreadPage {
            items,
            total,
            page,
            page_size,
        })
    }

    pub async fn thread_detail(&self, id: &str) -> Result<ThreadDetail, StoreError> {
        let thread = self.thread_by_id(id).await?;
        let goal = self.thread_goal(id).await?;
        let logs = self.thread_logs(id, 100).await?;
        let history = self.thread_history(id, 200)?;
        let file = thread
            .resolved_rollout_path
            .as_deref()
            .and_then(|path| file_info(Path::new(path)).ok());
        Ok(ThreadDetail {
            thread,
            goal,
            logs,
            history,
            file,
        })
    }

    pub async fn thread_events(
        &self,
        id: &str,
        query: EventQuery,
    ) -> Result<EventPage, StoreError> {
        let thread = self.thread_by_id(id).await?;
        let path = thread
            .resolved_rollout_path
            .as_deref()
            .ok_or_else(|| StoreError::NotFound(format!("rollout for thread {id}")))?;
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(100).clamp(1, 500);
        let q = normalize_query(query.q.as_deref());
        let mut total_matched = 0usize;
        let mut items = Vec::new();

        for_each_jsonl_value(Path::new(path), |index, raw| {
            let event = event_from_value(index, raw)?;
            if let Some(event_type) = query.event_type.as_deref()
                && !event_type.is_empty()
                && event.event_type != event_type
            {
                return Ok(());
            }
            if let Some(payload_type) = query.payload_type.as_deref()
                && !payload_type.is_empty()
                && event.payload_type.as_deref() != Some(payload_type)
            {
                return Ok(());
            }
            if let Some(role) = query.role.as_deref()
                && !role.is_empty()
                && event.role.as_deref() != Some(role)
            {
                return Ok(());
            }
            if let Some(q) = q.as_deref() {
                let hay = format!(
                    "{} {} {}",
                    event.event_type,
                    event.payload_type.as_deref().unwrap_or_default(),
                    event.display_text.as_deref().unwrap_or_default()
                );
                if !contains_ci(&hay, q) {
                    return Ok(());
                }
            }
            if total_matched >= offset && items.len() < limit {
                items.push(event);
            }
            total_matched += 1;
            Ok(())
        })?;

        Ok(EventPage {
            items,
            total_matched,
            offset,
            limit,
        })
    }

    pub fn thread_history_page(
        &self,
        id: &str,
        query: HistoryQuery,
    ) -> Result<HistoryPage, StoreError> {
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(20).clamp(1, 200);
        let history_path = self.history_path.read().unwrap();
        if !history_path.exists() {
            return Ok(HistoryPage {
                items: Vec::new(),
                total_matched: 0,
                offset,
                limit,
            });
        }

        let file = fs::File::open(&*history_path)?;
        let mut items = Vec::new();
        let mut total_matched = 0;
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(&line)?;
            let session_id = value
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if session_id != id {
                continue;
            }
            if total_matched >= offset && items.len() < limit {
                let ts = value.get("ts").and_then(Value::as_i64).unwrap_or_default();
                items.push(HistoryEntry {
                    session_id: session_id.to_string(),
                    ts,
                    ts_text: timestamp_to_text(ts),
                    text: value
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                });
            }
            total_matched += 1;
        }

        Ok(HistoryPage {
            items,
            total_matched,
            offset,
            limit,
        })
    }

    pub async fn search(&self, query: SearchQuery) -> Result<SearchResponse, StoreError> {
        let q = normalize_query(Some(&query.q))
            .ok_or_else(|| StoreError::BadRequest("q is required".to_string()))?;
        let limit = query.limit.unwrap_or(80).clamp(1, 300);
        let mut threads = self.load_threads().await?;
        if let Some(thread_id) = query.thread_id.as_deref() {
            threads.retain(|thread| thread.id == thread_id);
            if threads.is_empty() {
                return Err(StoreError::NotFound(thread_id.to_string()));
            }
        }
        let mut by_id = HashMap::new();
        for thread in &threads {
            by_id.insert(thread.id.clone(), thread.clone());
        }

        let mut items = Vec::new();
        for thread in &threads {
            for (field, text) in [
                ("title", thread.title.as_str()),
                ("cwd", thread.cwd.as_str()),
                ("first_user_message", thread.first_user_message.as_str()),
                ("preview", thread.preview.as_str()),
            ] {
                if contains_ci(text, &q) {
                    items.push(SearchHit {
                        thread_id: thread.id.clone(),
                        title: Some(thread.title.clone()),
                        cwd: Some(thread.cwd.clone()),
                        source: "threads".to_string(),
                        field: field.to_string(),
                        timestamp: Some(thread.updated_at_text.clone()),
                        snippet: snippet(text, &q),
                    });
                    if items.len() >= limit {
                        return Ok(SearchResponse { q, items, limit });
                    }
                }
            }
        }

        for entry in self.all_history()? {
            if !by_id.contains_key(&entry.session_id) {
                continue;
            }
            if contains_ci(&entry.text, &q) {
                let thread = by_id.get(&entry.session_id);
                items.push(SearchHit {
                    thread_id: entry.session_id.clone(),
                    title: thread.map(|t| t.title.clone()),
                    cwd: thread.map(|t| t.cwd.clone()),
                    source: "history".to_string(),
                    field: "text".to_string(),
                    timestamp: Some(entry.ts_text),
                    snippet: snippet(&entry.text, &q),
                });
                if items.len() >= limit {
                    return Ok(SearchResponse { q, items, limit });
                }
            }
        }

        for thread in &threads {
            let Some(path) = thread.resolved_rollout_path.as_deref() else {
                continue;
            };
            let mut reached_limit = false;
            for_each_jsonl_value(Path::new(path), |index, raw| {
                let event = event_from_value(index, raw)?;
                let Some(text) = event.display_text.as_deref() else {
                    return Ok(());
                };
                if contains_ci(text, &q) {
                    items.push(SearchHit {
                        thread_id: thread.id.clone(),
                        title: Some(thread.title.clone()),
                        cwd: Some(thread.cwd.clone()),
                        source: "events".to_string(),
                        field: event
                            .payload_type
                            .clone()
                            .unwrap_or_else(|| event.event_type.clone()),
                        timestamp: Some(event.timestamp),
                        snippet: snippet(text, &q),
                    });
                    if items.len() >= limit {
                        reached_limit = true;
                    }
                }
                Ok(())
            })?;
            if reached_limit {
                return Ok(SearchResponse { q, items, limit });
            }
        }

        Ok(SearchResponse { q, items, limit })
    }

    pub async fn export_thread(
        &self,
        id: &str,
        query: ExportQuery,
    ) -> Result<ExportResponse, StoreError> {
        let format = query.format.unwrap_or_else(|| "markdown".to_string());
        let detail = self.thread_detail(id).await?;
        match format.as_str() {
            "json" => self.export_thread_json(detail),
            "markdown" | "md" => self.export_thread_markdown(detail),
            other => Err(StoreError::BadRequest(format!(
                "unsupported export format: {other}"
            ))),
        }
    }

    fn export_thread_json(&self, detail: ThreadDetail) -> Result<ExportResponse, StoreError> {
        let mut events = Vec::new();
        if let Some(path) = detail.thread.resolved_rollout_path.as_deref() {
            for_each_jsonl_value(Path::new(path), |index, raw| {
                events.push(event_from_value(index, raw)?);
                Ok(())
            })?;
        }
        let content = serde_json::to_string_pretty(&serde_json::json!({
            "detail": detail,
            "events": events,
        }))?;
        Ok(ExportResponse {
            filename: format!("codex-thread-{}.json", safe_filename(&detail.thread.id)),
            content_type: "application/json;charset=utf-8".to_string(),
            content,
        })
    }

    fn export_thread_markdown(&self, detail: ThreadDetail) -> Result<ExportResponse, StoreError> {
        let mut content = String::new();
        content.push_str(&format!("# {}\n\n", detail.thread.title.trim()));
        content.push_str(&format!("- ID: `{}`\n", detail.thread.id));
        content.push_str(&format!("- CWD: `{}`\n", detail.thread.cwd));
        content.push_str(&format!("- Created: `{}`\n", detail.thread.created_at_text));
        content.push_str(&format!("- Updated: `{}`\n", detail.thread.updated_at_text));
        content.push_str(&format!("- Model: `{}`\n", detail.thread.model_provider));
        content.push_str(&format!("- Tokens: `{}`\n\n", detail.thread.tokens_used));

        if let Some(goal) = detail.goal.as_ref() {
            content.push_str("## Goal\n\n");
            content.push_str(&format!("- Status: `{}`\n", goal.status));
            content.push_str(&format!("- Objective: {}\n\n", goal.objective.trim()));
        }

        if !detail.history.is_empty() {
            content.push_str("## User History\n\n");
            for item in &detail.history {
                content.push_str(&format!("### {}\n\n{}\n\n", item.ts_text, item.text.trim()));
            }
        }

        content.push_str("## Timeline\n\n");
        if let Some(path) = detail.thread.resolved_rollout_path.as_deref() {
            for_each_jsonl_value(Path::new(path), |_, raw| {
                let event = event_from_value(0, raw)?;
                let Some(text) = event.display_text.as_deref() else {
                    return Ok(());
                };
                if text.trim().is_empty() {
                    return Ok(());
                }
                let label = event.payload_type.as_deref().unwrap_or(&event.event_type);
                let role = event.role.as_deref().unwrap_or("");
                content.push_str(&format!(
                    "### {} {} {}\n\n{}\n\n",
                    event.timestamp,
                    label,
                    role,
                    text.trim()
                ));
                Ok(())
            })?;
        }

        Ok(ExportResponse {
            filename: format!("codex-thread-{}.md", safe_filename(&detail.thread.id)),
            content_type: "text/markdown;charset=utf-8".to_string(),
            content,
        })
    }

    pub async fn update_event(
        &self,
        id: &str,
        index: usize,
        request: UpdateEventRequest,
    ) -> Result<ThreadWriteResult, StoreError> {
        self.rewrite_event(id, index, Some(request.raw)).await?;
        Ok(ThreadWriteResult {
            ok: true,
            thread_id: id.to_string(),
            message: "节点已更新".to_string(),
            backup_dir: None,
        })
    }

    pub async fn delete_event(
        &self,
        id: &str,
        index: usize,
        request: DeleteEventRequest,
    ) -> Result<ThreadWriteResult, StoreError> {
        if !request.confirm {
            return Err(StoreError::BadRequest(
                "confirm must be true before deleting event".to_string(),
            ));
        }
        if request.delete_after {
            self.delete_event_and_after(id, index).await?;
        } else {
            self.rewrite_event(id, index, None).await?;
        }
        Ok(ThreadWriteResult {
            ok: true,
            thread_id: id.to_string(),
            message: if request.delete_after {
                "节点及之后条目已删除".to_string()
            } else {
                "节点已删除".to_string()
            },
            backup_dir: None,
        })
    }

    async fn delete_event_and_after(&self, id: &str, index: usize) -> Result<(), StoreError> {
        self.rewrite_events(id, index, |line_index, _line| {
            if line_index >= index {
                RewriteEventAction::Delete
            } else {
                RewriteEventAction::Keep
            }
        })
        .await
    }

    async fn rewrite_event(
        &self,
        id: &str,
        index: usize,
        replacement: Option<Value>,
    ) -> Result<(), StoreError> {
        let replacement = replacement
            .map(|raw| serde_json::to_string(&raw))
            .transpose()?;
        self.rewrite_events(id, index, |line_index, _line| {
            if line_index == index {
                if let Some(raw) = replacement.as_ref() {
                    RewriteEventAction::Replace(raw.clone())
                } else {
                    RewriteEventAction::Delete
                }
            } else {
                RewriteEventAction::Keep
            }
        })
        .await
    }

    async fn rewrite_events<F>(
        &self,
        id: &str,
        index: usize,
        mut action_for: F,
    ) -> Result<(), StoreError>
    where
        F: FnMut(usize, &str) -> RewriteEventAction,
    {
        let thread = self.thread_by_id(id).await?;
        let path = thread
            .resolved_rollout_path
            .as_deref()
            .ok_or_else(|| StoreError::NotFound(format!("rollout for thread {id}")))?;
        rewrite_jsonl_events(Path::new(path), index, &mut action_for)
    }

    pub async fn archive_thread(
        &self,
        id: &str,
        archived: bool,
    ) -> Result<ThreadWriteResult, StoreError> {
        self.thread_by_id(id).await?;
        let state_db = self.state_db.read().unwrap().clone();
        let pool = self.connect_rw(&state_db).await?;
        let archived_at: Option<i64> = if archived { Some(now_seconds()) } else { None };
        let affected = sqlx::query("UPDATE threads SET archived = ?, archived_at = ? WHERE id = ?")
            .bind(if archived { 1i64 } else { 0i64 })
            .bind(archived_at)
            .bind(id)
            .execute(&pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(StoreError::NotFound(id.to_string()));
        }
        // WAL checkpoint: 强制将 WAL 写入主数据库，让 Codex Desktop 立即看到变更
        let _ = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&pool)
            .await;
        Ok(ThreadWriteResult {
            ok: true,
            thread_id: id.to_string(),
            message: if archived {
                "已归档"
            } else {
                "已取消归档"
            }
            .to_string(),
            backup_dir: None,
        })
    }

    pub async fn delete_preview(&self, id: &str) -> Result<DeletePreview, StoreError> {
        let thread = self.thread_by_id(id).await?;
        let files = self.files_for_thread(&thread);
        let mut database_rows = BTreeMap::new();
        database_rows.insert(
            "state_5.sqlite:threads".to_string(),
            self.count_state_thread(id).await?,
        );
        database_rows.insert(
            "goals_1.sqlite:thread_goals".to_string(),
            self.count_goal_thread(id).await?,
        );
        database_rows.insert(
            "logs_2.sqlite:logs".to_string(),
            self.count_logs_thread(id).await?,
        );
        let history_rows = self.thread_history(id, usize::MAX)?.len() as i64;

        Ok(DeletePreview {
            thread_id: id.to_string(),
            files: files
                .into_iter()
                .map(|path| FileImpact {
                    bytes: path.metadata().ok().map(|m| m.len()),
                    exists: path.exists(),
                    path: path.display().to_string(),
                })
                .collect(),
            database_rows,
            history_rows,
        })
    }

    pub async fn delete_thread(
        &self,
        id: &str,
        request: DeleteRequest,
    ) -> Result<ThreadWriteResult, StoreError> {
        if !request.confirm {
            return Err(StoreError::BadRequest(
                "confirm must be true before deleting".to_string(),
            ));
        }
        let thread = self.thread_by_id(id).await?;

        self.rewrite_history_without_thread(id)?;
        self.delete_db_rows(id).await?;
        for path in self.files_for_thread(&thread) {
            if path.exists() {
                fs::remove_file(path)?;
            }
        }

        Ok(ThreadWriteResult {
            ok: true,
            thread_id: id.to_string(),
            message: "已删除，未自动备份".to_string(),
            backup_dir: None,
        })
    }

    pub async fn clear_logs(
        &self,
        id: &str,
        request: ClearLogsRequest,
    ) -> Result<ThreadWriteResult, StoreError> {
        if !request.confirm {
            return Err(StoreError::BadRequest(
                "confirm must be true before clearing logs".to_string(),
            ));
        }
        self.thread_by_id(id).await?;
        let affected = self.delete_log_rows(id).await?;
        Ok(ThreadWriteResult {
            ok: true,
            thread_id: id.to_string(),
            message: format!("已清理 {affected} 条日志"),
            backup_dir: None,
        })
    }

    pub fn list_backups(&self) -> Result<BackupList, StoreError> {
        let metadata = self.load_metadata()?;
        let mut items: Vec<_> = metadata
            .backups
            .into_iter()
            .map(|backup| BackupInfo {
                bytes: directory_size(Path::new(&backup.path)),
                id: backup.id,
                thread_id: backup.thread_id,
                created_at: backup.created_at,
                note: backup.note,
                path: backup.path,
            })
            .collect();
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(BackupList { items })
    }

    pub async fn backup_thread(
        &self,
        id: &str,
        request: BackupRequest,
    ) -> Result<ThreadWriteResult, StoreError> {
        let thread = self.thread_by_id(id).await?;
        let backup_id = format!("{}-{}", timestamp_slug(), id);
        let backup_dir = self.trash_dir.join(&backup_id);
        fs::create_dir_all(&backup_dir)?;
        let mut files = Vec::new();

        for path in self.files_for_thread(&thread) {
            if path.exists() {
                files.push(backup_file_with_manifest(&path, &backup_dir, "session")?);
            }
        }
        for path in self.database_files() {
            if path.exists() {
                files.push(backup_file_with_manifest(&path, &backup_dir, "database")?);
            }
        }
        let history_path = self.history_path.read().unwrap();
        if history_path.exists() {
            files.push(backup_file_with_manifest(
                &history_path,
                &backup_dir,
                "history",
            )?);
        }

        let manifest = BackupManifest {
            id: backup_id.clone(),
            thread_id: id.to_string(),
            created_at: Utc::now().to_rfc3339(),
            note: request.note,
            path: backup_dir.display().to_string(),
            files,
        };
        fs::write(
            backup_dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest)?,
        )?;

        let mut metadata = self.load_metadata()?;
        metadata.backups.retain(|item| item.id != backup_id);
        metadata.backups.push(manifest);
        self.save_metadata(&metadata)?;

        Ok(ThreadWriteResult {
            ok: true,
            thread_id: id.to_string(),
            message: "已创建备份".to_string(),
            backup_dir: Some(backup_dir.display().to_string()),
        })
    }

    pub async fn restore_backup(
        &self,
        backup_id: &str,
        request: RestoreRequest,
    ) -> Result<ThreadWriteResult, StoreError> {
        if !request.confirm {
            return Err(StoreError::BadRequest(
                "confirm must be true before restoring".to_string(),
            ));
        }
        let manifest = self.backup_manifest(backup_id)?;
        let mut locked_files = Vec::new();
        let mut restored_files = Vec::new();
        for file in &manifest.files {
            let source = PathBuf::from(&file.backup_path);
            let target = PathBuf::from(&file.original_path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            // 尝试直接替换
            #[cfg(target_os = "windows")]
            let success = windows_replace_file(&source, &target).is_ok();
            #[cfg(not(target_os = "windows"))]
            let success = {
                if target.exists() {
                    let _ = fs::remove_file(&target);
                }
                fs::copy(&source, &target).is_ok()
            };
            if success {
                restored_files.push(file.original_path.clone());
            } else {
                locked_files.push(file.original_path.clone());
            }
        }
        if locked_files.is_empty() {
            // 全部成功，删除备份
            let backup_dir = PathBuf::from(&manifest.path);
            if backup_dir.exists() {
                let _ = fs::remove_dir_all(&backup_dir);
            }
            let mut metadata = self.load_metadata()?;
            metadata.backups.retain(|b| b.id != backup_id);
            let _ = self.save_metadata(&metadata);
            Ok(ThreadWriteResult {
                ok: true,
                thread_id: manifest.thread_id,
                message: "已从备份恢复并删除备份".to_string(),
                backup_dir: None,
            })
        } else if restored_files.is_empty() {
            // 全部失败
            Err(StoreError::BadRequest(format!(
                "以下文件被 Codex 进程锁定，请先退出 Codex 再恢复：\n{}",
                locked_files.join("\n")
            )))
        } else {
            // 部分成功
            let backup_dir = PathBuf::from(&manifest.path);
            if backup_dir.exists() {
                let _ = fs::remove_dir_all(&backup_dir);
            }
            let mut metadata = self.load_metadata()?;
            metadata.backups.retain(|b| b.id != backup_id);
            let _ = self.save_metadata(&metadata);
            Ok(ThreadWriteResult {
                ok: true,
                thread_id: manifest.thread_id,
                message: format!(
                    "已恢复 {} 个文件，{} 个文件被锁定（重启 Codex 后生效）",
                    restored_files.len(),
                    locked_files.len()
                ),
                backup_dir: None,
            })
        }
    }

    pub fn delete_backup(&self, backup_id: &str) -> Result<ThreadWriteResult, StoreError> {
        let manifest = self.backup_manifest(backup_id)?;
        let backup_dir = PathBuf::from(&manifest.path);
        if backup_dir.exists() {
            fs::remove_dir_all(&backup_dir)?;
        }
        let mut metadata = self.load_metadata()?;
        metadata.backups.retain(|b| b.id != backup_id);
        self.save_metadata(&metadata)?;
        Ok(ThreadWriteResult {
            ok: true,
            thread_id: manifest.thread_id,
            message: "备份已删除".to_string(),
            backup_dir: None,
        })
    }

    pub async fn update_data_dir(
        &self,
        request: UpdateDataDirRequest,
    ) -> Result<serde_json::Value, StoreError> {
        let data_dir = PathBuf::from(&request.data_dir);
        let data_dir = if data_dir.is_absolute() {
            data_dir
        } else {
            std::env::current_dir()?.join(data_dir)
        };

        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)?;
        }

        let state_db = data_dir.join("state_5.sqlite");
        let goals_db = data_dir.join("goals_1.sqlite");
        let logs_db = data_dir.join("logs_2.sqlite");
        let sessions_dir = data_dir.join("sessions");
        let history_path = data_dir.join("history.jsonl");

        // Update the store's internal state
        *self.data_dir.write().unwrap() = data_dir.clone();
        *self.state_db.write().unwrap() = state_db.clone();
        *self.goals_db.write().unwrap() = goals_db.clone();
        *self.logs_db.write().unwrap() = logs_db.clone();
        *self.sessions_dir.write().unwrap() = sessions_dir.clone();
        *self.history_path.write().unwrap() = history_path.clone();

        // Persist the updated data_dir to the config file
        let mut config_to_write = self.config.clone();
        config_to_write.data_dir = data_dir.display().to_string();
        let config_toml = toml::to_string_pretty(&config_to_write)
            .map_err(|e| StoreError::Other(format!("序列化配置失败: {e}")))?;
        fs::write(&self.config_path, config_toml)
            .map_err(|e| StoreError::Other(format!("写入配置文件失败: {e}")))?;

        Ok(serde_json::json!({
            "ok": true,
            "data_dir": data_dir.display().to_string(),
            "state_db": state_db.display().to_string(),
            "goals_db": goals_db.display().to_string(),
            "logs_db": logs_db.display().to_string(),
            "sessions_dir": sessions_dir.display().to_string(),
            "history_path": history_path.display().to_string(),
            "message": "数据目录已更新"
        }))
    }

    pub fn browse(&self, request: BrowseRequest) -> Result<BrowseResponse, StoreError> {
        let current = request
            .path
            .filter(|p| !p.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.data_dir.read().unwrap().clone());

        let current = if current.is_absolute() {
            current
        } else {
            std::env::current_dir()?.join(current)
        };

        let parent = current.parent().map(|p| p.display().to_string());

        let mut directories = Vec::new();

        // On Windows, always list available drive letters for quick drive switching
        if cfg!(target_os = "windows") {
            let current_prefix = current.to_string_lossy().to_lowercase();
            for letter in b'A'..=b'Z' {
                let drive = format!("{}:\\", letter as char);
                let drive_path = PathBuf::from(&drive);
                if drive_path.exists() && drive.to_lowercase() != current_prefix {
                    directories.push(BrowseItem {
                        name: format!("[{}]", drive),
                        path: drive,
                    });
                }
            }
        }

        if current.is_dir() {
            if let Ok(entries) = fs::read_dir(&current) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        directories.push(BrowseItem {
                            name,
                            path: path.display().to_string(),
                        });
                    }
                }
            }
        }

        directories.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        Ok(BrowseResponse {
            current: current.display().to_string(),
            parent,
            directories,
        })
    }

    pub fn update_title(
        &self,
        id: &str,
        request: UpdateTitleRequest,
    ) -> Result<ThreadWriteResult, StoreError> {
        let mut metadata = self.load_metadata()?;
        let title = request.title.trim().to_string();
        if title.is_empty() {
            metadata.thread_titles.remove(id);
        } else {
            metadata.thread_titles.insert(id.to_string(), title);
        }
        self.save_metadata(&metadata)?;
        Ok(ThreadWriteResult {
            ok: true,
            thread_id: id.to_string(),
            message: "标题已更新".to_string(),
            backup_dir: None,
        })
    }

    pub async fn update_runtime(
        &self,
        id: &str,
        request: UpdateRuntimeRequest,
    ) -> Result<ThreadWriteResult, StoreError> {
        let model_provider = request.model_provider.trim();
        if model_provider.is_empty() {
            return Err(StoreError::BadRequest(
                "model_provider cannot be empty".to_string(),
            ));
        }
        let sandbox_type = normalize_choice(
            &request.sandbox_type,
            &[
                "disabled",
                "read-only",
                "workspace-write",
                "danger-full-access",
            ],
            "sandbox_type",
        )?;
        let approval_mode = normalize_choice(
            &request.approval_mode,
            &["untrusted", "on-failure", "on-request", "never"],
            "approval_mode",
        )?;
        let memory_mode = normalize_choice(
            &request.memory_mode,
            &["enabled", "disabled"],
            "memory_mode",
        )?;
        let thread_source = request
            .thread_source
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        let reasoning_effort = request
            .reasoning_effort
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        let sandbox_policy = sandbox_policy_from_type(&sandbox_type);

        let state_db = self.state_db.read().unwrap().clone();
        let pool = self.connect_rw(&state_db).await?;
        let affected = sqlx::query(
            r#"
            UPDATE threads
            SET model_provider = ?,
                sandbox_policy = ?,
                approval_mode = ?,
                memory_mode = ?,
                thread_source = ?,
                reasoning_effort = ?
            WHERE id = ?
            "#,
        )
        .bind(model_provider)
        .bind(sandbox_policy)
        .bind(approval_mode)
        .bind(memory_mode)
        .bind(thread_source)
        .bind(reasoning_effort)
        .bind(id)
        .execute(&pool)
        .await?
        .rows_affected();

        if affected == 0 {
            return Err(StoreError::NotFound(format!("thread {id}")));
        }

        Ok(ThreadWriteResult {
            ok: true,
            thread_id: id.to_string(),
            message: "运行信息已更新".to_string(),
            backup_dir: None,
        })
    }

    fn model_cache_summary(&self) -> Option<ModelCacheSummary> {
        let path = self.data_dir.read().ok()?.join("models_cache.json");
        let raw = fs::read_to_string(path).ok()?;
        let value: Value = serde_json::from_str(&raw).ok()?;
        let models = value
            .get("models")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        let slug = item.get("slug").and_then(Value::as_str)?.to_string();
                        Some(ModelCacheItem {
                            display_name: item
                                .get("display_name")
                                .and_then(Value::as_str)
                                .unwrap_or(&slug)
                                .to_string(),
                            slug,
                            default_reasoning_level: item
                                .get("default_reasoning_level")
                                .and_then(Value::as_str)
                                .map(ToString::to_string),
                            context_window: item.get("context_window").and_then(Value::as_i64),
                            supports_parallel_tool_calls: item
                                .get("supports_parallel_tool_calls")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                            supports_image_detail_original: item
                                .get("supports_image_detail_original")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                            supports_search_tool: item
                                .get("supports_search_tool")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                            visibility: item
                                .get("visibility")
                                .and_then(Value::as_str)
                                .map(ToString::to_string),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Some(ModelCacheSummary {
            fetched_at: value
                .get("fetched_at")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            client_version: value
                .get("client_version")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            models,
        })
    }

    fn load_metadata(&self) -> Result<ManagerMetadata, StoreError> {
        if !self.metadata_path.exists() {
            return Ok(ManagerMetadata::default());
        }
        let text = fs::read_to_string(&self.metadata_path)?;
        if text.trim().is_empty() {
            return Ok(ManagerMetadata::default());
        }
        Ok(serde_json::from_str(&text)?)
    }

    fn save_metadata(&self, metadata: &ManagerMetadata) -> Result<(), StoreError> {
        if let Some(parent) = self.metadata_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.metadata_path, serde_json::to_string_pretty(metadata)?)?;
        Ok(())
    }

    fn backup_manifest(&self, backup_id: &str) -> Result<BackupManifest, StoreError> {
        let metadata = self.load_metadata()?;
        if let Some(item) = metadata
            .backups
            .into_iter()
            .find(|item| item.id == backup_id)
        {
            return Ok(item);
        }
        let manifest_path = self.trash_dir.join(backup_id).join("manifest.json");
        if manifest_path.exists() {
            return Ok(serde_json::from_str(&fs::read_to_string(manifest_path)?)?);
        }
        Err(StoreError::NotFound(format!("backup {backup_id}")))
    }

    async fn load_threads(&self) -> Result<Vec<ThreadSummary>, StoreError> {
        let metadata = self.load_metadata()?;
        let state_db = self.state_db.read().unwrap().clone();
        let pool = self.connect(&state_db).await?;
        let rows = sqlx::query(
            r#"
            SELECT id, rollout_path, created_at, updated_at, source, model_provider, cwd, title,
                   tokens_used, has_user_event, archived, cli_version, first_user_message, model,
                   reasoning_effort, sandbox_policy, approval_mode, memory_mode, thread_source,
                   preview, created_at_ms, updated_at_ms, recency_at_ms
            FROM threads
            "#,
        )
        .fetch_all(&pool)
        .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let id: String = row.try_get("id")?;
            let rollout_path: String = row.try_get("rollout_path")?;
            let created_at: i64 = row.try_get("created_at")?;
            let updated_at: i64 = row.try_get("updated_at")?;
            let created_at_ms: Option<i64> = row.try_get("created_at_ms").unwrap_or(None);
            let updated_at_ms: Option<i64> = row.try_get("updated_at_ms").unwrap_or(None);
            let created_at_ms = created_at_ms.unwrap_or(created_at * 1000);
            let updated_at_ms = updated_at_ms.unwrap_or(updated_at * 1000);
            let recency_at_ms: Option<i64> = row.try_get("recency_at_ms").unwrap_or(None);
            let recency_at_ms = recency_at_ms.unwrap_or(updated_at_ms);
            let resolved = self.resolve_rollout_path(&id, &rollout_path);
            let session_bytes = resolved
                .as_deref()
                .and_then(|path| path.metadata().ok())
                .map(|metadata| metadata.len())
                .unwrap_or(0);
            let original_title: String = row.try_get("title")?;
            let title = metadata
                .thread_titles
                .get(&id)
                .cloned()
                .unwrap_or_else(|| original_title.clone());
            items.push(ThreadSummary {
                id,
                rollout_path,
                resolved_rollout_path: resolved.map(|p| p.display().to_string()),
                created_at,
                updated_at,
                created_at_ms,
                updated_at_ms,
                created_at_text: timestamp_to_text(created_at),
                updated_at_text: timestamp_to_text(updated_at),
                source: row.try_get("source")?,
                thread_source: row.try_get("thread_source").unwrap_or(None),
                model_provider: row.try_get("model_provider")?,
                model: row.try_get("model").unwrap_or(None),
                reasoning_effort: row.try_get("reasoning_effort").unwrap_or(None),
                sandbox_policy: row.try_get("sandbox_policy").unwrap_or_default(),
                sandbox_type: sandbox_type(
                    &row.try_get::<String, _>("sandbox_policy")
                        .unwrap_or_default(),
                ),
                approval_mode: row.try_get("approval_mode").unwrap_or_default(),
                memory_mode: row.try_get("memory_mode").unwrap_or_default(),
                cwd: row.try_get("cwd")?,
                title,
                original_title,
                tokens_used: row.try_get("tokens_used")?,
                session_bytes,
                has_user_event: row.try_get::<i64, _>("has_user_event").unwrap_or_default() != 0,
                archived: row.try_get::<i64, _>("archived")? != 0,
                cli_version: row.try_get("cli_version").unwrap_or_default(),
                first_user_message: row.try_get("first_user_message").unwrap_or_default(),
                preview: row.try_get("preview").unwrap_or_default(),
                recency_at_ms,
                recency_at_text: timestamp_ms_to_text(recency_at_ms),
            });
        }
        Ok(items)
    }

    async fn thread_by_id(&self, id: &str) -> Result<ThreadSummary, StoreError> {
        self.load_threads()
            .await?
            .into_iter()
            .find(|thread| thread.id == id)
            .ok_or_else(|| StoreError::NotFound(format!("thread {id}")))
    }

    async fn thread_goal(&self, id: &str) -> Result<Option<ThreadGoal>, StoreError> {
        let goals_db = self.goals_db.read().unwrap().clone();
        let pool = self.connect(&goals_db).await?;
        let row = sqlx::query(
            r#"
            SELECT goal_id, objective, status, token_budget, tokens_used, time_used_seconds,
                   created_at_ms, updated_at_ms
            FROM thread_goals
            WHERE thread_id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&pool)
        .await?;
        Ok(row.map(|row| ThreadGoal {
            goal_id: row.try_get("goal_id").unwrap_or_default(),
            objective: row.try_get("objective").unwrap_or_default(),
            status: row.try_get("status").unwrap_or_default(),
            token_budget: row.try_get("token_budget").unwrap_or(None),
            tokens_used: row.try_get("tokens_used").unwrap_or_default(),
            time_used_seconds: row.try_get("time_used_seconds").unwrap_or_default(),
            created_at_ms: row.try_get("created_at_ms").unwrap_or_default(),
            updated_at_ms: row.try_get("updated_at_ms").unwrap_or_default(),
        }))
    }

    async fn thread_logs(&self, id: &str, limit: i64) -> Result<Vec<LogEntry>, StoreError> {
        let logs_db = self.logs_db.read().unwrap().clone();
        let pool = self.connect(&logs_db).await?;
        let rows = sqlx::query(
            r#"
            SELECT id, ts, ts_nanos, level, target, feedback_log_body, module_path, file, line,
                   process_uuid
            FROM logs
            WHERE thread_id = ?
            ORDER BY ts DESC, ts_nanos DESC, id DESC
            LIMIT ?
            "#,
        )
        .bind(id)
        .bind(limit)
        .fetch_all(&pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let ts = row.try_get("ts").unwrap_or_default();
                LogEntry {
                    id: row.try_get("id").unwrap_or_default(),
                    ts,
                    ts_nanos: row.try_get("ts_nanos").unwrap_or_default(),
                    ts_text: timestamp_to_text(ts),
                    level: row.try_get("level").unwrap_or_default(),
                    target: row.try_get("target").unwrap_or_default(),
                    body: row.try_get("feedback_log_body").unwrap_or(None),
                    module_path: row.try_get("module_path").unwrap_or(None),
                    file: row.try_get("file").unwrap_or(None),
                    line: row.try_get("line").unwrap_or(None),
                    process_uuid: row.try_get("process_uuid").unwrap_or(None),
                }
            })
            .collect())
    }

    pub fn update_history(
        &self,
        id: &str,
        ts: i64,
        request: UpdateHistoryRequest,
    ) -> Result<ThreadWriteResult, StoreError> {
        if request.text.trim().is_empty() {
            return Err(StoreError::BadRequest(
                "history text cannot be empty".to_string(),
            ));
        }
        self.rewrite_history(id, ts, Some(request.text))?;
        Ok(ThreadWriteResult {
            ok: true,
            thread_id: id.to_string(),
            message: "历史记录已更新".to_string(),
            backup_dir: None,
        })
    }

    pub fn delete_history(
        &self,
        id: &str,
        ts: i64,
        request: DeleteHistoryRequest,
    ) -> Result<ThreadWriteResult, StoreError> {
        if !request.confirm {
            return Err(StoreError::BadRequest(
                "confirm must be true before deleting history".to_string(),
            ));
        }
        self.rewrite_history(id, ts, None)?;
        Ok(ThreadWriteResult {
            ok: true,
            thread_id: id.to_string(),
            message: "历史记录已删除".to_string(),
            backup_dir: None,
        })
    }

    fn rewrite_history(
        &self,
        id: &str,
        ts: i64,
        replacement_text: Option<String>,
    ) -> Result<(), StoreError> {
        let history_path = self.history_path.read().unwrap().clone();
        if !history_path.exists() {
            return Err(StoreError::NotFound("history.jsonl".to_string()));
        }
        let file = fs::File::open(&history_path)?;
        let temp_path = history_path.with_extension("jsonl.tmp");
        let mut lines = Vec::new();
        let mut matches = 0usize;

        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let mut value: Value = serde_json::from_str(&line)?;
            let is_match = value
                .get("session_id")
                .and_then(Value::as_str)
                .is_some_and(|session_id| session_id == id)
                && value
                    .get("ts")
                    .and_then(Value::as_i64)
                    .is_some_and(|value_ts| value_ts == ts);

            if is_match {
                matches += 1;
                if matches > 1 {
                    return Err(StoreError::BadRequest(format!(
                        "multiple history rows matched session {id} ts {ts}"
                    )));
                }
                if let Some(text) = replacement_text.as_ref() {
                    if let Some(object) = value.as_object_mut() {
                        object.insert("text".to_string(), Value::String(text.clone()));
                    } else {
                        return Err(StoreError::BadRequest(
                            "history row is not a JSON object".to_string(),
                        ));
                    }
                    lines.push(serde_json::to_string(&value)?);
                }
                continue;
            }

            lines.push(line);
        }

        if matches == 0 {
            return Err(StoreError::NotFound(format!("history {id} {ts}")));
        }

        let mut output = lines.join("\n");
        if !output.is_empty() {
            output.push('\n');
        }
        fs::write(&temp_path, output)?;
        fs::rename(temp_path, history_path)?;
        Ok(())
    }

    fn thread_history(&self, id: &str, limit: usize) -> Result<Vec<HistoryEntry>, StoreError> {
        let history_path = self.history_path.read().unwrap();
        if !history_path.exists() {
            return Ok(Vec::new());
        }
        let file = fs::File::open(&*history_path)?;
        let mut items = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(&line)?;
            let session_id = value
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if session_id != id {
                continue;
            }
            let ts = value.get("ts").and_then(Value::as_i64).unwrap_or_default();
            items.push(HistoryEntry {
                session_id: session_id.to_string(),
                ts,
                ts_text: timestamp_to_text(ts),
                text: value
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            });
            if items.len() >= limit {
                break;
            }
        }
        Ok(items)
    }

    fn all_history(&self) -> Result<Vec<HistoryEntry>, StoreError> {
        let history_path = self.history_path.read().unwrap();
        if !history_path.exists() {
            return Ok(Vec::new());
        }
        let file = fs::File::open(&*history_path)?;
        let mut items = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(&line)?;
            let ts = value.get("ts").and_then(Value::as_i64).unwrap_or_default();
            items.push(HistoryEntry {
                session_id: value
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                ts,
                ts_text: timestamp_to_text(ts),
                text: value
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            });
        }
        Ok(items)
    }

    async fn count_state_thread(&self, id: &str) -> Result<i64, StoreError> {
        let state_db = self.state_db.read().unwrap().clone();
        self.count_rows(&state_db, "SELECT count(*) FROM threads WHERE id = ?", id)
            .await
    }

    async fn count_goal_thread(&self, id: &str) -> Result<i64, StoreError> {
        let goals_db = self.goals_db.read().unwrap().clone();
        self.count_rows(
            &goals_db,
            "SELECT count(*) FROM thread_goals WHERE thread_id = ?",
            id,
        )
        .await
    }

    async fn count_logs_thread(&self, id: &str) -> Result<i64, StoreError> {
        let logs_db = self.logs_db.read().unwrap().clone();
        self.count_rows(
            &logs_db,
            "SELECT count(*) FROM logs WHERE thread_id = ?",
            id,
        )
        .await
    }

    async fn count_rows(&self, db: &Path, sql: &str, id: &str) -> Result<i64, StoreError> {
        let pool = self.connect(db).await?;
        let row = sqlx::query(sql).bind(id).fetch_one(&pool).await?;
        Ok(row.try_get::<i64, _>(0)?)
    }

    async fn delete_db_rows(&self, id: &str) -> Result<(), StoreError> {
        let state_db = self.state_db.read().unwrap().clone();
        let state = self.connect_rw(&state_db).await?;
        let mut state_tx = state.begin().await?;
        sqlx::query("DELETE FROM threads WHERE id = ?")
            .bind(id)
            .execute(&mut *state_tx)
            .await?;
        state_tx.commit().await?;

        let goals_db = self.goals_db.read().unwrap().clone();
        let goals = self.connect_rw(&goals_db).await?;
        let mut goals_tx = goals.begin().await?;
        sqlx::query("DELETE FROM thread_goals WHERE thread_id = ?")
            .bind(id)
            .execute(&mut *goals_tx)
            .await?;
        goals_tx.commit().await?;

        self.delete_log_rows(id).await?;
        Ok(())
    }

    async fn delete_log_rows(&self, id: &str) -> Result<u64, StoreError> {
        let logs_db = self.logs_db.read().unwrap().clone();
        let logs = self.connect_rw(&logs_db).await?;
        let mut logs_tx = logs.begin().await?;
        let affected = sqlx::query("DELETE FROM logs WHERE thread_id = ?")
            .bind(id)
            .execute(&mut *logs_tx)
            .await?
            .rows_affected();
        logs_tx.commit().await?;
        Ok(affected)
    }

    fn rewrite_history_without_thread(&self, id: &str) -> Result<(), StoreError> {
        let history_path = self.history_path.read().unwrap();
        if !history_path.exists() {
            return Ok(());
        }
        let file = fs::File::open(&*history_path)?;
        let mut kept = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(&line)?;
            if value.get("session_id").and_then(Value::as_str) != Some(id) {
                kept.push(line);
            }
        }
        let mut output = kept.join("\n");
        if !output.is_empty() {
            output.push('\n');
        }
        fs::write(&*history_path, output)?;
        Ok(())
    }

    fn files_for_thread(&self, thread: &ThreadSummary) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(path) = thread.resolved_rollout_path.as_deref() {
            paths.push(PathBuf::from(path));
        }
        let zone = paths
            .first()
            .map(|path| PathBuf::from(format!("{}:Zone.Identifier", path.display())));
        if let Some(path) = zone {
            paths.push(path);
        }
        paths
    }

    fn database_files(&self) -> Vec<PathBuf> {
        let state_db = self.state_db.read().unwrap();
        let goals_db = self.goals_db.read().unwrap();
        let logs_db = self.logs_db.read().unwrap();
        [&*state_db, &*goals_db, &*logs_db]
            .into_iter()
            .flat_map(|db| {
                [
                    db.to_path_buf(),
                    PathBuf::from(format!("{}-wal", db.display())),
                    PathBuf::from(format!("{}-shm", db.display())),
                ]
            })
            .collect()
    }

    fn resolve_rollout_path(&self, id: &str, rollout_path: &str) -> Option<PathBuf> {
        let original = PathBuf::from(rollout_path);
        let sessions_dir = self.sessions_dir.read().unwrap();
        if let Some(pos) = rollout_path.find("/sessions/") {
            let relative = &rollout_path[pos + "/sessions/".len()..];
            let candidate = sessions_dir.join(relative);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        if let Some(name) = original.file_name() {
            let candidate = sessions_dir.join(name.to_string_lossy().to_string());
            if candidate.exists() {
                return Some(candidate);
            }
        }
        let matched = self.session_files().into_iter().find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains(id))
        });
        matched.or_else(|| original.exists().then_some(original))
    }

    fn session_files(&self) -> Vec<PathBuf> {
        let sessions_dir = self.sessions_dir.read().unwrap();
        if !sessions_dir.exists() {
            return Vec::new();
        }
        WalkDir::new(&*sessions_dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| entry.into_path())
            .filter(|path| {
                path.extension().and_then(|ext| ext.to_str()) == Some("jsonl")
                    && !path.to_string_lossy().contains(":Zone.Identifier")
            })
            .collect()
    }

    async fn connect(&self, path: &Path) -> Result<SqlitePool, StoreError> {
        let options = SqliteConnectOptions::from_str("sqlite:")?
            .filename(path)
            .create_if_missing(false)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .read_only(true);
        Ok(SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await?)
    }

    async fn connect_rw(&self, path: &Path) -> Result<SqlitePool, StoreError> {
        let options = SqliteConnectOptions::from_str("sqlite:")?
            .filename(path)
            .create_if_missing(false)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        Ok(SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?)
    }
}

fn for_each_jsonl_value<F>(path: &Path, mut on_value: F) -> Result<(), StoreError>
where
    F: FnMut(usize, Value) -> Result<(), StoreError>,
{
    let file = fs::File::open(path)?;
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str(line) {
            Ok(value) => on_value(index, value)?,
            Err(error) => {
                tracing::warn!(path = %path.display(), index, %error, "skipping invalid jsonl line");
            }
        }
    }
    Ok(())
}

/// 递归清理 JSON Value 中的乱码字符
fn clean_json_value(val: &Value) -> Value {
    match val {
        Value::String(s) => Value::String(clean_garbled_text(s)),
        Value::Array(arr) => Value::Array(arr.iter().map(clean_json_value).collect()),
        Value::Object(map) => {
            let cleaned: serde_json::Map<String, Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), clean_json_value(v)))
                .collect();
            Value::Object(cleaned)
        }
        other => other.clone(),
    }
}

fn event_from_value(index: usize, raw: Value) -> Result<SessionEvent, StoreError> {
    let timestamp = raw
        .get("timestamp")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let event_type = raw
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let payload = raw.get("payload").unwrap_or(&Value::Null);
    let payload_type = payload
        .get("type")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let role = payload
        .get("role")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let display_text =
        display_text(payload, &event_type, payload_type.as_deref()).map(|t| clean_garbled_text(&t));
    let raw = clean_json_value(&raw);
    Ok(SessionEvent {
        index,
        timestamp,
        event_type,
        payload_type,
        role,
        display_text,
        raw,
    })
}

fn display_text(payload: &Value, event_type: &str, payload_type: Option<&str>) -> Option<String> {
    match (event_type, payload_type) {
        ("response_item", Some("message")) => {
            let mut parts = Vec::new();
            if let Some(content) = payload.get("content").and_then(Value::as_array) {
                for item in content {
                    if let Some(text) = item.get("text").and_then(Value::as_str) {
                        parts.push(text.to_string());
                    }
                }
            }
            non_empty(parts.join("\n"))
        }
        ("response_item", Some("function_call")) => Some(format!(
            "{} {}",
            payload
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("function_call"),
            payload
                .get("arguments")
                .and_then(Value::as_str)
                .unwrap_or_default()
        )),
        ("response_item", Some("function_call_output")) => payload
            .get("output")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        ("response_item", Some("custom_tool_call")) => payload
            .get("input")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        ("response_item", Some("custom_tool_call_output")) => payload
            .get("output")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        ("response_item", Some("reasoning")) => payload
            .get("summary")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .and_then(non_empty),
        ("event_msg", Some("user_message")) | ("event_msg", Some("agent_message")) => payload
            .get("message")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        ("event_msg", Some("task_complete")) => payload
            .get("last_agent_message")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        ("event_msg", Some("token_count")) => token_count_text(payload),
        ("compacted", _) => payload
            .get("message")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        _ => None,
    }
}

fn token_count_text(payload: &Value) -> Option<String> {
    let info = payload.get("info")?;
    let total_usage = info.get("total_token_usage").unwrap_or(&Value::Null);
    let last_usage = info.get("last_token_usage").unwrap_or(&Value::Null);
    let total_tokens = total_usage
        .get("total_tokens")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let last_tokens = last_usage
        .get("total_tokens")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let input_tokens = total_usage
        .get("input_tokens")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let cached_input_tokens = total_usage
        .get("cached_input_tokens")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let output_tokens = total_usage
        .get("output_tokens")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let reasoning_output_tokens = total_usage
        .get("reasoning_output_tokens")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let context_window = info
        .get("model_context_window")
        .and_then(Value::as_i64)
        .unwrap_or_default();

    let mut parts = vec![format!(
        "总计 {} tokens，本轮 {}，输入 {}，缓存 {}，输出 {}，推理 {}",
        format_number(total_tokens),
        format_number(last_tokens),
        format_number(input_tokens),
        format_number(cached_input_tokens),
        format_number(output_tokens),
        format_number(reasoning_output_tokens),
    )];
    if context_window > 0 {
        parts.push(format!("上下文 {}", format_number(context_window)));
    }

    if let Some(rate_limits) = payload.get("rate_limits") {
        let primary = rate_limits
            .get("primary")
            .and_then(|value| value.get("used_percent"))
            .and_then(Value::as_f64);
        let secondary = rate_limits
            .get("secondary")
            .and_then(|value| value.get("used_percent"))
            .and_then(Value::as_f64);
        match (primary, secondary) {
            (Some(primary), Some(secondary)) => {
                parts.push(format!("限额 {:.0}%/{:.0}%", primary, secondary));
            }
            (Some(primary), None) => {
                parts.push(format!("限额 {:.0}%", primary));
            }
            _ => {}
        }
    }

    Some(parts.join("，"))
}

fn format_number(value: i64) -> String {
    let text = value.abs().to_string();
    let mut out = String::new();
    for (index, ch) in text.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    let mut out: String = out.chars().rev().collect();
    if value < 0 {
        out.insert(0, '-');
    }
    out
}

/// 清理 Codex 在中文 Windows 上捕获子进程输出时产生的乱码字符
/// - U+FFFD (replacement character): GBK 字节被错误解码为 UTF-8
/// - U+0000 (null bytes): UTF-16LE 编码的子进程输出
/// - 非常规 Unicode 字符 (Hebrew/Cyrillic 等): GBK 字节恰好构成合法 UTF-8
fn clean_garbled_text(text: &str) -> String {
    text.chars()
        .filter(|c| {
            !matches!(c,
                '\u{FFFD}'        // replacement character
                | '\u{0000}'      // null byte (UTF-16LE artifact)
                | '\u{0200}'..='\u{02FF}'  // Latin Extended-B (GBK misread)
                | '\u{0370}'..='\u{03FF}'  // Greek
                | '\u{0400}'..='\u{052F}'  // Cyrillic + Armenian
                | '\u{0590}'..='\u{08FF}'  // Hebrew + Arabic + other RTL
                | '\u{2000}'..='\u{2BFF}'  // various symbols (GBK misread)
            )
        })
        .collect()
}

fn file_info(path: &Path) -> Result<FileInfo, StoreError> {
    let meta = path.metadata()?;
    let modified = meta.modified().ok().and_then(system_time_to_text);
    Ok(FileInfo {
        path: path.display().to_string(),
        bytes: meta.len(),
        modified,
    })
}

fn backup_file_with_manifest(
    path: &Path,
    backup_dir: &Path,
    kind: &str,
) -> Result<BackupFile, StoreError> {
    let target_dir = backup_dir.join("files");
    fs::create_dir_all(&target_dir)?;
    let file_name = path
        .file_name()
        .ok_or_else(|| StoreError::Other(format!("cannot backup {}", path.display())))?;
    let mut target = target_dir.join(file_name);
    if target.exists() {
        let safe_name = path.display().to_string().replace(['/', '\\', ':'], "_");
        target = target_dir.join(safe_name);
    }
    fs::copy(path, &target)?;
    Ok(BackupFile {
        kind: kind.to_string(),
        original_path: path.display().to_string(),
        backup_path: target.display().to_string(),
        bytes: path.metadata().map(|meta| meta.len()).unwrap_or(0),
    })
}

fn path_from_config(value: &str) -> Result<PathBuf, StoreError> {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn directory_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| entry.metadata().ok().map(|meta| meta.len()))
        .sum()
}

fn count_by<I>(items: I) -> Vec<CountItem>
where
    I: IntoIterator<Item = String>,
{
    let mut map: HashMap<String, usize> = HashMap::new();
    for item in items {
        *map.entry(item).or_default() += 1;
    }
    let mut items: Vec<_> = map
        .into_iter()
        .map(|(name, count)| CountItem { name, count })
        .collect();
    items.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    items
}

fn counts_from_map(map: HashMap<String, usize>) -> Vec<CountItem> {
    let mut items: Vec<_> = map
        .into_iter()
        .map(|(name, count)| CountItem { name, count })
        .collect();
    items.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    items
}

fn normalize_query(q: Option<&str>) -> Option<String> {
    q.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
}

fn contains_ci(haystack: &str, q: &str) -> bool {
    haystack.to_lowercase().contains(q)
}

fn snippet(text: &str, q: &str) -> String {
    let lower = text.to_lowercase();
    let Some(pos) = lower.find(q) else {
        return text.chars().take(180).collect();
    };
    let start = text[..pos].chars().count().saturating_sub(60);
    let end = start + 180;
    let chars: Vec<_> = text.chars().collect();
    let mut out: String = chars[start..chars.len().min(end)].iter().collect();
    if start > 0 {
        out.insert_str(0, "...");
    }
    if chars.len() > end {
        out.push_str("...");
    }
    out
}

fn safe_filename(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn rewrite_jsonl_events<F>(
    path: &Path,
    target_index: usize,
    action_for: &mut F,
) -> Result<(), StoreError>
where
    F: FnMut(usize, &str) -> RewriteEventAction,
{
    let file = fs::File::open(path)?;
    let temp_path = path.with_extension("jsonl.tmp");
    let mut lines = Vec::new();
    let mut found = false;

    for (line_index, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line_index == target_index {
            found = true;
        }
        match action_for(line_index, &line) {
            RewriteEventAction::Keep => lines.push(line),
            RewriteEventAction::Delete => {}
            RewriteEventAction::Replace(next) => lines.push(next),
        }
    }

    if !found {
        return Err(StoreError::NotFound(format!("event {target_index}")));
    }

    let mut output = lines.join("\n");
    if !output.is_empty() {
        output.push('\n');
    }
    fs::write(&temp_path, output)?;
    fs::rename(temp_path, path)?;
    Ok(())
}

fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn timestamp_to_text(ts: i64) -> String {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| ts.to_string())
}

fn timestamp_ms_to_text(ts_ms: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(ts_ms)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| ts_ms.to_string())
}

fn sandbox_type(policy: &str) -> String {
    serde_json::from_str::<Value>(policy)
        .ok()
        .and_then(|value| {
            value
                .get("type")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            if policy.trim().is_empty() {
                "unknown".to_string()
            } else {
                policy.to_string()
            }
        })
}

fn sandbox_policy_from_type(sandbox_type: &str) -> String {
    match sandbox_type {
        "read-only" => serde_json::json!({ "type": "read-only" }).to_string(),
        "workspace-write" => serde_json::json!({
            "type": "workspace-write",
            "writable_roots": [],
            "network_access": false,
            "exclude_tmpdir_env_var": false,
            "exclude_slash_tmp": false,
        })
        .to_string(),
        "danger-full-access" => serde_json::json!({ "type": "danger-full-access" }).to_string(),
        _ => serde_json::json!({ "type": "disabled" }).to_string(),
    }
}

fn normalize_choice(value: &str, allowed: &[&str], field_name: &str) -> Result<String, StoreError> {
    let value = value.trim();
    if allowed.contains(&value) {
        Ok(value.to_string())
    } else {
        Err(StoreError::BadRequest(format!(
            "invalid {field_name}: {value}"
        )))
    }
}

fn system_time_to_text(time: SystemTime) -> Option<String> {
    let duration = time.duration_since(UNIX_EPOCH).ok()?;
    Some(timestamp_to_text(duration.as_secs() as i64))
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

fn timestamp_slug() -> String {
    Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

fn date_to_ms(value: &str, start: bool) -> Option<i64> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()?;
    let time = if start {
        date.and_hms_opt(0, 0, 0)?
    } else {
        date.and_hms_opt(23, 59, 59)?
    };
    Some(time.and_utc().timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_display_text_from_message() {
        let raw: Value = serde_json::json!({
            "timestamp": "2026-01-01T00:00:00Z",
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "hello"}]
            }
        });
        let event = event_from_value(0, raw).unwrap();
        assert_eq!(event.display_text.as_deref(), Some("hello"));
        assert_eq!(event.payload_type.as_deref(), Some("message"));
    }

    #[test]
    fn extracts_display_text_from_token_count() {
        let raw: Value = serde_json::json!({
            "timestamp": "2026-01-01T00:00:00Z",
            "type": "event_msg",
            "payload": {
                "type": "token_count",
                "info": {
                    "total_token_usage": {
                        "input_tokens": 24729,
                        "cached_input_tokens": 18176,
                        "output_tokens": 544,
                        "reasoning_output_tokens": 24,
                        "total_tokens": 25273
                    },
                    "last_token_usage": {
                        "total_tokens": 13178
                    },
                    "model_context_window": 258400
                },
                "rate_limits": {
                    "primary": { "used_percent": 1.0 },
                    "secondary": { "used_percent": 24.0 }
                }
            }
        });
        let event = event_from_value(0, raw).unwrap();
        assert_eq!(event.payload_type.as_deref(), Some("token_count"));
        assert_eq!(
            event.display_text.as_deref(),
            Some(
                "总计 25,273 tokens，本轮 13,178，输入 24,729，缓存 18,176，输出 544，推理 24，上下文 258,400，限额 1%/24%"
            )
        );
    }

    #[test]
    fn creates_snippet_around_match() {
        let text = "0123456789abcdefghijklmnopqrstuvwxyz";
        assert!(snippet(text, "mnop").contains("mnop"));
    }

    #[test]
    fn streams_jsonl_values_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        fs::write(
            &path,
            "{\"type\":\"one\",\"payload\":{}}\n{\"type\":\"two\",\"payload\":{}}\n",
        )
        .unwrap();
        let mut seen = Vec::new();
        for_each_jsonl_value(&path, |index, raw| {
            seen.push((index, raw["type"].as_str().unwrap().to_string()));
            Ok(())
        })
        .unwrap();
        assert_eq!(seen, vec![(0, "one".to_string()), (1, "two".to_string())]);
    }

    #[test]
    fn rewrites_jsonl_by_deleting_target_and_following_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        fs::write(
            &path,
            "{\"type\":\"one\"}\n{\"type\":\"two\"}\n{\"type\":\"three\"}\n",
        )
        .unwrap();

        rewrite_jsonl_events(&path, 1, &mut |line_index, _line| {
            if line_index >= 1 {
                RewriteEventAction::Delete
            } else {
                RewriteEventAction::Keep
            }
        })
        .unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"type\":\"one\"}\n");
    }

    #[test]
    fn safe_filename_replaces_separators() {
        assert_eq!(safe_filename("abc/def:ghi"), "abc-def-ghi");
    }
}
