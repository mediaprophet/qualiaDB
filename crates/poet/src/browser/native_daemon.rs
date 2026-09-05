//! Local Webizen / QualiaDB Native Daemon Discovery, IPC & Live Transport.
//!
//! Provides the communication bridge between the Poet frontend and the local
//! native Webizen daemon (`qualia-core-db` loopback server on known loopback ports).
//!
//! Supports:
//! - **Native Execution Mode** (only the graph and typed capabilities exposed by the connected daemon).
//! - **Standalone / WASM Sandbox Mode** (browser UI and local storage; native-only work is disabled).
//! - **Remote Query & Eval IPC** (`POST /query`, `POST /eval`, `POST /invoke`, `POST /gazetteer`).
//! - **Live SSE Event Streams** (`GET /pulse/events` for pulse messages, `GET /tensor/events` for graph revisions).
//! - **Dynamic Honesty Elevation** (elevates `"present"`/`"partial"` to `"live"` on connection).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use std::{cell::RefCell, collections::BTreeSet};

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    Document, Element, Event, EventSource, HtmlElement, MessageEvent, RequestInit, RequestMode,
    Response,
};

pub const DEFAULT_CANDIDATE_PORTS: &[u16] = &[4242, 4243, 8000, 3030, 8080];

// ---------------------------------------------------------------------------
// DTOs & Models
// ---------------------------------------------------------------------------

/// Information returned by the native Webizen daemon's `/health` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonHealthResponse {
    pub status: String,
    pub engine: Option<String>,
    pub version: Option<String>,
    pub dev_mode: Option<bool>,
    pub graph_quin_count: Option<usize>,
    pub graph_revision: Option<u64>,
}

/// Request for executing a SPARQL or graph query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeQueryRequest {
    pub query: String,
    #[serde(default)]
    pub format: Option<String>,
}

/// A real-time pulse publication event streamed from `/pulse/events`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PulseEvent {
    #[serde(default)]
    pub seq: u64,
    #[serde(default)]
    pub topic: String,
    #[serde(default)]
    pub payload_summary: String,
    #[serde(default)]
    pub timestamp: u64,
}

/// A graph revision update event streamed from `/tensor/events`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GraphRevisionEvent {
    #[serde(default)]
    pub revision: u64,
}

/// Request body for VibeScript evaluation on the native daemon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeEvalRequest {
    pub source: String,
    pub as_cell: bool,
    pub function: Option<String>,
}

/// Response returned from evaluating VibeScript on the native daemon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeEvalResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub value: String,
    pub diagnostic: Option<String>,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub committed: usize,
    #[serde(default)]
    pub honesty: String,
}

/// Request body for invoking one registered POET capability on the daemon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeInvokeRequest {
    pub id: String,
    #[serde(default)]
    pub args: serde_json::Value,
}

/// Request body for analyzing text with the daemon NLP gazetteer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeGazetteerRequest {
    pub source: String,
}

/// Gazetteer entity match hit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GazetteerHitDto {
    pub surface: String,
    pub iri: String,
    pub kind: String,
}

