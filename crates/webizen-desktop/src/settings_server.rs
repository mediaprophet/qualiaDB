//! Loopback settings portal on `127.0.0.1:8080` (tray "Open Settings").
//!
//! Serves a browser-accessible control panel plus the HTTP surface that
//! `webizen-studio` expects (`/manifest`, `/telemetry`).

use axum::{
    extract::{OriginalUri, Path, Query, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
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
use tauri::Manager;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::{ServeDir, ServeFile},
};
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
    companion_port: u16,
    graph_daemon_port: u16,
    graph_daemon_reachable: bool,
    graph_engine_version: Option<String>,
    qapps_protocol_port: u16,
    storage_path: String,
    inference_backend: String,
    daemon_running_flag: bool,
    job_queue: JobQueueCounts,
    services: Vec<crate::supervisor::ServiceSnapshot>,
    operations: Vec<crate::supervisor::OperationSnapshot>,
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
    if let Ok(dir) = std::env::var("WEBIZEN_STATIC_PORTAL_DIR") {
        let path = PathBuf::from(dir);
        if path.is_dir() {
            return path;
        }
        crate::desktop_log::record(
            "warn",
            format!(
                "WEBIZEN_STATIC_PORTAL_DIR does not exist: {}",
                path.display()
            ),
        );
    }

    let dev_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static/portal");
    let mut candidates = Vec::new();
    if let Some(resource_dir) = APP_HANDLE
        .get()
        .and_then(|app| app.path().resource_dir().ok())
    {
        candidates.push(resource_dir.join("portal"));
        candidates.push(resource_dir.join("static").join("portal"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("portal"));
            candidates.push(dir.join("static").join("portal"));
            candidates.push(dir.join("portal-dist"));
        }
    }
    candidates.push(dev_dir.clone());

    candidates
        .into_iter()
        .find(|path| path.is_dir())
        .unwrap_or(dev_dir)
}

