mod models;
mod store;

use std::{env, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use models::{
    BackupRequest, BrowseRequest, ClearLogsRequest, DeleteEventRequest, DeleteHistoryRequest,
    DeleteRequest, EventQuery, ExportQuery, ManagerConfig, RestoreRequest, SearchQuery,
    ThreadQuery, UpdateArchiveRequest, UpdateDataDirRequest, UpdateEventRequest,
    UpdateHistoryRequest, UpdateRuntimeRequest, UpdateTitleRequest,
};
use serde_json::json;
use store::{AppStore, StoreError};
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "codex_message_manage=info,tower_http=info".into()),
        )
        .init();

    let (config, config_path) = load_config()?;
    let addr: SocketAddr = config.bind.parse()?;

    let store = Arc::new(AppStore::with_config(config_path, config)?);
    store.validate().await?;

    let api = Router::new()
        .route("/health", get(health))
        .route("/config", get(get_config))
        .route("/overview", get(overview))
        .route("/stats", get(stats))
        .route("/backups", get(backups))
        .route("/backups/{id}/restore", post(restore_backup))
        .route("/backups/{id}/delete", post(delete_backup))
        .route("/threads", get(threads))
        .route("/threads/{id}", get(thread_detail))
        .route("/threads/{id}/history/{ts}", post(update_history))
        .route("/threads/{id}/history/{ts}/delete", post(delete_history))
        .route("/threads/{id}/events", get(thread_events))
        .route("/threads/{id}/events/{index}", post(update_event))
        .route("/threads/{id}/events/{index}/delete", post(delete_event))
        .route("/threads/{id}/export", get(export_thread))
        .route("/threads/{id}/archive", post(archive_thread))
        .route("/threads/{id}/backup", post(backup_thread))
        .route("/threads/{id}/title", post(update_title))
        .route("/threads/{id}/runtime", post(update_runtime))
        .route("/threads/{id}/logs/clear", post(clear_logs))
        .route("/threads/{id}/delete/preview", post(delete_preview))
        .route("/threads/{id}/delete", post(delete_thread))
        .route("/search", get(search))
        .route("/config/data-dir", post(update_data_dir))
        .route("/browse", get(browse))
        .with_state(store);

    let app = Router::new()
        .nest("/api", api)
        .fallback_service(ServeDir::new("frontend/dist").append_index_html_on_directories(true))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    tracing::info!("serving Codex manager at http://{addr}");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

