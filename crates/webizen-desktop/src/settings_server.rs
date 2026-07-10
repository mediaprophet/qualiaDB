//! Loopback settings portal on `127.0.0.1:8080` (tray "Open Settings").
//!
//! Serves a browser-accessible control panel plus the HTTP surface that
//! `webizen-studio` expects (`/manifest`, `/telemetry`).

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::{self, Stream};
use qualia_client_core::local_job_scheduler::{
    EnqueueJobRequest, JobQueueSnapshot, LocalJob, LocalJobKind, LocalJobScheduler,
};
use qualia_client_core::state::{AgentConfig, AppState};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tower_http::{cors::CorsLayer, services::ServeDir};
use webizen_render::scene_contract::SystemTelemetry;

pub const DEFAULT_SETTINGS_PORT: u16 = 8080;
static CURRENT_SETTINGS_PORT: AtomicU16 = AtomicU16::new(DEFAULT_SETTINGS_PORT);

const EMPTY_MANIFEST: &str =
    r#"{"pages":[],"theme_tokens":{},"themes":[],"environment_theme":{},"app_theme":{}}"#;

const WORKSPACE_MANIFEST_FILE: &str = "studio-workspace.json";

fn workspace_manifest_path(storage_path: &str) -> PathBuf {
    PathBuf::from(storage_path).join(WORKSPACE_MANIFEST_FILE)
}

fn load_persisted_manifest(storage_path: &str) -> String {
    let path = workspace_manifest_path(storage_path);
    match std::fs::read_to_string(&path) {
        Ok(body) if body.trim().is_empty() => EMPTY_MANIFEST.to_string(),
        Ok(body) => body,
        Err(_) => EMPTY_MANIFEST.to_string(),
    }
}

fn persist_manifest_to_disk(storage_path: &str, body: &str) -> Result<(), String> {
    let path = workspace_manifest_path(storage_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, body.as_bytes()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Clone)]
pub struct SettingsServerState {
    pub app_state: Arc<AppState>,
    pub manifest: Arc<Mutex<String>>,
    pub listen_port: Arc<Mutex<u16>>,
    pub static_root: PathBuf,
    pub host_api: crate::companion_gateway::HostApiHandle,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    port: u16,
}

#[derive(Serialize)]
struct StatusResponse {
    settings_port: u16,
    graph_daemon_port: u16,
    graph_daemon_reachable: bool,
    graph_engine_version: Option<String>,
    qapps_protocol_port: u16,
    storage_path: String,
    inference_backend: String,
    daemon_running_flag: bool,
    job_queue: JobQueueCounts,
}

#[derive(Serialize)]
struct JobQueueCounts {
    queued: usize,
    running: usize,
    completed: usize,
    failed: usize,
}

fn find_open_port(host: &str, start: u16) -> u16 {
    for port in start..=start.saturating_add(20) {
        if std::net::TcpListener::bind((host, port)).is_ok() {
            return port;
        }
    }
    start
}

fn static_portal_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static/portal")
}

fn studio_dist_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sidecar = dir.join("studio-dist");
            if sidecar.is_dir() {
                return sidecar;
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|crates| crates.join("webizen-studio").join("dist"))
        .unwrap_or_else(|| PathBuf::from("../webizen-studio/dist"))
}

/// Directory where the Studio WASM build is served from.
/// The `dx build --release` command outputs to `target/dx/webizen-studio/release/web/public/`.
/// A build script or manual copy step places the assets in `static/studio-wasm/`.
fn studio_wasm_dir() -> PathBuf {
    // Check for a manual override first
    if let Ok(dir) = std::env::var("QUALIA_STUDIO_WASM_DIR") {
        return PathBuf::from(dir);
    }
    // Prefer the bundled Studio dist used by Tauri itself. The desktop app is
    // now a first-class Tauri webview; /studio is kept for browser diagnostics
    // and external preview links.
    let dist_dir = studio_dist_dir();
    if dist_dir.is_dir() {
        return dist_dir;
    }
    // Development fallback: look for the Dioxus target directory.
    let target_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .and_then(|p| p.parent()) // project root
        .map(|root| root.join("target/dx/webizen-studio/release/web/public"))
        .unwrap_or_else(|| PathBuf::from("target/dx/webizen-studio/release/web/public"));
    if target_dir.is_dir() {
        return target_dir;
    }
    // Fallback: static/studio-wasm (populated by a build script)
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static/studio-wasm")
}