fn studio_dist_dir() -> PathBuf {
    if let Some(resource_dir) = APP_HANDLE
        .get()
        .and_then(|app| app.path().resource_dir().ok())
    {
        let bundled = resource_dir.join("studio-dist");
        if bundled.is_dir() {
            return bundled;
        }
    }
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

/// Documentation assets shared by the desktop Design Studio and GitHub Pages.
///
/// Release bundles place this tree at `portal-support`; development builds
/// read the checked-in `docs/` tree directly. This keeps the desktop page from
/// silently losing a newly imported module or WASM package.
fn portal_support_dir() -> PathBuf {
    let mut candidates = Vec::new();
    if let Some(resource_dir) = APP_HANDLE
        .get()
        .and_then(|app| app.path().resource_dir().ok())
    {
        candidates.push(resource_dir.join("portal-support"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("portal-support"));
        }
    }
    let docs_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .map(|root| root.join("docs"))
        .unwrap_or_else(|| PathBuf::from("../../docs"));
    candidates.push(docs_dir.clone());

    candidates
        .into_iter()
        .find(|path| path.is_dir())
        .unwrap_or(docs_dir)
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
    let configured_port = app_state
        .config
        .lock()
        .map(|config| config.settings_port)
        .unwrap_or(DEFAULT_SETTINGS_PORT);
    let port = if configured_port > 0 {
        // Try the user-specified port; if it's taken, fall back to auto-find
        if std::net::TcpListener::bind(("127.0.0.1", configured_port)).is_ok() {
            configured_port
        } else {
            eprintln!(
                "Settings port {} is in use, auto-finding an open port...",
                configured_port
            );
            find_open_port("127.0.0.1", DEFAULT_SETTINGS_PORT)
        }
    } else {
        find_open_port("127.0.0.1", DEFAULT_SETTINGS_PORT)
    };
    let storage_path = app_state
        .config
        .lock()
        .map(|config| config.storage_path.clone())
        .unwrap_or_else(|_| qualia_client_core::state::dirs_default_path());
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

    let companion_port = find_open_port("0.0.0.0", port.saturating_add(1));
    crate::companion_gateway::set_companion_listen_port(companion_port);

    let companion_host_api = host_api;
    tauri::async_runtime::spawn(async move {
        if let Err(err) = run_settings_server(state, port).await {
            if let Some(supervisor) = APP_HANDLE
                .get()
                .and_then(|app| app.try_state::<crate::supervisor::DesktopSupervisor>())
            {
                supervisor.service_failed("settings_api", err.clone());
            }
            crate::desktop_log::record("error", format!("Settings portal failed: {err}"));
        }
    });
    tauri::async_runtime::spawn(async move {
        if let Err(err) = run_companion_server(companion_host_api, companion_port).await {
            if let Some(supervisor) = APP_HANDLE
                .get()
                .and_then(|app| app.try_state::<crate::supervisor::DesktopSupervisor>())
            {
                supervisor.service_failed("companion_gateway", err.clone());
            }
            crate::desktop_log::record("error", format!("Companion gateway failed: {err}"));
        }
    });

    port
}

async fn run_settings_server(state: SettingsServerState, port: u16) -> Result<(), String> {
    let static_root = state.static_root.clone();
    let studio_root = studio_wasm_dir();
    let portal_support_root = portal_support_dir();
    if !static_root.is_dir() {
        crate::desktop_log::record(
            "error",
            format!(
                "Settings portal static dir missing; API server will still start: {}",
                static_root.display()
            ),
        );
    } else {
        crate::desktop_log::record(
            "info",
            format!("Settings portal static dir: {}", static_root.display()),
        );
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
        .route("/", get(portal_index_handler))
        .route("/dashboard", get(studio_index_handler))
        .route("/qapps", get(studio_index_handler))
        .route("/library", get(studio_index_handler))
        .route("/tools", get(studio_index_handler))
        .route("/nexus", get(studio_index_handler))
        .route("/communications", get(studio_index_handler))
        .route("/identity", get(studio_index_handler))
        .route("/agency", get(studio_index_handler))
        .route("/sanctuary", get(studio_index_handler))
        .route("/work", get(studio_index_handler))
        .route("/anatomy", get(studio_index_handler))
        .route("/clinical", get(studio_index_handler))
        .route("/chora", get(studio_index_handler))
        .route("/context-studio", get(studio_index_handler))
        .route("/qapp-studio", get(studio_index_handler))
        .route("/qapp-studio/{app_id}", get(studio_index_handler))
        .route("/10d-browser", get(studio_index_handler))
        .route("/gpu-viewport", get(studio_index_handler))
        .route("/render-preview", get(studio_index_handler))
        .route("/scene-interaction", get(studio_index_handler))
        .route("/about", get(studio_index_handler))
        .route("/settings", get(studio_index_handler))
        .route("/design-studio", get(design_studio_handler))
        .route("/design-studio.html", get(design_studio_handler))
        .route("/health", get(health_or_studio_handler))
        .route("/api/health", get(health_handler))
        .route("/shell", get(shell_handler))
        .route("/logs", get(studio_index_handler))
        .route("/desktop-logs", get(logs_page_handler))
        .route("/api/logs", get(logs_json_handler))
        .route("/api/logs/text", get(logs_text_handler))
        .route("/api/status", get(status_handler))
        // Installable remote Surface Controller (phone PWA) + view session API
        .route("/remote-controller", get(remote_controller_index))
        .route("/remote-controller/", get(remote_controller_index))
        .route("/remote-controller/{*path}", get(remote_controller_asset))
        .route("/api/view/session", get(view_session_handler))
        .route("/api/view/set_observer", post(view_set_observer_handler))
        .route("/api/view/morph", post(view_morph_handler))
        .route("/api/view/select_uri", post(view_select_uri_handler))
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
        // Multi-apparatus fleet: accept signed jobs + publish identity for peers
        .route("/api/fleet/jobs", post(fleet_accept_job_handler))
        .route("/api/fleet/identity", get(fleet_identity_handler))
        .route("/api/fleet/outbox/retry", post(fleet_retry_outbox_handler))
        .route("/api/telemetry", get(system_telemetry_handler))
        .route("/api/sparql/endpoints", get(sparql_endpoints_handler))
        .route("/api/sparql/query", post(sparql_query_handler))
        .route("/api/graph/verify", post(graph_verify_handler))
        .route("/api/q42/volumes", get(q42_volumes_handler))
        .route("/api/q42/inspect", post(q42_inspect_handler))
        .route("/api/q42/verify", post(q42_verify_handler))
        .route("/api/q42/magnet", post(q42_magnet_handler))
        .route("/api/q42/compact", post(q42_compact_handler))
        .route("/api/assets/catalog", get(assets_catalog_handler))
        .route("/api/assets/recommend", post(assets_recommend_handler))
        .route("/api/assets/enqueue", post(assets_enqueue_handler))
        .route("/generate_pane", post(generate_pane_handler))
        .route_service(
            "/portal.css",
            ServeFile::new(static_root.join("portal.css")),
        )
        .route_service("/portal.js", ServeFile::new(static_root.join("portal.js")))
        .route_service(
            "/settings.html",
            ServeFile::new(static_root.join("settings.html")),
        )
        .route_service("/menu.json", ServeFile::new(static_root.join("menu.json")))
        .route_service(
            "/coi-serviceworker.js",
            ServeFile::new(portal_support_root.join("coi-serviceworker.js")),
        )
        .nest_service("/resources", ServeDir::new(static_root.join("resources")))
        .nest_service(
            "/js",
            ServeDir::new(static_root.join("js"))
                .fallback(ServeDir::new(portal_support_root.join("js"))),
        )
        .nest_service(
            "/css",
            ServeDir::new(static_root.join("css"))
                .fallback(ServeDir::new(portal_support_root.join("css"))),
        )
        .nest_service(
            "/pkg/qualia",
            ServeDir::new(portal_support_root.join("pkg").join("qualia")),
        )
        // Studio WASM build — browser-accessible Studio UI at /studio/
        .nest_service("/assets", ServeDir::new(studio_root.join("assets")))
        .nest_service(
            "/studio",
            ServeDir::new(studio_root).append_index_html_on_directories(true),
        )
        .fallback(get(studio_spa_fallback))
        .layer(control_plane_cors(port))
        .with_state(state);

    // Bind all interfaces so a second machine on the LAN can deliver fleet jobs
    // to `/api/fleet/jobs` when control_base_url uses a non-loopback address.
    // Loopback clients continue to work via 127.0.0.1:{port}.
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("bind 0.0.0.0:{port}: {e}"))?;
    if let Some(supervisor) = APP_HANDLE
        .get()
        .and_then(|app| app.try_state::<crate::supervisor::DesktopSupervisor>())
    {
        supervisor.service_ready(
            "settings_api",
            format!("loopback control plane ready on 127.0.0.1:{port}"),
        );
    }
    crate::desktop_log::record(
        "info",
        format!("settings control plane listening on http://127.0.0.1:{port}/"),
    );
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("settings server: {e}"))
}

