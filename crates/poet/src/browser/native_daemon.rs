//! Local Webizen / QualiaDB Native Daemon Discovery, IPC & Live Transport.
//!
//! Provides the communication bridge between the Poet frontend and the local
//! native Webizen daemon (`qualia-core-db` loopback server on ports 8000, 3030, 4242, 8080).
//!
//! Supports:
//! - **Native Acceleration Mode** (direct hardware pipelines, resident graph, heavy GGUF/GPU compute, signed WAL persistence).
//! - **Standalone / WASM Sandbox Mode** (in-browser WASM AST engine, local storage, mocked hardware).
//! - **Remote Query & Eval IPC** (`POST /query`, `POST /eval`, `POST /gazetteer`, `POST /render/preview`).
//! - **Live SSE Event Streams** (`GET /pulse/events` for pulse messages, `GET /tensor/events` for graph revisions).
//! - **Dynamic Honesty Elevation** (elevates `"present"`/`"partial"` to `"live"` on connection).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use std::cell::RefCell;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event, EventSource, HtmlElement, MessageEvent, RequestInit, RequestMode, Response};

pub const DEFAULT_CANDIDATE_PORTS: &[u16] = &[8000, 3030, 4242, 8080];

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

/// Response returned from rendering a scene preview on the native GPU host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeRenderResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub honesty: String,
    #[serde(default)]
    pub node_count: usize,
    #[serde(default)]
    pub edge_count: usize,
    #[serde(default)]
    pub face_count: usize,
    pub data_uri: Option<String>,
    pub diagnostic: Option<String>,
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

/// Get the base URL of the connected daemon (e.g. `http://127.0.0.1:8000`).
pub fn get_connected_daemon_url() -> Option<String> {
    match get_daemon_state() {
        DaemonConnectionState::Connected { url, .. } => Some(url),
        _ => None,
    }
}

/// Elevate honesty label to `"live"` if daemon is connected; otherwise preserve base label.
pub fn effective_honesty(base_honesty: &str) -> &'static str {
    if is_daemon_connected() {
        "live"
    } else {
        match base_honesty {
            "live" => "live",
            "present" => "present",
            "missing" => "missing",
            _ => "partial",
        }
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

                set_daemon_state(DaemonConnectionState::Connected {
                    url: url.clone(),
                    port,
                    engine: health.engine.unwrap_or_else(|| "qualia-core-db".into()),
                    version: health.version.unwrap_or_else(|| "0.0.35".into()),
                    graph_quin_count: health.graph_quin_count.unwrap_or(0),
                    dev_mode: health.dev_mode.unwrap_or(false),
                });

                // Attach realtime SSE streams for pulse and graph revisions
                init_daemon_event_streams(&url);
                return;
            }
        }

        web_sys::console::log_1(
            &"[Webizen Probe] No native daemon running on local ports (running in Standalone WASM mode)".into(),
        );

        set_daemon_state(DaemonConnectionState::Offline {
            candidate_ports: ports.to_vec(),
            reason: "Connection refused on candidate loopback ports".into(),
        });
    });
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
    let json_val = wasm_bindgen_futures::JsFuture::from(json_promise).await.ok()?;
    serde_wasm_bindgen::from_value(json_val).ok()
}

// ---------------------------------------------------------------------------
// Remote Execution Endpoints (HTTP / JSON-RPC)
// ---------------------------------------------------------------------------

/// Post a SPARQL or graph query to the connected daemon.
pub async fn daemon_query(query: &str) -> Result<String, String> {
    let base_url = get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let url = format!("{base_url}/query");

    let req_body = serde_json::to_string(&NativeQueryRequest {
        query: query.to_string(),
        format: Some("json".to_string()),
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
    let base_url = get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
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
    let base_url = get_connected_daemon_url().ok_or_else(|| "No native daemon connected".to_string())?;
    let url = format!("{base_url}/gazetteer");

    let req_body = serde_json::to_string(&NativeGazetteerRequest {
        source: source.to_string(),
    })
    .map_err(|e| e.to_string())?;

    let resp_str = post_json_string(&url, &req_body).await?;
    serde_json::from_str(&resp_str).map_err(|e| e.to_string())
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

    if !resp.ok() {
        return Err(format!("Daemon returned status code {}", resp.status()));
    }

    let text_promise = resp
        .text()
        .map_err(|e| format!("Failed to read response text: {e:?}"))?;

    let text_val = wasm_bindgen_futures::JsFuture::from(text_promise)
        .await
        .map_err(|e| format!("Failed to resolve response text: {e:?}"))?;

    text_val
        .as_string()
        .ok_or_else(|| "Response text was not a string".to_string())
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
                    if !pulse.topic.is_empty() {
                        web_sys::console::log_1(
                            &format!("[Pulse Stream] Received topic: '{}' (seq: {})", pulse.topic, pulse.seq).into(),
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
                            &format!("[Graph Stream] Revision advanced to {}", rev_event.revision).into(),
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
        for i in 0..list.length() {
            if let Some(el) = list.item(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                render_badge_content(&el);
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
            let _ = badge.set_attribute("title", "Probing local ports (8000, 3030, 4242, 8080) for native daemon...");
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
            version: "0.0.35".into(),
            graph_quin_count: 42000,
            dev_mode: true,
        });

        assert!(is_daemon_connected());
        assert_eq!(
            get_connected_daemon_url(),
            Some("http://127.0.0.1:8000".into())
        );
        assert_eq!(effective_honesty("partial"), "live");
        assert_eq!(effective_honesty("present"), "live");
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