pub fn current_settings_port() -> u16 {
    CURRENT_SETTINGS_PORT.load(Ordering::Relaxed)
}

pub fn spawn_settings_server(
    app_state: Arc<AppState>,
    host_api: crate::companion_gateway::HostApiHandle,
) -> u16 {
    // Use the user-configured port from AgentConfig, or auto-find if set to 0
    let configured_port = app_state.config.lock().unwrap().settings_port;
    let port = if configured_port > 0 {
        // Try the user-specified port; if it's taken, fall back to auto-find
        if std::net::TcpListener::bind(("0.0.0.0", configured_port)).is_ok() {
            configured_port
        } else {
            eprintln!(
                "Settings port {} is in use, auto-finding an open port...",
                configured_port
            );
            find_open_port("0.0.0.0", DEFAULT_SETTINGS_PORT)
        }
    } else {
        find_open_port("0.0.0.0", DEFAULT_SETTINGS_PORT)
    };
    let storage_path = app_state.config.lock().unwrap().storage_path.clone();
    CURRENT_SETTINGS_PORT.store(port, Ordering::Relaxed);
    crate::desktop_log::record("info", format!("settings server selected port {port}"));
    let initial_manifest = load_persisted_manifest(&storage_path);
    let state = SettingsServerState {
        app_state,
        manifest: Arc::new(Mutex::new(initial_manifest)),
        listen_port: Arc::new(Mutex::new(port)),
        static_root: static_portal_dir(),
        host_api: host_api.clone(),
    };

    crate::companion_gateway::set_companion_listen_port(port);

    tauri::async_runtime::spawn(async move {
        if let Err(err) = run_settings_server(state, port).await {
            eprintln!("Settings portal failed: {err}");
        }
    });

    port
}

async fn run_settings_server(state: SettingsServerState, port: u16) -> Result<(), String> {
    let static_root = state.static_root.clone();
    let studio_root = studio_wasm_dir();
    if !static_root.is_dir() {
        return Err(format!(
            "Settings static dir missing: {}",
            static_root.display()
        ));
    }
    if !studio_root.is_dir() {
        crate::desktop_log::record(
            "warn",
            format!(
                "Studio dist dir missing for /studio: {}",
                studio_root.display()
            ),
        );
    }

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/shell", get(shell_handler))
        .route("/logs", get(logs_page_handler))
        .route("/api/logs", get(logs_json_handler))
        .route("/api/logs/text", get(logs_text_handler))
        .route("/api/status", get(status_handler))
        .route(
            "/api/config",
            get(get_config_handler).post(save_config_handler),
        )
        .route(
            "/manifest",
            get(get_manifest_handler).post(post_manifest_handler),
        )
        .route("/manifest/history", get(get_manifest_history_handler))
        .route("/manifest/undo-chain", get(get_manifest_undo_chain_handler))
        .route(
            "/manifest/undo-frame",
            post(post_manifest_undo_frame_handler),
        )
        .route("/manifest/replay/{revision}", post(replay_manifest_handler))
        .route("/telemetry", get(telemetry_handler))
        .route(
            "/api/jobs",
            get(list_jobs_handler).post(enqueue_job_handler),
        )
        .route("/api/jobs/{id}", get(get_job_handler))
        .route("/api/jobs/{id}/cancel", post(cancel_job_handler))
        .route("/api/telemetry", get(system_telemetry_handler))
        .route("/api/sparql/endpoints", get(sparql_endpoints_handler))
        .route("/api/sparql/query", post(sparql_query_handler))
        .route("/api/assets/catalog", get(assets_catalog_handler))
        .route("/api/assets/recommend", post(assets_recommend_handler))
        .route("/api/assets/enqueue", post(assets_enqueue_handler))
        .route("/generate_pane", post(generate_pane_handler))
        .route("/wellfair/companion/ingest", post(companion_ingest_route))
        .route("/mobile/stream", get(companion_ws_route))
        .route("/mobile/qr", get(companion_qr_route))
        .route(
            "/api/wellfair/companion/pairing",
            get(companion_pairing_route),
        )
        .route("/api/invoke/{cmd}", post(invoke_command_handler))
        // Studio WASM build — browser-accessible Studio UI at /studio/
        .nest_service("/assets", ServeDir::new(studio_root.join("assets")))
        .nest_service(
            "/studio",
            ServeDir::new(studio_root).append_index_html_on_directories(true),
        )
        .fallback_service(ServeDir::new(static_root).append_index_html_on_directories(true))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("bind 0.0.0.0:{port}: {e}"))?;
    println!(
        "Qualia settings + companion gateway on http://127.0.0.1:{port}/ (LAN ws://<host>:{port}/mobile/stream)"
    );
    crate::desktop_log::record(
        "info",
        format!("settings + companion gateway listening on http://127.0.0.1:{port}/"),
    );
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("settings server: {e}"))
}