fn control_plane_cors(port: u16) -> CorsLayer {
    let mut origins = vec![
        HeaderValue::from_static("http://tauri.localhost"),
        HeaderValue::from_static("https://tauri.localhost"),
        HeaderValue::from_static("tauri://localhost"),
    ];
    for origin in [
        format!("http://127.0.0.1:{port}"),
        format!("http://localhost:{port}"),
    ] {
        if let Ok(value) = HeaderValue::from_str(&origin) {
            origins.push(value);
        }
    }

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::ACCEPT, header::CONTENT_TYPE])
}

async fn run_companion_server(
    host_api: crate::companion_gateway::HostApiHandle,
    port: u16,
) -> Result<(), String> {
    let app = Router::new()
        .route("/mobile/stream", get(companion_ws_route))
        .route(
            "/wellfair/companion/ingest",
            post(crate::companion_gateway::companion_ingest_post),
        )
        .route(
            "/mobile/qr",
            get(crate::companion_gateway::companion_qr_route),
        )
        .route(
            "/api/wellfair/companion/pairing",
            get(crate::companion_gateway::companion_pairing_route),
        )
        // LAN phone controller (installable PWA shell; same view_* session as desktop)
        .route("/remote-controller", get(remote_controller_index))
        .route("/remote-controller/", get(remote_controller_index))
        .route("/remote-controller/{*path}", get(remote_controller_asset))
        .route("/api/view/session", get(view_session_handler))
        .route("/api/view/set_observer", post(view_set_observer_handler))
        .route("/api/view/morph", post(view_morph_handler))
        .route("/api/view/select_uri", post(view_select_uri_handler))
        .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024))
        .with_state(host_api);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("bind companion gateway 0.0.0.0:{port}: {e}"))?;
    if let Some(supervisor) = APP_HANDLE
        .get()
        .and_then(|app| app.try_state::<crate::supervisor::DesktopSupervisor>())
    {
        supervisor.service_ready(
            "companion_gateway",
            format!("paired LAN gateway ready on port {port}"),
        );
    }
    crate::desktop_log::record(
        "info",
        format!("paired companion gateway listening on ws://0.0.0.0:{port}/mobile/stream"),
    );
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("companion gateway: {e}"))
}