fn load_config() -> anyhow::Result<(ManagerConfig, PathBuf)> {
    let config_path = env::var("CODEX_MANAGER_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("codex-manager.toml"));
    let mut config = if config_path.exists() {
        toml::from_str::<ManagerConfig>(&std::fs::read_to_string(&config_path)?)?
    } else {
        ManagerConfig {
            data_dir: "codex-save".to_string(),
            bind: "127.0.0.1:5178".to_string(),
            trash_dir: ".codex-manager-trash".to_string(),
            metadata_path: ".codex-manager.json".to_string(),
        }
    };
    if let Some(data_dir) = env::var_os("CODEX_DATA_DIR") {
        config.data_dir = data_dir.to_string_lossy().to_string();
    }
    if let Ok(bind) = env::var("CODEX_MANAGER_BIND") {
        config.bind = bind;
    }
    Ok((config, config_path))
}

async fn health(State(store): State<Arc<AppStore>>) -> Response {
    axum::Json(json!({
        "ok": true,
        "data_dir": store.data_dir(),
    }))
    .into_response()
}

async fn get_config(State(store): State<Arc<AppStore>>) -> Response {
    match serde_json::to_value(store.config()) {
        Ok(value) => axum::Json(value).into_response(),
        Err(e) => json_error_response(e),
    }
}

async fn overview(State(store): State<Arc<AppStore>>) -> Response {
    match store.overview().await {
        Ok(overview) => match serde_json::to_value(overview) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn stats(State(store): State<Arc<AppStore>>) -> Response {
    match store.stats().await {
        Ok(stats) => match serde_json::to_value(stats) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn backups(State(store): State<Arc<AppStore>>) -> Response {
    match store.list_backups() {
        Ok(backups) => match serde_json::to_value(backups) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn threads(State(store): State<Arc<AppStore>>, Query(query): Query<ThreadQuery>) -> Response {
    match store.threads(query).await {
        Ok(threads) => match serde_json::to_value(threads) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn thread_detail(State(store): State<Arc<AppStore>>, Path(id): Path<String>) -> Response {
    match store.thread_detail(&id).await {
        Ok(detail) => match serde_json::to_value(detail) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn update_history(
    State(store): State<Arc<AppStore>>,
    Path((id, ts)): Path<(String, i64)>,
    axum::Json(request): axum::Json<UpdateHistoryRequest>,
) -> Response {
    match store.update_history(&id, ts, request) {
        Ok(result) => match serde_json::to_value(result) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn delete_history(
    State(store): State<Arc<AppStore>>,
    Path((id, ts)): Path<(String, i64)>,
    axum::Json(request): axum::Json<DeleteHistoryRequest>,
) -> Response {
    match store.delete_history(&id, ts, request) {
        Ok(result) => match serde_json::to_value(result) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn thread_events(
    State(store): State<Arc<AppStore>>,
    Path(id): Path<String>,
    Query(query): Query<EventQuery>,
) -> Response {
    match store.thread_events(&id, query).await {
        Ok(events) => match serde_json::to_value(events) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn search(State(store): State<Arc<AppStore>>, Query(query): Query<SearchQuery>) -> Response {
    match store.search(query).await {
        Ok(search) => match serde_json::to_value(search) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn export_thread(
    State(store): State<Arc<AppStore>>,
    Path(id): Path<String>,
    Query(query): Query<ExportQuery>,
) -> Response {
    match store.export_thread(&id, query).await {
        Ok(export) => match serde_json::to_value(export) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn update_event(
    State(store): State<Arc<AppStore>>,
    Path((id, index)): Path<(String, usize)>,
    axum::Json(request): axum::Json<UpdateEventRequest>,
) -> Response {
    match store.update_event(&id, index, request).await {
        Ok(result) => match serde_json::to_value(result) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn delete_event(
    State(store): State<Arc<AppStore>>,
    Path((id, index)): Path<(String, usize)>,
    axum::Json(request): axum::Json<DeleteEventRequest>,
) -> Response {
    match store.delete_event(&id, index, request).await {
        Ok(result) => match serde_json::to_value(result) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn backup_thread(
    State(store): State<Arc<AppStore>>,
    Path(id): Path<String>,
    axum::Json(request): axum::Json<BackupRequest>,
) -> Response {
    match store.backup_thread(&id, request).await {
        Ok(result) => match serde_json::to_value(result) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn restore_backup(
    State(store): State<Arc<AppStore>>,
    Path(id): Path<String>,
    axum::Json(request): axum::Json<RestoreRequest>,
) -> Response {
    match store.restore_backup(&id, request).await {
        Ok(result) => match serde_json::to_value(result) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn delete_backup(State(store): State<Arc<AppStore>>, Path(id): Path<String>) -> Response {
    match store.delete_backup(&id) {
        Ok(result) => match serde_json::to_value(result) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn update_title(
    State(store): State<Arc<AppStore>>,
    Path(id): Path<String>,
    axum::Json(request): axum::Json<UpdateTitleRequest>,
) -> Response {
    match store.update_title(&id, request) {
        Ok(result) => match serde_json::to_value(result) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn archive_thread(
    State(store): State<Arc<AppStore>>,
    Path(id): Path<String>,
    axum::Json(request): axum::Json<UpdateArchiveRequest>,
) -> Response {
    let archived = request.archived.unwrap_or(true);
    match store.archive_thread(&id, archived).await {
        Ok(result) => match serde_json::to_value(result) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn clear_logs(
    State(store): State<Arc<AppStore>>,
    Path(id): Path<String>,
    axum::Json(request): axum::Json<ClearLogsRequest>,
) -> Response {
    match store.clear_logs(&id, request).await {
        Ok(result) => match serde_json::to_value(result) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn delete_preview(State(store): State<Arc<AppStore>>, Path(id): Path<String>) -> Response {
    match store.delete_preview(&id).await {
        Ok(preview) => match serde_json::to_value(preview) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn delete_thread(
    State(store): State<Arc<AppStore>>,
    Path(id): Path<String>,
    axum::Json(request): axum::Json<DeleteRequest>,
) -> Response {
    match store.delete_thread(&id, request).await {
        Ok(result) => match serde_json::to_value(result) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn update_runtime(
    State(store): State<Arc<AppStore>>,
    Path(id): Path<String>,
    axum::Json(request): axum::Json<UpdateRuntimeRequest>,
) -> Response {
    match store.update_runtime(&id, request).await {
        Ok(result) => match serde_json::to_value(result) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

async fn update_data_dir(
    State(store): State<Arc<AppStore>>,
    axum::Json(request): axum::Json<UpdateDataDirRequest>,
) -> Response {
    match store.update_data_dir(request).await {
        Ok(result) => axum::Json(result).into_response(),
        Err(e) => store_error_response(e),
    }
}

async fn browse(
    State(store): State<Arc<AppStore>>,
    Query(query): Query<BrowseRequest>,
) -> Response {
    match store.browse(query) {
        Ok(browse) => match serde_json::to_value(browse) {
            Ok(value) => axum::Json(value).into_response(),
            Err(e) => json_error_response(e),
        },
        Err(e) => store_error_response(e),
    }
}

// Helper: convert StoreError to error Response
fn store_error_response(e: StoreError) -> Response {
    let (status, message) = match e {
        StoreError::NotFound(m) => (StatusCode::NOT_FOUND, m),
        StoreError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
        StoreError::Io(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        StoreError::Sqlx(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        StoreError::Json(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        StoreError::Other(m) => (StatusCode::INTERNAL_SERVER_ERROR, m),
    };
    (status, axum::Json(json!({ "error": message }))).into_response()
}

fn json_error_response(e: serde_json::Error) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        axum::Json(json!({ "error": e.to_string() })),
    )
        .into_response()
}