/// Response returned from the daemon NLP gazetteer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeGazetteerResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub token_count: usize,
    #[serde(default)]
    pub sentence_count: usize,
    #[serde(default)]
    pub sealed: usize,
    #[serde(default)]
    pub hits: Vec<GazetteerHitDto>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeRenderRequest {
    pub kind: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeRenderResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub honesty: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub node_count: usize,
    #[serde(default)]
    pub edge_count: usize,
    #[serde(default)]
    pub face_count: usize,
    pub data_uri: Option<String>,
    pub diagnostic: Option<String>,
    #[serde(default)]
    pub contract: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeLibraryQueryRequest {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub section: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub purposes: Vec<String>,
    #[serde(default)]
    pub projects: Vec<String>,
    #[serde(default)]
    pub media_types: Vec<String>,
    #[serde(default)]
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeLibraryIngestRequest {
    pub uri: String,
    pub media_type: String,
    pub text: String,
    pub section: Option<String>,
    pub sensitivity: Option<String>,
    #[serde(default)]
    pub projects: Vec<String>,
    #[serde(default)]
    pub purposes: Vec<String>,
    pub occurred_at: Option<i64>,
    pub place_label: Option<String>,
    pub lat: Option<f32>,
    pub lon: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeLibraryResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub honesty: String,
    #[serde(default)]
    pub data: serde_json::Value,
    pub diagnostic: Option<String>,
    pub code: Option<String>,
}

pub type NativeRecordResponse = NativeLibraryResponse;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeLlmRequest {
    pub model_path: String,
    pub prompt: String,
    #[serde(default)]
    pub graph_context: String,
    #[serde(default)]
    pub agent_did: String,
    #[serde(default)]
    pub principal_did: String,
    #[serde(default = "default_llm_token_budget")]
    pub max_tokens: u32,
    #[serde(default)]
    pub library_projects: Vec<String>,
    #[serde(default)]
    pub library_context_supplied: bool,
}

fn default_llm_token_budget() -> u32 {
    256
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeLlmResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub honesty: String,
    #[serde(default)]
    pub assertion_status: String,
    #[serde(default)]
    pub agent_did: String,
    #[serde(default)]
    pub model_path: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub draft: String,
    #[serde(default)]
    pub tokens_generated: u32,
    #[serde(default)]
    pub inference_duration_ms: u64,
    #[serde(default)]
    pub provenance_hashes: Vec<u64>,
    #[serde(default)]
    pub context_hash: u64,
    #[serde(default)]
    pub context_supplied: bool,
    #[serde(default)]
    pub repaired: bool,
    #[serde(default)]
    pub checks: Vec<serde_json::Value>,
    pub diagnostic: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeLlmJobStartResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub honesty: String,
    #[serde(default)]
    pub job_id: String,
    #[serde(default)]
    pub events_path: String,
    pub diagnostic: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct NativeLlmJobEvent {
    #[serde(default)]
    pub seq: u64,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

#[derive(Serialize)]
struct NativeLlmCancelRequest<'a> {
    job_id: &'a str,
}

#[derive(Serialize)]
struct NativeLlmModelLifecycleRequest<'a> {
    model_path: &'a str,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeLlmModel {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub bytes: u64,
    #[serde(default)]
    pub resident: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeLlmModelData {
    #[serde(default)]
    pub models: Vec<NativeLlmModel>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeLlmModelsResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub honesty: String,
    #[serde(default)]
    pub data: NativeLlmModelData,
    pub diagnostic: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeRecordQueryRequest {
    pub family: String,
    #[serde(default)]
    pub query: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeRecordUpsertRequest {
    pub family: String,
    pub title: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeRecordDeleteRequest {
    pub family: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeCapabilityEntry {
    pub id: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub semantics: String,
    #[serde(default)]
    pub available: bool,
    #[serde(default)]
    pub requires_native: bool,
    #[serde(default)]
    pub transport: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub honesty: String,
    #[serde(default)]
    pub effect_class: String,
    #[serde(default)]
    pub arg_schema: serde_json::Value,
    #[serde(default)]
    pub return_schema: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeCapabilityNegotiation {
    #[serde(default)]
    pub protocol: String,
    #[serde(default)]
    pub execution_host: String,
    #[serde(default)]
    pub capabilities: Vec<NativeCapabilityEntry>,
}

/// Dynamic connection state of the local Webizen daemon.
#[derive(Debug, Clone, PartialEq)]
pub enum DaemonConnectionState {
    Unchecked,
    Probing,
    Connected {
        url: String,
        port: u16,
        engine: String,
        version: String,
        graph_quin_count: usize,
        dev_mode: bool,
    },
    Offline {
        candidate_ports: Vec<u16>,
        reason: String,
    },
}

thread_local! {
    static DAEMON_STATE: RefCell<DaemonConnectionState> = RefCell::new(DaemonConnectionState::Unchecked);
    static ACTIVE_PULSE_STREAM: RefCell<Option<EventSource>> = RefCell::new(None);
    static ACTIVE_TENSOR_STREAM: RefCell<Option<EventSource>> = RefCell::new(None);
    static ACTIVE_LLM_STREAM: RefCell<Option<EventSource>> = RefCell::new(None);
    static NATIVE_CAPABILITY_IDS: RefCell<BTreeSet<String>> = RefCell::new(BTreeSet::new());
}

// ---------------------------------------------------------------------------
// State & Honesty Queries
// ---------------------------------------------------------------------------

/// Get the current connection state of the native daemon.
pub fn get_daemon_state() -> DaemonConnectionState {
    DAEMON_STATE.with(|s| s.borrow().clone())
}

/// Check if the native Webizen daemon is currently connected and active.
pub fn is_daemon_connected() -> bool {
    matches!(get_daemon_state(), DaemonConnectionState::Connected { .. })
}

/// Whether the connected daemon advertised a concrete native invoke contract.
pub fn native_capability_available(id: &str) -> bool {
    NATIVE_CAPABILITY_IDS.with(|ids| ids.borrow().contains(id))
}

fn native_capability_prefix_available(prefix: &str) -> bool {
    NATIVE_CAPABILITY_IDS.with(|ids| ids.borrow().iter().any(|id| id.starts_with(prefix)))
}

/// Get the base URL of the connected daemon (e.g. `http://127.0.0.1:8000`).
pub fn get_connected_daemon_url() -> Option<String> {
    match get_daemon_state() {
        DaemonConnectionState::Connected { url, .. } => Some(url),
        _ => None,
    }
}

/// Preserve a surface's declared honesty independently of daemon connectivity.
///
/// A connected daemon makes particular operations available; it does not turn
/// static or prototype data into a live result.
pub fn effective_honesty(base_honesty: &str) -> &'static str {
    match base_honesty {
        "live" => "live",
        "present" => "present",
        "missing" => "missing",
        _ => "partial",
    }
}

/// Update the thread-local state and refresh all badge UI elements in the DOM.
fn set_daemon_state(state: DaemonConnectionState) {
    DAEMON_STATE.with(|s| {
        *s.borrow_mut() = state;
    });

    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            update_all_status_badges(&doc);
        }
    }
}

// ---------------------------------------------------------------------------
// Asynchronous Daemon Probing & Health Discovery
// ---------------------------------------------------------------------------

/// Asynchronously probe candidate loopback ports for a running Webizen daemon.
pub fn spawn_daemon_probe() {
    set_daemon_state(DaemonConnectionState::Probing);

    wasm_bindgen_futures::spawn_local(async {
        let ports = DEFAULT_CANDIDATE_PORTS;
        for &port in ports {
            let url = format!("http://127.0.0.1:{port}");
            let health_url = format!("{url}/health");

            if let Some(health) = fetch_daemon_health(&health_url).await {
                web_sys::console::log_1(
                    &format!(
                        "[Webizen Probe] Found running native daemon at {url} (engine: {}, quins: {})",
                        health.engine.as_deref().unwrap_or("qualia-core-db"),
                        health.graph_quin_count.unwrap_or(0)
                    )
                    .into(),
                );

                refresh_native_capabilities(&url).await;
                set_daemon_state(DaemonConnectionState::Connected {
                    url: url.clone(),
                    port,
                    engine: health.engine.unwrap_or_else(|| "qualia-core-db".into()),
                    version: health.version.unwrap_or_else(|| crate::CRATE_STAMP.into()),
                    graph_quin_count: health.graph_quin_count.unwrap_or(0),
                    dev_mode: health.dev_mode.unwrap_or(false),
                });

                // Attach realtime SSE streams for pulse and graph revisions
                init_daemon_event_streams(&url);
                super::bind_observer_from_daemon();
                return;
            }
        }

        web_sys::console::log_1(
            &"[Webizen Probe] No native daemon running on local ports (running in Standalone WASM mode)".into(),
        );

        NATIVE_CAPABILITY_IDS.with(|ids| ids.borrow_mut().clear());
        set_daemon_state(DaemonConnectionState::Offline {
            candidate_ports: ports.to_vec(),
            reason: "Connection refused on candidate loopback ports".into(),
        });
    });
}

async fn refresh_native_capabilities(base_url: &str) {
    let url = format!("{base_url}/vibe/capabilities");
    let capabilities = fetch_json::<NativeCapabilityNegotiation>(&url)
        .await
        .map(|document| {
            document
                .capabilities
                .into_iter()
                .filter(|entry| entry.available)
                .map(|entry| entry.id)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    NATIVE_CAPABILITY_IDS.with(|ids| *ids.borrow_mut() = capabilities);
}

async fn fetch_json<T: serde::de::DeserializeOwned>(url: &str) -> Option<T> {
    let window = web_sys::window()?;
    let response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url))
        .await
        .ok()?
        .dyn_into::<Response>()
        .ok()?;
    if !response.ok() {
        return None;
    }
    let value = wasm_bindgen_futures::JsFuture::from(response.json().ok()?)
        .await
        .ok()?;
    serde_wasm_bindgen::from_value(value).ok()
}

async fn fetch_daemon_health(health_url: &str) -> Option<DaemonHealthResponse> {
    let window = web_sys::window()?;
    let promise = window.fetch_with_str(health_url);
    let resp_val = wasm_bindgen_futures::JsFuture::from(promise).await.ok()?;
    let resp: Response = resp_val.dyn_into().ok()?;

    if !resp.ok() {
        return None;
    }

    let json_promise = resp.json().ok()?;
    let json_val = wasm_bindgen_futures::JsFuture::from(json_promise)
        .await
        .ok()?;
    serde_wasm_bindgen::from_value(json_val).ok()
}

// ---------------------------------------------------------------------------
// Remote Execution Endpoints (HTTP / JSON-RPC)
// ---------------------------------------------------------------------------

/// Post a SPARQL or graph query to the connected daemon.
pub async fn daemon_query(query: &str) -> Result<String, String> {
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let url = format!("{base_url}/query");

    let req_body = serde_json::to_string(&NativeQueryRequest {
        query: query.to_string(),
        format: Some("json-ld".to_string()),
    })
    .map_err(|e| e.to_string())?;

    post_json_string(&url, &req_body).await
}

/// Evaluate VibeScript source on the connected daemon.
pub async fn daemon_eval(
    source: &str,
    as_cell: bool,
    function: Option<String>,
) -> Result<NativeEvalResponse, String> {
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let url = format!("{base_url}/eval");

    let req_body = serde_json::to_string(&NativeEvalRequest {
        source: source.to_string(),
        as_cell,
        function,
    })
    .map_err(|e| e.to_string())?;

    let resp_str = post_json_string(&url, &req_body).await?;
    serde_json::from_str(&resp_str).map_err(|e| e.to_string())
}

/// Analyze text with the daemon NLP gazetteer.
pub async fn daemon_gazetteer(source: &str) -> Result<NativeGazetteerResponse, String> {
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let url = format!("{base_url}/gazetteer");

    let req_body = serde_json::to_string(&NativeGazetteerRequest {
        source: source.to_string(),
    })
    .map_err(|e| e.to_string())?;

    let resp_str = post_json_string(&url, &req_body).await?;
    serde_json::from_str(&resp_str).map_err(|e| e.to_string())
}

/// Invoke a registered native POET capability with JSON-compatible arguments.
pub async fn daemon_invoke(
    id: &str,
    args: serde_json::Value,
) -> Result<NativeEvalResponse, String> {
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let url = format!("{base_url}/invoke");
    let req_body = serde_json::to_string(&NativeInvokeRequest {
        id: id.to_string(),
        args,
    })
    .map_err(|error| error.to_string())?;

    let resp_str = post_json_string(&url, &req_body).await?;
    serde_json::from_str(&resp_str).map_err(|error| error.to_string())
}

/// Request a genuine offscreen PNG from the registered native renderer.
pub async fn daemon_render_preview(
    kind: &str,
    width: u32,
    height: u32,
) -> Result<NativeRenderResponse, String> {
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let url = format!("{base_url}/render/preview");
    let req_body = serde_json::to_string(&NativeRenderRequest {
        kind: kind.to_string(),
        width: Some(width),
        height: Some(height),
    })
    .map_err(|error| error.to_string())?;
    let resp_str = post_json_string(&url, &req_body).await?;
    serde_json::from_str(&resp_str).map_err(|error| error.to_string())
}

pub async fn daemon_capabilities() -> Result<NativeCapabilityNegotiation, String> {
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let response = get_json_string(&format!("{base_url}/vibe/capabilities")).await?;
    serde_json::from_str(&response).map_err(|error| error.to_string())
}

pub async fn daemon_library_stats() -> Result<NativeLibraryResponse, String> {
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let response = get_json_string(&format!("{base_url}/library/stats")).await?;
    serde_json::from_str(&response).map_err(|error| error.to_string())
}

pub async fn daemon_library_query(
    request: NativeLibraryQueryRequest,
) -> Result<NativeLibraryResponse, String> {
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let body = serde_json::to_string(&request).map_err(|error| error.to_string())?;
    let response = post_json_string(&format!("{base_url}/library/query"), &body).await?;
    serde_json::from_str(&response).map_err(|error| error.to_string())
}

pub async fn daemon_library_ingest(
    request: NativeLibraryIngestRequest,
) -> Result<NativeLibraryResponse, String> {
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let body = serde_json::to_string(&request).map_err(|error| error.to_string())?;
    let response = post_json_string(&format!("{base_url}/library/ingest"), &body).await?;
    serde_json::from_str(&response).map_err(|error| error.to_string())
}

pub async fn daemon_llm_generate(request: NativeLlmRequest) -> Result<NativeLlmResponse, String> {
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let body = serde_json::to_string(&request).map_err(|error| error.to_string())?;
    let response = post_json_string(&format!("{base_url}/llm/generate"), &body).await?;
    serde_json::from_str(&response).map_err(|error| error.to_string())
}

pub async fn daemon_llm_start(
    request: NativeLlmRequest,
) -> Result<NativeLlmJobStartResponse, String> {
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let body = serde_json::to_string(&request).map_err(|error| error.to_string())?;
    let response = post_json_string(&format!("{base_url}/llm/jobs/start"), &body).await?;
    serde_json::from_str(&response).map_err(|error| error.to_string())
}

pub async fn daemon_llm_cancel(job_id: &str) -> Result<NativeLibraryResponse, String> {
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let body = serde_json::to_string(&NativeLlmCancelRequest { job_id })
        .map_err(|error| error.to_string())?;
    let response = post_json_string(&format!("{base_url}/llm/jobs/cancel"), &body).await?;
    serde_json::from_str(&response).map_err(|error| error.to_string())
}

/// Open the retained event stream for one local-model job. A subsequent call
/// replaces the prior stream so a browser workspace cannot accidentally leak
/// background decoders.
pub fn open_llm_job_stream(
    job_id: &str,
    mut on_event: impl FnMut(NativeLlmJobEvent) + 'static,
) -> Result<(), String> {
    close_llm_job_stream();
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let event_source = EventSource::new(&format!("{base_url}/llm/jobs/events?job_id={job_id}"))
        .map_err(|error| format!("Could not open local-model event stream: {error:?}"))?;
    let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
        let Some(data) = event.data().as_string() else {
            return;
        };
        match serde_json::from_str::<NativeLlmJobEvent>(&data) {
            Ok(event) => on_event(event),
            Err(error) => web_sys::console::warn_1(
                &format!("Ignored malformed local-model event: {error}").into(),
            ),
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    event_source.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();
    ACTIVE_LLM_STREAM.with(|stream| *stream.borrow_mut() = Some(event_source));
    Ok(())
}

pub fn close_llm_job_stream() {
    ACTIVE_LLM_STREAM.with(|stream| {
        if let Some(event_source) = stream.borrow_mut().take() {
            event_source.close();
        }
    });
}

pub async fn daemon_llm_models() -> Result<NativeLlmModelsResponse, String> {
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let response = get_json_string(&format!("{base_url}/llm/models")).await?;
    serde_json::from_str(&response).map_err(|error| error.to_string())
}

pub async fn daemon_llm_activate(model_path: &str) -> Result<NativeLibraryResponse, String> {
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let body = serde_json::to_string(&NativeLlmModelLifecycleRequest { model_path })
        .map_err(|error| error.to_string())?;
    let response = post_json_string(&format!("{base_url}/llm/models/activate"), &body).await?;
    serde_json::from_str(&response).map_err(|error| error.to_string())
}

pub async fn daemon_llm_evict() -> Result<NativeLibraryResponse, String> {
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let response = post_json_string(&format!("{base_url}/llm/models/evict"), "{}").await?;
    serde_json::from_str(&response).map_err(|error| error.to_string())
}

pub async fn daemon_records_query(
    request: NativeRecordQueryRequest,
) -> Result<NativeRecordResponse, String> {
    post_record("/records/query", &request).await
}

pub async fn daemon_records_upsert(
    request: NativeRecordUpsertRequest,
) -> Result<NativeRecordResponse, String> {
    post_record("/records/upsert", &request).await
}

pub async fn daemon_records_delete(
    request: NativeRecordDeleteRequest,
) -> Result<NativeRecordResponse, String> {
    post_record("/records/delete", &request).await
}

async fn post_record<T: Serialize>(
    path: &str,
    request: &T,
) -> Result<NativeRecordResponse, String> {
    let base_url =
        get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let body = serde_json::to_string(request).map_err(|error| error.to_string())?;
    let response = post_json_string(&format!("{base_url}{path}"), &body).await?;
    serde_json::from_str(&response).map_err(|error| error.to_string())
}

async fn get_json_string(url: &str) -> Result<String, String> {
    let window = web_sys::window().ok_or_else(|| "Window object unavailable".to_string())?;
    let value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|error| format!("Fetch failed: {error:?}"))?;
    let response: Response = value
        .dyn_into()
        .map_err(|_| "Response is not a valid Response object".to_string())?;
    response_text(response).await
}

/// Helper to execute an HTTP POST with JSON body using web-sys `fetch`.
async fn post_json_string(url: &str, json_body: &str) -> Result<String, String> {
    let window = web_sys::window().ok_or_else(|| "Window object unavailable".to_string())?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&JsValue::from_str(json_body));

    let request = web_sys::Request::new_with_str_and_init(url, &opts)
        .map_err(|e| format!("Failed to create request: {e:?}"))?;

    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("Failed to set header: {e:?}"))?;

    let resp_val = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {e:?}"))?;

    let resp: Response = resp_val
        .dyn_into()
        .map_err(|_| "Response is not a valid Response object".to_string())?;

    response_text(resp).await
}

async fn response_text(response: Response) -> Result<String, String> {
    let ok = response.ok();
    let status = response.status();
    let text_promise = response
        .text()
        .map_err(|e| format!("Failed to read response text: {e:?}"))?;
    let text_val = wasm_bindgen_futures::JsFuture::from(text_promise)
        .await
        .map_err(|e| format!("Failed to resolve response text: {e:?}"))?;
    let response_body = text_val
        .as_string()
        .ok_or_else(|| "Daemon response body was not text".to_string())?;
    if !ok {
        return Err(format!(
            "Daemon returned status {}: {}",
            status, response_body
        ));
    }
    Ok(response_body)
}

// ---------------------------------------------------------------------------
// Live SSE Event Streams (`/pulse/events` & `/tensor/events`)
// ---------------------------------------------------------------------------

/// Initialize both SSE streams when connected to a daemon.
fn init_daemon_event_streams(base_url: &str) {
    let pulse_url = format!("{base_url}/pulse/events");
    let tensor_url = format!("{base_url}/tensor/events");

    // Close any prior event sources
    ACTIVE_PULSE_STREAM.with(|s| {
        if let Some(es) = s.borrow_mut().take() {
            let _ = es.close();
        }
    });
    ACTIVE_TENSOR_STREAM.with(|s| {
        if let Some(es) = s.borrow_mut().take() {
            let _ = es.close();
        }
    });

    // 1. Pulse Event Stream
    if let Ok(es) = EventSource::new(&pulse_url) {
        let on_message = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Some(data) = e.data().as_string() {
                if let Ok(pulse) = serde_json::from_str::<PulseEvent>(&data) {
                    super::pulse_stream::render_event(&pulse);
                    if !pulse.topic.is_empty() {
                        web_sys::console::log_1(
                            &format!(
                                "[Pulse Stream] Received topic: '{}' (seq: {})",
                                pulse.topic, pulse.seq
                            )
                            .into(),
                        );
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        es.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        on_message.forget();

        ACTIVE_PULSE_STREAM.with(|s| *s.borrow_mut() = Some(es));
    }

    // 2. Graph Revision Stream
    if let Ok(es) = EventSource::new(&tensor_url) {
        let on_message = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Some(data) = e.data().as_string() {
                if let Ok(rev_event) = serde_json::from_str::<GraphRevisionEvent>(&data) {
                    if rev_event.revision > 0 {
                        web_sys::console::log_1(
                            &format!("[Graph Stream] Revision advanced to {}", rev_event.revision)
                                .into(),
                        );
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        es.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        on_message.forget();

        ACTIVE_TENSOR_STREAM.with(|s| *s.borrow_mut() = Some(es));
    }
}

// ---------------------------------------------------------------------------
// DOM Badges & Status Indicators
// ---------------------------------------------------------------------------

/// Build an interactive Webizen Native status badge with click-to-probe.
pub fn build_daemon_status_badge(document: &Document) -> Element {
    let badge = document.create_element("span").unwrap();
    badge.set_class_name("webizen-native-status-badge");
    let b_el: HtmlElement = badge.clone().dyn_into().unwrap();
    b_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 10px; font-weight: 600; padding: 2px 7px; \
         border-radius: var(--radius-xs); cursor: pointer; user-select: none; transition: all 0.2s; \
         display: inline-flex; align-items: center; gap: 4px;",
    );

    render_badge_content(&badge);

    let click_closure = Closure::wrap(Box::new(move |_e: Event| {
        web_sys::console::log_1(&"[Webizen Probe] Manual probe requested by user".into());
        spawn_daemon_probe();
    }) as Box<dyn FnMut(Event)>);
    badge
        .add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())
        .unwrap();
    click_closure.forget();

    badge
}

/// Refresh all rendered `.webizen-native-status-badge` elements across the DOM.
pub fn update_all_status_badges(document: &Document) {
    if let Ok(list) = document.query_selector_all(".webizen-native-status-badge") {
    super::docks::refresh_bottom_statusbar_in_document(document);
        for i in 0..list.length() {
            if let Some(el) = list.item(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                render_badge_content(&el);
            }
        }
    }

    let connected = is_daemon_connected();
    if let Ok(list) = document.query_selector_all("[data-requires-daemon=true]") {
        for index in 0..list.length() {
            let Some(button) = list
                .item(index)
                .and_then(|node| node.dyn_into::<Element>().ok())
            else {
                continue;
            };
            let capability_available = button
                .get_attribute("data-capability-id")
                .map(|id| native_capability_available(&id))
                .or_else(|| {
                    button
                        .get_attribute("data-capability-prefix")
                        .map(|prefix| native_capability_prefix_available(&prefix))
                })
                .unwrap_or(true);
            if connected && capability_available {
                let _ = button.remove_attribute("disabled");
                let _ = button.set_attribute("aria-disabled", "false");
                let _ = button.remove_attribute("data-disabled-reason");
                if let Some(title) = button.get_attribute("data-enabled-title") {
                    let _ = button.set_attribute("title", &title);
                }
            } else {
                let reason = if connected {
                    "The connected daemon does not advertise this native capability contract."
                } else {
                    "Requires a running local QualiaDB daemon."
                };
                let _ = button.set_attribute("disabled", "");
                let _ = button.set_attribute("aria-disabled", "true");
                let _ = button.set_attribute("data-disabled-reason", reason);
                let title = button
                    .get_attribute("data-enabled-title")
                    .unwrap_or_default();
                let _ = button.set_attribute("title", &format!("{title} Unavailable: {reason}"));
            }
        }
    }
}

fn render_badge_content(badge: &Element) {
    let state = get_daemon_state();
    let b_el: Result<HtmlElement, _> = badge.clone().dyn_into();
    let Ok(b_el) = b_el else { return };

    match state {
        DaemonConnectionState::Unchecked | DaemonConnectionState::Probing => {
            b_el.style().set_css_text(
                "background: rgba(255, 184, 52, 0.12); color: var(--accent-amber, #ffb834); \
                 border: 1px solid rgba(255, 184, 52, 0.3); font-family: var(--font-mono); \
                 font-size: 10px; font-weight: 600; padding: 2px 7px; border-radius: var(--radius-xs); \
                 cursor: pointer; display: inline-flex; align-items: center; gap: 4px;",
            );
            badge.set_text_content(Some("\u{25CB} Probing Webizen\u{2026}"));
            let _ = badge.set_attribute(
                "title",
                "Probing known local ports for a native QualiaDB daemon...",
            );
        }
        DaemonConnectionState::Connected {
            port,
            graph_quin_count,
            ..
        } => {
            b_el.style().set_css_text(
                "background: rgba(0, 242, 169, 0.12); color: var(--accent-emerald, #00f2a9); \
                 border: 1px solid rgba(0, 242, 169, 0.3); font-family: var(--font-mono); \
                 font-size: 10px; font-weight: 600; padding: 2px 7px; border-radius: var(--radius-xs); \
                 cursor: pointer; display: inline-flex; align-items: center; gap: 4px;",
            );
            badge.set_text_content(Some(&format!("\u{25CF} Native: Connected (:{port})")));
            let _ = badge.set_attribute(
                "title",
                &format!(
                    "QualiaDB Native Daemon Active on port {port} · {graph_quin_count} Quins in graph. Click to re-probe."
                ),
            );
        }
        DaemonConnectionState::Offline { .. } => {
            b_el.style().set_css_text(
                "background: rgba(94, 115, 148, 0.15); color: var(--text-muted, #94a3b8); \
                 border: 1px solid var(--border-subtle, rgba(255,255,255,0.08)); font-family: var(--font-mono); \
                 font-size: 10px; font-weight: 600; padding: 2px 7px; border-radius: var(--radius-xs); \
                 cursor: pointer; display: inline-flex; align-items: center; gap: 4px;",
            );
            badge.set_text_content(Some("\u{25CB} Native: Standalone WASM"));
            let _ = badge.set_attribute(
                "title",
                "Local Webizen daemon offline. Running in in-browser WASM mode. Click to probe for running daemon.",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_state_default() {
        assert_eq!(get_daemon_state(), DaemonConnectionState::Unchecked);
        assert!(!is_daemon_connected());
    }

    #[test]
    fn test_daemon_state_connected_query() {
        set_daemon_state(DaemonConnectionState::Connected {
            url: "http://127.0.0.1:8000".into(),
            port: 8000,
            engine: "qualia-core-db".into(),
            version: crate::CRATE_STAMP.into(),
            graph_quin_count: 42000,
            dev_mode: true,
        });

        assert!(is_daemon_connected());
        assert_eq!(
            get_connected_daemon_url(),
            Some("http://127.0.0.1:8000".into())
        );
        assert_eq!(effective_honesty("partial"), "partial");
        assert_eq!(effective_honesty("present"), "present");
    }

    #[test]
    fn test_effective_honesty_offline() {
        set_daemon_state(DaemonConnectionState::Offline {
            candidate_ports: vec![8000, 3030],
            reason: "test offline".into(),
        });
        assert!(!is_daemon_connected());
        assert_eq!(effective_honesty("partial"), "partial");
        assert_eq!(effective_honesty("present"), "present");
        assert_eq!(effective_honesty("missing"), "missing");
        assert_eq!(effective_honesty("live"), "live");
    }

    #[test]
    fn test_pulse_event_deserialization() {
        let json = r#"{"seq":42,"topic":"audit:event","payload_summary":"test payload","timestamp":1700000000}"#;
        let event: PulseEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.seq, 42);
        assert_eq!(event.topic, "audit:event");
        assert_eq!(event.payload_summary, "test payload");
        assert_eq!(event.timestamp, 1700000000);
    }

    #[test]
    fn test_graph_revision_deserialization() {
        let json = r#"{"revision":1337}"#;
        let event: GraphRevisionEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.revision, 1337);
    }

    #[test]
    fn test_query_request_serialization() {
        let req = NativeQueryRequest {
            query: "SELECT ?s WHERE { ?s ?p ?o }".to_string(),
            format: Some("json".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("SELECT ?s"));
    }
}