fn remote_controller_file(path: &str) -> Option<(String, &'static str)> {
    use qualia_cooperative_core::qapp_package::{generate_remote_controller_pwa, PwaContent};
    let bundle = generate_remote_controller_pwa();
    let name = if path.is_empty() || path == "/" {
        "index.html"
    } else {
        path.trim_start_matches('/')
    };
    let file = bundle.get(name)?;
    let ctype = match name {
        "index.html" => "text/html; charset=utf-8",
        "manifest.webmanifest" => "application/manifest+json",
        "sw.js" | "app.js" => "application/javascript; charset=utf-8",
        "icon.svg" => "image/svg+xml",
        _ => "application/octet-stream",
    };
    let body = match &file.content {
        PwaContent::Text(s) => s.clone(),
        PwaContent::Bytes(b) => String::from_utf8_lossy(b).into_owned(),
    };
    Some((body, ctype))
}

async fn remote_controller_index() -> Response {
    match remote_controller_file("index.html") {
        Some((body, ctype)) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, ctype),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            body,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn remote_controller_asset(Path(path): Path<String>) -> Response {
    match remote_controller_file(&path) {
        Some((body, ctype)) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, ctype),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            body,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn view_session_handler() -> Response {
    match serde_json::to_string(&qualia_client_core::view_host::get_session_snapshot()) {
        Ok(body) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            body,
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct ViewStatusBody {
    status: String,
}

async fn view_set_observer_handler(Json(body): Json<ViewStatusBody>) -> Response {
    use qualia_client_core::view_host::{get_session_snapshot, parse_observer, set_observer};
    set_observer(parse_observer(&body.status));
    match serde_json::to_string(&get_session_snapshot()) {
        Ok(s) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            s,
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct ViewMorphBody {
    mode: String,
}

async fn view_morph_handler(Json(body): Json<ViewMorphBody>) -> Response {
    match qualia_client_core::view_host::morph_json(&body.mode) {
        Ok(v) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            v.to_string(),
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}

#[derive(Deserialize)]
struct ViewSelectUriBody {
    uri: String,
}

async fn view_select_uri_handler(Json(body): Json<ViewSelectUriBody>) -> Response {
    use qualia_client_core::view_host::{get_session_snapshot, select_entity_uri};
    select_entity_uri(&body.uri);
    match serde_json::to_string(&get_session_snapshot()) {
        Ok(s) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            s,
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn portal_index_handler(State(state): State<SettingsServerState>) -> Response {
    portal_file_response(&state.static_root, "index.html", "text/html; charset=utf-8").await
}

async fn design_studio_handler(State(state): State<SettingsServerState>) -> Response {
    portal_file_response(
        &state.static_root,
        "design-studio.html",
        "text/html; charset=utf-8",
    )
    .await
}

async fn studio_index_handler() -> Response {
    portal_file_response(&studio_wasm_dir(), "index.html", "text/html; charset=utf-8").await
}

fn accepts_html(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|part| part.trim().starts_with("text/html"))
        })
}

async fn health_or_studio_handler(
    State(state): State<SettingsServerState>,
    headers: HeaderMap,
) -> Response {
    if accepts_html(&headers) {
        studio_index_handler().await
    } else {
        health_handler(State(state)).await.into_response()
    }
}

async fn studio_spa_fallback(OriginalUri(uri): OriginalUri) -> Response {
    let last_segment = uri.path().rsplit('/').next().unwrap_or_default();
    if last_segment.contains('.') {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            format!("Webizen asset not found: {}", uri.path()),
        )
            .into_response();
    }
    studio_index_handler().await
}

async fn portal_file_response(
    root: &PathBuf,
    relative_path: &str,
    content_type: &'static str,
) -> Response {
    let path = root.join(relative_path);
    match tokio::fs::read(&path).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, content_type)], bytes).into_response(),
        Err(err) => {
            crate::desktop_log::record(
                "error",
                format!("failed to serve portal file {}: {err}", path.display()),
            );
            (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                format!("Portal asset not found: {}", path.display()),
            )
                .into_response()
        }
    }
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
    let (services, operations) = APP_HANDLE
        .get()
        .and_then(|app| app.try_state::<crate::supervisor::DesktopSupervisor>())
        .map(|supervisor| (supervisor.services(), supervisor.operations()))
        .unwrap_or_default();

    Json(StatusResponse {
        settings_port,
        companion_port: crate::companion_gateway::companion_listen_port(),
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
        services,
        operations,
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

async fn fleet_accept_job_handler(
    Json(envelope): Json<qualia_client_core::identity_plane::FleetJobEnvelope>,
) -> Result<(StatusCode, Json<LocalJob>), (StatusCode, String)> {
    match qualia_client_core::identity_plane::accept_fleet_job_envelope(envelope) {
        Ok(job) => Ok((StatusCode::ACCEPTED, Json(job))),
        Err(e) => Err((StatusCode::BAD_REQUEST, e)),
    }
}

async fn fleet_identity_handler(
) -> Result<Json<qualia_client_core::identity_plane::IdentityPlaneSnapshot>, (StatusCode, String)> {
    qualia_client_core::identity_plane::get_identity_plane()
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}

async fn fleet_retry_outbox_handler() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match qualia_client_core::identity_plane::fleet_jobs::retry_remote_outbox() {
        Ok(n) => Ok(Json(serde_json::json!({ "delivered": n }))),
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

#[derive(Debug, Deserialize)]
struct GraphProofRequest {
    source_path: String,
    q42_path: String,
    memory_mib: Option<u64>,
    temp_gib: Option<u64>,
}

async fn graph_verify_handler(
    Json(request): Json<GraphProofRequest>,
) -> Result<Json<qualia_core_db::graph_proof::GraphProofReport>, (StatusCode, String)> {
    let memory_mib = request.memory_mib.unwrap_or(32);
    let temp_gib = request.temp_gib.unwrap_or(32);
    let memory_limit_bytes = memory_mib
        .checked_mul(1024 * 1024)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or((
            StatusCode::BAD_REQUEST,
            "memory_mib is too large".to_string(),
        ))?;
    let temporary_byte_budget = temp_gib
        .checked_mul(1024 * 1024 * 1024)
        .ok_or((StatusCode::BAD_REQUEST, "temp_gib is too large".to_string()))?;
    let report = tokio::task::spawn_blocking(move || {
        qualia_core_db::graph_proof::prove_cli_ntriples_q42_equivalence(
            std::path::Path::new(&request.source_path),
            std::path::Path::new(&request.q42_path),
            qualia_core_db::graph_proof::GraphProofOptions {
                memory_limit_bytes,
                temporary_byte_budget,
            },
        )
    })
    .await
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("graph proof task: {error}"),
        )
    })?
    .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    Ok(Json(report))
}

#[derive(Debug, Deserialize)]
struct Q42PathRequest {
    path: String,
    level: Option<String>,
}

async fn q42_volumes_handler(
) -> Result<Json<qualia_client_core::api::Q42VolumeWorkspace>, (StatusCode, String)> {
    tokio::task::spawn_blocking(qualia_client_core::api::list_q42_volumes)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("q42 list task: {error}"),
            )
        })?
        .map(Json)
        .map_err(|error| (StatusCode::BAD_REQUEST, error))
}