async fn logs_json_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "log_file": crate::desktop_log::log_path(),
        "entries": crate::desktop_log::recent_entries(),
    }))
}

async fn logs_text_handler() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        crate::desktop_log::recent_text(),
    )
}

async fn logs_page_handler() -> impl IntoResponse {
    const LOGS_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Webizen Desktop Logs</title>
<style>
html,body{margin:0;min-height:100%;background:#101014;color:#e8e4df;font:13px/1.5 ui-monospace,SFMono-Regular,Consolas,monospace}
header{display:flex;align-items:center;gap:12px;padding:12px 16px;border-bottom:1px solid #2b2b35;background:#171720;position:sticky;top:0}
h1{font:600 14px/1.2 system-ui,sans-serif;margin:0}
button,a{border:1px solid #3d3d49;background:#20202b;color:#e8e4df;border-radius:6px;padding:6px 10px;text-decoration:none;cursor:pointer}
button:hover,a:hover{background:#2b2b38}
#path{color:#a7a2ba;font-family:ui-monospace,SFMono-Regular,Consolas,monospace;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
pre{white-space:pre-wrap;margin:0;padding:16px}
.warn{color:#f9c74f}.error{color:#f87171}.info{color:#9ae6b4}
</style>
</head>
<body>
<header>
<h1>Webizen Desktop Logs</h1>
<button id="refresh">Refresh</button>
<a href="/api/logs/text" target="_blank">Raw</a>
<span id="path"></span>
</header>
<pre id="log">Loading...</pre>
<script>
async function refresh(){
  const res = await fetch('/api/logs');
  const json = await res.json();
  document.getElementById('path').textContent = json.log_file || '';
  const lines = (json.entries || []).map(e => `${e.ts} [${e.level}] ${e.message}`);
  document.getElementById('log').textContent = lines.join('\n') || 'No log entries yet.';
}
document.getElementById('refresh').onclick = refresh;
refresh();
setInterval(refresh, 3000);
</script>
</body>
</html>"#;
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        LOGS_HTML,
    )
}

async fn health_handler(State(state): State<SettingsServerState>) -> Json<HealthResponse> {
    let port = *state.listen_port.lock().unwrap();
    Json(HealthResponse {
        status: "ok",
        service: "qualia-settings-portal",
        port,
    })
}

async fn shell_handler() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(crate::shell::shell_html::SHELL_HTML.to_string().into())
        .unwrap()
}

/// Generic command invocation proxy — allows the native Studio (and any
/// browser client) to call any Tauri command via REST.
///
/// POST /api/invoke/{cmd} with JSON body → command result as JSON
///
/// This dispatches through the webview's `on_message` IPC handler, which
/// routes to the same `generate_handler!` registry that the webview uses.
async fn invoke_command_handler(
    _state: State<SettingsServerState>,
    Path(cmd): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    use tauri::Manager;

    let app = APP_HANDLE.get();
    let Some(handle) = app else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "App handle not initialised",
        )
            .into_response();
    };

    // Get the main webview — commands are dispatched through it
    let webview = match handle.get_webview_window("main") {
        Some(w) => w,
        None => {
            return (StatusCode::SERVICE_UNAVAILABLE, "Main webview not found").into_response();
        }
    };

    // Build the IPC request — use the actual invoke key from the app handle
    let invoke_key = handle.invoke_key().to_string();
    let url = if cfg!(windows) || cfg!(target_os = "android") {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    };
    let request = tauri::webview::InvokeRequest {
        cmd: cmd.clone(),
        body: tauri::ipc::InvokeBody::Json(body),
        headers: Default::default(),
        url: url
            .parse()
            .unwrap_or_else(|_| "http://tauri.localhost".parse().unwrap()),
        invoke_key,
        callback: tauri::ipc::CallbackFn(0),
        error: tauri::ipc::CallbackFn(1),
    };

    // Dispatch through the webview's on_message handler.
    // Use a tokio oneshot channel so we can await the result without blocking
    // the async runtime — on_message schedules the callback on the event loop.
    let (tx, rx) = tokio::sync::oneshot::channel();
    webview.on_message(
        request,
        Box::new(move |_window, _cmd, response, _callback, _error| {
            let _ = tx.send(response);
        }),
    );

    // Await the result with a 10-second timeout
    let result = tokio::time::timeout(std::time::Duration::from_secs(10), rx).await;
    match result {
        Ok(Ok(tauri::ipc::InvokeResponse::Ok(body))) => {
            let json = match body {
                tauri::ipc::InvokeResponseBody::Json(s) => {
                    serde_json::from_str::<serde_json::Value>(&s).unwrap_or(serde_json::Value::Null)
                }
                tauri::ipc::InvokeResponseBody::Raw(bytes) => {
                    serde_json::from_slice::<serde_json::Value>(&bytes)
                        .unwrap_or(serde_json::Value::Null)
                }
            };
            Json(json).into_response()
        }
        Ok(Ok(tauri::ipc::InvokeResponse::Err(e))) => {
            let err_val = e.0;
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err_val)).into_response()
        }
        Ok(Err(_)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Command '{cmd}' channel closed"),
        )
            .into_response(),
        Err(_) => (
            StatusCode::REQUEST_TIMEOUT,
            format!("Command '{cmd}' timed out (10s)"),
        )
            .into_response(),
    }
}

