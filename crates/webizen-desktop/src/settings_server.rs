//! Loopback settings portal on `127.0.0.1:8080` (tray "Open Settings").
//!
//! Serves a browser-accessible control panel plus the HTTP surface that
//! `webizen-studio` expects (`/manifest`, `/telemetry`).

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use qualia_client_core::local_job_scheduler::{
    EnqueueJobRequest, JobQueueSnapshot, LocalJob, LocalJobKind, LocalJobScheduler,
};
use webizen_render::scene_contract::SystemTelemetry;
use futures_util::stream::{self, Stream};
use qualia_client_core::state::{AgentConfig, AppState};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tower_http::{cors::CorsLayer, services::ServeDir};

pub const DEFAULT_SETTINGS_PORT: u16 = 8080;

const EMPTY_MANIFEST: &str =
    r#"{"pages":[],"theme_tokens":{},"themes":[],"environment_theme":{},"app_theme":{}}"#;

#[derive(Clone)]
pub struct SettingsServerState {
    pub app_state: Arc<AppState>,
    pub manifest: Arc<Mutex<String>>,
    pub listen_port: Arc<Mutex<u16>>,
    pub static_root: PathBuf,
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

pub fn spawn_settings_server(app_state: Arc<AppState>) -> u16 {
    let port = find_open_port("127.0.0.1", DEFAULT_SETTINGS_PORT);
    let state = SettingsServerState {
        app_state,
        manifest: Arc::new(Mutex::new(EMPTY_MANIFEST.to_string())),
        listen_port: Arc::new(Mutex::new(port)),
        static_root: static_portal_dir(),
    };

    tauri::async_runtime::spawn(async move {
        if let Err(err) = run_settings_server(state, port).await {
            eprintln!("Settings portal failed: {err}");
        }
    });

    port
}

async fn run_settings_server(state: SettingsServerState, port: u16) -> Result<(), String> {
    let static_root = state.static_root.clone();
    if !static_root.is_dir() {
        return Err(format!(
            "Settings static dir missing: {}",
            static_root.display()
        ));
    }

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/api/status", get(status_handler))
        .route("/api/config", get(get_config_handler).post(save_config_handler))
        .route("/manifest", get(get_manifest_handler).post(post_manifest_handler))
        .route("/telemetry", get(telemetry_handler))
        .route("/api/jobs", get(list_jobs_handler).post(enqueue_job_handler))
        .route("/api/jobs/:id", get(get_job_handler))
        .route("/api/jobs/:id/cancel", post(cancel_job_handler))
        .route("/api/telemetry", get(system_telemetry_handler))
        .route("/api/sparql/endpoints", get(sparql_endpoints_handler))
        .route("/api/sparql/query", post(sparql_query_handler))
        .route("/api/assets/catalog", get(assets_catalog_handler))
        .route("/api/assets/recommend", post(assets_recommend_handler))
        .route("/api/assets/enqueue", post(assets_enqueue_handler))
        .nest_service("/", ServeDir::new(static_root).append_index_html_on_directories(true))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("bind 127.0.0.1:{port}: {e}"))?;
    println!("Qualia settings portal listening on http://127.0.0.1:{port}/");
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("settings server: {e}"))
}

async fn health_handler(State(state): State<SettingsServerState>) -> Json<HealthResponse> {
    let port = *state.listen_port.lock().unwrap();
    Json(HealthResponse {
        status: "ok",
        service: "qualia-settings-portal",
        port,
    })
}

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
    let version = res
        .json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|v| v.get("engine_version").and_then(|x| x.as_str()).map(str::to_string));
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
    (
        [(header::CONTENT_TYPE, "application/json")],
        body,
    )
}

async fn post_manifest_handler(
    State(state): State<SettingsServerState>,
    body: String,
) -> StatusCode {
    if body.trim().is_empty() {
        return StatusCode::BAD_REQUEST;
    }
    *state.manifest.lock().unwrap() = body;
    StatusCode::NO_CONTENT
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

async fn get_job_handler(
    Path(id): Path<String>,
) -> Result<Json<LocalJob>, (StatusCode, String)> {
    match LocalJobScheduler::global().get(&id) {
        Ok(Some(job)) => Ok(Json(job)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "job not found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

async fn cancel_job_handler(
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    match LocalJobScheduler::global().cancel(&id) {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((StatusCode::NOT_FOUND, "job not found or not cancellable".to_string())),
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
        Some((
            Ok(Event::default().data(payload.to_string())),
            tick + 1,
        ))
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
    let parsed: serde_json::Value =
        serde_json::from_str(SPARQL_ENDPOINTS_JSON).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
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
        let endpoint = body
            .endpoint
            .filter(|s| !s.trim().is_empty())
            .ok_or((StatusCode::BAD_REQUEST, "remote target requires endpoint".to_string()))?;
        let res = client
            .post(&endpoint)
            .header("Content-Type", "application/sparql-query")
            .header("Accept", "application/sparql-results+json")
            .body(query.to_string())
            .send()
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, format!("remote SPARQL failed: {e}")))?;
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
        content_type.parse().map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "bad content-type".to_string()))?,
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
) -> Result<Json<qualia_client_core::asset_recommendations::AssetRecommendationsResponse>, (StatusCode, String)>
{
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