async fn q42_inspect_handler(
    Json(request): Json<Q42PathRequest>,
) -> Result<Json<qualia_core_db::q42_volume::Q42InspectReport>, (StatusCode, String)> {
    tokio::task::spawn_blocking(move || {
        qualia_client_core::api::inspect_q42_volume(request.path)
    })
    .await
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("q42 inspect task: {error}"),
        )
    })?
    .map(Json)
    .map_err(|error| (StatusCode::BAD_REQUEST, error))
}

async fn q42_verify_handler(
    Json(request): Json<Q42PathRequest>,
) -> Result<Json<qualia_core_db::q42_volume::Q42VerifySetReport>, (StatusCode, String)> {
    tokio::task::spawn_blocking(move || {
        qualia_client_core::api::verify_q42_volume(request.path, request.level)
    })
    .await
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("q42 verify task: {error}"),
        )
    })?
    .map(Json)
    .map_err(|error| (StatusCode::BAD_REQUEST, error))
}

async fn q42_magnet_handler(
    Json(request): Json<Q42PathRequest>,
) -> Result<Json<qualia_client_core::api::Q42MagnetResult>, (StatusCode, String)> {
    tokio::task::spawn_blocking(move || {
        qualia_client_core::api::magnet_q42_volume(request.path)
    })
    .await
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("q42 magnet task: {error}"),
        )
    })?
    .map(Json)
    .map_err(|error| (StatusCode::BAD_REQUEST, error))
}