pub static APP_HANDLE: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();

async fn status_handler(State(state): State<SettingsServerState>) -> Json<StatusResponse> {
    let config = state.app_state.config.lock().unwrap().clone();
    let graph_port = qualia_client_core::api::get_active_daemon_port();
    let (reachable, engine_version) = probe_graph_daemon(graph_port).await;
    let daemon_flag = *state.app_state.daemon_running.lock().unwrap();
    let settings_port = *state.listen_port.lock().unwrap();
    let jobs = LocalJobScheduler::global()
        .snapshot()
        .unwrap_or(JobQueueSnapshot {
            jobs: vec![],
            queued: 0,
            running: 0,
            completed: 0,
            failed: 0,
        });

    Json(StatusResponse {
        settings_port,
        graph_daemon_port: graph_port,
        graph_daemon_reachable: reachable,
        graph_engine_version: engine_version,
        qapps_protocol_port: qualia_client_core::qapps_protocol::qualia_protocol_port(),
        storage_path: config.storage_path,
        inference_backend: config.inference_backend,
        daemon_running_flag: daemon_flag,
        job_queue: JobQueueCounts {
            queued: jobs.queued,
            running: jobs.running,
            completed: jobs.completed,
            failed: jobs.failed,
        },
    })
}

async fn probe_graph_daemon(port: u16) -> (bool, Option<String>) {
    let url = format!("http://127.0.0.1:{port}/health");
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_millis(800))
        .build()
    else {
        return (false, None);
    };
    let Ok(res) = client.get(&url).send().await else {
        return (false, None);
    };
    if !res.status().is_success() {
        return (false, None);
    }
    let version = res.json::<serde_json::Value>().await.ok().and_then(|v| {
        v.get("engine_version")
            .and_then(|x| x.as_str())
            .map(str::to_string)
    });
    (true, version)
}

async fn get_config_handler(State(state): State<SettingsServerState>) -> Json<AgentConfig> {
    Json(state.app_state.config.lock().unwrap().clone())
}

async fn save_config_handler(
    Json(body): Json<AgentConfig>,
) -> Result<StatusCode, (StatusCode, String)> {
    qualia_client_core::api::save_config(body).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_manifest_handler(State(state): State<SettingsServerState>) -> impl IntoResponse {
    let body = state.manifest.lock().unwrap().clone();
    ([(header::CONTENT_TYPE, "application/json")], body)
}

async fn post_manifest_handler(
    State(state): State<SettingsServerState>,
    body: String,
) -> StatusCode {
    if body.trim().is_empty() {
        return StatusCode::BAD_REQUEST;
    }
    let storage_path = state.app_state.config.lock().unwrap().storage_path.clone();
    if let Err(err) = persist_manifest_to_disk(&storage_path, &body) {
        eprintln!("workspace manifest persist failed: {err}");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    match qualia_client_core::studio_workspace_wal::append_workspace_deploy(&storage_path, &body) {
        Ok(revision) => {
            if let Err(err) = qualia_client_core::studio_workspace_wal::persist_revision_snapshot(
                &storage_path,
                revision,
                &body,
            ) {
                eprintln!("studio revision snapshot failed: {err}");
            }
        }
        Err(err) => eprintln!("studio workspace WAL append failed: {err}"),
    }
    *state.manifest.lock().unwrap() = body;
    StatusCode::NO_CONTENT
}

async fn replay_manifest_handler(
    State(state): State<SettingsServerState>,
    Path(revision): Path<u64>,
) -> impl IntoResponse {
    let storage_path = state.app_state.config.lock().unwrap().storage_path.clone();
    match qualia_client_core::studio_workspace_wal::replay_workspace_manifest(
        &storage_path,
        revision,
    ) {
        Ok(body) => {
            if let Err(err) = persist_manifest_to_disk(&storage_path, &body) {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": err })),
                )
                    .into_response();
            }
            *state.manifest.lock().unwrap() = body.clone();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                body,
            )
                .into_response()
        }
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": err })),
        )
            .into_response(),
    }
}

async fn get_manifest_history_handler(
    State(state): State<SettingsServerState>,
) -> impl IntoResponse {
    let storage_path = state.app_state.config.lock().unwrap().storage_path.clone();
    match qualia_client_core::studio_workspace_wal::list_deploy_history(&storage_path) {
        Ok(records) => (StatusCode::OK, Json(records)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err })),
        )
            .into_response(),
    }
}

async fn system_telemetry_handler() -> Json<SystemTelemetry> {
    Json(crate::telemetry_hooks::collect_system_telemetry())
}

async fn list_jobs_handler() -> Result<Json<JobQueueSnapshot>, (StatusCode, String)> {
    LocalJobScheduler::global()
        .snapshot()
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}

async fn enqueue_job_handler(
    Json(body): Json<EnqueueJobRequest>,
) -> Result<(StatusCode, Json<LocalJob>), (StatusCode, String)> {
    LocalJobScheduler::global()
        .enqueue(body.kind)
        .map(|job| (StatusCode::ACCEPTED, Json(job)))
        .map_err(|e| (StatusCode::BAD_REQUEST, e))
}

async fn get_job_handler(Path(id): Path<String>) -> Result<Json<LocalJob>, (StatusCode, String)> {
    match LocalJobScheduler::global().get(&id) {
        Ok(Some(job)) => Ok(Json(job)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "job not found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

async fn cancel_job_handler(Path(id): Path<String>) -> Result<StatusCode, (StatusCode, String)> {
    match LocalJobScheduler::global().cancel(&id) {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            "job not found or not cancellable".to_string(),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

async fn telemetry_handler() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::unfold(0u64, |tick| async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let mut sys = sysinfo::System::new_all();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        let payload = serde_json::json!({
            "tick": tick,
            "cpu_percent": sys.global_cpu_usage(),
            "ram_gb": sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
            "ts": chrono::Utc::now().to_rfc3339(),
        });
        Some((Ok(Event::default().data(payload.to_string())), tick + 1))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[derive(Debug, Deserialize)]
struct SparqlProxyRequest {
    query: String,
    #[serde(default = "default_sparql_target")]
    target: String,
    endpoint: Option<String>,
}

fn default_sparql_target() -> String {
    "local".to_string()
}

#[derive(Serialize, Deserialize)]
struct SparqlEndpointInfo {
    id: String,
    name: String,
    endpoint: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    domains: Vec<String>,
}

#[derive(Serialize)]
struct SparqlEndpointsResponse {
    local_daemon_port: u16,
    local_daemon_reachable: bool,
    endpoints: Vec<SparqlEndpointInfo>,
}

const SPARQL_ENDPOINTS_JSON: &str = include_str!("../static/portal/sparql-endpoints.json");

async fn sparql_endpoints_handler() -> Result<Json<SparqlEndpointsResponse>, (StatusCode, String)> {
    let parsed: serde_json::Value = serde_json::from_str(SPARQL_ENDPOINTS_JSON)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let endpoints: Vec<SparqlEndpointInfo> = parsed
        .get("endpoints")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let port = qualia_client_core::api::get_active_daemon_port();
    let (reachable, _) = probe_graph_daemon(port).await;
    Ok(Json(SparqlEndpointsResponse {
        local_daemon_port: port,
        local_daemon_reachable: reachable,
        endpoints,
    }))
}

async fn sparql_query_handler(
    Json(body): Json<SparqlProxyRequest>,
) -> Result<(HeaderMap, String), (StatusCode, String)> {
    let query = body.query.trim();
    if query.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "empty query".to_string()));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let (status, content_type, text) = if body.target == "local" {
        let port = qualia_client_core::api::get_active_daemon_port();
        let url = format!("http://127.0.0.1:{port}/query");
        let res = client
            .post(&url)
            .header("Content-Type", "application/sparql-query")
            .header("Accept", "application/sparql-results+json")
            .body(query.to_string())
            .send()
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, format!("daemon unreachable: {e}")))?;
        let status = res.status();
        let ct = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/json")
            .to_string();
        let text = res
            .text()
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
        (status, ct, text)
    } else {
        let endpoint = body.endpoint.filter(|s| !s.trim().is_empty()).ok_or((
            StatusCode::BAD_REQUEST,
            "remote target requires endpoint".to_string(),
        ))?;
        let res = client
            .post(&endpoint)
            .header("Content-Type", "application/sparql-query")
            .header("Accept", "application/sparql-results+json")
            .body(query.to_string())
            .send()
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("remote SPARQL failed: {e}"),
                )
            })?;
        let status = res.status();
        let ct = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/json")
            .to_string();
        let text = res
            .text()
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
        (status, ct, text)
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        content_type.parse().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "bad content-type".to_string(),
            )
        })?,
    );
    if !status.is_success() {
        return Err((StatusCode::BAD_GATEWAY, text));
    }
    Ok((headers, text))
}