async fn q42_compact_handler(
    Json(request): Json<Q42PathRequest>,
) -> Result<Json<qualia_client_core::api::Q42CompactResult>, (StatusCode, String)> {
    tokio::task::spawn_blocking(move || {
        qualia_client_core::api::compact_q42_volume(request.path)
    })
    .await
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("q42 compact task: {error}"),
        )
    })?
    .map(Json)
    .map_err(|error| (StatusCode::BAD_REQUEST, error))
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
    State(host_api): State<crate::companion_gateway::HostApiHandle>,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> impl IntoResponse {
    crate::companion_gateway::companion_ws_upgrade(State(host_api), ws).await
}

#[cfg(test)]
mod ui_route_tests {
    use super::*;

    #[test]
    fn browser_navigation_accepts_html_but_api_probes_do_not() {
        let mut browser_headers = HeaderMap::new();
        browser_headers.insert(
            header::ACCEPT,
            HeaderValue::from_static("text/html,application/xhtml+xml"),
        );
        assert!(accepts_html(&browser_headers));

        let mut api_headers = HeaderMap::new();
        api_headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));
        assert!(!accepts_html(&api_headers));
    }

    #[test]
    fn design_studio_support_tree_contains_boot_assets() {
        let root = portal_support_dir();
        for relative in [
            "css/tailwind-built.css",
            "css/site-nav.css",
            "js/qualia-debug.js",
            "pkg/qualia/qualia.js",
            "pkg/qualia/qualia_bg.wasm",
            "coi-serviceworker.js",
        ] {
            assert!(
                root.join(relative).is_file(),
                "missing Design Studio support asset: {relative}"
            );
        }
    }

    #[test]
    fn design_studio_portal_contains_its_local_entry_assets() {
        let root = static_portal_dir();
        for relative in [
            "design-studio.html",
            "css/design-studio.css",
            "js/design-studio-app.js",
            "js/asset-recommendations.js",
        ] {
            assert!(
                root.join(relative).is_file(),
                "missing Design Studio portal asset: {relative}"
            );
        }
    }

    #[tokio::test]
    async fn spa_fallback_serves_pages_but_not_missing_assets() {
        let page = studio_spa_fallback(OriginalUri("/talk".parse().unwrap())).await;
        assert_eq!(page.status(), StatusCode::OK);

        let asset = studio_spa_fallback(OriginalUri("/missing.js".parse().unwrap())).await;
        assert_eq!(asset.status(), StatusCode::NOT_FOUND);
    }
}