#[derive(Serialize)]
struct AssetCatalogSummary {
    llm_count: usize,
    ontology_count: usize,
    llms: Vec<AssetCatalogLlm>,
    ontologies: Vec<AssetCatalogOntology>,
}

#[derive(Serialize)]
struct AssetCatalogLlm {
    id: String,
    name: String,
    size_mb: Option<u32>,
    ram_estimate_mb: Option<u32>,
    recommended_for: Option<Vec<String>>,
}

#[derive(Serialize)]
struct AssetCatalogOntology {
    id: String,
    name: String,
    domain: Option<String>,
    size_estimate_mb: Option<f64>,
    tags: Option<Vec<String>>,
}

fn load_resource_catalog() -> Result<qualia_core_db::resource_catalog::ResourceCatalog, String> {
    qualia_core_db::resource_catalog::load_default().map_err(|e| format!("{e:?}"))
}

async fn assets_catalog_handler() -> Result<Json<AssetCatalogSummary>, (StatusCode, String)> {
    let cat = load_resource_catalog().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(AssetCatalogSummary {
        llm_count: cat.llms.len(),
        ontology_count: cat.ontologies.len(),
        llms: cat
            .llms
            .iter()
            .map(|l| AssetCatalogLlm {
                id: l.id.clone(),
                name: l.name.clone(),
                size_mb: l.size_mb,
                ram_estimate_mb: l.ram_estimate_mb.or(l.size_mb),
                recommended_for: l.recommended_for.clone(),
            })
            .collect(),
        ontologies: cat
            .ontologies
            .iter()
            .map(|o| AssetCatalogOntology {
                id: o.id.clone(),
                name: o.name.clone(),
                domain: o.domain.clone(),
                size_estimate_mb: o.size_estimate_mb,
                tags: o.tags.clone(),
            })
            .collect(),
    }))
}

#[derive(Debug, Deserialize)]
struct AssetsRecommendRequest {
    #[serde(default)]
    pub device: qualia_client_core::asset_recommendations::DeviceProfileInput,
    #[serde(default)]
    pub design: qualia_client_core::asset_recommendations::DesignContextInput,
}

async fn assets_recommend_handler(
    State(state): State<SettingsServerState>,
    Json(body): Json<AssetsRecommendRequest>,
) -> Result<
    Json<qualia_client_core::asset_recommendations::AssetRecommendationsResponse>,
    (StatusCode, String),
> {
    let cat = load_resource_catalog().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let device = qualia_client_core::asset_recommendations::device_profile_from_input(&body.device);
    let storage = state.app_state.config.lock().unwrap().storage_path.clone();
    let resp = qualia_client_core::asset_recommendations::recommend_assets(
        &cat,
        &device,
        &body.design,
        Some(std::path::Path::new(&storage)),
    );
    Ok(Json(resp))
}

#[derive(Debug, Deserialize)]
struct AssetsEnqueueRequest {
    pub kind: String,
    #[serde(default)]
    pub ontology_id: Option<String>,
}

async fn generate_pane_handler(
    Json(body): Json<qualia_client_core::studio_pane_generator::GeneratePaneRequest>,
) -> Result<Json<qualia_client_core::studio_pane_generator::PaneGenerationPlan>, (StatusCode, String)>
{
    let prompt = body.prompt.trim();
    if prompt.is_empty() && body.ontology_domain.as_deref().unwrap_or("").is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "prompt or ontology_domain required".to_string(),
        ));
    }
    let req = body;
    let plan = tokio::task::spawn_blocking(move || {
        qualia_client_core::studio_pane_llm::generate_panes_with_llm_or_fallback(&req)
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("generate join: {e}"),
        )
    })?;
    Ok(Json(plan))
}

#[derive(Deserialize)]
struct UndoFrameQuery {
    #[serde(default)]
    stack_index: u16,
}

async fn post_manifest_undo_frame_handler(
    State(state): State<SettingsServerState>,
    Query(query): Query<UndoFrameQuery>,
    body: String,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if body.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "empty undo frame".to_string()));
    }
    let storage_path = state.app_state.config.lock().unwrap().storage_path.clone();
    let stack_index = query.stack_index;
    let frame_seq = qualia_client_core::studio_workspace_wal::append_undo_frame(
        &storage_path,
        stack_index,
        &body,
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::json!({ "frame_seq": frame_seq })))
}

async fn get_manifest_undo_chain_handler(
    State(state): State<SettingsServerState>,
) -> impl IntoResponse {
    let storage_path = state.app_state.config.lock().unwrap().storage_path.clone();
    match qualia_client_core::studio_workspace_wal::recover_undo_chain_manifests(&storage_path) {
        Ok(manifests) => {
            let parsed: Vec<serde_json::Value> = manifests
                .iter()
                .filter_map(|m| serde_json::from_str(m).ok())
                .collect();
            (
                StatusCode::OK,
                Json(serde_json::json!({ "manifests": parsed })),
            )
                .into_response()
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err })),
        )
            .into_response(),
    }
}

async fn assets_enqueue_handler(
    Json(body): Json<AssetsEnqueueRequest>,
) -> Result<(StatusCode, Json<LocalJob>), (StatusCode, String)> {
    let kind = match body.kind.as_str() {
        "ontology_catalog_import" => {
            let id = body
                .ontology_id
                .filter(|s| !s.trim().is_empty())
                .ok_or((StatusCode::BAD_REQUEST, "ontology_id required".to_string()))?;
            LocalJobKind::OntologyCatalogImport { ontology_id: id }
        }
        "bundled_ontology_seed" => LocalJobKind::BundledOntologySeed {
            ontology_id: body.ontology_id,
        },
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("unsupported enqueue kind: {other}"),
            ))
        }
    };
    LocalJobScheduler::global()
        .enqueue(kind)
        .map(|job| (StatusCode::ACCEPTED, Json(job)))
        .map_err(|e| (StatusCode::BAD_REQUEST, e))
}

async fn companion_ws_route(
    State(state): State<SettingsServerState>,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> impl IntoResponse {
    crate::companion_gateway::companion_ws_upgrade(State(state.host_api), ws).await
}

async fn companion_ingest_route(
    State(state): State<SettingsServerState>,
    Json(bundle): Json<wellfare_core::companion_sync::CompanionHealthBundle>,
) -> Result<Json<crate::companion_gateway::IngestAck>, (StatusCode, String)> {
    let json =
        serde_json::to_string(&bundle).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    crate::companion_gateway::companion_ingest_post(State(state.host_api), json)
        .await
        .map_err(|(code, msg)| (code, msg))
}

async fn companion_pairing_route(
    State(state): State<SettingsServerState>,
) -> Json<crate::companion_gateway::CompanionPairingInfo> {
    let port = *state.listen_port.lock().unwrap();
    Json(crate::companion_gateway::companion_pairing_info(port))
}

async fn companion_qr_route(State(state): State<SettingsServerState>) -> impl IntoResponse {
    let port = *state.listen_port.lock().unwrap();
    let info = crate::companion_gateway::companion_pairing_info(port);
    let svg = crate::companion_gateway::companion_qr_svg(&info.ws_url);
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/svg+xml")],
        svg,
    )
}
